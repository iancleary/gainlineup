use std::default::Default;
use std::fmt;

use crate::constants;

/// A single block (stage) in an RF cascade, such as an amplifier, attenuator, or filter.
///
/// # Examples
///
/// ```
/// use gainlineup::Block;
///
/// // Low-noise amplifier with 30 dB gain, 1.5 dB noise figure, and +15 dBm output P1dB
/// let lna = Block {
///     name: "LNA".to_string(),
///     gain_db: 30.0,
///     noise_figure_db: 1.5,
///     output_p1db_dbm: Some(15.0),
///     output_ip3_dbm: Some(30.0),
/// };
///
/// assert_eq!(lna.output_power(-40.0), -10.0);
/// assert_eq!(lna.power_gain(-40.0), 30.0);
/// ```
#[doc(alias = "block")]
#[doc(alias = "stage")]
#[doc(alias = "cascade")]
#[derive(Clone, Debug)]
pub struct Block {
    /// Human-readable name of this block (e.g. "LNA", "Filter").
    pub name: String,
    /// Small-signal gain in dB (negative for loss).
    pub gain_db: f64,
    /// Noise figure in dB.
    pub noise_figure_db: f64,
    /// Output-referred 1 dB compression point in dBm, if applicable.
    #[doc(alias = "P1dB")]
    pub output_p1db_dbm: Option<f64>,
    /// Output-referred third-order intercept point in dBm, if applicable.
    #[doc(alias = "OIP3")]
    pub output_ip3_dbm: Option<f64>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Block {{ name: {}, gain: {} dB, noise_figure: {} dB",
            self.name, self.gain_db, self.noise_figure_db
        )?;
        if let Some(output_p1db) = self.output_p1db_dbm {
            write!(f, ", output_p1db: {} dBm", output_p1db)?;
        }
        if let Some(output_ip3) = self.output_ip3_dbm {
            write!(f, ", output_ip3: {} dBm", output_ip3)?;
        }
        write!(f, " }}")
    }
}

impl Default for Block {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            gain_db: 0.0,
            noise_figure_db: 0.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        }
    }
}
impl Block {
    /// Equivalent noise temperature of this block in Kelvin.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 1.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let temp = lna.noise_temperature();
    /// assert!(temp > 0.0 && temp < 100.0); // ~75 K for 1 dB NF
    /// ```
    #[must_use]
    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.noise_figure_db)
    }

    /// Noise factor (linear, unitless) of this block.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let block = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 10.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let nf = block.noise_factor();
    /// assert!((nf - 2.0).abs() < 0.01); // 3 dB NF ≈ factor of 2
    /// ```
    #[must_use]
    pub fn noise_factor(&self) -> f64 {
        rfconversions::noise::noise_factor_from_noise_figure(self.noise_figure_db)
    }

    /// Input-referred noise power in dBm: `(F-1) × k × T × B`.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let noise = amp.input_noise_power(1.0e6);
    /// assert!(noise < -100.0); // thermal noise is very low
    /// ```
    #[must_use]
    pub fn input_noise_power(&self, bandwidth: f64) -> f64 {
        let noise_factor = self.noise_factor();
        let noise_temperature = self.noise_temperature();

        let f_minus_1 = noise_factor - 1.0;

        let ktb = constants::BOLTZMANN * noise_temperature * bandwidth;

        rfconversions::power::watts_to_dbm(f_minus_1 * ktb)
    }

    /// Output noise power in dBm: input noise power plus gain, with compression limiting.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: None,
    /// };
    /// let noise_out = amp.output_noise_power(1.0e6);
    /// assert!(noise_out < -80.0); // noise floor well below signal levels
    /// ```
    #[must_use]
    pub fn output_noise_power(&self, bandwidth: f64) -> f64 {
        #[cfg(feature = "debug-print")]
        println!("START BLOCK output_noise_power");

        let input_noise_power = self.input_noise_power(bandwidth);

        #[cfg(feature = "debug-print")]
        println!(
            "Input Noise Power (block.input_noise_power): (dBm) {}",
            input_noise_power
        );

        let output_noise_power_without_compression = input_noise_power + self.gain_db;

        #[cfg(feature = "debug-print")]
        println!(
            "Output Noise Power without compression: (dBm) {}",
            output_noise_power_without_compression
        );

        let output_noise_power_dbm = if let Some(output_p1db_dbm) = self.output_p1db_dbm {
            if output_noise_power_without_compression > output_p1db_dbm + 1.0 {
                output_p1db_dbm + 1.0
            } else {
                output_noise_power_without_compression
            }
        } else {
            output_noise_power_without_compression
        };

        #[cfg(feature = "debug-print")]
        let noise_power_gain = output_noise_power_dbm - input_noise_power;
        #[cfg(feature = "debug-print")]
        println!("Noise Power Gain: (dB) {}", noise_power_gain);

        #[cfg(feature = "debug-print")]
        println!("Output Noise Power: (dBm) {}", output_noise_power_dbm);

        #[cfg(feature = "debug-print")]
        println!("END BLOCK output_noise_power");

        output_noise_power_dbm
    }

    /// Output power in dBm for a given input power, applying compression if P1dB is set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "PA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 5.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: None,
    /// };
    /// // Linear region
    /// assert_eq!(amp.output_power(-30.0), -10.0);
    /// // Compressed: output clamped to P1dB + 1
    /// assert_eq!(amp.output_power(0.0), 11.0);
    /// ```
    #[must_use]
    pub fn output_power(&self, input_power: f64) -> f64 {
        // this is a simple calculation, which could be upgrade to
        // use the compression curve of a block, if present,
        // or the compression parameters from a piecewise amplifier model
        // where there is a linear region, a compression region, and a saturation region
        // those regions are defined by the compression point, a curve fit within te region of compression, and the saturation point
        // the curve fit is a simple linear fit between the compression point and the saturation point
        // the compression point is the point where the compression begins
        // the saturation point is the point where the compression ends
        // this probably would be a polynomial fit, but for initial documentation of the idea,
        // it's a simple linear fit
        let output_power_without_compression = input_power + self.gain_db;
        if let Some(op1db) = self.output_p1db_dbm {
            if output_power_without_compression > op1db + 1.0 {
                return op1db + 1.0;
            }
        }
        output_power_without_compression
    }

    /// Power gain in dB at a given input power, accounting for compression.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: None,
    /// };
    /// assert_eq!(amp.power_gain(-30.0), 20.0); // linear
    /// assert!(amp.power_gain(0.0) < 20.0);     // compressed
    /// ```
    #[must_use]
    pub fn power_gain(&self, input_power: f64) -> f64 {
        self.output_power(input_power) - input_power
    }

    // ----- Dynamic Range -----

    /// Output-referred linear dynamic range in dB.
    ///
    /// DR = P1dB_out - noise_floor_out
    ///
    /// Returns None if `output_p1db_dbm` is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: None,
    /// };
    /// let dr = lna.dynamic_range_db(1.0e6).unwrap();
    /// assert!(dr > 100.0); // typical LNA dynamic range
    /// ```
    #[must_use]
    pub fn dynamic_range_db(&self, bandwidth_hz: f64) -> Option<f64> {
        let p1db = self.output_p1db_dbm?;
        let noise_floor = self.output_noise_power(bandwidth_hz);
        Some(p1db - noise_floor)
    }

    /// Input-referred linear dynamic range in dB.
    ///
    /// DR_in = input_P1dB - input_noise_floor
    ///
    /// Useful for receiver front-end analysis. Returns None if `output_p1db_dbm` is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let lna = Block {
    ///     name: "LNA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: None,
    /// };
    /// let dr = lna.input_dynamic_range_db(1.0e6).unwrap();
    /// assert!(dr > 100.0);
    /// ```
    #[must_use]
    pub fn input_dynamic_range_db(&self, bandwidth_hz: f64) -> Option<f64> {
        let input_p1db = self.output_p1db_dbm? - self.gain_db;
        let input_noise = self.input_noise_power(bandwidth_hz);
        Some(input_p1db - input_noise)
    }

    // ----- AM-AM Curves -----

    /// Generate AM-AM curve from a slice of input powers.
    ///
    /// Returns Vec of `(Pin_dBm, Pout_dBm)` pairs. Uses the existing compression
    /// model (linear below P1dB, clamped at P1dB + 1 dB above).
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Driver".to_string(),
    ///     gain_db: 15.0,
    ///     noise_figure_db: 4.0,
    ///     output_p1db_dbm: Some(20.0),
    ///     output_ip3_dbm: None,
    /// };
    /// let curve = amp.am_am_curve(&[-30.0, -20.0, -10.0]);
    /// assert_eq!(curve.len(), 3);
    /// assert_eq!(curve[0], (-30.0, -15.0)); // linear: -30 + 15 = -15
    /// ```
    #[must_use]
    pub fn am_am_curve(&self, input_powers_dbm: &[f64]) -> Vec<(f64, f64)> {
        input_powers_dbm
            .iter()
            .map(|&pin| (pin, self.output_power(pin)))
            .collect()
    }

    /// Generate AM-AM curve with evenly spaced input power sweep.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "PA".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 5.0,
    ///     output_p1db_dbm: Some(30.0),
    ///     output_ip3_dbm: None,
    /// };
    /// let sweep = amp.am_am_sweep(-40.0, -20.0, 10.0);
    /// assert_eq!(sweep.len(), 3); // -40, -30, -20
    /// ```
    #[must_use]
    pub fn am_am_sweep(&self, start_dbm: f64, stop_dbm: f64, step_db: f64) -> Vec<(f64, f64)> {
        let powers = sweep_range(start_dbm, stop_dbm, step_db);
        self.am_am_curve(&powers)
    }

    /// Generate gain compression curve from a slice of input powers.
    ///
    /// Returns Vec of `(Pin_dBm, Gain_dB)` pairs. Shows gain compression directly.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: None,
    /// };
    /// let curve = amp.gain_compression_curve(&[-30.0, 0.0]);
    /// assert_eq!(curve[0].1, 20.0); // full gain at low power
    /// assert!(curve[1].1 < 20.0);   // compressed at high power
    /// ```
    #[must_use]
    pub fn gain_compression_curve(&self, input_powers_dbm: &[f64]) -> Vec<(f64, f64)> {
        input_powers_dbm
            .iter()
            .map(|&pin| (pin, self.power_gain(pin)))
            .collect()
    }

    /// Generate gain compression curve with evenly spaced input power sweep.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: Some(10.0),
    ///     output_ip3_dbm: None,
    /// };
    /// let sweep = amp.gain_compression_sweep(-40.0, 0.0, 10.0);
    /// assert_eq!(sweep.len(), 5);
    /// assert_eq!(sweep[0].1, 20.0); // linear at -40 dBm
    /// ```
    #[must_use]
    pub fn gain_compression_sweep(
        &self,
        start_dbm: f64,
        stop_dbm: f64,
        step_db: f64,
    ) -> Vec<(f64, f64)> {
        let powers = sweep_range(start_dbm, stop_dbm, step_db);
        self.gain_compression_curve(&powers)
    }

    // ----- IMD Products from IP3 -----

    /// Third-order IMD product output power for a two-tone test.
    ///
    /// ```text
    /// IM3_out = 3 * Pout_per_tone - 2 * OIP3
    /// ```
    ///
    /// Returns None if `output_ip3_dbm` is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: Some(30.0),
    /// };
    /// // Pin = -30 → Pout = -10, IM3 = 3×(-10) - 2×30 = -90 dBm
    /// let im3 = amp.imd3_output_power_dbm(-30.0).unwrap();
    /// assert!((im3 - (-90.0)).abs() < 0.01);
    /// ```
    #[must_use]
    pub fn imd3_output_power_dbm(&self, input_power_per_tone_dbm: f64) -> Option<f64> {
        let oip3 = self.output_ip3_dbm?;
        let pout = self.output_power(input_power_per_tone_dbm);
        Some(3.0 * pout - 2.0 * oip3)
    }

    /// IM3 rejection: carrier power minus IM3 power (in dB).
    ///
    /// ```text
    /// rejection = Pout - IM3_out = 2 * (OIP3 - Pout)
    /// ```
    ///
    /// Positive = IM3 is below carrier. Higher is better.
    /// Returns None if `output_ip3_dbm` is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: Some(30.0),
    /// };
    /// let rejection = amp.imd3_rejection_db(-30.0).unwrap();
    /// assert!((rejection - 80.0).abs() < 0.01); // 2 × (30 - (-10)) = 80 dB
    /// ```
    #[must_use]
    pub fn imd3_rejection_db(&self, input_power_per_tone_dbm: f64) -> Option<f64> {
        let oip3 = self.output_ip3_dbm?;
        let pout = self.output_power(input_power_per_tone_dbm);
        Some(2.0 * (oip3 - pout))
    }

    /// Full two-tone IMD3 sweep.
    ///
    /// Returns Vec of [`Imd3Point`] with carrier and IM3 levels at each input power.
    /// Returns empty Vec if `output_ip3_dbm` is not set.
    ///
    /// # Examples
    ///
    /// ```
    /// use gainlineup::Block;
    ///
    /// let amp = Block {
    ///     name: "Amp".to_string(),
    ///     gain_db: 20.0,
    ///     noise_figure_db: 3.0,
    ///     output_p1db_dbm: None,
    ///     output_ip3_dbm: Some(30.0),
    /// };
    /// let sweep = amp.imd3_sweep(-40.0, -20.0, 10.0);
    /// assert_eq!(sweep.len(), 3);
    /// // Rejection decreases as input power increases
    /// assert!(sweep[0].rejection_db > sweep[2].rejection_db);
    /// ```
    #[must_use]
    pub fn imd3_sweep(&self, start_dbm: f64, stop_dbm: f64, step_db: f64) -> Vec<Imd3Point> {
        let oip3 = match self.output_ip3_dbm {
            Some(v) => v,
            None => return vec![],
        };
        let powers = sweep_range(start_dbm, stop_dbm, step_db);
        powers
            .iter()
            .map(|&pin| {
                let pout = self.output_power(pin);
                let im3 = 3.0 * pout - 2.0 * oip3;
                Imd3Point {
                    input_per_tone_dbm: pin,
                    output_per_tone_dbm: pout,
                    im3_output_dbm: im3,
                    rejection_db: pout - im3,
                }
            })
            .collect()
    }
}

/// A single point from a two-tone IMD3 sweep.
///
/// # Examples
///
/// ```
/// use gainlineup::Block;
///
/// let amp = Block {
///     name: "Amp".to_string(),
///     gain_db: 20.0,
///     noise_figure_db: 3.0,
///     output_p1db_dbm: None,
///     output_ip3_dbm: Some(30.0),
/// };
/// let sweep = amp.imd3_sweep(-30.0, -30.0, 1.0);
/// let point = &sweep[0];
/// assert!((point.input_per_tone_dbm - (-30.0)).abs() < 0.01);
/// assert!((point.rejection_db - 80.0).abs() < 0.01);
/// ```
#[doc(alias = "IMD3")]
#[doc(alias = "intermodulation")]
#[derive(Clone, Debug)]
pub struct Imd3Point {
    /// Input power per tone (dBm)
    pub input_per_tone_dbm: f64,
    /// Output power per tone (dBm)
    pub output_per_tone_dbm: f64,
    /// Third-order IMD product output power (dBm)
    pub im3_output_dbm: f64,
    /// Rejection: carrier minus IM3 (dB). Higher is better.
    pub rejection_db: f64,
}

impl fmt::Display for Imd3Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Imd3Point {{ Pin: {:.1} dBm, Pout: {:.1} dBm, IM3: {:.1} dBm, rejection: {:.1} dB }}",
            self.input_per_tone_dbm,
            self.output_per_tone_dbm,
            self.im3_output_dbm,
            self.rejection_db
        )
    }
}

/// Generate evenly spaced power sweep values.
fn sweep_range(start_dbm: f64, stop_dbm: f64, step_db: f64) -> Vec<f64> {
    let mut powers = vec![];
    let mut pin = start_dbm;
    while pin <= stop_dbm + step_db * 0.01 {
        powers.push(pin);
        pin += step_db;
    }
    powers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let block = Block::default();
        assert_eq!(block.gain_db, 0.0);
        assert_eq!(block.noise_figure_db, 0.0);
        assert_eq!(block.output_p1db_dbm, None);
        assert_eq!(block.noise_temperature(), 0.0);
    }

    #[test]
    fn output_power() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn output_power_with_compression_below_threshold() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-20.0),
            output_ip3_dbm: None,
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -20.0);

        let power_gain = amplifier.power_gain(input_power);
        assert_eq!(power_gain, 10.0);
    }

    #[test]
    fn output_power_with_compression_above_threshold() {
        let input_power: f64 = -25.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-20.0),
            output_ip3_dbm: None,
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -19.0);

        let power_gain = amplifier.power_gain(input_power);
        assert_eq!(power_gain, 6.0);
    }

    #[test]
    fn output_noise_power_without_compression() {
        let bandwidth: f64 = 1.0e6;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let output_noise_power = amplifier.output_noise_power(bandwidth);

        // With 1 MHz bandwidth, 3 dB NF (290K), thermal noise ~= -114 dBm
        // After 10 dB gain: -114 + 10 = -104 dBm
        assert!(
            (output_noise_power - (-104.02)).abs() < 0.01,
            "Expected output noise power around -104.02 dBm, got {}",
            output_noise_power
        );
    }

    #[test]
    fn output_noise_power_with_compression_below_threshold() {
        let bandwidth: f64 = 1.0e6;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-20.0), // P1dB well above noise floor
            output_ip3_dbm: None,
        };
        let output_noise_power = amplifier.output_noise_power(bandwidth);

        // Noise is -104 dBm, well below P1dB of -20 dBm, so no compression
        assert!(
            (output_noise_power - (-104.02)).abs() < 0.01,
            "Noise should not compress when well below P1dB. Expected -104.02 dBm, got {}",
            output_noise_power
        );
    }

    // ----- Dynamic Range Tests -----

    #[test]
    fn dynamic_range_with_p1db() {
        let amp = Block {
            name: "LNA".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        };
        let dr = amp.dynamic_range_db(1e6).unwrap();
        // P1dB = 10 dBm, noise floor ≈ -114 + 20 = -94 dBm → DR ≈ 104 dB
        assert!(
            dr > 100.0 && dr < 115.0,
            "Expected DR ~104 dB, got {:.1}",
            dr
        );
    }

    #[test]
    fn dynamic_range_no_p1db() {
        let block = Block::default();
        assert!(block.dynamic_range_db(1e6).is_none());
    }

    #[test]
    fn input_dynamic_range() {
        let amp = Block {
            name: "LNA".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0), // input P1dB = 10 - 20 = -10 dBm
            output_ip3_dbm: None,
        };
        let dr = amp.input_dynamic_range_db(1e6).unwrap();
        // input P1dB = -10, input noise ≈ -114 dBm → DR ≈ 104 dB
        assert!(
            dr > 100.0 && dr < 115.0,
            "Expected input DR ~104 dB, got {:.1}",
            dr
        );
    }

    // ----- AM-AM Tests -----

    #[test]
    fn am_am_curve_linear() {
        let amp = Block {
            name: "Linear Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let curve = amp.am_am_curve(&[-30.0, -20.0, -10.0]);
        assert_eq!(curve.len(), 3);
        assert_eq!(curve[0], (-30.0, -10.0));
        assert_eq!(curve[1], (-20.0, 0.0));
        assert_eq!(curve[2], (-10.0, 10.0));
    }

    #[test]
    fn am_am_curve_with_compression() {
        let amp = Block {
            name: "Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        };
        let curve = amp.am_am_curve(&[-30.0, -10.0, 0.0, 10.0]);
        // -30 + 20 = -10 (linear)
        assert_eq!(curve[0].1, -10.0);
        // -10 + 20 = 10 (at P1dB, still linear)
        assert_eq!(curve[1].1, 10.0);
        // 0 + 20 = 20 > P1dB+1=11, so clamps to 11
        assert_eq!(curve[2].1, 11.0);
        // 10 + 20 = 30 > 11, clamps to 11
        assert_eq!(curve[3].1, 11.0);
    }

    #[test]
    fn am_am_sweep_count() {
        let amp = Block {
            name: "Amp".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        };
        let sweep = amp.am_am_sweep(-40.0, -20.0, 5.0);
        assert_eq!(sweep.len(), 5); // -40, -35, -30, -25, -20
    }

    #[test]
    fn gain_compression_shows_compression() {
        let amp = Block {
            name: "Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        };
        let curve = amp.gain_compression_curve(&[-30.0, 0.0]);
        // Linear region: full 20 dB gain
        assert_eq!(curve[0].1, 20.0);
        // Compressed: 11 - 0 = 11 dB gain
        assert_eq!(curve[1].1, 11.0);
    }

    // ----- IMD3 Tests -----

    #[test]
    fn imd3_output_power() {
        let amp = Block {
            name: "Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(30.0),
        };
        // Pin = -30 dBm → Pout = -10 dBm
        // IM3 = 3*(-10) - 2*(30) = -30 - 60 = -90 dBm
        let im3 = amp.imd3_output_power_dbm(-30.0).unwrap();
        assert!((im3 - (-90.0)).abs() < 0.01);
    }

    #[test]
    fn imd3_rejection() {
        let amp = Block {
            name: "Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(30.0),
        };
        // Pin = -30 → Pout = -10, rejection = 2*(30 - (-10)) = 80 dB
        let rejection = amp.imd3_rejection_db(-30.0).unwrap();
        assert!((rejection - 80.0).abs() < 0.01);
    }

    #[test]
    fn imd3_no_ip3() {
        let amp = Block::default();
        assert!(amp.imd3_output_power_dbm(-30.0).is_none());
        assert!(amp.imd3_rejection_db(-30.0).is_none());
    }

    #[test]
    fn imd3_3to1_slope() {
        // IM3 products follow 3:1 slope: 3 dB increase per 1 dB input increase
        let amp = Block {
            name: "Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None, // no compression to keep it clean
            output_ip3_dbm: Some(30.0),
        };
        let im3_at_m30 = amp.imd3_output_power_dbm(-30.0).unwrap();
        let im3_at_m29 = amp.imd3_output_power_dbm(-29.0).unwrap();
        let delta = im3_at_m29 - im3_at_m30;
        assert!(
            (delta - 3.0).abs() < 0.01,
            "IM3 should increase 3 dB per 1 dB input, got {:.2} dB",
            delta
        );
    }

    #[test]
    fn imd3_sweep_points() {
        let amp = Block {
            name: "Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: Some(30.0),
        };
        let sweep = amp.imd3_sweep(-40.0, -20.0, 5.0);
        assert_eq!(sweep.len(), 5);
        // Verify rejection decreases as input power increases
        assert!(sweep[0].rejection_db > sweep[4].rejection_db);
    }

    #[test]
    fn imd3_sweep_no_ip3_empty() {
        let amp = Block::default();
        let sweep = amp.imd3_sweep(-40.0, -20.0, 5.0);
        assert!(sweep.is_empty());
    }

    #[test]
    fn output_noise_power_with_compression_above_threshold() {
        // To test noise compression, we need noise that actually exceeds P1dB
        // Use very high bandwidth to increase noise power
        let bandwidth: f64 = 1.0e9; // Very high bandwidth
        let amplifier = super::Block {
            name: "High Noise Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-80.0), // P1dB that noise will exceed
            output_ip3_dbm: None,
        };
        let output_noise_power = amplifier.output_noise_power(bandwidth);

        // With very high bandwidth, noise exceeds P1dB, should compress to P1dB + 1 dB
        assert_eq!(
            output_noise_power, -79.0,
            "Noise should compress to P1dB + 1 dB when above threshold"
        );
    }
}
