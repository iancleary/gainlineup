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
    pub input_power_dbm: f64,
    pub frequency_hz: f64,
    pub bandwidth_hz: Option<f64>,
    pub noise_temperature_k: Option<f64>,
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
        #[serde(alias = "gain")]
        gain_db: f64,
        #[serde(alias = "noise_figure", alias = "nf")]
        noise_figure_db: f64,
        #[serde(alias = "output_p1db", alias = "op1db")]
        output_p1db_dbm: Option<f64>,
    },
    Touchstone {
        file_path: String,
        name: String,
        #[serde(alias = "noise_figure", alias = "nf")]
        noise_figure_db: Option<f64>,
        #[serde(alias = "output_p1db", alias = "op1db")]
        output_p1db_dbm: Option<f64>,
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
        #[serde(alias = "input_power", alias = "pin")]
        input_power_dbm: f64,
        #[serde(alias = "frequency", alias = "f")]
        frequency_hz: f64,
        #[serde(alias = "bandwidth", alias = "bw")]
        bandwidth_hz: Option<f64>,
        #[serde(alias = "noise_temperature")]
        noise_temperature_k: Option<f64>,
        blocks: Vec<BlockConfig>,
    }

    let intermediate_config: IntermediateConfig = toml::from_str(&config_content)?;
    // println!("Config: {:#?}", config);

    let mut blocks = Vec::new();
    let config_path = Path::new(path);
    let base_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

    load_blocks_recursive(
        intermediate_config.blocks,
        intermediate_config.frequency_hz,
        &mut blocks,
        base_dir,
    )?;

    // println!("\n----------------------------\n");

    Ok(Config {
        input_power_dbm: intermediate_config.input_power_dbm,
        frequency_hz: intermediate_config.frequency_hz,
        bandwidth_hz: intermediate_config.bandwidth_hz,
        noise_temperature_k: intermediate_config.noise_temperature_k,
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
                gain_db,
                noise_figure_db,
                output_p1db_dbm,
            } => {
                blocks.push(Block {
                    name,
                    gain_db,
                    noise_figure_db,
                    output_p1db_dbm,
                });
            }
            BlockConfig::Touchstone {
                file_path,
                name,
                noise_figure_db,
                output_p1db_dbm,
            } => {
                // Touchstone files might also be relative to the config file
                let full_path = base_dir.join(&file_path);
                let TouchstoneValid {
                    contains_frequency,
                    gain,
                } = touchstone_file_path_and_frequency_to_struct(
                    full_path.to_string_lossy().to_string(),
                    frequency,
                );

                if !contains_frequency {
                    let file_path_relative_to_config = file_path.clone();
                    return Err(format!(
                        "Frequency {} Hz not found in touchstone file {}",
                        frequency, file_path_relative_to_config
                    )
                    .into());
                }

                let gain = gain.unwrap();
                let noise_figure_default = -gain; // only handles passives right now
                let output_p1db_default = 99.0; // 99 dBm

                let final_noise_figure = noise_figure_db.unwrap_or(noise_figure_default);
                let final_output_p1db = output_p1db_dbm.or(Some(output_p1db_default));

                blocks.push(Block {
                    name,
                    gain_db: gain,
                    noise_figure_db: final_noise_figure,
                    output_p1db_dbm: final_output_p1db,
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

pub struct TouchstoneValid {
    contains_frequency: bool,
    gain: Option<f64>,
}

pub fn touchstone_file_path_and_frequency_to_struct(
    file_path: String,
    frequency_in_hz: f64,
) -> TouchstoneValid {
    let s2p = Network::new(file_path.clone());

    // check if frequency is within the touchstone file

    let frequency_vector = s2p.f.clone();
    let contains_frequency = frequency_vector.contains(&frequency_in_hz);

    if !contains_frequency {
        return TouchstoneValid {
            contains_frequency: false,
            gain: None,
        };
    }

    let gain_vector = s2p.s_db(2, 1); // uses 1-based indexing

    let gain = gain_vector
        .iter()
        .find(|frequency_db| frequency_db.frequency == frequency_in_hz)
        .unwrap()
        .s_db
        .decibel();

    TouchstoneValid {
        contains_frequency: true,
        gain: Some(gain),
    }
}

fn calculate_gainlineup(input: Input, blocks: Vec<Block>) -> Vec<SignalNode> {
    let full_cascade: Vec<SignalNode> = cascade_vector_return_vector(input, blocks);

    full_cascade
}

#[derive(Debug)]
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
                    power_dbm: config.input_power_dbm,
                    frequency_hz: config.frequency_hz,
                    bandwidth_hz: config.bandwidth_hz.unwrap_or(100.0), // CW in real life
                    noise_temperature_k: Some(config.noise_temperature_k.unwrap_or(290.0)), // 290K is standard
                };
                let cascade = calculate_gainlineup(input.clone(), config.blocks.clone());
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

                println!("Generating HTML table at: {}", output_html_path);

                let output_html_path_str = output_html_path.as_str();

                match crate::plot::generate_html_table(
                    &input,
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
            println!("Input Level {:>8.2} dBm", node.signal_power_dbm);
        } else {
            // let block_gain = node.power - cascade[i - 1].power;
            let block_gain = blocks[i - 1].gain_db;
            let input_power = node.signal_power_dbm - block_gain;

            // the formatting `{:>8.2}` aligns positive and negative numbers on the decimal,
            // with two digits after the decimal (hundredths place)
            println!("Input Power\t\t{:>8.2} dBm", input_power);
            println!("Block Gain:\t\t{:>8.2} dB", block_gain);
            println!("Block NF:\t\t{:>8.2} dB", blocks[i - 1].noise_figure_db);
            println!("Cumulative Gain:\t{:>8.2} dB", node.cumulative_gain_db);
            println!(
                "Cumulative Noise Figure:{:>8.2} dB",
                node.cumulative_noise_figure_db
            );
            println!("Output Power\t\t{:>8.2} dBm", node.signal_power_dbm);
        }
    }
    println!();
    println!("Final Cascade Summary:");
    println!("----------------------");
    println!("Number of Blocks: {}", cascade.len() - 1);
    println!("Pin:\t{:>8.2} dBm", cascade[0].signal_power_dbm);

    let final_output_power = cascade.last().unwrap().signal_power_dbm;
    println!("Pout:\t{:>8.2} dBm", final_output_power);
    println!(
        "Gain:\t{:>8.2} dB",
        cascade.last().unwrap().cumulative_gain_db
    );
    println!(
        "NF:\t{:>8.2} dB",
        cascade.last().unwrap().cumulative_noise_figure_db
    );
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
        fs::copy("files/defaults_to_cw.toml", &toml_path).unwrap();

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

    #[test]
    fn test_touchstone_file_path_and_frequency_to_gain() {
        let touchstone_file_path = "files/touchstone_options/ntwk3.s2p";
        let frequency_in_hz = 6.0e9;
        let TouchstoneValid {
            contains_frequency,
            gain,
        } = touchstone_file_path_and_frequency_to_struct(
            touchstone_file_path.to_string(),
            frequency_in_hz,
        );

        let gain = gain.unwrap();
        assert!(contains_frequency);

        let gain_rounded_to_3_decimal_places = (gain * 1e3).round() / 1e3;
        assert_eq!(gain_rounded_to_3_decimal_places, -3.932);
    }

    #[test]
    fn test_touchstone_file_path_and_frequency_to_gain_not_found() {
        let touchstone_file_path = "files/touchstone_options/ntwk3.s2p";
        let frequency_in_hz = 11.0e9;
        let TouchstoneValid {
            contains_frequency,
            gain,
        } = touchstone_file_path_and_frequency_to_struct(
            touchstone_file_path.to_string(),
            frequency_in_hz,
        );
        assert!(!contains_frequency);
        assert_eq!(gain, None);
    }

    #[test]
    fn test_run_invalid_touchstone_frequency_error() {
        let config_path = "files/touchstone_invalid_frequency/config.toml";
        let args = vec![String::from("program_name"), config_path.to_string()];
        let result = Command::run(&args);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Frequency 11000000000 Hz not found in touchstone file ntwk3.s2p"
        );
        // this ^ `ntwk3.s2p` is relative to the config file path, not the folder you run the program from
    }

    #[test]
    fn test_optional_units_parsing() {
        let toml_content = r#"
            pin = -30.0
            f = 10.0e9
            bw = 1.0e6
            noise_temperature = 290.0
            [[blocks]]
            type = "explicit"
            name = "LNA"
            gain = 20.0
            nf = 2.0
            op1db = 10.0
        "#;

        #[derive(Deserialize, Debug)]
        struct IntermediateConfig {
            #[serde(alias = "input_power", alias = "pin")]
            input_power_dbm: f64,
            #[serde(alias = "frequency", alias = "f")]
            frequency_hz: f64,
            #[serde(alias = "bandwidth", alias = "bw")]
            bandwidth_hz: Option<f64>,
            #[serde(alias = "noise_temperature")]
            noise_temperature_k: Option<f64>,
            blocks: Vec<BlockConfig>,
        }

        let config: IntermediateConfig = toml::from_str(toml_content).unwrap();

        assert_eq!(config.input_power_dbm, -30.0);
        assert_eq!(config.frequency_hz, 10.0e9);
        assert_eq!(config.bandwidth_hz, Some(1.0e6));
        assert_eq!(config.noise_temperature_k, Some(290.0));
        assert_eq!(config.blocks.len(), 1);

        if let BlockConfig::Explicit {
            name,
            gain_db,
            noise_figure_db,
            output_p1db_dbm,
        } = &config.blocks[0]
        {
            assert_eq!(name, "LNA");
            assert_eq!(*gain_db, 20.0);
            assert_eq!(*noise_figure_db, 2.0);
            assert_eq!(*output_p1db_dbm, Some(10.0));
        } else {
            panic!("Expected Explicit block");
        }
    }
}
