use std::fs;
use std::path::Path;

use touchstone::Network;

use serde::Deserialize;

pub mod cli;
mod file_operations;
mod open;
mod plot;

// the input to our `create_user` handler
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub gain: f64,                                 // dB
    pub noise_figure: f64, // dB, nf would be ambiguous between noise factor and noise figure
    pub output_1db_compression_point: Option<f64>, // dBm
}

impl Block {
    pub fn new(
        name: String,
        gain: f64,
        noise_figure: f64,
        output_1db_compression_point: Option<f64>,
    ) -> Block {
        Block {
            name,
            gain,
            noise_figure,
            output_1db_compression_point,
        }
    }
}

// the input is independent of the blocks (e.g. a signal generator, what comes before the first block)
#[derive(Clone, Debug)]
pub struct Input {
    pub input_power: f64,
    pub frequency: f64,
    pub bandwidth: Option<f64>,
}

// nodes in the cascade, with the first node taking values from the Input struct and first Block struct
#[derive(Clone, Debug)]
pub struct CascadeBlock {
    pub name: String,
    pub input_signal_power: f64,            // dBm
    pub output_signal_power: f64,           // dBm
    pub gain: f64,                          // dB
    pub cumulative_gain: f64,               // cumulative, dB (set to 0 at start)
    pub noise_figure: f64,                  // dB
    pub cumulative_noise_figure: f64,       // cumulative, dB 
    pub output_1db_compression_point: Option<f64>, // dBm (defaults to None, which excludes any compression curve behavior)
}

impl CascadeBlock {
    pub fn new(
        name: String,
        block: Block,
        input_signal_power: f64,
        cumulative_gain_at_input: f64, // at output of block
        cumulative_noise_figure_at_input: f64, // at output of block
    ) -> CascadeBlock {
        CascadeBlock {
            name,
            input_signal_power,
            // output_signal_power,
            gain,
            noise_figure,
            input_cumulative_gain,
            // output_cumulative_gain, // 
            noise_figure,
            input_cumulative_noise_figure,
            // output_cumulative_noise_figure,
        }
    }

    // returns output power, handling compression point if present
    pub fn output_signal_power(&self) -> f64 {
        let output_power_without_compression = self.input_signal_power + self.block.gain;
        
        // this is a simple calculation, which could be upgrade to 
        // use the compression curve of a block, if present,
        // or the compression parameters from a piecewise amplifier model
        // where there is a linear region, a compression region, and a saturation region
        // those regions are defined by the compression point, a curve fit within te region of compression, and the saturation point
        // the curve fit is a simple linear fit between the compression point and the saturation point
        // the compression point is the point where the compression begins
        // the saturation point is the point where the compression ends
        // this probably would be a polynomial fit, but for initial documentation of the idea,
        // it's a simple linear fit
        if let Some(op1db) = self.block.output_1db_compression_point {
            if output_power_without_compression > op1db + 1.0 {
                return op1db + 1.0;
            }
        }
        output_power_without_compression
    }

    pub fn output_noise_spectral_density(&self) -> f64 {
        // k in J/K, multiplied by 1000 for dBm
        let k_boltzmann = 1.380649e-23;
        let w_to_mw = 1000.0; // mW -> multiply W by 1000 to get mW count (dBW to dBm)
        let noise_spectral_density_linear = k_boltzmann * self.noise_temperature * w_to_mw;
        // convert linear mW to dBm
        10.0 * noise_spectral_density_linear.log10() // dBm/Hz
    }

    // noise temperature is the noise temperature of the block itself
    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.noise_figure)
    }

    // cumulative noise figure is the noise figure at the output of the block
    pub fn cumulative_noise_figure(&self) -> f64 {
        let block_noise_temperature = self.noise_temperature();
        let cumulative_noise_temperature = block_noise_temperature + self.cumulative_noise_temperature;
        rfconversions::noise::noise_figure_from_noise_temperature(noise_temperature)
    }

    // signal to noise ratio is the signal to noise ratio at the output of the block
    pub fn output_signal_to_noise_ratio(&self, bandwidth: f64) -> f64 {
        self.output_signal_power - self.noise_power(bandwidth)
    }

    pub fn noise_power(&self, bandwidth: f64) -> f64 {
        self.noise_spectral_density() + 10.0 * bandwidth.log10()
    }
}

// the structure of the toml files
//
// Config is the top level toml file
//
#[derive(Debug)]
pub struct Config {
    pub input_power: f64,
    pub frequency: f64,
    pub bandwidth: Option<f64>,
    pub noise_temperature: Option<f64>,
    pub blocks: Vec<Block>,
}

#[derive(Deserialize, Debug)]
struct IncludedConfig {
    blocks: Vec<BlockConfig>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BlockConfig {
    Explicit {
        name: String,
        gain: f64,
        noise_figure: f64,
        output_1db_compression_point: Option<f64>,
    },
    Touchstone {
        file_path: String,
        name: String,
        noise_figure: Option<f64>,
        output_1db_compression_point: Option<f64>,
    },
    Include {
        path: String,
    },
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    // println!("\n----------------------------\n");
    // println!("Loading Config: {}", path);
    let config_content = fs::read_to_string(path)?;
    // println!("Config Content: {}", config_content);

    // We need an intermediate struct to parse the TOML because Config now holds Vec<Block>
    // but the TOML contains BlockConfigs
    #[derive(Deserialize)]
    struct IntermediateConfig {
        input_power: f64,
        frequency: f64,
        bandwidth: Option<f64>,
        noise_temperature: Option<f64>,
        blocks: Vec<BlockConfig>,
    }

    let intermediate_config: IntermediateConfig = toml::from_str(&config_content)?;
    // println!("Config: {:#?}", config);

    let mut blocks = Vec::new();
    let config_path = Path::new(path);
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

    load_blocks_recursive(
        intermediate_config.blocks,
        intermediate_config.frequency,
        &mut blocks,
        base_dir,
    )?;

    // println!("\n----------------------------\n");

    Ok(Config {
        input_power: intermediate_config.input_power,
        frequency: intermediate_config.frequency,
        bandwidth: intermediate_config.bandwidth,
        noise_temperature: intermediate_config.noise_temperature,
        blocks,
    })
}

fn load_blocks_recursive(
    block_configs: Vec<BlockConfig>,
    frequency: f64,
    blocks: &mut Vec<Block>,
    base_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    for block_config in block_configs {
        match block_config {
            BlockConfig::Explicit {
                name,
                gain,
                noise_figure,
                output_1db_compression_point,
            } => {
                blocks.push(Block {
                    name,
                    gain,
                    noise_figure,
                    output_1db_compression_point,
                });
            }
            BlockConfig::Touchstone {
                file_path,
                name,
                noise_figure,
                output_1db_compression_point,
            } => {
                // Touchstone files might also be relative to the config file
                let full_path = base_dir.join(&file_path);
                let gain = touchstone_file_path_and_frequency_to_gain(
                    full_path.to_string_lossy().to_string(),
                    frequency,
                );

                let noise_figure_default = -gain; // only handles passives right now
                let output_1db_compression_point_default = 99.0; // 99 dBm

                let final_noise_figure = noise_figure.unwrap_or(noise_figure_default);
                let final_output_1db_compression_point =
                    output_1db_compression_point.or(Some(output_1db_compression_point_default));

                blocks.push(Block {
                    name,
                    gain,
                    noise_figure: final_noise_figure,
                    output_1db_compression_point: final_output_1db_compression_point,
                });
            }
            BlockConfig::Include { path } => {
                let included_path = base_dir.join(&path);
                // println!("Loading Included Config: {}", included_path.display());
                let content = fs::read_to_string(&included_path)?;
                let included: IncludedConfig = toml::from_str(&content)?;

                let new_base_dir = included_path.parent().unwrap_or_else(|| Path::new("."));
                load_blocks_recursive(included.blocks, frequency, blocks, new_base_dir)?;
            }
        }
    }
    Ok(())
}

pub fn touchstone_file_path_and_frequency_to_gain(file_path: String, frequency_in_hz: f64) -> f64 {
    let s2p = Network::new(file_path.clone());

    let gain_vector = s2p.s_db(2, 1); // uses 1-based indexing

    let gain = gain_vector
        .iter()
        .find(|frequency_db| frequency_db.frequency == frequency_in_hz)
        .unwrap()
        .s_db
        .decibel();

    gain
}


// returns output signal node, handling compression point if present
pub fn cascade_node(signal: SignalNode, block1: Block) -> SignalNode {
    let output_node_name = block1.name + " Output";
    let block_noise_temperature =
        rfconversions::noise::noise_temperature_from_noise_figure(block1.noise_figure);

    // handle compression point (simple approach with P1dB)
    let output_power_without_compression = signal.signal_power + block1.gain;
    let output_power = if let Some(op1db) = block1.output_1db_compression_point {
        if output_power_without_compression > op1db + 1.0 {
            op1db + 1.0
        } else {
            output_power_without_compression
        }
    } else {
        output_power_without_compression
    };

    let stage_gain = output_power - signal.signal_power;

    let output_noise_temperature = if signal.cumulative_gain.is_some() {
        // only benefits from gain before the block

        let cumulative_gain_linear =
            rfconversions::power::db_to_linear(signal.cumulative_gain.unwrap());
        signal.noise_temperature + block_noise_temperature / cumulative_gain_linear
    } else {
        // only benefits from gain before the block
        signal.noise_temperature + block_noise_temperature
    };

    let cumulative_gain = if signal.cumulative_gain.is_some() {
        signal.cumulative_gain.unwrap() + stage_gain
    } else {
        stage_gain
    };

    SignalNode {
        name: output_node_name,
        signal_power: output_power,
        noise_temperature: output_noise_temperature,
        cumulative_gain: Some(cumulative_gain),
    }
}

// returns final output signal node, handling compression point if present
pub fn cascade_vector_return_output(input_signal: SignalNode, blocks: Vec<Block>) -> SignalNode {
    let mut cascading_signal = input_signal;

    for block in blocks {
        cascading_signal = cascade_node(cascading_signal, block);
    }
    cascading_signal
}

// returns vector of output signal nodes, handling compression point if present
pub fn cascade_vector_return_vector(
    input_signal: SignalNode,
    blocks: Vec<Block>,
) -> Vec<SignalNode> {
    let mut cascading_signal = input_signal;
    let mut node_vector: Vec<SignalNode> = vec![cascading_signal.clone()];
    for block in blocks.iter() {
        cascading_signal = cascade_node(cascading_signal, block.clone());
        node_vector.push(cascading_signal.clone());
    }
    node_vector
}

// This module contains tests for the cascade function and the Node struct

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn one_part() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let output_power = super::cascade(input_power, amplifier);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn one_part_new() {
        let input_power: f64 = -30.0;
        let name = "Simple Amplifier".to_string();
        let gain = 10.0;
        let noise_figure = 3.0;
        let amplifier = super::Block::new(name, gain, noise_figure, None);
        let output_power = super::cascade(input_power, amplifier);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn one_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let output_node = super::cascade_node(input_node, amplifier);

        assert_eq!(output_node.signal_power, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.202456829285537);
    }

    #[test]
    fn one_part_lna_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };

        let output_node = super::cascade_node(input_node, amplifier);

        assert_eq!(output_node.signal_power, 0.0);
        assert_eq!(output_node.name, "Low Noise Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.0124584457866126);
    }

    #[test]
    fn two_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_1db_compression_point: None,
        };
        let intermediate_node = super::cascade_node(input_node, amplifier);

        assert_eq!(intermediate_node.cumulative_gain.unwrap(), 30.0);

        let output_node = super::cascade_node(intermediate_node, attenuator);

        assert_eq!(output_node.signal_power, -6.0);
        assert_eq!(output_node.cumulative_gain.unwrap(), 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.018922107070044);
    }

    #[test]
    fn two_part_node_cascade_vector_return_output() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_1db_compression_point: None,
        };
        let blocks = vec![amplifier, attenuator];
        let output_node = super::cascade_vector_return_output(input_node, blocks);

        assert_eq!(output_node.signal_power, -6.0);
        assert_eq!(output_node.cumulative_gain.unwrap(), 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.018922107070044);
    }

    #[test]
    fn two_part_node_cascade_vector_return_vector() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_1db_compression_point: None,
        };
        let blocks = vec![amplifier, attenuator];
        let cascade_vector = super::cascade_vector_return_vector(input_node, blocks);

        let output_node = cascade_vector.last().unwrap();
        assert_eq!(output_node.signal_power, -6.0);
        assert_eq!(output_node.cumulative_gain.unwrap(), 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.018922107070044);
    }

    #[test]
    fn two_part_node_cascade_vector_return_vector_with_compression() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
        let low_noise_amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: Some(5.0),
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_1db_compression_point: None,
        };
        let high_power_amplifier = super::Block {
            name: "High Power Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: Some(20.0),
        };
        let blocks = vec![low_noise_amplifier, attenuator, high_power_amplifier];
        let cascade_vector = super::cascade_vector_return_vector(input_node, blocks);

        let output_node = cascade_vector.last().unwrap();
        assert_eq!(output_node.signal_power, 21.0);
        assert_eq!(output_node.cumulative_gain.unwrap(), 51.0);

        assert_eq!(output_node.name, "High Power Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.020645644372404);
    }

    use std::fs;
    use std::path::Path;

    use crate::{cascade_vector_return_vector, SignalNode};

    // Helper to parse config for tests
    fn parse_test_config(content: &str) -> Result<Config, Box<dyn std::error::Error>> {
        #[derive(Deserialize)]
        struct IntermediateConfig {
            input_power: f64,
            frequency: f64,
            bandwidth: Option<f64>,
            noise_temperature: Option<f64>,
            blocks: Vec<BlockConfig>,
        }
        let intermediate_config: IntermediateConfig = toml::from_str(content)?;
        let mut blocks = Vec::new();
        // For tests, we assume base_dir is current dir or not important for explicit blocks
        let base_dir = Path::new(".");
        load_blocks_recursive(
            intermediate_config.blocks,
            intermediate_config.frequency,
            &mut blocks,
            base_dir,
        )?;
        Ok(Config {
            input_power: intermediate_config.input_power,
            frequency: intermediate_config.frequency,
            bandwidth: intermediate_config.bandwidth,
            noise_temperature: intermediate_config.noise_temperature,
            blocks,
        })
    }

    #[test]
    fn test_load_simple_config() {
        let cwd = std::env::current_dir().unwrap();
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "files/simple_config.toml".to_string());
        let full_path_to_config = cwd.join(config_path);
        let config_content = fs::read_to_string(full_path_to_config.display().to_string()).unwrap();
        let config = parse_test_config(&config_content).unwrap();
        assert_eq!(config.input_power, -70.0);
        assert_eq!(config.frequency, 6.0e9);
        assert_eq!(config.blocks.len(), 3);
    }

    #[test]
    fn test_load_include_config() {
        let cwd = std::env::current_dir().unwrap();
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "files/include_directive/config.toml".to_string());
        let full_path_to_config = cwd.join(config_path);
        // We need to use load_config here to handle includes correctly relative to file path
        let config = load_config(&full_path_to_config.display().to_string()).unwrap();
        assert_eq!(config.blocks.len(), 6);
    }

    #[test]
    fn test_compression() {
        let cwd = std::env::current_dir().unwrap();
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "files/compression/compression_test.toml".to_string());
        let full_path_to_config = cwd.join(config_path);
        // We need to use load_config here to handle includes correctly relative to file path
        let config = load_config(&full_path_to_config.display().to_string()).unwrap();
        assert_eq!(config.blocks.len(), 3);

        let input_node = SignalNode::new("Input".to_string(), config.input_power, 290.0, Some(0.0));
        let cascade = cascade_vector_return_vector(input_node, config.blocks);

        assert_eq!(cascade.last().unwrap().signal_power, 21.0);
    }

    #[test]
    fn test_touchstone_options() {
        let cwd = std::env::current_dir().unwrap();
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "files/touchstone_options/config.toml".to_string());
        let full_path_to_config = cwd.join(config_path);
        // We need to use load_config here to handle includes correctly relative to file path
        let config = load_config(&full_path_to_config.display().to_string()).unwrap();
        assert_eq!(config.blocks.len(), 3);
    }
}
