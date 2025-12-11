use gainlineup::cascade_vector_return_vector;
use gainlineup::Block;
use gainlineup::Input;
use gainlineup::SignalNode;

fn main() {
    println!("\n----------------------------\n");
    run();
    println!("\n----------------------------\n");
}

fn run() {
    println!("Run function executed");

    // Add your code logic here
    const INPUT_POWER: f64 = 10.0; // dBm

    let input_node = Input {
        power: INPUT_POWER,
        frequency: 1.0e9,
        bandwidth: 1.0e6, // Hz, leave as 0.0 or omit for CW
    };

    let cable_from_signal_generator = Block {
        name: "Cable Run from Signal Generator to DUT".to_string(),
        gain: -6.0,
        noise_figure: 6.0,
        output_p1db: None,
    };

    let line_amp: Block = Block {
        name: "Line Amp at X GHz".to_string(),
        gain: 22.0,
        noise_figure: 6.0,
        output_p1db: None,
    };

    let cable_run_to_spectrum_analyzer: Block = Block {
        name: "Cable Run from DUT to Spectrum Analyzer".to_string(),
        gain: -6.0,
        noise_figure: 6.0,
        output_p1db: None,
    };

    let blocks = vec![
        cable_from_signal_generator.clone(),
        line_amp.clone(),
        cable_run_to_spectrum_analyzer.clone(),
    ];

    let full_cascade: Vec<SignalNode> =
        cascade_vector_return_vector(input_node.clone(), blocks.clone());

    // println!("{:>8.2} dBm", node.power);`

    for (i, node) in full_cascade.iter().enumerate() {
        println!();
        println!("Node {}: {}", i, node.name);

        if i == 0 {
            // the formatting `{:>8.2}` aligns positive and negative numbers on the decimal,
            // with two digits after the decimal (hundredths place)
            println!("Input Level {:>8.2} dBm", node.power);
        } else {
            let block_gain = full_cascade[i].power - full_cascade[i - 1].power;
            let input_power = node.power - block_gain;

            // the formatting `{:>8.2}` aligns positive and negative numbers on the decimal,
            // with two digits after the decimal (hundredths place)
            println!("Input Power\t{:>8.2} dBm", input_power);
            println!(
                "Block Gain:\t{:>8.2} dB    (Cumulative Gain: {:>8.2} dB)",
                block_gain, node.cumulative_gain
            );
            println!("Noise Figure:\t{:>8.2} dB", node.noise_figure);
            println!("Output Power\t{:>8.2} dBm", node.power);
        }
    }
    println!();
    println!("Final Cascade Summary:");
    println!("----------------------");
    println!("Number of Blocks: {}", full_cascade.len() - 1);
    println!("Pin:\t{:>8.2} dBm", full_cascade[0].power);

    let final_output_power = full_cascade.last().unwrap().power;
    println!("Pout:\t{:>8.2} dBm", final_output_power);
    println!(
        "Gain:\t{:>8.2} dB",
        full_cascade.last().unwrap().cumulative_gain
    );
    println!(
        "Noise Figure:\t{:>8.2} dB",
        full_cascade.last().unwrap().noise_figure
    );
}
