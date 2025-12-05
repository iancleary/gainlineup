use std::fs;
use std::path::Path;

use crate::block_from_touchstone_file_path_and_frequency_passive;
use crate::Block;

use serde::Deserialize;
use toml;

#[derive(Deserialize, Debug)]
struct Config {
    input_power: f64,
    frequency: f64,
    blocks: Vec<BlockConfig>,
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
    },
    Include {
        path: String,
    },
}

pub fn load_config(path: &str) -> Result<Vec<Block>, Box<dyn std::error::Error>> {
    println!("\n----------------------------\n");
    println!("Loading Config: {}", path);
    let config_content = fs::read_to_string(path)?;
    println!("Config Content: {}", config_content);
    let config: Config = toml::from_str(&config_content)?;
    println!("Config: {:#?}", config);

    let mut blocks = Vec::new();
    let config_path = Path::new(path);
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

    load_blocks_recursive(config.blocks, config.frequency, &mut blocks, base_dir)?;

    println!("\n----------------------------\n");
    Ok(blocks)
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
            BlockConfig::Touchstone { file_path } => {
                // Touchstone files might also be relative to the config file
                let full_path = base_dir.join(file_path);
                blocks.push(block_from_touchstone_file_path_and_frequency_passive(
                    full_path.to_string_lossy().to_string(),
                    frequency,
                ));
            }
            BlockConfig::Include { path } => {
                let included_path = base_dir.join(&path);
                println!("Loading Included Config: {}", included_path.display());
                let content = fs::read_to_string(&included_path)?;
                let included: IncludedConfig = toml::from_str(&content)?;

                let new_base_dir = included_path.parent().unwrap_or_else(|| Path::new("."));
                load_blocks_recursive(included.blocks, frequency, blocks, new_base_dir)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs;
    use std::path::Path;
    use toml;

    use crate::{cascade_vector_return_vector, SignalNode};

    #[test]
    fn test_load_simple_config() {
        let cwd = std::env::current_dir().unwrap();
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "tests/simple_config.toml".to_string());
        let full_path_to_config = cwd.join(config_path);
        let config_content = fs::read_to_string(full_path_to_config.display().to_string()).unwrap();
        let config: Config = toml::from_str(&config_content).unwrap();
        assert_eq!(config.input_power, -70.0);
        assert_eq!(config.frequency, 6.0e9);
        assert_eq!(config.blocks.len(), 3);
    }

    #[test]
    fn test_load_include_config() {
        let cwd = std::env::current_dir().unwrap();
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "tests/include_directive/config.toml".to_string());
        let full_path_to_config = cwd.join(config_path);
        let config_content = fs::read_to_string(full_path_to_config.display().to_string()).unwrap();
        let config: Config = toml::from_str(&config_content).unwrap();
        let mut blocks = Vec::new();
        let config_path = Path::new(&full_path_to_config);
        let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        load_blocks_recursive(config.blocks, config.frequency, &mut blocks, base_dir).unwrap();
        assert_eq!(blocks.len(), 6);
    }

    #[test]
    fn test_compression() {
        let cwd = std::env::current_dir().unwrap();
        let config_path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "tests/compression/compression_test.toml".to_string());
        let full_path_to_config = cwd.join(config_path);
        let config_content = fs::read_to_string(full_path_to_config.display().to_string()).unwrap();
        let config: Config = toml::from_str(&config_content).unwrap();
        let mut blocks = Vec::new();
        let config_path = Path::new(&full_path_to_config);
        let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
        load_blocks_recursive(config.blocks, config.frequency, &mut blocks, base_dir).unwrap();
        assert_eq!(blocks.len(), 3);

        let input_node = SignalNode {
            name: "Input".to_string(),
            power: config.input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0,
        };
        let cascade = cascade_vector_return_vector(input_node, blocks);

        assert_eq!(cascade.last().unwrap().power, 21.0);
    }
}
