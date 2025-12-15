use std::default::Default;
use std::fmt;

use crate::block::Block;

#[derive(Clone, Debug)]
pub struct SignalNode {
    pub name: String,          // name of node, like "Input" or "Amplifier 1 Output"
    pub signal_frequency: f64, // Hz
    pub signal_bandwidth: f64, // Hz
    pub signal_power: f64,     // dBm
    pub noise_power: f64,      // dBm
    pub cumulative_noise_figure: f64, // dB, linear
    pub cumulative_gain: f64,  // cumulative, dB (set to 0 at start)
    pub cumulative_noise_temperature: Option<f64>,
}

impl fmt::Display for SignalNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "SignalNode {{ name: {}, signal_power: {}, noise_power: {}, signal_frequency: {}, signal_bandwidth: {}, cumulative_noise_figure: {}, cumulative_gain: {} }}",
            self.name, self.signal_power, self.noise_power, self.signal_frequency, self.signal_bandwidth, self.cumulative_noise_figure, self.cumulative_gain
        )
    }
}

impl Default for SignalNode {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            signal_frequency: 0.0, // placeholder value, you should change this
            signal_bandwidth: 0.0, // placeholder value, you should change this
            signal_power: 0.0,     // placeholder value, you should change this
            noise_power: 0.0,      // placeholder value, you should change this
            cumulative_noise_figure: 0.0, // no contribution
            cumulative_gain: 1.0,  // default assuming start of cascade
            cumulative_noise_temperature: None,
        }
    }
}

impl SignalNode {
    pub fn noise_spectral_density(&self) -> f64 {
        let noise_spectral_density_dbm_per_hz =
            self.noise_power - self.signal_bandwidth.log10() * 10.0;

        println!(
            "Noise Spectral Density: (dBm/Hz) {}",
            noise_spectral_density_dbm_per_hz
        );

        noise_spectral_density_dbm_per_hz
    }

    pub fn signal_to_noise_ratio(&self) -> f64 {
        let signal_to_noise_ratio_db = self.signal_power - self.noise_power;

        println!("Signal to Noise Ratio: (dB) {}", signal_to_noise_ratio_db);

        signal_to_noise_ratio_db
    }

    pub fn cascade_block(&self, block: &Block) -> SignalNode {
        println!("START NODE Cascade_block");
        let output_node_name = block.name.clone() + " Output";

        let block_noise_factor =
            rfconversions::noise::noise_factor_from_noise_figure(block.noise_figure);

        let block_noise_temperature =
            rfconversions::noise::noise_temperature_from_noise_factor(block_noise_factor);

        let stage_gain_linear = rfconversions::power::db_to_linear(block.gain);
        let cumulative_gain_linear =
            rfconversions::power::db_to_linear(self.cumulative_gain) + stage_gain_linear;

        // handle compression point
        // this is a simplification in that you can compress the block with noise
        let output_power_without_compression = self.signal_power + block.gain;
        let output_power = if let Some(output_p1db) = block.output_p1db {
            if output_power_without_compression > output_p1db + 1.0 {
                output_p1db + 1.0
            } else {
                output_power_without_compression
            }
        } else {
            output_power_without_compression
        };

        let stage_power_gain = output_power - self.signal_power;

        let cumulative_noise_factor =
            self.noise_factor() + (block_noise_factor - 1.0) / cumulative_gain_linear;

        let cumulative_noise_figure =
            rfconversions::noise::noise_figure_from_noise_factor(cumulative_noise_factor);

        let cumulative_noise_temperature = if self.cumulative_noise_temperature.is_some() {
            let noise_temperature = self.cumulative_noise_temperature.unwrap();
            Some(noise_temperature + block_noise_temperature / stage_gain_linear)
        } else {
            Some(270.0 + block_noise_temperature / stage_gain_linear)
        };

        let input_noise_power = self.noise_power;
        println!("Input Noise Power: (dBm) {}", input_noise_power);
        let output_noise_power_from_node_dbm = input_noise_power + stage_power_gain;

        let output_noise_power_from_block_dbm =
            block.output_noise_power(self.signal_bandwidth, self.signal_power);

        println!(
            "Output Noise Power from Node: (dBm) {}",
            output_noise_power_from_node_dbm
        );
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

        println!(
            "Total Noise Power at Output: (W) {}",
            total_noise_power_at_output_watts
        );

        let total_noise_power_at_output_dbm =
            rfconversions::power::watts_to_dbm(total_noise_power_at_output_watts);

        println!(
            "Total Noise Power at Output: (dBm) {}",
            total_noise_power_at_output_dbm
        );

        let output_frequency = self.signal_frequency;
        let output_bandwidth = self.signal_bandwidth;

        // TODO: handle frequency and bandwidth changes, i.e. mixers, filters, etc.

        println!("END NODE Cascade_block");

        SignalNode {
            name: output_node_name,
            signal_frequency: output_frequency,
            signal_bandwidth: output_bandwidth,
            signal_power: output_power,
            noise_power: total_noise_power_at_output_dbm,
            cumulative_noise_figure,
            cumulative_gain: self.cumulative_gain + stage_power_gain,
            cumulative_noise_temperature,
        }
    }

    pub fn noise_factor(&self) -> f64 {
        rfconversions::noise::noise_factor_from_noise_figure(self.cumulative_noise_figure)
    }

    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.cumulative_noise_figure)
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn one_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power: input_power,
            noise_power: -174.0 * 10.0 * f64::log10(1.0e6), // assumes T = 290 K
            signal_frequency: 1.0e9,                        // Hz
            signal_bandwidth: 1.0e6,                        // Hz
            cumulative_noise_figure: 5.0,                   // cumulative noise figure
            cumulative_gain: 0.0, // starting/initial/input node of cascade
            cumulative_noise_temperature: None,
        };
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 5.0,
            output_p1db: None,
        };
        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.signal_power, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.262);
        assert_eq!(output_node.signal_frequency, 1.0e9);
        assert_eq!(output_node.signal_bandwidth, 1.0e6);
    }

    #[test]
    fn one_part_lna_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power: input_power,
            signal_frequency: 1.0e9,                        // Hz
            signal_bandwidth: 1.0e6,                        // Hz
            noise_power: -174.0 * 10.0 * f64::log10(1.0e6), // assumes T = 290 K
            cumulative_noise_figure: 5.0,                   // cumulative noise figure
            cumulative_gain: 0.0, // starting/initial/input node of cascade
            cumulative_noise_temperature: None,
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain: 30.0,
            noise_figure: 3.0,
            output_p1db: None,
        };

        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.signal_power, 0.0);
        assert_eq!(output_node.name, "Low Noise Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.001);
        assert_eq!(output_node.signal_frequency, 1.0e9);
        assert_eq!(output_node.signal_bandwidth, 1.0e6);
    }

    #[test]
    fn two_part_node() {
        let input_power: f64 = -30.0;
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power: input_power,
            signal_frequency: 1.0e9,                        // Hz
            signal_bandwidth: 1.0e6,                        // Hz
            noise_power: -174.0 * 10.0 * f64::log10(1.0e6), // assumes T = 290 K
            cumulative_noise_figure: 5.0,                   // cumulative noise figure
            cumulative_gain: 0.0, // starting/initial/input node of cascade
            cumulative_noise_temperature: None,
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

        assert_eq!(output_node.signal_power, -6.0);
        assert_eq!(output_node.cumulative_gain, 24.0);

        assert_eq!(output_node.name, "Attenuator Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 5.005);
        assert_eq!(output_node.signal_frequency, 1.0e9);
        assert_eq!(output_node.signal_bandwidth, 1.0e6);
    }
    #[test]
    fn test_noise_spectral_density() {
        let node = super::SignalNode {
            name: "Test Node".to_string(),
            signal_power: -50.0,
            signal_frequency: 1e9,
            signal_bandwidth: 1e6,
            noise_power: -174.0 + 10.0 * f64::log10(1.0e6), // assumes T approximately = 290 K
            cumulative_noise_figure: 3.0103,                // F=2
            cumulative_gain: 0.0,
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

    // #[test]
    // fn test_noise_power() {
    //     // T = 290K (F=2, NF~3dB)
    //     // B = 1Hz
    //     // Noise Power = kTB = 4e-21 W = -174 dBm.
    //     let node_1hz = super::SignalNode {
    //         name: "1Hz Node".to_string(),
    //         signal_power: 0.0,
    //         signal_frequency: 1e9,
    //         signal_bandwidth: 1.0,
    //         noise_power: -174.0 * 10.0 * f64::log10(1.0), // assumes T = 290 K
    //         cumulative_noise_figure: 3.0103,              // F=2
    //         cumulative_gain: 0.0,
    //         cumulative_noise_temperature: None,
    //     };
    //     let np_1hz = node_1hz.noise_power();
    //     assert!(
    //         (np_1hz - -173.97).abs() < 0.1,
    //         "Noise power for 1Hz, 290K should be approx -174 dBm"
    //     );

    //     // Test Bandwidth scaling
    //     // B = 1MHz (10^6). Noise power should be -174 + 60 = -114 dBm.
    //     let node_1mhz = super::SignalNode {
    //         name: "1MHz Node".to_string(),
    //         signal_power: 0.0,
    //         signal_frequency: 1e9,
    //         signal_bandwidth: 1.0e6,
    //         noise_power: -174.0 * 10.0 * f64::log10(1.0e6), // assumes T = 290 K
    //         cumulative_noise_figure: 3.0103,                // F=2
    //         cumulative_gain: 0.0,
    //         cumulative_noise_temperature: None,
    //     };
    //     let np_1mhz = node_1mhz.noise_power;
    //     assert!(
    //         (np_1mhz - -113.97).abs() < 0.1,
    //         "Noise power for 1MHz, 290K should be approx -114 dBm"
    //     );
    // }

    // #[test]
    // fn test_signal_to_noise_ratio() {
    //     let node = super::SignalNode {
    //         name: "SNR Node".to_string(),
    //         signal_power: -100.0, // Signal
    //         signal_frequency: 1e9,
    //         signal_bandwidth: 1.0,                        // 1Hz
    //         noise_power: -174.0 * 10.0 * f64::log10(1.0), // assumes T = 290 K
    //         cumulative_noise_figure: 3.0103,              // Noise Power ~ -174 dBm
    //         cumulative_gain: 0.0,
    //         cumulative_noise_temperature: None,
    //     };
    //     // SNR = -100 - (-174) = 74 dB
    //     let snr = node.signal_to_noise_ratio();
    //     assert!((snr - 73.97).abs() < 0.1, "SNR should be approx 74 dB");
    // }

    // #[test]
    // fn test_edge_cases() {
    //     // Zero Bandwidth
    //     let node_0bw = super::SignalNode {
    //         name: "Zero BW".to_string(),
    //         power: 0.0,
    //         frequency: 1e9,
    //         bandwidth: 0.0,
    //         noise_figure: 3.0,
    //         cumulative_gain: 0.0,
    //         cumulative_noise_temperature: None,
    //     };
    //     // Power = k*T*0 = 0 Watts.
    //     // dBm = 10*log10(0) = -inf.
    //     let np_0bw = node_0bw.noise_power();
    //     assert!(
    //         np_0bw == f64::NEG_INFINITY,
    //         "Noise power for 0Hz bandwidth should be -inf dBm"
    //     );

    //     // Zero Kelvin (Absolute Zero) - Approximation by low NF?
    //     // NF -> 0 dB implies F=1. T = 290(1-1) = 0K? No T = 290(F-1). Yes if F=1 then T=0.
    //     let node_0k = super::SignalNode {
    //         name: "Zero K".to_string(),
    //         power: 0.0,
    //         frequency: 1e9,
    //         bandwidth: 1e6,
    //         noise_figure: 0.0, // F=1, T=0
    //         cumulative_gain: 0.0,
    //         cumulative_noise_temperature: None,
    //     };
    //     // T=0 => Power = 0 Watts => -inf dBm
    //     let np_0k = node_0k.noise_power();
    //     // Note: float equality with -inf is tricky, usually check is_infinite & sign
    //     assert!(
    //         np_0k == f64::NEG_INFINITY,
    //         "Noise power for 0K should be -inf dBm"
    //     );
    // }
}
