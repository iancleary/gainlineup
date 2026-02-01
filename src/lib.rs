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

pub use block::Block;
pub use input::Input;
pub use node::SignalNode;

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
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
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
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
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
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
        };
        let high_power_amplifier = super::Block {
            name: "High Power Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(20.0),
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
