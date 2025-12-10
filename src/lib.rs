mod block;
pub mod cli;
mod file_operations;
mod node;
mod open;
mod plot;

pub use block::Block;
pub use node::SignalNode;

// returns final output signal node, handling compression point if present
pub fn cascade_vector_return_output(input: SignalNode, blocks: Vec<Block>) -> SignalNode {
    let mut cascading_signal = input;

    for block in blocks {
        cascading_signal = cascading_signal.cascade_block(&block);
    }
    cascading_signal
}

// returns vector of output signal nodes, handling compression point if present
pub fn cascade_vector_return_vector(
    input_signal: SignalNode,
    blocks: Vec<Block>,
) -> Vec<SignalNode> {
    let mut cascading_signal = input_signal;
    let mut node_vector: Vec<SignalNode> = vec![cascading_signal.clone()];
    for block in blocks.iter() {
        cascading_signal = cascading_signal.cascade_block(block);
        node_vector.push(cascading_signal.clone());
    }
    node_vector
}

#[cfg(test)]
mod tests {
    #[test]
    fn two_part_node_cascade_vector_return_output() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_figure: 0.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_p1db: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_p1db: None,
        };
        let blocks = vec![amplifier, attenuator];
        let output_node = super::cascade_vector_return_output(input_node, blocks);

        assert_eq!(output_node.power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        assert_eq!(output_node.noise_figure, 3.018922107070044);
    }

    #[test]
    fn two_part_node_cascade_vector_return_vector() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_figure: 0.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_p1db: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_p1db: None,
        };
        let blocks = vec![amplifier, attenuator];
        let cascade_vector = super::cascade_vector_return_vector(input_node, blocks);

        let output_node = cascade_vector.last().unwrap();
        assert_eq!(output_node.power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        assert_eq!(output_node.noise_figure, 3.018922107070044);
    }

    #[test]
    fn two_part_node_cascade_vector_return_vector_with_compression() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_figure: 0.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let low_noise_amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_p1db: Some(5.0),
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_p1db: None,
        };
        let high_power_amplifier = super::Block {
            name: "High Power Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_p1db: Some(20.0),
        };
        let blocks = vec![low_noise_amplifier, attenuator, high_power_amplifier];
        let cascade_vector = super::cascade_vector_return_vector(input_node, blocks);

        let output_node = cascade_vector.last().unwrap();
        assert_eq!(output_node.power, 21.0);
        assert_eq!(output_node.cumulative_gain, 51.0);

        assert_eq!(output_node.name, "High Power Amplifier Output");
        assert_eq!(output_node.noise_figure, 3.020645644372404);
    }
}
