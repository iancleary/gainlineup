// the input to our `create_user` handler
#[derive(Debug)]
pub struct GainBlock {
    name: String,
    gain: f64, // dB
    noise_figure: f64, // dB, nf would be ambiguous between noise factor and noise figure
}

#[derive(Debug)]
pub struct SignalNode {
    name: String,
    power: f64, // dBm
    noise_temperature: f64, // cumulative, dB
    cumulative_gain: f64, // cumulative, dB (set to 0 at start)
}

pub fn db_to_linear(value: f64) -> f64 {
    10.0_f64.powf(value / 10.0)
}

pub fn linear_to_db(value: f64) -> f64 {
    10.0 * f64::log10(value)
}

pub fn cascade(input_power: f64, block1: GainBlock) -> f64 {
    input_power + block1.gain
}

pub fn cascade_node(signal: SignalNode, block1: GainBlock) -> SignalNode {
    let output_node_name = block1.name + " Output";
    let block_noise_temperature = rfconversions::noise::noise_temperature_from_noise_figure(block1.noise_figure);
    let cumulative_gain_linear = db_to_linear(signal.cumulative_gain) + db_to_linear(block1.gain);
    SignalNode {
        name: output_node_name,
        power: signal.power + block1.gain,
        noise_temperature: signal.noise_temperature + block_noise_temperature/cumulative_gain_linear,
        cumulative_gain: signal.cumulative_gain + block1.gain
    }
}

#[cfg(test)]
mod tests {
    use crate::cascade::cascade;
    use crate::cascade::cascade_node;
    use crate::cascade::GainBlock;
    use crate::cascade::SignalNode;

    #[test]
    fn one_part() {
        let input_power: f64 = -30.0;
        let amplifier = GainBlock {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
        };
        let output_power = cascade(input_power, amplifier);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn one_part_node() {
        let input_power: f64 = -30.0;
        let input_node = SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = GainBlock {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
        };
        let output_node = cascade_node(input_node, amplifier);

        assert_eq!(output_node.power, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(output_node.noise_temperature);
        assert_eq!(output_noise_figure, 3.202456829285537);
    }

    #[test]
    fn one_part_lna_node() {
        let input_power: f64 = -30.0;
        let input_node = SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = GainBlock {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
        };
       
        let output_node = cascade_node(input_node, amplifier);

        assert_eq!(output_node.power, 0.0);
        assert_eq!(output_node.name, "Low Noise Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(output_node.noise_temperature);
        assert_eq!(output_noise_figure, 3.0124584457866126);
    }

    #[test]
    fn two_part_node() {
        let input_power: f64 = -30.0;
        let input_node = SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_temperature: 290.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = GainBlock {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
        };
        let attenuator = GainBlock {
            name: "Attenuator".to_string(),
            gain: -6.0,
            noise_figure: 6.0,
        };
        let intermediate_node = cascade_node(input_node, amplifier);

        assert_eq!(intermediate_node.cumulative_gain, 30.0);

        let output_node = cascade_node(intermediate_node, attenuator);

        assert_eq!(output_node.power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = rfconversions::noise::noise_figure_from_noise_temperature(output_node.noise_temperature);
        assert_eq!(output_noise_figure, 3.018922107070044);
    }
}
