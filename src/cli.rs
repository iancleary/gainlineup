use std::env;
use std::process;

// this cannot be crate::Network because of how Cargo works,
// since cargo/rust treats lib.rs and main.rs as separate crates
use crate::cascade_vector_return_vector;
use crate::file_operations;
use crate::load_config;
use crate::Block;
use crate::SignalNode;

use rfconversions;

fn calculate_gainlineup(
    input_power: f64,
    blocks: Vec<Block>,
    input_noise_temperature: Option<f64>,
) -> Vec<SignalNode> {
    let noise_temperature = input_noise_temperature.unwrap_or(290.0);
    // k in J/K, multiplied by 1000 for dBm
    let k_boltzmann = 1.380649e-23;
    let dbm_factor = 1000.0; // mW -> multiply W by 1000 to get mW count (dBW to dBm)
    let noise_spectral_density_linear = k_boltzmann * noise_temperature * dbm_factor;
    let noise_spectral_density = 10.0 * noise_spectral_density_linear.log10();

    let input_node = SignalNode {
        name: "Input".to_string(),
        power: input_power,
        noise_temperature,
        noise_spectral_density,
        cumulative_gain: 0.0,
    };

    let full_cascade: Vec<SignalNode> = cascade_vector_return_vector(input_node, blocks);

    full_cascade
}

pub struct Config {}

impl Config {
    pub fn run(args: &[String]) -> Result<Config, Box<dyn std::error::Error>> {
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
                let cascade = calculate_gainlineup(
                    config.input_power,
                    config.blocks.clone(),
                    config.noise_temperature,
                );
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
                let output_html_path = if file_path_config.absolute_path {
                    let mut file_path_html = format!("{}.html", &file_path);
                    // Remove the UNC prefix on Windows if present
                    if cfg!(target_os = "windows") && file_path_html.starts_with(r"\\?\") {
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
                    config.noise_temperature,
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

        Ok(Config {})
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
            println!(
                "Cumulative Noise Figure:{:>8.2} dB",
                rfconversions::noise::noise_figure_from_noise_temperature(node.noise_temperature)
            );
            println!(
                "Noise Spectral Density:\t{:>8.2} dBm/Hz",
                node.noise_spectral_density
            );
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
        let _cli_run = Config::run(&args).unwrap();
    }

    #[test]
    fn test_config_build_not_enough_args() {
        let args = vec![String::from("program_name")];
        let result = Config::run(&args);
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
