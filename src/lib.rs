mod block;

#[cfg(feature = "cli")]
pub mod cli;
mod constants;
mod file_operations;
mod input;
mod node;
mod open;

#[cfg(feature = "plot")]
mod plot;

mod amplifier_model;

pub use amplifier_model::{AmplifierModel, AmplifierModelBuilder, AmplifierPoint};
pub use block::{Block, Imd3Point};
pub use input::Input;
pub use node::{DynamicRange, SignalNode};

// returns final output signal node, handling compression point if present
pub fn cascade_vector_return_output(input: Input, blocks: Vec<Block>) -> SignalNode {
    let mut cascading_signal: SignalNode = SignalNode::default(); // will be overwritten in first iteration

    for (i, block) in blocks.iter().enumerate() {
        if i == 0 {
            cascading_signal = input.cascade_block(block);
        } else {
            cascading_signal = cascading_signal.cascade_block(block);
        }
    }

    cascading_signal
}

// returns vector of output signal nodes, handling compression point if present
pub fn cascade_vector_return_vector(input: Input, blocks: Vec<Block>) -> Vec<SignalNode> {
    let mut cascading_signal: SignalNode = SignalNode::default(); // will be overwritten in first iteration

    // initialize node vector without input node, since the signal nodes are created in the loop and start with the output of the first block
    let mut node_vector: Vec<SignalNode> = vec![];
    for (i, block) in blocks.iter().enumerate() {
        if i == 0 {
            cascading_signal = input.cascade_block(block);
        } else {
            cascading_signal = cascading_signal.cascade_block(block);
        }
        node_vector.push(cascading_signal.clone());
    }
    node_vector
}

/// Sweep input power through a cascade of blocks and return the AM-AM curve.
///
/// For each input power, the signal is passed through every block in sequence
/// using each block's compression model. Returns Vec of `(Pin_dBm, Pout_dBm)`.
pub fn cascade_am_am_sweep(
    blocks: &[Block],
    start_dbm: f64,
    stop_dbm: f64,
    step_db: f64,
) -> Vec<(f64, f64)> {
    let mut powers = vec![];
    let mut pin = start_dbm;
    while pin <= stop_dbm + step_db * 0.01 {
        powers.push(pin);
        pin += step_db;
    }
    powers
        .iter()
        .map(|&pin| {
            let mut power = pin;
            for block in blocks {
                power = block.output_power(power);
            }
            (pin, power)
        })
        .collect()
}

/// Sweep input power through a cascade and return gain compression curve.
///
/// Returns Vec of `(Pin_dBm, Gain_dB)` showing total cascade gain vs. input power.
pub fn cascade_gain_compression_sweep(
    blocks: &[Block],
    start_dbm: f64,
    stop_dbm: f64,
    step_db: f64,
) -> Vec<(f64, f64)> {
    cascade_am_am_sweep(blocks, start_dbm, stop_dbm, step_db)
        .iter()
        .map(|&(pin, pout)| (pin, pout - pin))
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn two_part_node_cascade_vector_return_output() {
        let input_power: f64 = -30.0;
        let input = super::Input {
            power_dbm: input_power,
            frequency_hz: 1.0e9, // 1 GHz
            bandwidth_hz: 0.0,   // CW
            noise_temperature_k: Some(270.0),
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let blocks = vec![amplifier, attenuator];
        let output_node = super::cascade_vector_return_output(input, blocks);

        assert_eq!(output_node.signal_power_dbm, -6.0);
        assert_eq!(output_node.cumulative_gain_db, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_node.cumulative_noise_figure_db * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 3.006);
    }

    #[test]
    fn two_part_node_cascade_vector_return_vector() {
        let input_power: f64 = -30.0;
        let input = super::Input {
            power_dbm: input_power,
            frequency_hz: 1.0e9, // 1 GHz
            bandwidth_hz: 0.0,   // CW
            noise_temperature_k: Some(270.0),
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let blocks = vec![amplifier, attenuator];
        let cascade_vector = super::cascade_vector_return_vector(input, blocks);

        let output_node = cascade_vector.last().unwrap();
        assert_eq!(output_node.signal_power_dbm, -6.0);
        assert_eq!(output_node.cumulative_gain_db, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_node.cumulative_noise_figure_db * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 3.006);
    }

    #[test]
    fn cascade_am_am_linear() {
        let blocks = vec![
            super::Block {
                name: "LNA".to_string(),
                gain_db: 20.0,
                noise_figure_db: 3.0,
                output_p1db_dbm: None,
                output_ip3_dbm: None,
            },
            super::Block {
                name: "Atten".to_string(),
                gain_db: -6.0,
                noise_figure_db: 6.0,
                output_p1db_dbm: None,
                output_ip3_dbm: None,
            },
        ];
        let sweep = super::cascade_am_am_sweep(&blocks, -40.0, -20.0, 10.0);
        assert_eq!(sweep.len(), 3);
        // Total gain = 20 - 6 = 14 dB
        assert!((sweep[0].1 - (-26.0)).abs() < 0.01); // -40 + 14 = -26
        assert!((sweep[1].1 - (-16.0)).abs() < 0.01); // -30 + 14 = -16
        assert!((sweep[2].1 - (-6.0)).abs() < 0.01); // -20 + 14 = -6
    }

    #[test]
    fn cascade_am_am_with_compression() {
        let blocks = vec![
            super::Block {
                name: "LNA".to_string(),
                gain_db: 30.0,
                noise_figure_db: 3.0,
                output_p1db_dbm: Some(5.0),
                output_ip3_dbm: None,
            },
            super::Block {
                name: "Driver".to_string(),
                gain_db: 10.0,
                noise_figure_db: 5.0,
                output_p1db_dbm: Some(15.0),
                output_ip3_dbm: None,
            },
        ];
        let sweep = super::cascade_am_am_sweep(&blocks, -50.0, 0.0, 10.0);
        // At -50: LNA out = -20, Driver out = -10 (linear)
        assert!((sweep[0].1 - (-10.0)).abs() < 0.01);
        // At high power, should compress
        let last = sweep.last().unwrap();
        assert!(last.1 <= 16.0, "Should compress at high input");
    }

    #[test]
    fn cascade_gain_compression() {
        let blocks = vec![super::Block {
            name: "Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        }];
        let sweep = super::cascade_gain_compression_sweep(&blocks, -40.0, 0.0, 10.0);
        // At -40: linear, gain = 20
        assert!((sweep[0].1 - 20.0).abs() < 0.01);
        // At 0: compressed, gain < 20
        let last = sweep.last().unwrap();
        assert!(last.1 < 20.0, "Gain should compress at high input");
    }

    #[test]
    fn two_part_node_cascade_vector_return_vector_with_compression() {
        let input_power: f64 = -30.0;
        let input = super::Input {
            power_dbm: input_power,
            frequency_hz: 1.0e9, // 1 GHz
            bandwidth_hz: 0.0,   // CW
            noise_temperature_k: Some(270.0),
        };
        let low_noise_amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(5.0),
            output_ip3_dbm: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let high_power_amplifier = super::Block {
            name: "High Power Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(20.0),
            output_ip3_dbm: None,
        };
        let blocks = vec![low_noise_amplifier, attenuator, high_power_amplifier];
        let cascade_vector = super::cascade_vector_return_vector(input, blocks);

        let output_node = cascade_vector.last().unwrap();
        assert_eq!(output_node.signal_power_dbm, 21.0);
        assert_eq!(output_node.cumulative_gain_db, 51.0);

        assert_eq!(output_node.name, "High Power Amplifier Output");

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_node.cumulative_noise_figure_db * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 3.008);
    }
}
