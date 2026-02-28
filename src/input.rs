use std::default::Default;
use std::fmt;

use crate::block::Block;
use crate::constants;
use crate::node::SignalNode;

/// The input signal that enters the RF cascade.
///
/// # Examples
///
/// ```
/// use gainlineup::Input;
///
/// // 1 GHz signal at -30 dBm with 10 MHz bandwidth
/// let input = Input::new(1.0e9, 10.0e6, -30.0, Some(290.0));
/// assert_eq!(input.power_dbm, -30.0);
///
/// // Using struct literal
/// let input = Input {
///     frequency_hz: 1.0e9,
///     bandwidth_hz: 1.0e6,
///     power_dbm: -50.0,
///     noise_temperature_k: Some(270.0),
/// };
/// ```
#[doc(alias = "signal")]
#[doc(alias = "input power")]
#[derive(Clone, Debug)]
pub struct Input {
    /// Center frequency of the input signal in Hz.
    pub frequency_hz: f64,
    /// Bandwidth of the input signal in Hz.
    pub bandwidth_hz: f64,
    /// Input signal power in dBm.
    pub power_dbm: f64,
    /// Noise temperature of the input in Kelvin (defaults to 270 K if `None`).
    pub noise_temperature_k: Option<f64>,
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
    /// Create a new input signal.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Input;
    ///
    /// let input = Input::new(2.4e9, 20.0e6, -40.0, Some(290.0));
    /// assert_eq!(input.frequency_hz, 2.4e9);
    /// assert_eq!(input.bandwidth_hz, 20.0e6);
    /// ```
    #[must_use]
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

    /// Noise spectral density in dBm/Hz.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Input;
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
    /// let nsd = input.noise_spectral_density();
    /// assert!((nsd - (-174.0)).abs() < 0.1); // ~-174 dBm/Hz at 290 K
    /// ```
    #[must_use]
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

    /// Input noise power in dBm (kTB thermal noise).
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Input;
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
    /// let noise = input.noise_power();
    /// // kTB at 290K, 1 MHz ≈ -114 dBm
    /// assert!((noise - (-114.0)).abs() < 0.1);
    /// ```
    #[must_use]
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

    /// Cascade the input signal through a block, producing a [`SignalNode`].
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::{Input, Block};
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(270.0));
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 30.0,
    ///     noise_figure_db: 1.5,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let output = input.cascade_block(&lna);
    /// assert_eq!(output.signal_power_dbm, 0.0); // -30 + 30 = 0 dBm
    /// assert_eq!(output.name, "LNA Output");
    /// ```
    #[must_use]
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
            let noise_floor_dbm =
                -174.0 + 10.0 * self.bandwidth_hz.log10() + cumulative_noise_figure;
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
            output_p1db_dbm: block.output_p1db_dbm,
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

    #[test]
    fn test_default() {
        let input = Input::default();
        assert_eq!(input.frequency_hz, 0.0);
        assert_eq!(input.bandwidth_hz, 100.0);
        assert_eq!(input.power_dbm, 0.0);
        assert_eq!(input.noise_temperature_k, None);
    }

    #[test]
    fn test_display() {
        let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
        let s = format!("{}", input);
        assert!(s.contains("1000000000")); // frequency
        assert!(s.contains("1000000")); // bandwidth
        assert!(s.contains("-30")); // power
    }

    #[test]
    fn test_noise_spectral_density_at_290k() {
        // At 290 K, NSD should be approximately -174 dBm/Hz
        let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
        let nsd = input.noise_spectral_density();
        assert!(
            (nsd - (-174.0)).abs() < 0.1,
            "NSD at 290K should be ~-174 dBm/Hz, got {}",
            nsd
        );
    }

    #[test]
    fn test_noise_spectral_density_defaults_to_270k() {
        // None temperature should default to 270 K
        let input = Input::new(1.0e9, 1.0e6, -30.0, None);
        let nsd = input.noise_spectral_density();
        // kT at 270K: 10*log10(1.38e-23 * 270) + 30 ≈ -174.32 dBm/Hz
        assert!(
            (nsd - (-174.32)).abs() < 0.1,
            "NSD at 270K should be ~-174.32 dBm/Hz, got {}",
            nsd
        );
    }

    #[test]
    fn test_noise_power_at_standard_conditions() {
        // At 290K, 1 MHz BW: kTB = -174 + 60 = -114 dBm
        let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
        let np = input.noise_power();
        assert!(
            (np - (-114.0)).abs() < 0.1,
            "Noise power at 290K/1MHz should be ~-114 dBm, got {}",
            np
        );
    }

    #[test]
    fn test_noise_power_scales_with_bandwidth() {
        // Doubling bandwidth adds 3 dB to noise power
        let input_1mhz = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
        let input_2mhz = Input::new(1.0e9, 2.0e6, -30.0, Some(290.0));
        let diff = input_2mhz.noise_power() - input_1mhz.noise_power();
        assert!(
            (diff - 3.01).abs() < 0.1,
            "Doubling BW should add ~3 dB, got {} dB difference",
            diff
        );
    }

    #[test]
    fn test_cascade_block_with_explicit_noise_temperature() {
        // Verify that providing Some(290.0) uses 290K not 270K default
        let input_290 = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
        let input_270 = Input::new(1.0e9, 1.0e6, -30.0, Some(270.0));

        let block = Block {
            name: "LNA".to_string(),
            gain_db: 20.0,
            noise_figure_db: 2.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };

        let node_290 = input_290.cascade_block(&block);
        let node_270 = input_270.cascade_block(&block);

        // Higher input temperature → higher cumulative noise temperature
        assert!(
            node_290.cumulative_noise_temperature.unwrap()
                > node_270.cumulative_noise_temperature.unwrap(),
            "290K input should produce higher cumulative noise temp than 270K"
        );
    }

    #[test]
    fn test_cascade_block_with_ip3() {
        let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
        let block = Block {
            name: "LNA".to_string(),
            gain_db: 20.0,
            noise_figure_db: 2.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: Some(25.0),
        };
        let node = input.cascade_block(&block);

        // OIP3 should pass through from block
        assert_eq!(node.cumulative_oip3_dbm, Some(25.0));
        // SFDR should be calculated
        assert!(node.sfdr_db.is_some());
        assert!(node.sfdr_db.unwrap() > 0.0);
    }

    #[test]
    fn test_cascade_block_no_ip3_means_no_sfdr() {
        let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
        let block = Block {
            name: "Filter".to_string(),
            gain_db: -3.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let node = input.cascade_block(&block);

        assert_eq!(node.cumulative_oip3_dbm, None);
        assert_eq!(node.sfdr_db, None);
    }

    #[test]
    fn test_cascade_block_attenuator() {
        // A passive attenuator: gain = -10 dB, NF = 10 dB (matched)
        let input = Input::new(1.0e9, 1.0e6, -20.0, Some(290.0));
        let atten = Block {
            name: "10dB Pad".to_string(),
            gain_db: -10.0,
            noise_figure_db: 10.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let node = input.cascade_block(&atten);

        // Signal: -20 + (-10) = -30 dBm
        assert!((node.signal_power_dbm - (-30.0)).abs() < 0.01);
        assert!((node.cumulative_gain_db - (-10.0)).abs() < 0.01);
        assert!((node.cumulative_noise_figure_db - 10.0).abs() < 0.01);
    }
}
