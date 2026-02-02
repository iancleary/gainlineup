use std::default::Default;
use std::fmt;

use crate::block::Block;

#[derive(Clone, Debug)]
pub struct SignalNode {
    pub name: String,             // name of node, like "Input" or "Amplifier 1 Output"
    pub signal_frequency_hz: f64, // Hz
    pub signal_bandwidth_hz: f64, // Hz
    pub signal_power_dbm: f64,    // dBm
    pub noise_power_dbm: f64,     // dBm
    pub cumulative_noise_figure_db: f64, // dB, linear
    pub cumulative_gain_db: f64,  // cumulative, dB (set to 0 at start)
    pub cumulative_noise_temperature: Option<f64>,
}

impl fmt::Display for SignalNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SignalNode {{ name: {}, signal_power: {}, noise_power: {}, signal_frequency: {}, signal_bandwidth: {}, cumulative_noise_figure: {}, cumulative_gain: {} }}",
            self.name, self.signal_power_dbm, self.noise_power_dbm, self.signal_frequency_hz, self.signal_bandwidth_hz, self.cumulative_noise_figure_db, self.cumulative_gain_db
        )
    }
}

impl Default for SignalNode {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            signal_frequency_hz: 0.0, // placeholder value, you should change this
            signal_bandwidth_hz: 0.0, // placeholder value, you should change this
            signal_power_dbm: 0.0,    // placeholder value, you should change this
            noise_power_dbm: 0.0,     // placeholder value, you should change this
            cumulative_noise_figure_db: 0.0, // no contribution
            cumulative_gain_db: 1.0,  // default assuming start of cascade
            cumulative_noise_temperature: None,
        }
    }
}

impl SignalNode {
    pub fn noise_spectral_density(&self) -> f64 {
        let noise_spectral_density_dbm_per_hz =
            self.noise_power_dbm - self.signal_bandwidth_hz.log10() * 10.0;

        #[cfg(feature = "debug-print")]
        println!(
            "Noise Spectral Density: (dBm/Hz) {}",
            noise_spectral_density_dbm_per_hz
        );

        noise_spectral_density_dbm_per_hz
    }

    pub fn signal_to_noise_ratio_db(&self) -> f64 {
        let signal_to_noise_ratio_db = self.signal_power_dbm - self.noise_power_dbm;

        #[cfg(feature = "debug-print")]
        println!("Signal to Noise Ratio: (dB) {}", signal_to_noise_ratio_db);

        signal_to_noise_ratio_db
    }

    pub fn cascade_block(&self, block: &Block) -> SignalNode {
        #[cfg(feature = "debug-print")]
        println!("START NODE Cascade_block");

        let output_node_name = block.name.clone() + " Output";

        let block_noise_factor =
            rfconversions::noise::noise_factor_from_noise_figure(block.noise_figure_db);

        let block_noise_temperature =
            rfconversions::noise::noise_temperature_from_noise_factor(block_noise_factor);

        let stage_gain_linear = rfconversions::power::db_to_linear(block.gain_db);
        let cumulative_gain_linear =
            rfconversions::power::db_to_linear(self.cumulative_gain_db) + stage_gain_linear;

        // handle compression point
        // this is a simplification in that you can compress the block with noise
        let output_power_without_compression = self.signal_power_dbm + block.gain_db;
        let output_power_dbm = if let Some(output_p1db_dbm) = block.output_p1db_dbm {
            if output_power_without_compression > output_p1db_dbm + 1.0 {
                output_p1db_dbm + 1.0
            } else {
                output_power_without_compression
            }
        } else {
            output_power_without_compression
        };

        let stage_power_gain = output_power_dbm - self.signal_power_dbm;

        let cumulative_noise_factor =
            self.noise_factor() + (block_noise_factor - 1.0) / cumulative_gain_linear;

        let cumulative_noise_figure =
            rfconversions::noise::noise_figure_from_noise_factor(cumulative_noise_factor);

        let cumulative_noise_temperature =
            if let Some(noise_temperature) = self.cumulative_noise_temperature {
                Some(noise_temperature + block_noise_temperature / cumulative_gain_linear)
            } else {
                Some(270.0 + block_noise_temperature / cumulative_gain_linear)
            };

        let input_noise_power_dbm = self.noise_power_dbm;

        #[cfg(feature = "debug-print")]
        println!("Input Noise Power: (dBm) {}", input_noise_power_dbm);

        // handle compression point separately (as they are separate signals)
        let output_noise_power_without_compression = input_noise_power_dbm + block.gain_db;
        let output_noise_power_from_node_dbm = if let Some(output_p1db_dbm) = block.output_p1db_dbm
        {
            if output_noise_power_without_compression > output_p1db_dbm + 1.0 {
                output_p1db_dbm + 1.0
            } else {
                output_noise_power_without_compression
            }
        } else {
            output_noise_power_without_compression
        };

        // output noise power from block (independent of compression TODO: check this)
        let output_noise_power_from_block_dbm = block.output_noise_power(self.signal_bandwidth_hz);

        #[cfg(feature = "debug-print")]
        println!(
            "Output Noise Power from Node: (dBm) {}",
            output_noise_power_from_node_dbm
        );

        #[cfg(feature = "debug-print")]
        println!(
            "Output Noise Power from Block: (dBm) {}",
            output_noise_power_from_block_dbm
        );

        let output_noise_power_from_node_watts =
            rfconversions::power::dbm_to_watts(output_noise_power_from_node_dbm);

        let output_noise_power_from_block_watts =
            rfconversions::power::dbm_to_watts(output_noise_power_from_block_dbm);

        let total_noise_power_at_output_watts =
            output_noise_power_from_node_watts + output_noise_power_from_block_watts;

        #[cfg(feature = "debug-print")]
        println!(
            "Total Noise Power at Output: (W) {}",
            total_noise_power_at_output_watts
        );

        let total_noise_power_at_output_dbm =
            rfconversions::power::watts_to_dbm(total_noise_power_at_output_watts);

        #[cfg(feature = "debug-print")]
        println!(
            "Total Noise Power at Output: (dBm) {}",
            total_noise_power_at_output_dbm
        );

        let output_frequency_hz = self.signal_frequency_hz;
        let output_bandwidth_hz = self.signal_bandwidth_hz;

        // TODO: handle frequency and bandwidth changes, i.e. mixers, filters, etc.

        #[cfg(feature = "debug-print")]
        println!("END NODE Cascade_block");

        SignalNode {
            name: output_node_name,
            signal_frequency_hz: output_frequency_hz,
            signal_bandwidth_hz: output_bandwidth_hz,
            signal_power_dbm: output_power_dbm,
            noise_power_dbm: total_noise_power_at_output_dbm,
            cumulative_noise_figure_db: cumulative_noise_figure,
            cumulative_gain_db: self.cumulative_gain_db + stage_power_gain,
            cumulative_noise_temperature,
        }
    }

    pub fn noise_factor(&self) -> f64 {
        rfconversions::noise::noise_factor_from_noise_figure(self.cumulative_noise_figure_db)
    }

    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.cumulative_noise_figure_db)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn one_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: input_power,
            noise_power_dbm: -174.0 * 10.0 * f64::log10(1.0e6), // assumes T = 290 K
            signal_frequency_hz: 1.0e9,                         // Hz
            signal_bandwidth_hz: 1.0e6,                         // Hz
            cumulative_noise_figure_db: 5.0,                    // cumulative noise figure
            cumulative_gain_db: 0.0, // starting/initial/input node of cascade
            cumulative_noise_temperature: None,
        };
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 5.0,
            output_p1db_dbm: None,
        };
        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.signal_power_dbm, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure_db;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.262);
        assert_eq!(output_node.signal_frequency_hz, 1.0e9);
        assert_eq!(output_node.signal_bandwidth_hz, 1.0e6);
    }

    #[test]
    fn one_part_lna_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: input_power,
            signal_frequency_hz: 1.0e9,                         // Hz
            signal_bandwidth_hz: 1.0e6,                         // Hz
            noise_power_dbm: -174.0 * 10.0 * f64::log10(1.0e6), // assumes T = 290 K
            cumulative_noise_figure_db: 5.0,                    // cumulative noise figure
            cumulative_gain_db: 0.0, // starting/initial/input node of cascade
            cumulative_noise_temperature: None,
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
        };

        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.signal_power_dbm, 0.0);
        assert_eq!(output_node.name, "Low Noise Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure_db;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.001);
        assert_eq!(output_node.signal_frequency_hz, 1.0e9);
        assert_eq!(output_node.signal_bandwidth_hz, 1.0e6);
    }

    #[test]
    fn two_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: input_power,
            signal_frequency_hz: 1.0e9,                         // Hz
            signal_bandwidth_hz: 1.0e6,                         // Hz
            noise_power_dbm: -174.0 * 10.0 * f64::log10(1.0e6), // assumes T = 290 K
            cumulative_noise_figure_db: 5.0,                    // cumulative noise figure
            cumulative_gain_db: 0.0, // starting/initial/input node of cascade
            cumulative_noise_temperature: None,
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
        let intermediate_node = input_node.cascade_block(&amplifier);

        assert_eq!(intermediate_node.cumulative_gain_db, 30.0);

        let output_node = intermediate_node.cascade_block(&attenuator);

        assert_eq!(output_node.signal_power_dbm, -6.0);
        assert_eq!(output_node.cumulative_gain_db, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure_db;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.005);
        assert_eq!(output_node.signal_frequency_hz, 1.0e9);
        assert_eq!(output_node.signal_bandwidth_hz, 1.0e6);
    }
    #[test]
    fn test_noise_spectral_density() {
        let node = super::SignalNode {
            name: "Test Node".to_string(),
            signal_power_dbm: -50.0,
            signal_frequency_hz: 1e9,
            signal_bandwidth_hz: 1e6,
            noise_power_dbm: -174.0 + 10.0 * f64::log10(1.0e6), // assumes T approximately = 290 K
            cumulative_noise_figure_db: 3.0103,                 // F=2
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
        };

        // Case 1: Standard ~290K noise temperature (NF=3dB implies F=2, T=290K if T0=290K? No, T = T0 * (F-1). If F=2, T=290. Total Noise Temp = T_source + T_added. SOurce is usually 290K.
        // Wait, the functions use `self.noise_temperature()`.
        // `noise_temperature_from_noise_figure` usually assumes the noise figure represents the added noise.
        // If this is a node, does `noise_figure` represent the cumulative noise figure at that point? Yes.
        // And `noise_temperature()` converts that cumulative NF to a temperature.
        // T_sys = 290 * (F_sys - 1).
        // Let's check `rfconversions` logic or assume standard.
        // If NF=3.0102999566 dB (F=2.0), T = 290K.
        // k*T = 1.38e-23 * 290 = 4.002e-21 W/Hz.
        // dBm/Hz = 10 * log10(4.002e-21 * 1000) = 10 * log10(4.002e-18) = -173.98 dBm/Hz.

        // We can check against a known value or calculating it.
        let nsd = node.noise_spectral_density();
        // Allow some float error
        assert!(
            nsd < -173.98 && nsd > -174.02,
            "NSD should be around -174 dBm/Hz for thermal noise"
        );
    }

    #[test]
    fn test_signal_to_noise_ratio() {
        let node = super::SignalNode {
            name: "SNR Node".to_string(),
            signal_power_dbm: -100.0, // Signal
            signal_frequency_hz: 1e9,
            signal_bandwidth_hz: 1.0,                         // 1Hz
            noise_power_dbm: -174.0 + 10.0 * f64::log10(1.0), // assumes T = 290 K
            cumulative_noise_figure_db: 3.0103,               // Noise Power ~ -174 dBm
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
        };
        // SNR = -100 - (-174) = 74 dB
        let snr_db = node.signal_to_noise_ratio_db();
        assert!((snr_db - 73.97).abs() < 0.1, "SNR should be approx 74 dB");
    }

    #[test]
    fn test_default_cumulative_noise_temperature_regression_amplifier() {
        // This test ensures that if the input node has no cumulative noise temperature set (None),
        // the cascade logic defaults to a base temperature (currently 270.0 K) plus the block's contribution.

        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1.0e9,
            signal_bandwidth_hz: 1.0e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 0.0,
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
        };

        // 1. Verify input node has None for cumulative_noise_temperature
        assert!(
            input_node.cumulative_noise_temperature.is_none(),
            "Input node should have None for cumulative_noise_temperature"
        );

        // Create a dummy block with 0 dB gain and 0 dB noise figure.
        // With 0 dB gain (linear 1.0) and 0 dB NF (Factor 1.0),
        // block_noise_temperature = 290 * (1 - 1) = 0.
        // cumulative_gain_linear = db_to_linear(0) + db_to_linear(0) = 1 + 1 = 2 (Logic in code is additive?)
        // Wait, line 76: `rfconversions::power::db_to_linear(self.cumulative_gain) + stage_gain_linear;`
        // If cumulative_gain is 0.0 -> linear 1.0. stage_gain is 0.0 -> linear 1.0. Sum is 2.0.
        // Code: Some(270.0 + block_noise_temperature / cumulative_gain_linear)
        // clean 270.0 + 0 / 2.0 = 270.0.
        let block = super::Block {
            name: "Dummy Block".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
        };

        let output_node = input_node.cascade_block(&block);

        // 2. Verify output node has the expected default temperature
        if let Some(temp) = output_node.cumulative_noise_temperature {
            assert!(
                (temp - 296.2387337).abs() < 0.001,
                "Expected default cumulative noise temperature of ~296.2387337 K, got {} K",
                temp
            );
        } else {
            panic!("Output node should have a cumulative noise temperature");
        }
    }

    #[test]
    fn test_default_cumulative_noise_temperature_regression_lossy() {
        // This test ensures that if the input node has no cumulative noise temperature set (None),
        // the cascade logic defaults to a base temperature (currently 270.0 K) plus the block's contribution.

        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1.0e9,
            signal_bandwidth_hz: 1.0e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 0.0,
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
        };

        // 1. Verify input node has None for cumulative_noise_temperature
        assert!(
            input_node.cumulative_noise_temperature.is_none(),
            "Input node should have None for cumulative_noise_temperature"
        );

        // Create a dummy block with 0 dB gain and 0 dB noise figure.
        // With 0 dB gain (linear 1.0) and 0 dB NF (Factor 1.0),
        // block_noise_temperature = 290 * (1 - 1) = 0.
        // cumulative_gain_linear = db_to_linear(0) + db_to_linear(0) = 1 + 1 = 2 (Logic in code is additive?)
        // Wait, line 76: `rfconversions::power::db_to_linear(self.cumulative_gain) + stage_gain_linear;`
        // If cumulative_gain is 0.0 -> linear 1.0. stage_gain is 0.0 -> linear 1.0. Sum is 2.0.
        // Code: Some(270.0 + block_noise_temperature / cumulative_gain_linear)
        // clean 270.0 + 0 / 2.0 = 270.0.
        let block = super::Block {
            name: "Dummy Block".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
        };

        let output_node = input_node.cascade_block(&block);

        // 2. Verify output node has the expected default temperature
        if let Some(temp) = output_node.cumulative_noise_temperature {
            assert!(
                (temp - 960.951599).abs() < 0.001,
                "Expected default cumulative noise temperature of ~960.951599 K, got {} K",
                temp
            );
        } else {
            panic!("Output node should have a cumulative noise temperature");
        }
    }

    #[test]
    fn test_cascade_block_with_compression() {
        // Test that signal compresses but noise doesn't when signal is high and noise is low
        // This verifies that signal and noise compression are handled independently
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: 0.0,   // High signal power
            noise_power_dbm: -100.0, // Low noise power
            signal_frequency_hz: 1.0e9,
            signal_bandwidth_hz: 1.0e6,
            cumulative_noise_figure_db: 0.0,
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
        };

        // Block with 20 dB gain and output P1dB at 10 dBm
        // Signal: 0 dBm + 20 dB = 20 dBm (exceeds P1dB) -> should compress to ~11 dBm
        // Noise: -100 dBm + 20 dB = -80 dBm (well below P1dB) -> should NOT compress
        let block = super::Block {
            name: "Compressing Amplifier".to_string(),
            gain_db: 20.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: Some(10.0), // Compression point at 10 dBm output
        };

        let output_node = input_node.cascade_block(&block);

        // Verify signal compressed to P1dB + 1 dB
        assert_eq!(
            output_node.signal_power_dbm, 11.0,
            "Signal should compress to P1dB + 1 dB = 11 dBm (not 20 dBm)"
        );

        // Verify cumulative gain reflects signal compression (11 dB actual gain, not 20 dB)
        assert_eq!(
            output_node.cumulative_gain_db, 11.0,
            "Cumulative gain should be 11 dB (compressed), not 20 dB"
        );

        // Verify noise does NOT compress - it should get the full 20 dB gain
        // -100 dBm input + 20 dB gain + noise figure contribution ≈ -78 to -80 dBm
        // This is well below the 10 dBm P1dB point, so no compression should occur
        // Note: The actual noise calculation includes contributions from the block's noise figure,
        // so we check that noise power is well below the compression point
        assert!(
            output_node.noise_power_dbm < -50.0,
            "Noise power should be well below P1dB (got {} dBm), indicating no compression",
            output_node.noise_power_dbm
        );

        // Verify that the difference between output noise and input noise is close to the
        // block's gain_db (not the compressed signal gain of 11 dB)
        // The noise gain should be around 20-21 dB due to the full gain plus noise figure
        let noise_gain = output_node.noise_power_dbm - input_node.noise_power_dbm;
        assert!(
            noise_gain > 15.0 && noise_gain < 25.0,
            "Noise should experience close to full 20 dB gain (got {} dB), not the compressed signal gain of 11 dB",
            noise_gain
        );
    }
}
