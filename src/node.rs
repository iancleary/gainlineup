use std::default::Default;
use std::fmt;

use crate::block::Block;

#[derive(Clone, Debug)]
pub struct SignalNode {
    pub name: String,         // name of node, like "Input" or "Amplifier 1 Output"
    pub power: f64,           // dBm
    pub frequency: f64,       // Hz
    pub bandwidth: f64,       // Hz
    pub noise_figure: f64,    // dB, linear
    pub cumulative_gain: f64, // cumulative, dB (set to 0 at start)
}

impl fmt::Display for SignalNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SignalNode {{ name: {}, power: {}, frequency: {}, bandwidth: {}, noise_figure: {}, cumulative_gain: {} }}",
            self.name, self.power, self.frequency, self.bandwidth, self.noise_figure, self.cumulative_gain
        )
    }
}

impl Default for SignalNode {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            power: 0.0,           // placeholder value, you should change this
            frequency: 0.0,       // placeholder value, you should change this
            bandwidth: 0.0,       // placeholder value, you should change this
            noise_figure: 0.0,    // no contribution
            cumulative_gain: 1.0, // default assuming start of cascade
        }
    }
}

impl SignalNode {
    pub fn new(
        name: String,
        power: f64,
        frequency: f64,
        bandwidth: f64,
        noise_figure: f64,
        cumulative_gain: f64,
    ) -> SignalNode {
        SignalNode {
            name,
            power,
            frequency,
            bandwidth,
            noise_figure,
            cumulative_gain,
        }
    }

    pub fn noise_spectral_density(&self) -> f64 {
        // let k = rfconversions::constants::BOLTZMANN;
        let k = 1.380649e-23;
        let t = self.noise_temperature();
        let noise_spectral_density = k * t;

        println!("Noise Spectral Density: (W/Hz) {}", noise_spectral_density);

        let noise_spectral_density_dbm_per_hz =
            rfconversions::power::watts_to_dbm(noise_spectral_density);

        println!(
            "Noise Spectral Density: (dBm/Hz) {}",
            noise_spectral_density_dbm_per_hz
        );

        noise_spectral_density_dbm_per_hz
    }

    pub fn noise_power(&self) -> f64 {
        // let k = rfconversions::constants::BOLTZMANN;
        let k = 1.380649e-23;
        let t = self.noise_temperature();
        let noise_power = k * t * self.bandwidth;

        println!("Noise Power: (W) {}", noise_power);

        let noise_power_dbm = rfconversions::power::watts_to_dbm(noise_power);

        println!("Noise Power: (dBm) {}", noise_power_dbm);

        noise_power_dbm
    }

    pub fn signal_to_noise_ratio(&self) -> f64 {
        // dBm - dBm = dB
        self.power - self.noise_power()
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

        let output_frequency = self.frequency;
        let output_bandwidth = self.bandwidth;

        // TODO: handle frequency and bandwidth changes, i.e. mixers, filters, etc.

        SignalNode {
            name: output_node_name,
            power: output_power,
            frequency: output_frequency,
            bandwidth: output_bandwidth,
            noise_figure: cumulative_noise_figure,
            cumulative_gain: self.cumulative_gain + stage_gain,
        }
    }

    pub fn noise_factor(&self) -> f64 {
        rfconversions::noise::noise_factor_from_noise_figure(self.noise_figure)
    }

    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.noise_figure)
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
            frequency: 1.0e9,     // Hz
            bandwidth: 1.0e6,     // Hz
            noise_figure: 5.0,    // cumulative noise figure
            cumulative_gain: 0.0, // starting/initial/input node of cascade
        };
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 5.0,
            output_p1db: None,
        };
        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.power, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.noise_figure;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.262);
        assert_eq!(output_node.frequency, 1.0e9);
        assert_eq!(output_node.bandwidth, 1.0e6);
    }

    #[test]
    fn one_part_lna_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            frequency: 1.0e9,     // Hz
            bandwidth: 1.0e6,     // Hz
            noise_figure: 5.0,    // cumulative noise figure
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

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.001);
        assert_eq!(output_node.frequency, 1.0e9);
        assert_eq!(output_node.bandwidth, 1.0e6);
    }

    #[test]
    fn two_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            power: input_power,
            frequency: 1.0e9,     // Hz
            bandwidth: 1.0e6,     // Hz
            noise_figure: 5.0,    // cumulative noise figure
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

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.005);
        assert_eq!(output_node.frequency, 1.0e9);
        assert_eq!(output_node.bandwidth, 1.0e6);
    }
}
