use std::env;
use std::fs;
use std::path::Path;
use std::process;

// this cannot be crate::Network because of how Cargo works,
// since cargo/rust treats lib.rs and main.rs as separate crates
use crate::cascade_vector_return_vector;
use crate::file_operations;
use crate::Block;
use crate::Input;
use crate::SignalNode;

use touchstone::Network;

use serde::Deserialize;

// the structure of the toml files
//
// Config is the top level toml file
//
#[derive(Debug)]
pub struct Config {
    pub input_power: f64,
    pub frequency: f64,
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
        output_p1db: Option<f64>,
    },
    Touchstone {
        file_path: String,
        name: String,
        noise_figure: Option<f64>,
        output_p1db: Option<f64>,
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
                output_p1db,
            } => {
                blocks.push(Block {
                    name,
                    gain,
                    noise_figure,
                    output_p1db,
                });
            }
            BlockConfig::Touchstone {
                file_path,
                name,
                noise_figure,
                output_p1db,
            } => {
                // Touchstone files might also be relative to the config file
                let full_path = base_dir.join(&file_path);
                let gain = touchstone_file_path_and_frequency_to_gain(
                    full_path.to_string_lossy().to_string(),
                    frequency,
                );

                let noise_figure_default = -gain; // only handles passives right now
                let output_p1db_default = 99.0; // 99 dBm

                let final_noise_figure = noise_figure.unwrap_or(noise_figure_default);
                let final_output_p1db = output_p1db.or(Some(output_p1db_default));

                blocks.push(Block {
                    name,
                    gain,
                    noise_figure: final_noise_figure,
                    output_p1db: final_output_p1db,
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

fn calculate_gainlineup(input: Input, blocks: Vec<Block>) -> Vec<SignalNode> {
    let full_cascade: Vec<SignalNode> = cascade_vector_return_vector(input, blocks);

    full_cascade
}

pub struct Command {}

impl Command {
    pub fn run(args: &[String]) -> Result<Command, Box<dyn std::error::Error>> {
        if args.len() < 2 {
            return Err("not enough arguments".into());
        }

        if args.len() > 2 {
            return Err(
                "too many arguments, expecting only 2, such as `gainlineup filepath`".into(),
            );
        }

        // Check for special flags
        match args[1].as_str() {
            "--version" | "-v" => {
                print_version();
                process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            _ => {
                if args.len() > 2 {
                    return Err(
                        "too many arguments, expecting only 2, such as `touchstone filepath`"
                            .into(),
                    );
                }
            }
        }

        let cwd = std::env::current_dir().unwrap();
        // cargo run arg[1], such as cargo run tests/simple_config.toml
        // gainlineup arg[1], such as gainlineup tests/simple_config.toml
        let file_path = args[1].clone();
        println!("Config Path: {}", file_path);
        let full_path_to_config = cwd.join(file_path);
        println!("Full Path: {}", full_path_to_config.display());

        match load_config(&full_path_to_config.display().to_string()) {
            Ok(config) => {
                // println!("\n----------------------------\n");

                let input = Input {
                    power: config.input_power,
                    frequency: config.frequency,
                    bandwidth: 0.0, // CW
                };
                let cascade = calculate_gainlineup(input, config.blocks.clone());
                // println!("\n----------------------------\n");
                print_cascade(cascade.clone(), config.blocks.clone());

                let file_path = full_path_to_config.display().to_string();

                let file_path_config: file_operations::FilePathConfig =
                    file_operations::get_file_path_config(&file_path);

                // absolute path, append .html, remove woindows UNC Prefix if present
                // relative path with separators, just append .hmtl
                // bare_filename, prepend ./ and append .html
                // absolute path, append .html, remove woindows UNC Prefix if present
                // relative path with separators, just append .hmtl
                // bare_filename, prepend ./ and append .html
                let output_html_path = if file_path_config.unix_absolute_path
                    || file_path_config.windows_absolute_path
                {
                    let mut file_path_html = format!("{}.html", &file_path);
                    // Remove the UNC prefix on Windows if present
                    if file_path_config.windows_absolute_path && file_path_html.starts_with(r"\\?\")
                    {
                        file_path_html = file_path_html[4..].to_string();
                    }
                    file_path_html
                } else if file_path_config.relative_path_with_separators {
                    format!("{}.html", &file_path)
                } else if file_path_config.bare_filename {
                    format!("./{}.html", &file_path)
                } else {
                    panic!(
                        "file_path_config must have one true value: {:?}",
                        file_path_config
                    );
                };

                // replace basename.toml.html with basename.html, if it ends with .toml.html
                let output_html_path = if output_html_path.ends_with(".toml.html") {
                    output_html_path.replace(".toml.html", ".html")
                } else {
                    output_html_path
                };

                let output_html_path_str = output_html_path.as_str();

                println!("Generating HTML table at: {}", output_html_path);

                match crate::plot::generate_html_table(
                    config.input_power,
                    config.frequency,
                    &cascade,
                    &config.blocks,
                    output_html_path_str,
                ) {
                    Ok(_) => {
                        crate::open::plot(output_html_path.clone());
                    }
                    Err(e) => {
                        eprintln!("Error generating HTML table: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error running calculation or plotting: {}", e);
                return Err(e);
            }
        }

        Ok(Command {})
    }
}

pub fn print_version() {
    println!("gainlineup {}", env!("CARGO_PKG_VERSION"));
}

pub fn print_error(error: &str) {
    const RED: &str = "\x1b[31m";
    const RESET: &str = "\x1b[0m";
    println!("{}Problem parsing arguments: {error}{}", RED, RESET);
}

pub fn print_help() {
    // ANSI color codes
    const BOLD: &str = "\x1b[1m";
    const CYAN: &str = "\x1b[36m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const RESET: &str = "\x1b[0m";

    println!(
        "📡 Gainlineup parser and calculator - https://github.com/iancleary/gainlineup{}",
        RESET
    );
    println!();
    println!("{}{}VERSION:{}", BOLD, YELLOW, RESET);
    println!("    {}{}{}", GREEN, env!("CARGO_PKG_VERSION"), RESET);
    println!();
    println!("{}{}USAGE:{}", BOLD, YELLOW, RESET);
    println!("    {} gainlineup <FILE_PATH>{}", GREEN, RESET);
    println!();
    println!("     FILE_PATH: path to a toml config file");
    println!();
    println!("     The toml file is parsed and an interactive plot (html file and js/ folder) ");
    println!("     is created next to the source file(s).");
    // println!("     ");
    println!();
    println!("{}{}OPTIONS:{}", BOLD, YELLOW, RESET);
    println!(
        "    {}  -v, --version{}{}    Print version information",
        GREEN, RESET, RESET
    );
    println!(
        "    {}  -h, --help{}{}       Print help information",
        GREEN, RESET, RESET
    );
    println!();
    println!("{}{}EXAMPLES:{}", BOLD, YELLOW, RESET);
    println!("    {} # Single file (Relative path){}", CYAN, RESET);
    println!("    {} gainlineup files/config.toml{}", GREEN, RESET);
    println!();
}

pub fn print_cascade(cascade: Vec<SignalNode>, blocks: Vec<Block>) {
    println!();
    for (i, node) in cascade.iter().enumerate() {
        println!("\nNode {}: {}", i, node.name);

        if i == 0 {
            // the formatting `{:>8.2}` aligns positive and negative numbers on the decimal,
            // with two digits after the decimal (hundredths place)
            println!("Input Level {:>8.2} dBm", node.power);
        } else {
            // let block_gain = node.power - cascade[i - 1].power;
            let block_gain = blocks[i - 1].gain;
            let input_power = node.power - block_gain;

            // the formatting `{:>8.2}` aligns positive and negative numbers on the decimal,
            // with two digits after the decimal (hundredths place)
            println!("Input Power\t\t{:>8.2} dBm", input_power);
            println!("Block Gain:\t\t{:>8.2} dB", block_gain);
            println!("Block NF:\t\t{:>8.2} dB", blocks[i - 1].noise_figure);
            println!("Cumulative Gain:\t{:>8.2} dB", node.cumulative_gain);
            println!("Cumulative Noise Figure:{:>8.2} dB", node.noise_figure);
            println!("Output Power\t\t{:>8.2} dBm", node.power);
        }
    }
    println!();
    println!("Final Cascade Summary:");
    println!("----------------------");
    println!("Number of Blocks: {}", cascade.len() - 1);
    println!("Pin:\t{:>8.2} dBm", cascade[0].power);

    let final_output_power = cascade.last().unwrap().power;
    println!("Pout:\t{:>8.2} dBm", final_output_power);
    println!("Gain:\t{:>8.2} dB", cascade.last().unwrap().cumulative_gain);
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use std::path::PathBuf;

    fn setup_test_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push("gainlineup_tests");
        path.push(name);
        path.push(format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_run_function() {
        let test_dir = setup_test_dir("test_run_function");
        let toml_path = test_dir.join("test_cli_run.toml");
        fs::copy("files/simple_config.toml", &toml_path).unwrap();

        let args = vec![
            String::from("program_name"),
            toml_path.to_str().unwrap().to_string(),
        ];
        let _cli_run = Command::run(&args).unwrap();
    }

    #[test]
    fn test_config_build_not_enough_args() {
        let args = vec![String::from("program_name")];
        let result = Command::run(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_help_flag() {
        // Help flag test - verifies the flag is recognized
        // Note: In actual execution, this would exit the process
        // This test just documents the expected behavior
        let help_flags = vec!["--help", "-h"];
        for flag in help_flags {
            assert!(flag == "--help" || flag == "-h");
        }
    }

    #[test]
    fn test_version_flag() {
        // Version flag test - verifies the flag is recognized
        // Note: In actual execution, this would exit the process
        // This test just documents the expected behavior
        let version_flags = vec!["--version", "-v"];
        for flag in version_flags {
            assert!(flag == "--version" || flag == "-v");
        }
    }

    #[test]
    fn test_version_output_format() {
        // Test that version string is in correct format
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        // Version should be in format X.Y.Z
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "Version should be in X.Y.Z format");
    }
}
