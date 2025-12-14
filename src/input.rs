use std::default::Default;
use std::fmt;

use crate::block::Block;
use crate::constants;
use crate::node::SignalNode;

// the input is the signal that enters the cascade, which is different than the nodes
// that are the outputs of each block, see block.rs for more details
#[derive(Clone, Debug)]
pub struct Input {
    pub frequency: f64,                 // Hz, center frequency of signal
    pub bandwidth: f64,                 // Hz, width of signal
    pub power: f64,                     // dBm, power of signal
    pub noise_temperature: Option<f64>, // K, noise temperature of signal
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Input {{ frequency: {}, bandwidth: {}, power: {} }}",
            self.frequency, self.bandwidth, self.power
        )
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            frequency: 0.0,   // placeholder value, you should change this (0 Hz)
            bandwidth: 100.0, // placeholder value, you should change this (100 Hz)
            // https://www.w8ji.com/cw_bandwidth_described.htm describes how CW signals are generally made, which require non-zero bandwidth
            power: 0.0, // placeholder value, you should change this (0 dBm)
            noise_temperature: None,
        }
    }
}

impl Input {
    pub fn new(
        frequency: f64,
        bandwidth: f64,
        power: f64,
        noise_temperature: Option<f64>,
    ) -> Input {
        Input {
            frequency,
            bandwidth,
            power,
            noise_temperature,
        }
    }

    pub fn noise_spectral_density(&self) -> f64 {
        let k = constants::BOLTZMANN;
        let t = self.noise_temperature.unwrap_or(270.0);
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

    // input noise power (kTB), thermal noise
    pub fn noise_power(&self) -> f64 {
        let k = constants::BOLTZMANN;
        let t = self.noise_temperature.unwrap_or(270.0);
        let noise_power = k * t * self.bandwidth;

        println!("Noise Power: (W) {}", noise_power);

        let noise_power_dbm = rfconversions::power::watts_to_dbm(noise_power);

        println!("Noise Power: (dBm) {}", noise_power_dbm);

        noise_power_dbm
    }

    pub fn cascade_block(&self, block: &Block) -> SignalNode {
        println!("Start INPUT");

        let output_node_name = block.name.clone() + " Output";

        let block_noise_factor =
            rfconversions::noise::noise_factor_from_noise_figure(block.noise_figure);

        let block_noise_temperature =
            rfconversions::noise::noise_temperature_from_noise_factor(block_noise_factor);

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

        let stage_power_gain = output_power - self.power;

        let stage_power_gain_linear = rfconversions::power::db_to_linear(stage_power_gain);

        let cumulative_noise_factor = block_noise_factor;

        let cumulative_noise_figure =
            rfconversions::noise::noise_figure_from_noise_factor(cumulative_noise_factor);

        let cumulative_noise_temperature = if self.noise_temperature.is_some() {
            let noise_temperature = self.noise_temperature.unwrap();
            Some(noise_temperature + block_noise_temperature / stage_power_gain_linear)
        } else {
            Some(270.0 + block_noise_temperature / stage_power_gain_linear)
        };

        let input_noise_power = self.noise_power();

        println!("Input Noise Power: (dBm) {}", input_noise_power);
        let output_noise_power_from_input_dbm = input_noise_power + stage_power_gain;

        let output_noise_power_from_block_dbm =
            block.output_noise_power(self.bandwidth, self.power);

        println!(
            "Output Noise Power from Input: (dBm) {}",
            output_noise_power_from_input_dbm
        );
        println!(
            "Output Noise Power from Block: (dBm) {}",
            output_noise_power_from_block_dbm
        );

        let output_noise_power_from_input_watts =
            rfconversions::power::dbm_to_watts(output_noise_power_from_input_dbm);

        let output_noise_power_from_block_watts =
            rfconversions::power::dbm_to_watts(output_noise_power_from_block_dbm);

        let total_noise_power_at_output_watts =
            output_noise_power_from_input_watts + output_noise_power_from_block_watts;

        println!(
            "Total Noise Power at Output: (W) {}",
            total_noise_power_at_output_watts
        );

        let output_noise_power_at_output_dbm =
            rfconversions::power::watts_to_dbm(total_noise_power_at_output_watts);

        println!(
            "Output Noise Power at Output: (dBm) {}",
            output_noise_power_at_output_dbm
        );

        println!("End INPUT");

        SignalNode {
            name: output_node_name,
            signal_power: output_power,
            signal_frequency: self.frequency,
            signal_bandwidth: self.bandwidth,
            cumulative_noise_figure,
            cumulative_gain: stage_power_gain,
            cumulative_noise_temperature,
            noise_power: output_noise_power_at_output_dbm,
        }
    }
}
