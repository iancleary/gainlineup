# gainlineup

Gain Lineups for RF Engineering

## Example

```rust
use gainlineup::cascade::cascade_vector_return_output;
use gainlineup::cascade::cascade_vector_return_vector;
use gainlineup::cascade::GainBlock;
use gainlineup::cascade::SignalNode;

const INPUT_POWER: f64 = 10.0; // dBm

let input_node = SignalNode {
    name: "Input".to_string(),
    power: INPUT_POWER,
    noise_temperature: 290.0,
    cumulative_gain: 0.0, // starting/initial/input node of cascade
};

let cable_from_signal_generator = GainBlock {
    name: "SMA Cable from Signal Generator".to_string(),
    gain: -6.0,
    noise_figure: 6.0,
};

let line_amp: GainBlock = GainBlock {
    name: "Line Amp at X GHz".to_string(),
    gain: 22.0,
    noise_figure: 6.0,
};

let cable_run_to_spectrum_analzyer: GainBlock = GainBlock {
    name: "Cable Run Device Under Test (DUT) to Spectrum Analyzer".to_string(),
    gain: -12.0,
    noise_figure: 12.0,
};

let blocks = vec![
        cable_from_signal_generator.clone(),
        line_amp.clone(),
        cable_run_to_spectrum_analzyer.clone(),
    ];

let full_cascade: Vec<SignalNode> =
  cascade_vector_return_vector(input_node.clone(), blocks.clone());

// println!("{:>8.2} dBm", node.power);`


for (i, node) in full_cascade.iter().enumerate() {
  println!("");
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
          "Block Gain:\t{:>8.2} dB\t(Cumulative Gain: {:>8.2} dB)",
          block_gain, node.cumulative_gain
      );
      println!("Output Power\t{:>8.2} dBm", node.power);
  }
}
```

> `println!("Output Power\t{:>8.2} dBm", node.power);`
> 
> the formatting `{:>8.2}` aligns positive and negative numbers on the decimal,
> 
> with two digits after the decimal (hundredths place)
