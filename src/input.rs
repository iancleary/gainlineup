use std::default::Default;
use std::fmt;

use crate::block::Block;
use crate::constants;
use crate::node::SignalNode;

// the input is the signal that enters the cascade, which is different than the nodes
// that are the outputs of each block, see block.rs for more details
#[derive(Clone, Debug)]
pub struct Input {
    pub frequency_hz: f64,                // Hz, center frequency of signal
    pub bandwidth_hz: f64,                // Hz, width of signal
    pub power_dbm: f64,                   // dBm, power of signal
    pub noise_temperature_k: Option<f64>, // K, noise temperature of signal
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Input {{ frequency: {}, bandwidth: {}, power: {} }}",
            self.frequency_hz, self.bandwidth_hz, self.power_dbm
        )
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            frequency_hz: 0.0,   // placeholder value, you should change this (0 Hz)
            bandwidth_hz: 100.0, // placeholder value, you should change this (100 Hz)
            // https://www.w8ji.com/cw_bandwidth_described.htm describes how CW signals are generally made, which require non-zero bandwidth
            power_dbm: 0.0, // placeholder value, you should change this (0 dBm)
            noise_temperature_k: None,
        }
    }
}

impl Input {
    pub fn new(
        frequency_hz: f64,
        bandwidth_hz: f64,
        power_dbm: f64,
        noise_temperature_k: Option<f64>,
    ) -> Input {
        Input {
            frequency_hz,
            bandwidth_hz,
            power_dbm,
            noise_temperature_k,
        }
    }

    pub fn noise_spectral_density(&self) -> f64 {
        let k = constants::BOLTZMANN;
        let t = self.noise_temperature_k.unwrap_or(270.0);
        let noise_spectral_density = k * t;

        #[cfg(feature = "debug-print")]
        println!("Noise Spectral Density: (W/Hz) {}", noise_spectral_density);

        let noise_spectral_density_dbm_per_hz =
            rfconversions::power::watts_to_dbm(noise_spectral_density);

        #[cfg(feature = "debug-print")]
        println!(
            "Noise Spectral Density: (dBm/Hz) {}",
            noise_spectral_density_dbm_per_hz
        );

        noise_spectral_density_dbm_per_hz
    }

    // input noise power (kTB), thermal noise
    pub fn noise_power(&self) -> f64 {
        let k = constants::BOLTZMANN;
        let t = self.noise_temperature_k.unwrap_or(270.0);
        let noise_power = k * t * self.bandwidth_hz;

        #[cfg(feature = "debug-print")]
        println!("Noise Power: (W) {}", noise_power);

        let noise_power_dbm = rfconversions::power::watts_to_dbm(noise_power);

        #[cfg(feature = "debug-print")]
        println!("Noise Power: (dBm) {}", noise_power_dbm);

        noise_power_dbm
    }

    pub fn cascade_block(&self, block: &Block) -> SignalNode {
        #[cfg(feature = "debug-print")]
        println!("Start INPUT");

        let output_node_name = block.name.clone() + " Output";

        let block_noise_factor =
            rfconversions::noise::noise_factor_from_noise_figure(block.noise_figure_db);

        let block_noise_temperature =
            rfconversions::noise::noise_temperature_from_noise_factor(block_noise_factor);

        // handle compression point
        let output_power_dbm_without_compression = self.power_dbm + block.gain_db;
        let output_power_dbm = if let Some(output_p1db_dbm) = block.output_p1db_dbm {
            if output_power_dbm_without_compression > output_p1db_dbm + 1.0 {
                output_p1db_dbm + 1.0
            } else {
                output_power_dbm_without_compression
            }
        } else {
            output_power_dbm_without_compression
        };

        let stage_power_gain_db = output_power_dbm - self.power_dbm;

        let stage_power_gain_linear = rfconversions::power::db_to_linear(stage_power_gain_db);

        let cumulative_noise_factor = block_noise_factor;

        let cumulative_noise_figure =
            rfconversions::noise::noise_figure_from_noise_factor(cumulative_noise_factor);

        let cumulative_noise_temperature =
            if let Some(noise_temperature_k) = self.noise_temperature_k {
                Some(noise_temperature_k + block_noise_temperature / stage_power_gain_linear)
            } else {
                Some(270.0 + block_noise_temperature / stage_power_gain_linear)
            };

        let input_noise_power = self.noise_power();

        #[cfg(feature = "debug-print")]
        println!("Input Noise Power: (dBm) {}", input_noise_power);

        let output_noise_power_from_input_dbm = input_noise_power + stage_power_gain_db;

        let output_noise_power_from_block_dbm = block.output_noise_power(self.bandwidth_hz);

        #[cfg(feature = "debug-print")]
        println!(
            "Output Noise Power from Input: (dBm) {}",
            output_noise_power_from_input_dbm
        );

        #[cfg(feature = "debug-print")]
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

        #[cfg(feature = "debug-print")]
        println!(
            "Total Noise Power at Output: (W) {}",
            total_noise_power_at_output_watts
        );

        let output_noise_power_at_output_dbm =
            rfconversions::power::watts_to_dbm(total_noise_power_at_output_watts);

        #[cfg(feature = "debug-print")]
        println!(
            "Output Noise Power at Output: (dBm) {}",
            output_noise_power_at_output_dbm
        );

        #[cfg(feature = "debug-print")]
        println!("End INPUT");

        // OIP3: first block in chain, just use block's OIP3
        let cumulative_oip3_dbm = block.output_ip3_dbm;

        // SFDR calculation
        let sfdr_db = cumulative_oip3_dbm.map(|oip3| {
            let noise_floor_dbm = -174.0
                + 10.0 * self.bandwidth_hz.log10()
                + cumulative_noise_figure;
            2.0 / 3.0 * (oip3 - noise_floor_dbm)
        });

        SignalNode {
            name: output_node_name,
            signal_power_dbm: output_power_dbm,
            signal_frequency_hz: self.frequency_hz,
            signal_bandwidth_hz: self.bandwidth_hz,
            cumulative_noise_figure_db: cumulative_noise_figure,
            cumulative_gain_db: stage_power_gain_db,
            cumulative_noise_temperature,
            noise_power_dbm: output_noise_power_at_output_dbm,
            cumulative_oip3_dbm,
            sfdr_db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cascade_block() {
        let input = Input::new(100.0, 100.0, 0.0, None);
        let block = Block {
            name: "Test Block".to_string(),
            gain_db: 10.0,
            noise_figure_db: 10.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let signal_node = input.cascade_block(&block);
        assert_eq!(signal_node.name, "Test Block Output");
        assert_eq!(signal_node.signal_power_dbm, 10.0);
        assert_eq!(signal_node.signal_frequency_hz, 100.0);
        assert_eq!(signal_node.signal_bandwidth_hz, 100.0);
        assert_eq!(signal_node.cumulative_noise_figure_db, 10.0);
        assert_eq!(signal_node.cumulative_gain_db, 10.0);
        // 10 dB NF = factor 10, T = 290*(10-1) = 2610K
        // Added temp = 2610/10 = 261K
        // Total = 270 + 261 = 531K
        assert_eq!(signal_node.cumulative_noise_temperature, Some(531.0));
        // Noise power calculation: k*T*B where T=531K, B=100Hz
        assert!(
            (signal_node.noise_power_dbm - (-124.84)).abs() < 0.01,
            "Expected noise power around -124.84 dBm, got {}",
            signal_node.noise_power_dbm
        );
    }

    #[test]
    fn test_cascade_block_with_compression() {
        // Test that signal compresses but noise doesn't when signal is high and noise is low
        // Input: 0 dBm signal, thermal noise at 270K
        let input = Input::new(1.0e9, 1.0e6, 0.0, None);

        // Block: 20 dB gain, P1dB at 10 dBm output
        // Expected: signal output = 0 + 20 = 20 dBm (exceeds P1dB), so compresses to 11 dBm
        // Expected: noise << P1dB, so no compression on noise
        let block = Block {
            name: "Compressing Amplifier".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        };

        let signal_node = input.cascade_block(&block);

        assert_eq!(signal_node.name, "Compressing Amplifier Output");

        // Signal should compress to P1dB + 1 dB = 11 dBm (not 20 dBm)
        assert_eq!(signal_node.signal_power_dbm, 11.0);

        // Cumulative gain should reflect signal compression (11 dB actual gain, not 20 dB)
        assert_eq!(signal_node.cumulative_gain_db, 11.0);

        // Noise power should be well below the P1dB compression point
        // For 1 MHz bandwidth at ~270K, thermal noise is around -114 dBm
        // After block with 20 dB gain and 3 dB NF, noise should be around -91 dBm
        // This is well below the 10 dBm P1dB point, so no compression
        assert!(
            signal_node.noise_power_dbm < -50.0,
            "Noise power should be well below P1dB (got {} dBm), indicating no compression",
            signal_node.noise_power_dbm
        );

        assert_eq!(signal_node.signal_frequency_hz, 1.0e9);
        assert_eq!(signal_node.signal_bandwidth_hz, 1.0e6);
    }
}
