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
    const INPUT_POWER_DBM: f64 = 10.0; // dBm

    let input_node = Input {
        power_dbm: INPUT_POWER_DBM,
        frequency_hz: 1.0e9,
        bandwidth_hz: 1.0e6, // Hz, leave as 0.0 or omit for CW
        noise_temperature_k: None,
    };

    let cable_from_signal_generator = Block {
        name: "Cable Run from Signal Generator to DUT".to_string(),
        gain_db: -6.0,
        noise_figure_db: 6.0,
        output_p1db_dbm: None,
        output_ip3_dbm: None,
    };

    let line_amp: Block = Block {
        name: "Line Amp at X GHz".to_string(),
        gain_db: 22.0,
        noise_figure_db: 6.0,
        output_p1db_dbm: None,
        output_ip3_dbm: Some(35.0),
    };

    let cable_run_to_spectrum_analyzer: Block = Block {
        name: "Cable Run from DUT to Spectrum Analyzer".to_string(),
        gain_db: -6.0,
        noise_figure_db: 6.0,
        output_p1db_dbm: None,
        output_ip3_dbm: None,
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
            println!("Input Level {:>8.2} dBm", node.signal_power_dbm);
        } else {
            let block_gain =
                full_cascade[i].signal_power_dbm - full_cascade[i - 1].signal_power_dbm;
            let input_power = node.signal_power_dbm - block_gain;

            // the formatting `{:>8.2}` aligns positive and negative numbers on the decimal,
            // with two digits after the decimal (hundredths place)
            println!("Input Power\t{:>8.2} dBm", input_power);
            println!(
                "Block Gain:\t{:>8.2} dB    (Cumulative Gain: {:>8.2} dB)",
                block_gain, node.cumulative_gain_db
            );
            println!("Noise Figure:\t{:>8.2} dB", node.cumulative_noise_figure_db);
            println!("Output Power\t{:>8.2} dBm", node.signal_power_dbm);
        }
    }
    println!();
    println!("Final Cascade Summary:");
    println!("----------------------");
    println!("Number of Blocks: {}", full_cascade.len() - 1);
    println!("Pin:\t{:>8.2} dBm", full_cascade[0].signal_power_dbm);

    let final_output_power = full_cascade.last().unwrap().signal_power_dbm;
    println!("Pout:\t{:>8.2} dBm", final_output_power);
    println!(
        "Gain:\t{:>8.2} dB",
        full_cascade.last().unwrap().cumulative_gain_db
    );
    println!(
        "Noise Figure:\t{:>8.2} dB",
        full_cascade.last().unwrap().cumulative_noise_figure_db
    );
}
```

The output is similar to the following:

```

-Node 0: Cable Run from Signal Generator to DUT Output
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
gainlineup files/wideband.toml
```

The contents of that file are:

```toml
input_power_dbm = -80.0
frequency_hz = 6.0e9
bandwidth_hz = 1.0e6

[[blocks]]
type = "explicit"
name = "Low Noise Amplifier"
gain_db = 20.0
noise_figure_db = 3.0

[[blocks]]
type = "explicit"
name = "Mixer"
gain_db = 10.0
noise_figure_db = 6.0

[[blocks]]
type = "explicit"
name = "IF Amplifier"
gain_db = 15.0
noise_figure_db = 5.0
```

The output will be a html file in the same directory as the toml file.

```bash
Node 0: Low Noise Amplifier Output
Input Level   -60.00 dBm

Node 1: Mixer Output
Input Power               -70.00 dBm
Block Gain:                20.00 dB
Block NF:                   3.00 dB
Cumulative Gain:           30.00 dB
Cumulative Noise Figure:    3.06 dB
Output Power              -50.00 dBm

Node 2: IF Amplifier Output
Input Power               -45.00 dBm
Block Gain:                10.00 dB
Block NF:                   6.00 dB
Cumulative Gain:           45.00 dB
Cumulative Noise Figure:    3.06 dB
Output Power              -35.00 dBm

Final Cascade Summary:
----------------------
Number of Blocks: 2
Pin:      -60.00 dBm
Pout:     -35.00 dBm
Gain:      45.00 dB
NF:         3.06 dB
'/Users/iancleary/Development/gainlineup/files/wideband.toml' is a Unix Absolute path.
Generating HTML table at: /Users/iancleary/Development/gainlineup/files/wideband.toml.html
file_path in get_file_url function: /Users/iancleary/Development/gainlineup/files/wideband.toml.html
You can open the plot in your browser at:
file:///Users/iancleary/Development/gainlineup/files/wideband.toml.html
Attempting to open plot in your default browser...
Success! Opening: file:///Users/iancleary/Development/gainlineup/files/wideband.toml.html
```

You can view an example of the html output at `files/wideband.toml.html`

[![HTML file created for the files directory by running `gainlineup files/wideband.toml` in the root of this directory](https://github.com/iancleary/gainlineup/blob/main/files/wideband.toml.html.png?raw=true)](https://github.com/iancleary/gainlineup/tree/main/files/wideband.toml.html)

You can view the HTML source file itself here directly: [files/wideband.toml.html](https://github.com/iancleary/gainlineup/tree/main/files/wideband.toml.html).


### Configuration Field Aliases

You can use the following field names with or without unit suffixes. The suffixes are the default, but the aliases are supported for brevity:

| Original Field | Alias (Optional) |
| :--- | :--- |
| `gain_db` | `gain` |
| `noise_figure_db` | `noise_figure`, `nf` ([Noise Figure is NF, Noise Factor is F](https://en.wikipedia.org/wiki/Noise_figure)) |
| `output_p1db_dbm` | `output_p1db`, `op1db` |
| `output_ip3_dbm` | `output_ip3`, `oip3` |
| `input_power_dbm` | `input_power`, `pin` |
| `frequency_hz` | `frequency`, `f` |
| `bandwidth_hz` | `bandwidth`, `bw` |
| `noise_temperature_k` | `noise_temperature` |

You could define the same configuration as above with the following toml:

```toml
pin = -60.0
f = 6.0e9
bw = 1.0e6

[[blocks]]
type = "explicit"
name = "Low Noise Amplifier"
gain = 20.0
nf = 3.0

[[blocks]]
type = "explicit"
name = "Mixer"
gain = 10.0
nf = 6.0

[[blocks]]
type = "explicit"
name = "IF Amplifier"
gain = 15.0
nf = 5.0
```

> However, the unit suffixes are still supported, and are recommended for clarity.
> Be careful as aliases hide the unit suffixes and might cause unexpected behavior, if you are assuming a different unit suffix than the code...
> For example, if you assume `pin` is in dBW, but the code assumes `pin` is in dBm, you will get unexpected results.  Similarly, if you assume `f` is in GHz, but the code assumes `f` is in Hz, you will get unexpected results.
> Also note that nf is the lower case shorthand for noise figure, since NF is used for Noise Figure, while F is used for Noise Factor, see [Wikipedia](https://en.wikipedia.org/wiki/Noise_factor) for more information.

## Cascade Analysis

The cascade calculates the following cumulative parameters at each node:

| Parameter | Description |
| :--- | :--- |
| Signal Power (dBm) | Cumulative signal level through the chain |
| Gain (dB) | Cumulative gain |
| Noise Figure (dB) | Cascaded noise figure (Friis equation) |
| OIP3 (dBm) | Cascaded output third-order intercept point |
| SFDR (dB) | Spur-free dynamic range: 2/3 × (OIP3 − noise floor) |

OIP3 and SFDR are computed when blocks have `output_ip3_dbm` set. SFDR requires bandwidth > 0 to calculate the noise floor.

## Features

### Debug Output

To enable debug printing, you can enable the `debug-print` feature.

```toml
[dependencies]
gainlineup = { version = "0.18.0", features = ["debug-print"] }
```
