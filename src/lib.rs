// the input to our `create_user` handler
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub gain: f64,                                 // dB
    pub noise_figure: f64, // dB, nf would be ambiguous between noise factor and noise figure
    pub output_1db_compression_point: Option<f64>, // dBm
}

impl Block {
    pub fn new(
        name: String,
        gain: f64,
        noise_figure: f64,
        output_1db_compression_point: Option<f64>,
    ) -> Block {
        Block {
            name,
            gain,
            noise_figure,
            output_1db_compression_point,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SignalNode {
    pub name: String,
    pub power: f64,             // dBm
    pub noise_temperature: f64, // cumulative, dB
    pub cumulative_gain: f64,   // cumulative, dB (set to 0 at start)
}

// returns output power, handling compression point if present
pub fn cascade(input_power: f64, block1: Block) -> f64 {
    let output_power_without_compression = input_power + block1.gain;
    if let Some(op1db) = block1.output_1db_compression_point {
        if output_power_without_compression > op1db + 1.0 {
            return op1db + 1.0;
        }
    }
    output_power_without_compression
}

// returns output signal node, handling compression point if present
pub fn cascade_node(signal: SignalNode, block1: Block) -> SignalNode {
    let output_node_name = block1.name + " Output";
    let block_noise_temperature =
        rfconversions::noise::noise_temperature_from_noise_figure(block1.noise_figure);
    let cumulative_gain_linear = rfconversions::power::db_to_linear(signal.cumulative_gain)
        + rfconversions::power::db_to_linear(block1.gain);

    // handle compression point
    let output_power_without_compression = signal.power + block1.gain;
    let output_power = if let Some(op1db) = block1.output_1db_compression_point {
        if output_power_without_compression > op1db + 1.0 {
            op1db + 1.0
        } else {
            output_power_without_compression
        }
    } else {
        output_power_without_compression
    };

    SignalNode {
        name: output_node_name,
        power: output_power,
        noise_temperature: signal.noise_temperature
            + block_noise_temperature / cumulative_gain_linear,
        cumulative_gain: signal.cumulative_gain + block1.gain,
    }
}

// returns final output signal node, handling compression point if present
pub fn cascade_vector_return_output(input_signal: SignalNode, blocks: Vec<Block>) -> SignalNode {
    let mut cascading_signal = input_signal;

    for block in blocks {
        cascading_signal = cascade_node(cascading_signal, block);
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
        cascading_signal = cascade_node(cascading_signal, block.clone());
        node_vector.push(cascading_signal.clone());
    }
    node_vector
}

// This module contains tests for the cascade function and the Node struct

#[cfg(test)]
mod tests {
    #[test]
    fn one_part() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let output_power = super::cascade(input_power, amplifier);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn one_part_new() {
        let input_power: f64 = -30.0;
        let name = "Simple Amplifier".to_string();
        let gain = 10.0;
        let noise_figure = 3.0;
        let amplifier = super::Block::new(name, gain, noise_figure, None);
        let output_power = super::cascade(input_power, amplifier);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn one_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let output_node = super::cascade_node(input_node, amplifier);

        assert_eq!(output_node.power, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.202456829285537);
    }

    #[test]
    fn one_part_lna_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };

        let output_node = super::cascade_node(input_node, amplifier);

        assert_eq!(output_node.power, 0.0);
        assert_eq!(output_node.name, "Low Noise Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.0124584457866126);
    }

    #[test]
    fn two_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_1db_compression_point: None,
        };
        let intermediate_node = super::cascade_node(input_node, amplifier);

        assert_eq!(intermediate_node.cumulative_gain, 30.0);

        let output_node = super::cascade_node(intermediate_node, attenuator);

        assert_eq!(output_node.power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.018922107070044);
    }

    #[test]
    fn two_part_node_cascade_vector_return_output() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_1db_compression_point: None,
        };
        let blocks = vec![amplifier, attenuator];
        let output_node = super::cascade_vector_return_output(input_node, blocks);

        assert_eq!(output_node.power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.018922107070044);
    }

    #[test]
    fn two_part_node_cascade_vector_return_vector() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_1db_compression_point: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
            output_1db_compression_point: None,
        };
        let blocks = vec![amplifier, attenuator];
        let cascade_vector = super::cascade_vector_return_vector(input_node, blocks);

        let output_node = cascade_vector.last().unwrap();
        assert_eq!(output_node.power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(
            output_node.noise_temperature,
        );
        assert_eq!(output_noise_figure, 3.018922107070044);
    }
}
