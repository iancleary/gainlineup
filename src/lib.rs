// use std::fs;
// use std::path::Path;

// use touchstone::Network;

// use serde::Deserialize;

// pub mod cli;
// mod file_operations;
// mod open;
// mod plot;


// the input is independent of the blocks (e.g. a signal generator, what comes before the first block)
#[derive(Clone, Copy,Debug)]
pub struct Input {
    pub power: f64,
    pub frequency: f64,
    pub bandwidth: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct CumulativeParameters {
    pub gain: f64,
    pub noise_figure: f64, 
    pub noise_temperature: f64,
    // pub noise_spectral_density: f64,
    // pub signal_to_noise_ratio: f64,
}

impl CumulativeParameters {
    pub fn default() -> CumulativeParameters {
        CumulativeParameters {
            gain: 0.0,
            noise_figure: 0.0,
            noise_temperature: 290.0,
        }
    }
}

// the definition of a block in the cascade
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub gain: f64,                                 // dB
    pub noise_figure: f64, // dB, nf would be ambiguous between noise factor and noise figure
    pub output_p1db: Option<f64>, // dBm, compression point
}

// the cumulative parameters at the input and output of a block
#[derive(Clone, Debug)]
pub struct CascadedBlock {
    pub block: Block,
    pub input: Input, 
    pub cumulative_at_input: CumulativeParameters,
}



// nodes in the cascade, with the first node taking values from the Input struct and first Block struct
// the cumulative parameters are contained by parameters or functions of this struct
impl CascadedBlock {
    pub fn new(
        block: Block,
        input: Input,
        cumulative_at_input: CumulativeParameters, 
    ) -> CascadedBlock {

        // just instantiate the struct, since functions are helpful after the struct is created
        CascadedBlock {
            block,
            input,
            cumulative_at_input,
        }
    }

    // returns output power, handling compression point if present
    pub fn output_power(&self) -> f64 {
        let output_power_without_compression = self.input.power + self.block.gain;
        
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
        if let Some(op1db) = self.block.output_p1db {
            if output_power_without_compression > op1db + 1.0 {
                return op1db + 1.0;
            }
        }
        output_power_without_compression
    }

    pub fn gain_with_compression(&self) -> f64 {
        self.output_power() - self.input.power
    }

    pub fn noise_figure(&self) -> f64 {
        self.block.noise_figure
    }

    // pub fn output_noise_spectral_density(&self) -> f64 {
    //     // k in J/K, multiplied by 1000 for dBm
    //     let k_boltzmann = 1.380649e-23;
    //     let w_to_mw = 1000.0; // mW -> multiply W by 1000 to get mW count (dBW to dBm)
    //     let 
    //     let noise_spectral_density_linear = k_boltzmann * .noise_figure * w_to_mw;
    //     // convert linear mW to dBm
    //     10.0 * noise_spectral_density_linear.log10() // dBm/Hz
    // }

    // noise temperature is the noise temperature of the block itself
    pub fn input_noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.cumulative_at_input.noise_figure)
    }

    pub fn output_noise_temperature(&self) -> f64 {
        let input_noise_figure = self.cumulative_at_input.noise_figure;
        let input_noise_temperature = rfconversions::noise::noise_temperature_from_noise_figure(input_noise_figure);

        let block_noise_temperature = rfconversions::noise::noise_temperature_from_noise_figure(self.block.noise_figure);
        let block_gain = self.block.gain;

        let output_noise_temperature = input_noise_temperature + block_noise_temperature / rfconversions::power::db_to_linear(block_gain);
        output_noise_temperature
    }

    // cumulative noise figure is the noise figure at the output of the block
    pub fn output_noise_figure(&self) -> f64 {
        let cumulative_noise_temperature = self.output_noise_temperature();
        
        let cumulative_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(cumulative_noise_temperature);

        cumulative_noise_figure
    }

    // signal to noise ratio is the signal to noise ratio at the output of the block
    // pub fn output_signal_to_noise_ratio(&self, bandwidth: f64) -> f64 {
    //     self.output_signal_power() - self.noise_power(bandwidth)
    // }

    // pub fn noise_power(&self, bandwidth: f64) -> f64 {
    //     self.noise_spectral_density() + 10.0 * bandwidth.log10()
    // }
}


// the structure of the toml files
//
// Config is the top level toml file
//
// #[derive(Debug)]
// pub struct Config {
//     pub input_power: f64,
//     pub frequency: f64,
//     pub bandwidth: Option<f64>,
//     pub noise_temperature: Option<f64>,
//     pub blocks: Vec<Block>,
// }

// #[derive(Deserialize, Debug)]
// struct IncludedConfig {
//     blocks: Vec<BlockConfig>,
// }

// #[derive(Deserialize, Debug)]
// #[serde(tag = "type", rename_all = "snake_case")]
// enum BlockConfig {
//     Explicit {
//         name: String,
//         gain: f64,
//         noise_figure: f64,
//         output_p1db: Option<f64>,
//     },
//     Touchstone {
//         file_path: String,
//         name: String,
//         noise_figure: Option<f64>,
//         output_p1db: Option<f64>,
//     },
//     Include {
//         path: String,
//     },
// }

// pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
//     // println!("\n----------------------------\n");
//     // println!("Loading Config: {}", path);
//     let config_content = fs::read_to_string(path)?;
//     // println!("Config Content: {}", config_content);

//     // We need an intermediate struct to parse the TOML because Config now holds Vec<Block>
//     // but the TOML contains BlockConfigs
//     #[derive(Deserialize)]
//     struct IntermediateConfig {
//         input_power: f64,
//         frequency: f64,
//         bandwidth: Option<f64>,
//         noise_temperature: Option<f64>,
//         blocks: Vec<BlockConfig>,
//     }

//     let intermediate_config: IntermediateConfig = toml::from_str(&config_content)?;
//     // println!("Config: {:#?}", config);

//     let mut blocks = Vec::new();
//     let config_path = Path::new(path);
//     let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

//     load_blocks_recursive(
//         intermediate_config.blocks,
//         intermediate_config.frequency,
//         &mut blocks,
//         base_dir,
//     )?;

//     // println!("\n----------------------------\n");

//     Ok(Config {
//         input_power: intermediate_config.input_power,
//         frequency: intermediate_config.frequency,
//         bandwidth: intermediate_config.bandwidth,
//         noise_temperature: intermediate_config.noise_temperature,
//         blocks,
//     })
// }

// fn load_blocks_recursive(
//     block_configs: Vec<BlockConfig>,
//     frequency: f64,
//     blocks: &mut Vec<Block>,
//     base_dir: &Path,
// ) -> Result<(), Box<dyn std::error::Error>> {
//     for block_config in block_configs {
//         match block_config {
//             BlockConfig::Explicit {
//                 name,
//                 gain,
//                 noise_figure,
//                 output_p1db,
//             } => {
//                 blocks.push(Block {
//                     name,
//                     gain,
//                     noise_figure,
//                     output_p1db,
//                 });
//             }
//             BlockConfig::Touchstone {
//                 file_path,
//                 name,
//                 noise_figure,
//                 output_p1db,
//             } => {
//                 // Touchstone files might also be relative to the config file
//                 let full_path = base_dir.join(&file_path);
//                 let gain = touchstone_file_path_and_frequency_to_gain(
//                     full_path.to_string_lossy().to_string(),
//                     frequency,
//                 );

//                 let noise_figure_default = -gain; // only handles passives right now
//                 let output_p1db_default = 99.0; // 99 dBm

//                 let final_noise_figure = noise_figure.unwrap_or(noise_figure_default);
//                 let final_output_p1db = output_p1db.unwrap_or(output_p1db_default);

//                 blocks.push(Block {
//                     name,
//                     gain,
//                     noise_figure: final_noise_figure,
//                     output_p1db: final_output_p1db,
//                 });
//             }
//             BlockConfig::Include { path } => {
//                 let included_path = base_dir.join(&path);
//                 // println!("Loading Included Config: {}", included_path.display());
//                 let content = fs::read_to_string(&included_path)?;
//                 let included: IncludedConfig = toml::from_str(&content)?;

//                 let new_base_dir = included_path.parent().unwrap_or_else(|| Path::new("."));
//                 load_blocks_recursive(included.blocks, frequency, blocks, new_base_dir)?;
//             }
//         }
//     }
//     Ok(())
// }

// pub fn touchstone_file_path_and_frequency_to_gain(file_path: String, frequency_in_hz: f64) -> f64 {
//     let s2p = Network::new(file_path.clone());

//     let gain_vector = s2p.s_db(2, 1); // uses 1-based indexing

//     let gain = gain_vector
//         .iter()
//         .find(|frequency_db| frequency_db.frequency == frequency_in_hz)
//         .unwrap()
//         .s_db
//         .decibel();

//     gain
// }


// returns output signal node, handling compression point if present
pub fn cascade(input: Input, block: Block) -> CascadedBlock {

    let cumulative_at_input = CumulativeParameters::default();

    CascadedBlock::new(block, input, cumulative_at_input)
}

// // returns final output signal node, handling compression point if present
// pub fn cascade_vector_return_output(input_signal: SignalNode, blocks: Vec<Block>) -> SignalNode {
//     let mut cascading_signal = input_signal;

//     for block in blocks {
//         cascading_signal = cascade_node(cascading_signal, block);
//     }
//     cascading_signal
// }

// // returns vector of output signal nodes, handling compression point if present
// pub fn cascade_vector_return_vector(
//     input_signal: SignalNode,
//     blocks: Vec<Block>,
// ) -> Vec<SignalNode> {
//     let mut cascading_signal = input_signal;
//     let mut node_vector: Vec<SignalNode> = vec![cascading_signal.clone()];
//     for block in blocks.iter() {
//         cascading_signal = cascade_node(cascading_signal, block.clone());
//         node_vector.push(cascading_signal.clone());
//     }
//     node_vector
// }

// // This module contains tests for the cascade function and the Node struct

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn one_part() {
        let input_power: f64 = -30.0;
        let input = Input {
            power: input_power,
            frequency: 1.0,
            bandwidth: None,
        };
        let amplifier = Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_p1db: None,
        };
        let cascaded_block = cascade(input, amplifier);

        assert_eq!(cascaded_block.output_power(), -20.0);
    }

}



//     #[test]
//     fn one_part_new() {
//         let input_power: f64 = -30.0;
//         let name = "Simple Amplifier".to_string();
//         let gain = 10.0;
//         let noise_figure = 3.0;
//         let amplifier = super::Block::new(name, gain, noise_figure, None);
//         let output_power = super::cascade(input_power, amplifier);

//         assert_eq!(output_power, -20.0);
//     }

//     #[test]
//     fn one_part_node() {
//         let input_power: f64 = -30.0;
//         let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
//         let amplifier = super::Block {
//             name: "Simple Amplifier".to_string(),
//             gain: 10.0,
//             noise_figure: 3.0,
//             output_1db_compression_point: None,
//         };
//         let output_node = super::cascade_node(input_node, amplifier);

//         assert_eq!(output_node.signal_power, -20.0);
//         assert_eq!(output_node.name, "Simple Amplifier Output");
//         // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
//         let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
//             output_node.noise_temperature,
//         );
//         assert_eq!(output_noise_figure, 3.202456829285537);
//     }

//     #[test]
//     fn one_part_lna_node() {
//         let input_power: f64 = -30.0;
//         let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
//         let amplifier = super::Block {
//             name: "Low Noise Amplifier".to_string(),
//             gain: 30.0,
//             noise_figure: 3.0,
//             output_1db_compression_point: None,
//         };

//         let output_node = super::cascade_node(input_node, amplifier);

//         assert_eq!(output_node.signal_power, 0.0);
//         assert_eq!(output_node.name, "Low Noise Amplifier Output");
//         // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
//         let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
//             output_node.noise_temperature,
//         );
//         assert_eq!(output_noise_figure, 3.0124584457866126);
//     }

//     #[test]
//     fn two_part_node() {
//         let input_power: f64 = -30.0;
//         let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
//         let amplifier = super::Block {
//             name: "Low Noise Amplifier".to_string(),
//             gain: 30.0,
//             noise_figure: 3.0,
//             output_1db_compression_point: None,
//         };
//         let attenuator = super::Block {
//             name: "Attenuator".to_string(),
//             gain: -6.0,
//             noise_figure: 6.0,
//             output_1db_compression_point: None,
//         };
//         let intermediate_node = super::cascade_node(input_node, amplifier);

//         assert_eq!(intermediate_node.cumulative_gain.unwrap(), 30.0);

//         let output_node = super::cascade_node(intermediate_node, attenuator);

//         assert_eq!(output_node.signal_power, -6.0);
//         assert_eq!(output_node.cumulative_gain.unwrap(), 24.0);

//         assert_eq!(output_node.name, "Attenuator Output");
//         // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
//         let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
//             output_node.noise_temperature,
//         );
//         assert_eq!(output_noise_figure, 3.018922107070044);
//     }

//     #[test]
//     fn two_part_node_cascade_vector_return_output() {
//         let input_power: f64 = -30.0;
//         let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
//         let amplifier = super::Block {
//             name: "Low Noise Amplifier".to_string(),
//             gain: 30.0,
//             noise_figure: 3.0,
//             output_1db_compression_point: None,
//         };
//         let attenuator = super::Block {
//             name: "Attenuator".to_string(),
//             gain: -6.0,
//             noise_figure: 6.0,
//             output_1db_compression_point: None,
//         };
//         let blocks = vec![amplifier, attenuator];
//         let output_node = super::cascade_vector_return_output(input_node, blocks);

//         assert_eq!(output_node.signal_power, -6.0);
//         assert_eq!(output_node.cumulative_gain.unwrap(), 24.0);

//         assert_eq!(output_node.name, "Attenuator Output");
//         // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
//         let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
//             output_node.noise_temperature,
//         );
//         assert_eq!(output_noise_figure, 3.018922107070044);
//     }

//     #[test]
//     fn two_part_node_cascade_vector_return_vector() {
//         let input_power: f64 = -30.0;
//         let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
//         let amplifier = super::Block {
//             name: "Low Noise Amplifier".to_string(),
//             gain: 30.0,
//             noise_figure: 3.0,
//             output_1db_compression_point: None,
//         };
//         let attenuator = super::Block {
//             name: "Attenuator".to_string(),
//             gain: -6.0,
//             noise_figure: 6.0,
//             output_1db_compression_point: None,
//         };
//         let blocks = vec![amplifier, attenuator];
//         let cascade_vector = super::cascade_vector_return_vector(input_node, blocks);

//         let output_node = cascade_vector.last().unwrap();
//         assert_eq!(output_node.signal_power, -6.0);
//         assert_eq!(output_node.cumulative_gain.unwrap(), 24.0);

//         assert_eq!(output_node.name, "Attenuator Output");
//         // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
//         let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
//             output_node.noise_temperature,
//         );
//         assert_eq!(output_noise_figure, 3.018922107070044);
//     }

//     #[test]
//     fn two_part_node_cascade_vector_return_vector_with_compression() {
//         let input_power: f64 = -30.0;
//         let input_node = super::SignalNode::new("Input".to_string(), input_power, 290.0, Some(0.0));
//         let low_noise_amplifier = super::Block {
//             name: "Low Noise Amplifier".to_string(),
//             gain: 30.0,
//             noise_figure: 3.0,
//             output_1db_compression_point: Some(5.0),
//         };
//         let attenuator = super::Block {
//             name: "Attenuator".to_string(),
//             gain: -6.0,
//             noise_figure: 6.0,
//             output_1db_compression_point: None,
//         };
//         let high_power_amplifier = super::Block {
//             name: "High Power Amplifier".to_string(),
//             gain: 30.0,
//             noise_figure: 3.0,
//             output_1db_compression_point: Some(20.0),
//         };
//         let blocks = vec![low_noise_amplifier, attenuator, high_power_amplifier];
//         let cascade_vector = super::cascade_vector_return_vector(input_node, blocks);

//         let output_node = cascade_vector.last().unwrap();
//         assert_eq!(output_node.signal_power, 21.0);
//         assert_eq!(output_node.cumulative_gain.unwrap(), 51.0);

//         assert_eq!(output_node.name, "High Power Amplifier Output");
//         // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
//         let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
//             output_node.noise_temperature,
//         );
//         assert_eq!(output_noise_figure, 3.020645644372404);
//     }

//     use std::fs;
//     use std::path::Path;

//     use crate::{cascade_vector_return_vector, SignalNode};

//     // Helper to parse config for tests
//     fn parse_test_config(content: &str) -> Result<Config, Box<dyn std::error::Error>> {
//         #[derive(Deserialize)]
//         struct IntermediateConfig {
//             input_power: f64,
//             frequency: f64,
//             bandwidth: Option<f64>,
//             noise_temperature: Option<f64>,
//             blocks: Vec<BlockConfig>,
//         }
//         let intermediate_config: IntermediateConfig = toml::from_str(content)?;
//         let mut blocks = Vec::new();
//         // For tests, we assume base_dir is current dir or not important for explicit blocks
//         let base_dir = Path::new(".");
//         load_blocks_recursive(
//             intermediate_config.blocks,
//             intermediate_config.frequency,
//             &mut blocks,
//             base_dir,
//         )?;
//         Ok(Config {
//             input_power: intermediate_config.input_power,
//             frequency: intermediate_config.frequency,
//             bandwidth: intermediate_config.bandwidth,
//             noise_temperature: intermediate_config.noise_temperature,
//             blocks,
//         })
//     }

//     #[test]
//     fn test_load_simple_config() {
//         let cwd = std::env::current_dir().unwrap();
//         let config_path = std::env::args()
//             .nth(1)
//             .unwrap_or_else(|| "files/simple_config.toml".to_string());
//         let full_path_to_config = cwd.join(config_path);
//         let config_content = fs::read_to_string(full_path_to_config.display().to_string()).unwrap();
//         let config = parse_test_config(&config_content).unwrap();
//         assert_eq!(config.input_power, -70.0);
//         assert_eq!(config.frequency, 6.0e9);
//         assert_eq!(config.blocks.len(), 3);
//     }

//     #[test]
//     fn test_load_include_config() {
//         let cwd = std::env::current_dir().unwrap();
//         let config_path = std::env::args()
//             .nth(1)
//             .unwrap_or_else(|| "files/include_directive/config.toml".to_string());
//         let full_path_to_config = cwd.join(config_path);
//         // We need to use load_config here to handle includes correctly relative to file path
//         let config = load_config(&full_path_to_config.display().to_string()).unwrap();
//         assert_eq!(config.blocks.len(), 6);
//     }

//     #[test]
//     fn test_compression() {
//         let cwd = std::env::current_dir().unwrap();
//         let config_path = std::env::args()
//             .nth(1)
//             .unwrap_or_else(|| "files/compression/compression_test.toml".to_string());
//         let full_path_to_config = cwd.join(config_path);
//         // We need to use load_config here to handle includes correctly relative to file path
//         let config = load_config(&full_path_to_config.display().to_string()).unwrap();
//         assert_eq!(config.blocks.len(), 3);

//         let input_node = SignalNode::new("Input".to_string(), config.input_power, 290.0, Some(0.0));
//         let cascade = cascade_vector_return_vector(input_node, config.blocks);

//         assert_eq!(cascade.last().unwrap().signal_power, 21.0);
//     }

//     #[test]
//     fn test_touchstone_options() {
//         let cwd = std::env::current_dir().unwrap();
//         let config_path = std::env::args()
//             .nth(1)
//             .unwrap_or_else(|| "files/touchstone_options/config.toml".to_string());
//         let full_path_to_config = cwd.join(config_path);
//         // We need to use load_config here to handle includes correctly relative to file path
//         let config = load_config(&full_path_to_config.display().to_string()).unwrap();
//         assert_eq!(config.blocks.len(), 3);
//     }