use gainlineup::cascade_vector_return_vector;
use gainlineup::file::load_config;
use gainlineup::print_cascade;
use gainlineup::{Block, SignalNode};

fn main() {
    println!("\n----------------------------\n");

    let cwd = std::env::current_dir().unwrap();
    // println!("Current Directory: {}", cwd.display());

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <config_file_path>", args[0]);
        std::process::exit(1);
    }

    let config_path = &args[1];

    println!("Config Path: {}", config_path);

    // println!("\n----------------------------\n");
    let full_path_to_config = cwd.join(config_path);
    // println!("Full Path to Config: {}", full_path_to_config.display());

    match load_config(&full_path_to_config.display().to_string()) {
        Ok(config) => {
            // println!("\n----------------------------\n");
            let cascade = calculate_gainlineup(config.input_power, config.blocks.clone());
            // println!("\n----------------------------\n");
            print_cascade(cascade, config.blocks);
        }
        Err(e) => {
            eprintln!("Error running calculation: {}", e);
        }
    }
}

fn calculate_gainlineup(input_power: f64, blocks: Vec<Block>) -> Vec<SignalNode> {
    let input_node = SignalNode {
        name: "Input".to_string(),
        power: input_power,
        noise_temperature: 290.0,
        cumulative_gain: 0.0,
    };

    let full_cascade: Vec<SignalNode> = cascade_vector_return_vector(input_node, blocks);

    full_cascade
}
