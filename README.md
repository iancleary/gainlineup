# gainlineup

Gain Lineups for RF Engineering

## Example

Here is an example program to use (copy/paste) in `main.rs` of a new `cargo new example-lineup` project.

```rust
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
    println!("Noise Figure:\t{:>8.2} dB", full_cascade.last().unwrap().noise_figure);
}
```

The output is similar to the following:

```

----------------------------

Run function executed

Node 0: Cable Run from Signal Generator to DUT Output
Input Level     4.00 dBm

Node 1: Line Amp at X GHz Output
Input Power         4.00 dBm
Block Gain:        22.00 dB    (Cumulative Gain:    16.00 dB)
Noise Figure:       6.02 dB
Output Power       26.00 dBm

Node 2: Cable Run from DUT to Spectrum Analyzer Output
Input Power        26.00 dBm
Block Gain:        -6.00 dB    (Cumulative Gain:    10.00 dB)
Noise Figure:       6.10 dB
Output Power       20.00 dBm

Final Cascade Summary:
----------------------
Number of Blocks: 2
Pin:        4.00 dBm
Pout:      20.00 dBm
Gain:      10.00 dB
Noise Figure:       6.10 dB

----------------------------

```

The command line interface is more interactive, and allows for the user to use toml files to define the Input, the Blocks, and run the cascade, turning the output into a html file.

```bash
gainlineup files/defaults_to_cw.toml
```

This will generate a html file in the same directory as the toml file.

```bash
Node 0: Low Noise Amplifier Output
Input Level   -50.00 dBm

Node 1: Mixer Output
Input Power               -60.00 dBm
Block Gain:                20.00 dB
Block NF:                   3.00 dB
Cumulative Gain:           30.00 dB
Cumulative Noise Figure:    3.06 dB
Output Power              -40.00 dBm

Node 2: IF Amplifier Output
Input Power               -35.00 dBm
Block Gain:                10.00 dB
Block NF:                   6.00 dB
Cumulative Gain:           45.00 dB
Cumulative Noise Figure:    3.06 dB
Output Power              -25.00 dBm

Final Cascade Summary:
----------------------
Number of Blocks: 2
Pin:      -50.00 dBm
Pout:     -25.00 dBm
Gain:      45.00 dB

'C:\Users\iancleary\Development\gainlineup\files\wideband.toml' is a Windows Absolute path.
Generating HTML table at: C:\Users\iancleary\Development\gainlineup\files\wideband.toml.html
file_path in get_file_url function: C:\Users\iancleary\Development\gainlineup\files\wideband.toml.html
You can open the plot in your browser at:
file:///C:/Users/icleary/Development/gainlineup/files/wideband.toml.html
Attempting to open plot in your default browser...
Success! Opening: file:///C:/Users/icleary/Development/gainlineup/files/wideband.toml.html
```

You can view an example of the html output at `files/wideband.toml.html`

[![HTML file created for the files directory by running `gainlineup files/wideband.toml` in the root of this directory](https://github.com/iancleary/gainlineup/blob/main/files/wideband.toml.html.png?raw=true)](https://github.com/iancleary/gainlineup/tree/main/files/wideband.toml.html)

You can view the HTML source file itself here directly: [files/wideband.toml.html](https://github.com/iancleary/gainlineup/tree/main/files/wideband.toml.html).
