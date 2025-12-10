use std::default::Default;
use std::fmt;

use crate::block::Block;

#[derive(Clone, Debug)]
pub struct SignalNode {
    pub name: String,         // name of node, like "Input" or "Amplifier 1 Output"
    pub power: f64,           // dBm
    pub noise_figure: f64,    // dB, linear
    pub cumulative_gain: f64, // cumulative, dB (set to 0 at start)
}

impl fmt::Display for SignalNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SignalNode {{ name: {}, power: {}, noise_figure: {}, cumulative_gain: {} }}",
            self.name, self.power, self.noise_figure, self.cumulative_gain
        )
    }
}

impl Default for SignalNode {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            power: 0.0,           // placeholder value, you should change this
            noise_figure: 0.0,    // no contribution
            cumulative_gain: 0.0, // default assuming start of cascade
        }
    }
}

impl SignalNode {
    pub fn new(name: String, power: f64, noise_figure: f64, cumulative_gain: f64) -> SignalNode {
        SignalNode {
            name,
            power,
            noise_figure,
            cumulative_gain,
        }
    }

    pub fn cascade_block(&self, block: &Block) -> SignalNode {
        let output_node_name = block.name.clone() + " Output";

        let block_noise_factor =
            rfconversions::noise::noise_factor_from_noise_figure(block.noise_figure);
        let cumulative_gain_linear = rfconversions::power::db_to_linear(self.cumulative_gain)
            + rfconversions::power::db_to_linear(block.gain);

        // handle compression point
        let output_power_without_compression = self.power + block.gain;
        let output_power = if let Some(output_p1db) = block.output_p1db {
            if output_power_without_compression > output_p1db + 1.0 {
                output_p1db + 1.0
            } else {
                output_power_without_compression
            }
        } else {
            output_power_without_compression
        };

        let stage_gain = output_power - self.power;

        let cumulative_noise_factor =
            self.noise_factor() + (block_noise_factor - 1.0) / cumulative_gain_linear;

        let cumulative_noise_figure =
            rfconversions::noise::noise_figure_from_noise_factor(cumulative_noise_factor);

        SignalNode {
            name: output_node_name,
            power: output_power,
            noise_figure: cumulative_noise_figure,
            cumulative_gain: self.cumulative_gain + stage_gain,
        }
    }

    pub fn noise_factor(&self) -> f64 {
        rfconversions::noise::noise_factor_from_noise_figure(self.noise_figure)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn one_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            noise_figure: 0.0,
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_p1db: None,
        };
        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.power, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.noise_figure;
        assert_eq!(output_noise_figure, 3.202456829285537);
    }

    #[test]
    fn one_part_lna_node() {
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

        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.power, 0.0);
        assert_eq!(output_node.name, "Low Noise Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.noise_figure;
        assert_eq!(output_noise_figure, 3.0124584457866126);
    }

    #[test]
    fn two_part_node() {
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
        let intermediate_node = input_node.cascade_block(&amplifier);

        assert_eq!(intermediate_node.cumulative_gain, 30.0);

        let output_node = intermediate_node.cascade_block(&attenuator);

        assert_eq!(output_node.power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.noise_figure;
        assert_eq!(output_noise_figure, 3.018922107070044);
    }
}
