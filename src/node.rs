use std::default::Default;
use std::fmt;

use crate::block::Block;

/// Summary of dynamic range metrics at a given node in the cascade.
///
/// # Examples
///
/// ```
/// use gainlineup::{Input, Block, SignalNode};
///
/// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(270.0));
/// let lna = Block {
///     name: "LNA".to_string(),
///     gain_db: 20.0,
///     noise_figure_db: 2.0,
///     output_p1db_dbm: Some(10.0),
///     output_ip3_dbm: Some(25.0),
/// };
/// let node = input.cascade_block(&lna);
/// let dr = node.dynamic_range_summary().unwrap();
/// assert!(dr.linear_dr_db > 90.0);
/// ```
#[doc(alias = "dynamic range")]
#[doc(alias = "DR")]
#[derive(Clone, Debug)]
pub struct DynamicRange {
    /// Linear dynamic range: output P1dB minus noise floor (dB).
    pub linear_dr_db: f64,
    /// Spur-free dynamic range (dB), from existing SFDR calculation.
    pub sfdr_db: Option<f64>,
    /// Minimum detectable signal: noise floor at this node (dBm).
    pub mds_dbm: f64,
    /// Maximum input power before compression: input P1dB = output P1dB − cumulative gain (dBm).
    pub max_input_dbm: f64,
}

impl fmt::Display for DynamicRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DynamicRange {{ linear_dr: {:.1} dB, sfdr: {}, mds: {:.1} dBm, max_input: {:.1} dBm }}",
            self.linear_dr_db,
            match self.sfdr_db {
                Some(v) => format!("{:.1} dB", v),
                None => "N/A".to_string(),
            },
            self.mds_dbm,
            self.max_input_dbm
        )
    }
}

/// Output signal at a node in the RF cascade, containing power, noise, and gain information.
///
/// # Examples
///
/// ```
/// use gainlineup::{Input, Block, SignalNode};
///
/// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(270.0));
/// let lna = Block {
///     name: "LNA".to_string(),
///     gain_db: 30.0,
///     noise_figure_db: 1.5,
///     output_p1db_dbm: None,
///     output_ip3_dbm: None,
/// };
/// let node = input.cascade_block(&lna);
/// assert_eq!(node.signal_power_dbm, 0.0);
/// assert_eq!(node.cumulative_gain_db, 30.0);
/// let snr = node.signal_to_noise_ratio_db();
/// assert!(snr > 50.0);
/// ```
#[doc(alias = "signal chain")]
#[doc(alias = "cascade")]
#[doc(alias = "NF")]
#[doc(alias = "noise figure")]
#[derive(Clone, Debug)]
pub struct SignalNode {
    /// Name of this node (e.g. "LNA Output").
    pub name: String,
    /// Signal center frequency in Hz.
    pub signal_frequency_hz: f64,
    /// Signal bandwidth in Hz.
    pub signal_bandwidth_hz: f64,
    /// Signal power at this node in dBm.
    pub signal_power_dbm: f64,
    /// Total noise power at this node in dBm.
    pub noise_power_dbm: f64,
    /// Cumulative noise figure through the cascade in dB.
    pub cumulative_noise_figure_db: f64,
    /// Cumulative gain through the cascade in dB.
    pub cumulative_gain_db: f64,
    /// Cumulative noise temperature in Kelvin, if available.
    pub cumulative_noise_temperature: Option<f64>,
    /// Cascaded output-referred IP3 in dBm, if available.
    pub cumulative_oip3_dbm: Option<f64>,
    /// Spur-free dynamic range in dB, if OIP3 is available.
    pub sfdr_db: Option<f64>,
    /// Output P1dB at this node in dBm, if applicable.
    pub output_p1db_dbm: Option<f64>,
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        }
    }
}

impl SignalNode {
    /// Noise spectral density in dBm/Hz at this node.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::{Input, Block};
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 2.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let node = input.cascade_block(&lna);
    /// let nsd = node.noise_spectral_density();
    /// assert!(nsd < -140.0); // well below signal levels
    /// ```
    #[must_use]
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

    /// Signal-to-noise ratio in dB at this node.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::{Input, Block};
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 2.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let node = input.cascade_block(&lna);
    /// let snr = node.signal_to_noise_ratio_db();
    /// assert!(snr > 50.0);
    /// ```
    #[must_use]
    pub fn signal_to_noise_ratio_db(&self) -> f64 {
        let signal_to_noise_ratio_db = self.signal_power_dbm - self.noise_power_dbm;

        #[cfg(feature = "debug-print")]
        println!("Signal to Noise Ratio: (dB) {}", signal_to_noise_ratio_db);

        signal_to_noise_ratio_db
    }

    /// Cascade this node through another block, producing a new [`SignalNode`].
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
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let atten = Block {
    ///     name: "Attenuator".to_string(),
    ///     gain_db: -6.0,
    ///     noise_figure_db: 6.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let after_lna = input.cascade_block(&lna);
    /// let after_atten = after_lna.cascade_block(&atten);
    /// assert_eq!(after_atten.signal_power_dbm, -6.0); // -30 + 30 - 6
    /// ```
    #[must_use]
    pub fn cascade_block(&self, block: &Block) -> SignalNode {
        #[cfg(feature = "debug-print")]
        println!("START NODE Cascade_block");

        let output_node_name = block.name.clone() + " Output";

        let block_noise_factor =
            rfconversions::noise::noise_factor_from_noise_figure(block.noise_figure_db);

        let block_noise_temperature =
            rfconversions::noise::noise_temperature_from_noise_factor(block_noise_factor);

        let cumulative_gain_linear =
            rfconversions::power::db_to_linear(self.cumulative_gain_db);

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

        // Cascaded OIP3 calculation
        let cumulative_oip3_dbm = match (self.cumulative_oip3_dbm, block.output_ip3_dbm) {
            (Some(prev_oip3_dbm), Some(block_oip3_dbm)) => {
                let prev_oip3_linear = rfconversions::power::dbm_to_watts(prev_oip3_dbm);
                let block_oip3_linear = rfconversions::power::dbm_to_watts(block_oip3_dbm);
                let gain_linear = rfconversions::power::db_to_linear(block.gain_db);
                let inv_cascade = gain_linear / prev_oip3_linear + 1.0 / block_oip3_linear;
                Some(rfconversions::power::watts_to_dbm(1.0 / inv_cascade))
            }
            (None, Some(block_oip3_dbm)) => Some(block_oip3_dbm),
            _ => None,
        };

        // SFDR calculation
        let new_cumulative_gain_db = self.cumulative_gain_db + stage_power_gain;
        let sfdr_db = cumulative_oip3_dbm.map(|oip3| {
            let noise_floor_dbm =
                -174.0 + 10.0 * output_bandwidth_hz.log10() + cumulative_noise_figure;
            2.0 / 3.0 * (oip3 - noise_floor_dbm)
        });

        SignalNode {
            name: output_node_name,
            signal_frequency_hz: output_frequency_hz,
            signal_bandwidth_hz: output_bandwidth_hz,
            signal_power_dbm: output_power_dbm,
            noise_power_dbm: total_noise_power_at_output_dbm,
            cumulative_noise_figure_db: cumulative_noise_figure,
            cumulative_gain_db: new_cumulative_gain_db,
            cumulative_noise_temperature,
            cumulative_oip3_dbm,
            sfdr_db,
            output_p1db_dbm: block.output_p1db_dbm,
        }
    }

    /// Cumulative noise factor (linear) at this node.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::{Input, Block};
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let node = input.cascade_block(&lna);
    /// let nf = node.noise_factor();
    /// assert!((nf - 2.0).abs() < 0.01); // 3 dB ≈ factor 2
    /// ```
    #[must_use]
    pub fn noise_factor(&self) -> f64 {
        rfconversions::noise::noise_factor_from_noise_figure(self.cumulative_noise_figure_db)
    }

    /// Cumulative noise temperature in Kelvin at this node.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::{Input, Block};
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let node = input.cascade_block(&lna);
    /// let temp = node.noise_temperature();
    /// assert!(temp > 200.0 && temp < 400.0); // ~290 K for 3 dB NF
    /// ```
    #[must_use]
    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.cumulative_noise_figure_db)
    }

    /// Linear dynamic range at this node in dB.
    ///
    /// `output_p1db_dbm - noise_power_dbm`
    ///
    /// Returns `None` if `output_p1db_dbm` is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::{Input, Block};
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(270.0));
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 2.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: None,
    /// };
    /// let node = input.cascade_block(&lna);
    /// let dr = node.dynamic_range_db().unwrap();
    /// assert!(dr > 90.0);
    /// ```
    #[must_use]
    pub fn dynamic_range_db(&self) -> Option<f64> {
        let p1db = self.output_p1db_dbm?;
        Some(p1db - self.noise_power_dbm)
    }

    /// Build a [`DynamicRange`] summary from this node's fields.
    ///
    /// Returns `None` if `output_p1db_dbm` is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::{Input, Block};
    ///
    /// let input = Input::new(1.0e9, 1.0e6, -30.0, Some(270.0));
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 2.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: Some(25.0),
    /// };
    /// let node = input.cascade_block(&lna);
    /// let summary = node.dynamic_range_summary().unwrap();
    /// assert!(summary.linear_dr_db > 90.0);
    /// assert!(summary.sfdr_db.is_some());
    /// ```
    #[must_use]
    pub fn dynamic_range_summary(&self) -> Option<DynamicRange> {
        let output_p1db = self.output_p1db_dbm?;
        let linear_dr_db = output_p1db - self.noise_power_dbm;
        let max_input_dbm = output_p1db - self.cumulative_gain_db;
        Some(DynamicRange {
            linear_dr_db,
            sfdr_db: self.sfdr_db,
            mds_dbm: self.noise_power_dbm,
            max_input_dbm,
        })
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        };
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 5.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.signal_power_dbm, -20.0);
        assert_eq!(output_node.name, "Simple Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure_db;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 7.263);
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };

        let output_node = input_node.cascade_block(&amplifier);

        assert_eq!(output_node.signal_power_dbm, 0.0);
        assert_eq!(output_node.name, "Low Noise Amplifier Output");
        // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
        let output_noise_figure = output_node.cumulative_noise_figure_db;

        // round to 3 decimal places for comparison, because floating point math is not exact
        let rounded_noise_figure = (output_noise_figure * 1e3).round() / 1e3;
        assert_eq!(rounded_noise_figure, 6.188);
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        };
        let amplifier = super::Block {
            name: "Low Noise Amplifier".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
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
        assert_eq!(rounded_noise_figure, 6.191);
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
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
            output_ip3_dbm: None,
        };

        let output_node = input_node.cascade_block(&block);

        // 2. Verify output node has the expected default temperature
        if let Some(temp) = output_node.cumulative_noise_temperature {
            assert!(
                (temp - 558.626071).abs() < 0.001,
                "Expected default cumulative noise temperature of ~558.626071 K, got {} K",
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
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
            output_ip3_dbm: None,
        };

        let output_node = input_node.cascade_block(&block);

        // 2. Verify output node has the expected default temperature
        if let Some(temp) = output_node.cumulative_noise_temperature {
            assert!(
                (temp - 1134.510795).abs() < 0.001,
                "Expected default cumulative noise temperature of ~1134.510795 K, got {} K",
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
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        };

        // Block with 20 dB gain and output P1dB at 10 dBm
        // Signal: 0 dBm + 20 dB = 20 dBm (exceeds P1dB) -> should compress to ~11 dBm
        // Noise: -100 dBm + 20 dB = -80 dBm (well below P1dB) -> should NOT compress
        let block = super::Block {
            name: "Compressing Amplifier".to_string(),
            gain_db: 20.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: Some(10.0), // Compression point at 10 dBm output
            output_ip3_dbm: None,
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

    #[test]
    fn test_cascaded_oip3_two_stage() {
        // LNA (gain=30dB, OIP3=+20dBm) → Attenuator (gain=-6dB, no IP3)
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1.0e9,
            signal_bandwidth_hz: 1.0e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 0.0,
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        };

        let lna = super::Block {
            name: "LNA".to_string(),
            gain_db: 30.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(20.0),
        };

        let attenuator = super::Block {
            name: "Attenuator".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };

        let after_lna = input_node.cascade_block(&lna);
        assert_eq!(after_lna.cumulative_oip3_dbm, Some(20.0));

        // Attenuator has no OIP3 → cascade result is None
        let after_atten = after_lna.cascade_block(&attenuator);
        assert_eq!(after_atten.cumulative_oip3_dbm, None);
    }

    #[test]
    fn test_cascaded_oip3_three_stage() {
        // LNA (gain=20, OIP3=+30) → Mixer (gain=-8, OIP3=+15) → IF Amp (gain=25, OIP3=+25)
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1.0e9,
            signal_bandwidth_hz: 1.0e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 0.0,
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        };

        let lna = super::Block {
            name: "LNA".to_string(),
            gain_db: 20.0,
            noise_figure_db: 2.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(30.0),
        };

        let mixer = super::Block {
            name: "Mixer".to_string(),
            gain_db: -8.0,
            noise_figure_db: 8.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(15.0),
        };

        let if_amp = super::Block {
            name: "IF Amp".to_string(),
            gain_db: 25.0,
            noise_figure_db: 4.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(25.0),
        };

        let n1 = input_node.cascade_block(&lna);
        assert_eq!(n1.cumulative_oip3_dbm, Some(30.0));

        let n2 = n1.cascade_block(&mixer);
        // 1/OIP3_new = G_mixer_linear/OIP3_lna_linear + 1/OIP3_mixer_linear
        // G_mixer = 10^(-8/10) = 0.158489
        // OIP3_lna = 10^(30/10) * 0.001 = 1.0 W
        // OIP3_mixer = 10^(15/10) * 0.001 = 0.031623 W
        // 1/OIP3_new = 0.158489/1.0 + 1/0.031623 = 0.158489 + 31.623 = 31.7815
        // OIP3_new = 0.031465 W = 10*log10(0.031465/0.001) = 14.978 dBm
        let oip3_2 = n2.cumulative_oip3_dbm.unwrap();
        assert!(
            (oip3_2 - 14.978).abs() < 0.1,
            "Expected ~14.978, got {}",
            oip3_2
        );

        let n3 = n2.cascade_block(&if_amp);
        // Cascaded again with IF amp
        let oip3_3 = n3.cumulative_oip3_dbm.unwrap();
        // Should be less than min(25, 14.978) since cascade always degrades
        assert!(oip3_3 < 25.0, "Cascaded OIP3 should be < 25 dBm");
        assert!(n3.sfdr_db.is_some(), "SFDR should be computed");
    }

    #[test]
    fn test_sfdr_calculation() {
        let input_node = super::SignalNode {
            name: "Input".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1.0e9,
            signal_bandwidth_hz: 1.0e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 0.0,
            cumulative_gain_db: 0.0,
            cumulative_noise_temperature: None,
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: None,
        };

        let lna = super::Block {
            name: "LNA".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(30.0),
        };

        let node = input_node.cascade_block(&lna);
        let sfdr = node.sfdr_db.unwrap();
        // noise_floor = -174 + 10*log10(1e6) + cumulative_nf
        // SFDR = 2/3 * (OIP3 - noise_floor)
        let expected_noise_floor = -174.0 + 60.0 + node.cumulative_noise_figure_db;
        let expected_sfdr = 2.0 / 3.0 * (30.0 - expected_noise_floor);
        assert!(
            (sfdr - expected_sfdr).abs() < 0.01,
            "Expected SFDR ~{}, got {}",
            expected_sfdr,
            sfdr
        );
    }

    // ----- Phase 4: Dynamic Range at Node Level -----

    #[test]
    fn dynamic_range_db_returns_p1db_minus_noise_floor() {
        let node = super::SignalNode {
            name: "Test".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1e9,
            signal_bandwidth_hz: 1e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 3.0,
            cumulative_gain_db: 20.0,
            cumulative_noise_temperature: None,
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: Some(10.0),
        };
        let dr = node.dynamic_range_db().unwrap();
        assert!((dr - 110.0).abs() < 1e-10, "Expected 110 dB, got {}", dr);
    }

    #[test]
    fn dynamic_range_db_none_without_p1db() {
        let node = super::SignalNode::default();
        assert!(node.dynamic_range_db().is_none());
    }

    #[test]
    fn dynamic_range_summary_populates_all_fields() {
        let node = super::SignalNode {
            name: "Test".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1e9,
            signal_bandwidth_hz: 1e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 3.0,
            cumulative_gain_db: 20.0,
            cumulative_noise_temperature: None,
            cumulative_oip3_dbm: Some(30.0),
            sfdr_db: Some(80.0),
            output_p1db_dbm: Some(10.0),
        };
        let summary = node.dynamic_range_summary().unwrap();
        // linear_dr = 10 - (-100) = 110
        assert!((summary.linear_dr_db - 110.0).abs() < 1e-10);
        // sfdr from node
        assert_eq!(summary.sfdr_db, Some(80.0));
        // mds = noise_power_dbm
        assert!((summary.mds_dbm - (-100.0)).abs() < 1e-10);
        // max_input = output_p1db - gain = 10 - 20 = -10
        assert!((summary.max_input_dbm - (-10.0)).abs() < 1e-10);
    }

    #[test]
    fn mds_equals_noise_power() {
        let node = super::SignalNode {
            name: "Test".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1e9,
            signal_bandwidth_hz: 1e6,
            noise_power_dbm: -95.0,
            cumulative_noise_figure_db: 3.0,
            cumulative_gain_db: 20.0,
            cumulative_noise_temperature: None,
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: Some(10.0),
        };
        let summary = node.dynamic_range_summary().unwrap();
        assert!((summary.mds_dbm - node.noise_power_dbm).abs() < 1e-10);
    }

    #[test]
    fn max_input_equals_output_p1db_minus_gain() {
        let node = super::SignalNode {
            name: "Test".to_string(),
            signal_power_dbm: -30.0,
            signal_frequency_hz: 1e9,
            signal_bandwidth_hz: 1e6,
            noise_power_dbm: -100.0,
            cumulative_noise_figure_db: 3.0,
            cumulative_gain_db: 25.0,
            cumulative_noise_temperature: None,
            cumulative_oip3_dbm: None,
            sfdr_db: None,
            output_p1db_dbm: Some(15.0),
        };
        let summary = node.dynamic_range_summary().unwrap();
        // 15 - 25 = -10
        assert!((summary.max_input_dbm - (-10.0)).abs() < 1e-10);
    }
}
