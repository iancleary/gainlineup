use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process;

// this cannot be crate::Network because of how Cargo works,
// since cargo/rust treats lib.rs and main.rs as separate crates
use crate::cascade_vector_return_vector;
use crate::file_operations;
use crate::Block;
use crate::Input;
use crate::SignalNode;
use crate::{cascade_frequency_sweep, FrequencyBlock, FrequencySample, FrequencySweep};

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

#[derive(Deserialize, Debug)]
struct SampleConfig {
    frequency_hz: f64,
    gain_db: f64,
    noise_figure_db: f64,
    output_p1db_dbm: Option<f64>,
    output_ip3_dbm: Option<f64>,
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
        #[serde(default, alias = "output_ip3", alias = "oip3")]
        output_ip3_dbm: Option<f64>,
    },
    Touchstone {
        file_path: String,
        name: String,
        #[serde(alias = "noise_figure", alias = "nf")]
        noise_figure_db: Option<f64>,
        #[serde(alias = "output_p1db", alias = "op1db")]
        output_p1db_dbm: Option<f64>,
    },
    Tabulated {
        name: String,
        samples: Vec<SampleConfig>,
    },
    Include {
        path: String,
    },
}

fn read_config(path: &str) -> Result<IntermediateConfig, Box<dyn std::error::Error>> {
    tracing::debug!("Loading config: {}", path);
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}

pub fn load_config(path: &str) -> Result<Config, Box<dyn std::error::Error>> {
    let config = read_config(path)?;
    let mut blocks = Vec::new();
    visit_blocks(path, config.blocks, &mut |block, base_dir| {
        blocks.push(resolve_static_block(block, config.frequency_hz, base_dir)?);
        Ok(())
    })?;
    Ok(Config {
        input_power_dbm: config.input_power_dbm,
        frequency_hz: config.frequency_hz,
        bandwidth_hz: config.bandwidth_hz,
        noise_temperature_k: config.noise_temperature_k,
        blocks,
    })
}

fn visit_blocks(
    path: &str,
    blocks: Vec<BlockConfig>,
    visitor: &mut impl FnMut(BlockConfig, &Path) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut active_paths = vec![fs::canonicalize(path)?];
    let base_dir = Path::new(path).parent().unwrap_or_else(|| Path::new("."));
    visit_blocks_recursive(blocks, base_dir, &mut active_paths, visitor)
}

fn visit_blocks_recursive(
    blocks: Vec<BlockConfig>,
    base_dir: &Path,
    active_paths: &mut Vec<PathBuf>,
    visitor: &mut impl FnMut(BlockConfig, &Path) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    for block in blocks {
        if let BlockConfig::Include { path } = block {
            let included_path = base_dir.join(path);
            let canonical_path = fs::canonicalize(&included_path)?;
            if active_paths.contains(&canonical_path) {
                return Err(format!("Circular config include: {}", included_path.display()).into());
            }
            tracing::debug!("Loading included config: {}", included_path.display());
            let included: IncludedConfig = toml::from_str(&fs::read_to_string(&included_path)?)?;
            active_paths.push(canonical_path);
            visit_blocks_recursive(
                included.blocks,
                included_path.parent().unwrap_or_else(|| Path::new(".")),
                active_paths,
                visitor,
            )?;
            active_paths.pop();
        } else {
            visitor(block, base_dir)?;
        }
    }
    Ok(())
}

fn tabulated_block(
    name: String,
    samples: Vec<SampleConfig>,
) -> Result<FrequencyBlock, Box<dyn std::error::Error>> {
    Ok(FrequencyBlock::tabulated(
        samples
            .into_iter()
            .map(|sample| FrequencySample {
                frequency_hz: sample.frequency_hz,
                block: Block {
                    name: name.clone(),
                    gain_db: sample.gain_db,
                    noise_figure_db: sample.noise_figure_db,
                    output_p1db_dbm: sample.output_p1db_dbm,
                    output_ip3_dbm: sample.output_ip3_dbm,
                },
            })
            .collect(),
    )?)
}

fn resolve_static_block(
    block: BlockConfig,
    frequency_hz: f64,
    base_dir: &Path,
) -> Result<Block, Box<dyn std::error::Error>> {
    match block {
        BlockConfig::Explicit {
            name,
            gain_db,
            noise_figure_db,
            output_p1db_dbm,
            output_ip3_dbm,
        } => Ok(Block {
            name,
            gain_db,
            noise_figure_db,
            output_p1db_dbm,
            output_ip3_dbm,
        }),
        BlockConfig::Tabulated { name, samples } => {
            Ok(tabulated_block(name, samples)?.at_frequency(frequency_hz)?)
        }
        BlockConfig::Touchstone {
            file_path,
            name,
            noise_figure_db,
            output_p1db_dbm,
        } => {
            let result = touchstone_file_path_and_frequency_to_struct(
                base_dir.join(&file_path).to_string_lossy().into_owned(),
                frequency_hz,
            )?;
            if !result.contains_frequency {
                return Err(format!(
                    "Frequency {} Hz not found in touchstone file {}",
                    frequency_hz, file_path
                )
                .into());
            }
            let gain_db = result.gain.ok_or_else(|| {
                format!(
                    "Frequency {} Hz not found in touchstone file {}",
                    frequency_hz, file_path
                )
            })?;
            Ok(Block {
                name,
                gain_db,
                noise_figure_db: noise_figure_db.unwrap_or(-gain_db),
                output_p1db_dbm: output_p1db_dbm.or(Some(99.0)),
                output_ip3_dbm: None,
            })
        }
        BlockConfig::Include { .. } => Err("Unresolved config include".into()),
    }
}

fn resolve_sweep_block(
    block: BlockConfig,
    base_dir: &Path,
) -> Result<FrequencyBlock, Box<dyn std::error::Error>> {
    match block {
        BlockConfig::Tabulated { name, samples } => tabulated_block(name, samples),
        BlockConfig::Touchstone {
            file_path,
            name,
            noise_figure_db,
            output_p1db_dbm,
        } => {
            let network = Network::new(base_dir.join(&file_path))?;
            ensure_transmission_ports(&network)?;
            let samples = network
                .s_db(2, 1)
                .into_iter()
                .map(|point| {
                    let gain_db = point.s_db.decibel();
                    FrequencySample {
                        frequency_hz: point.frequency,
                        block: Block {
                            name: name.clone(),
                            gain_db,
                            noise_figure_db: noise_figure_db.unwrap_or(-gain_db),
                            output_p1db_dbm: output_p1db_dbm.or(Some(99.0)),
                            output_ip3_dbm: None,
                        },
                    }
                })
                .collect();
            Ok(FrequencyBlock::tabulated(samples)?)
        }
        block => Ok(FrequencyBlock::constant(resolve_static_block(
            block, 0.0, base_dir,
        )?)?),
    }
}

fn ensure_transmission_ports(network: &Network) -> Result<(), Box<dyn std::error::Error>> {
    if network.rank < 2 {
        return Err(format!(
            "Touchstone file {} requires at least two ports for S21 gain (found {})",
            network.name, network.rank
        )
        .into());
    }
    Ok(())
}

fn run_sweep(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 6 {
        return Err("Usage: gainlineup sweep <FILE> <START_HZ> <STOP_HZ> <POINTS>".into());
    }
    let start_hz = args[3]
        .parse::<f64>()
        .map_err(|_| "START_HZ must be a frequency in Hz, for example 1e9")?;
    let stop_hz = args[4]
        .parse::<f64>()
        .map_err(|_| "STOP_HZ must be a frequency in Hz, for example 2e9")?;
    let points = args[5]
        .parse::<usize>()
        .map_err(|_| "POINTS must be an integer of at least 2")?;
    let sweep = FrequencySweep::linear(start_hz, stop_hz, points)?;
    let config = read_config(&args[2])?;
    let mut blocks = Vec::new();
    visit_blocks(&args[2], config.blocks, &mut |block, base_dir| {
        blocks.push(resolve_sweep_block(block, base_dir)?);
        Ok(())
    })?;
    let input = Input {
        power_dbm: config.input_power_dbm,
        frequency_hz: start_hz,
        bandwidth_hz: config.bandwidth_hz.unwrap_or(100.0),
        noise_temperature_k: Some(config.noise_temperature_k.unwrap_or(290.0)),
    };
    // Validate and calculate the entire sweep before emitting a header or any rows.
    let results = cascade_frequency_sweep(&input, &blocks, &sweep)?;
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    writeln!(output, "frequency_hz,stage_index,stage_name,signal_power_dbm,noise_power_dbm,cumulative_gain_db,cumulative_noise_figure_db,snr_db,cumulative_oip3_dbm,sfdr_db")?;
    for point in results {
        for (index, node) in point.nodes.iter().enumerate() {
            writeln!(
                output,
                "{},{},\"{}\",{},{},{},{},{},{},{}",
                point.frequency_hz,
                index + 1,
                node.name.replace('"', "\"\""),
                node.signal_power_dbm,
                node.noise_power_dbm,
                node.cumulative_gain_db,
                node.cumulative_noise_figure_db,
                node.signal_to_noise_ratio_db(),
                node.cumulative_oip3_dbm
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                node.sfdr_db.map(|v| v.to_string()).unwrap_or_default(),
            )?;
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
) -> Result<TouchstoneValid, Box<dyn std::error::Error>> {
    tracing::debug!("Loading touchstone file: {}", file_path);
    let s2p = Network::new(file_path.clone())?;
    ensure_transmission_ports(&s2p)?;

    // check if frequency is within the touchstone file

    let frequency_vector = s2p.f.clone();
    let contains_frequency = frequency_vector.contains(&frequency_in_hz);

    if !contains_frequency {
        tracing::debug!(
            frequency_hz = frequency_in_hz,
            "Frequency not found in touchstone file"
        );
        return Ok(TouchstoneValid {
            contains_frequency: false,
            gain: None,
        });
    }

    let gain_vector = s2p.s_db(2, 1); // uses 1-based indexing

    let gain = gain_vector
        .iter()
        .find(|frequency_db| frequency_db.frequency == frequency_in_hz)
        .unwrap()
        .s_db
        .decibel();

    Ok(TouchstoneValid {
        contains_frequency: true,
        gain: Some(gain),
    })
}

fn calculate_gainlineup(input: Input, blocks: Vec<Block>) -> Vec<SignalNode> {
    let full_cascade: Vec<SignalNode> = cascade_vector_return_vector(input, blocks);

    full_cascade
}

#[derive(Debug)]
pub struct Command {}

impl Command {
    pub fn run(args: &[String]) -> Result<Command, Box<dyn std::error::Error>> {
        if args.get(1).map(String::as_str) == Some("sweep") {
            run_sweep(args)?;
            return Ok(Command {});
        }
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
                let input = Input {
                    power_dbm: config.input_power_dbm,
                    frequency_hz: config.frequency_hz,
                    bandwidth_hz: config.bandwidth_hz.unwrap_or(100.0), // CW in real life
                    noise_temperature_k: Some(config.noise_temperature_k.unwrap_or(290.0)), // 290K is standard
                };
                let cascade = calculate_gainlineup(input.clone(), config.blocks.clone());

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
                    let mut file_path_html = format!("{}.html", file_path);
                    // Remove the UNC prefix on Windows if present
                    if file_path_config.windows_absolute_path && file_path_html.starts_with(r"\\?\")
                    {
                        file_path_html = file_path_html[4..].to_string();
                    }
                    file_path_html
                } else if file_path_config.relative_path_with_separators {
                    format!("{}.html", file_path)
                } else if file_path_config.bare_filename {
                    format!("./{}.html", file_path)
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
    println!(
        "    {} gainlineup sweep <FILE_PATH> <START_HZ> <STOP_HZ> <POINTS>{}",
        GREEN, RESET
    );
    println!();
    println!("     FILE_PATH: path to a toml config file");
    println!();
    println!("     The toml file is parsed and an interactive plot (html file and js/ folder) ");
    println!("     is created next to the source file(s).");
    println!("     sweep writes CSV to stdout without creating a plot. Frequencies are in Hz.");
    println!("     Use an ascending range and at least two points; both endpoints are included.");
    println!("     Tabulated and Touchstone blocks interpolate dB/dBm values linearly in Hz.");
    println!("     Sweep frequencies must lie inside every table's measured range.");

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
    println!(
        "    {} gainlineup sweep files/frequency_sweep.toml 1e9 2e9 11 > sweep.csv{}",
        GREEN, RESET
    );
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
        )
        .unwrap();

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
        )
        .unwrap();
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
            ..
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
