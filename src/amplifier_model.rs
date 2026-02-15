use std::fmt;

use crate::block::Block;

/// A single point from a combined AM-AM + AM-PM sweep.
#[derive(Clone, Debug)]
pub struct AmplifierPoint {
    /// Input power (dBm).
    pub input_dbm: f64,
    /// Output power (dBm).
    pub output_dbm: f64,
    /// Power gain (dB).
    pub gain_db: f64,
    /// AM-PM phase shift (degrees), if AM-PM coefficient is set.
    pub phase_shift_deg: Option<f64>,
}

impl fmt::Display for AmplifierPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AmplifierPoint {{ Pin: {:.1} dBm, Pout: {:.1} dBm, Gain: {:.1} dB, Δφ: {} }}",
            self.input_dbm,
            self.output_dbm,
            self.gain_db,
            match self.phase_shift_deg {
                Some(v) => format!("{:.2}°", v),
                None => "N/A".to_string(),
            }
        )
    }
}

/// Amplifier model wrapping a [`Block`] with optional AM-PM characterization.
///
/// This is intentionally separate from `Block` to keep the core cascade model
/// simple while allowing richer amplifier analysis when needed.
#[derive(Clone, Debug)]
pub struct AmplifierModel<'a> {
    /// The underlying block with gain, NF, P1dB, and IP3.
    pub block: &'a Block,
    /// AM-PM conversion coefficient in °/dB near P1dB.
    pub am_pm_coefficient_deg_per_db: Option<f64>,
    /// Saturated output power (dBm).
    pub saturation_power_dbm: Option<f64>,
}

impl<'a> AmplifierModel<'a> {
    /// Create an amplifier model with no AM-PM characterization.
    pub fn new(block: &'a Block) -> Self {
        Self {
            block,
            am_pm_coefficient_deg_per_db: None,
            saturation_power_dbm: None,
        }
    }

    /// Create an amplifier model with AM-PM coefficient.
    pub fn with_am_pm(block: &'a Block, coeff_deg_per_db: f64) -> Self {
        Self {
            block,
            am_pm_coefficient_deg_per_db: Some(coeff_deg_per_db),
            saturation_power_dbm: None,
        }
    }

    /// Create an amplifier model with saturation power.
    pub fn with_saturation(block: &'a Block, psat_dbm: f64) -> Self {
        Self {
            block,
            am_pm_coefficient_deg_per_db: None,
            saturation_power_dbm: Some(psat_dbm),
        }
    }

    /// Return a builder for configuring optional fields.
    pub fn builder(block: &'a Block) -> AmplifierModelBuilder<'a> {
        AmplifierModelBuilder {
            block,
            am_pm_coefficient_deg_per_db: None,
            saturation_power_dbm: None,
        }
    }

    /// Input P1dB in dBm (output P1dB minus small-signal gain).
    fn input_p1db_dbm(&self) -> Option<f64> {
        self.block.output_p1db_dbm.map(|p| p - self.block.gain_db)
    }

    /// AM-PM phase shift in degrees at a given input power.
    ///
    /// Simple model: `Δφ = coeff × max(0, Pin − (input_P1dB − backoff_margin))`
    /// where the phase shift ramps linearly as input approaches and exceeds the
    /// input-referred P1dB. At deep backoff the phase shift is zero.
    ///
    /// Returns `None` if no AM-PM coefficient is set or if `output_p1db_dbm` is not
    /// set on the underlying block.
    pub fn phase_shift_at(&self, input_power_dbm: f64) -> Option<f64> {
        let coeff = self.am_pm_coefficient_deg_per_db?;
        let input_p1db = self.input_p1db_dbm()?;
        // Phase shift relative to input P1dB: zero when well below, positive near/above
        let delta = input_power_dbm - input_p1db;
        // Allow phase shift to go negative (deep backoff) but clamp at 0
        let phase = coeff * delta.max(0.0);
        Some(phase)
    }

    /// Combined AM-AM + AM-PM sweep.
    ///
    /// Returns one [`AmplifierPoint`] for each step from `start_dbm` to `stop_dbm`.
    pub fn am_am_am_pm_sweep(
        &self,
        start_dbm: f64,
        stop_dbm: f64,
        step_db: f64,
    ) -> Vec<AmplifierPoint> {
        let mut results = Vec::new();
        let mut pin = start_dbm;
        while pin <= stop_dbm + step_db * 0.01 {
            let pout = self.block.output_power(pin);
            let gain = pout - pin;
            let phase = self.phase_shift_at(pin);
            results.push(AmplifierPoint {
                input_dbm: pin,
                output_dbm: pout,
                gain_db: gain,
                phase_shift_deg: phase,
            });
            pin += step_db;
        }
        results
    }

    /// Required input backoff (dB below input P1dB) to stay within a phase shift target.
    ///
    /// Returns the backoff in dB (positive means below P1dB). For example, if
    /// `max_phase_deg` is 5° and the coefficient is 10 °/dB, the amplifier can
    /// tolerate 0.5 dB above input P1dB, so the backoff is −0.5 dB (i.e., you can
    /// actually be 0.5 dB *above* P1dB). More typically, a tight phase budget
    /// requires operating below P1dB.
    ///
    /// Returns `None` if no AM-PM coefficient is set or coefficient is zero.
    pub fn backoff_for_target_phase(&self, max_phase_deg: f64) -> Option<f64> {
        let coeff = self.am_pm_coefficient_deg_per_db?;
        if coeff == 0.0 {
            return None;
        }
        // Phase = coeff * max(0, Pin - input_P1dB)
        // We want phase <= max_phase_deg
        // coeff * (Pin - P1dB_in) = max_phase_deg
        // Pin = P1dB_in + max_phase_deg / coeff
        // Backoff = P1dB_in - Pin = -(max_phase_deg / coeff)
        // Negative backoff means you can exceed P1dB; positive means you must stay below.
        let allowed_above_p1db = max_phase_deg / coeff;
        Some(-allowed_above_p1db)
    }

    /// EVM contribution from AM-PM distortion at a given input power.
    ///
    /// Approximation: `EVM ≈ sin(Δφ)` for small angles, expressed as a ratio (not %).
    ///
    /// Returns `None` if phase shift is unavailable.
    pub fn evm_from_am_pm(&self, input_power_dbm: f64) -> Option<f64> {
        let phase_deg = self.phase_shift_at(input_power_dbm)?;
        let phase_rad = phase_deg.to_radians();
        Some(phase_rad.sin().abs())
    }
}

/// Builder for [`AmplifierModel`].
#[derive(Clone, Debug)]
pub struct AmplifierModelBuilder<'a> {
    block: &'a Block,
    am_pm_coefficient_deg_per_db: Option<f64>,
    saturation_power_dbm: Option<f64>,
}

impl<'a> AmplifierModelBuilder<'a> {
    /// Set the AM-PM conversion coefficient (°/dB).
    pub fn am_pm_coefficient(mut self, coeff_deg_per_db: f64) -> Self {
        self.am_pm_coefficient_deg_per_db = Some(coeff_deg_per_db);
        self
    }

    /// Set the saturated output power (dBm).
    pub fn saturation_power(mut self, psat_dbm: f64) -> Self {
        self.saturation_power_dbm = Some(psat_dbm);
        self
    }

    /// Build the [`AmplifierModel`].
    pub fn build(self) -> AmplifierModel<'a> {
        AmplifierModel {
            block: self.block,
            am_pm_coefficient_deg_per_db: self.am_pm_coefficient_deg_per_db,
            saturation_power_dbm: self.saturation_power_dbm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_block() -> Block {
        Block {
            name: "Test Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0), // input P1dB = -10 dBm
            output_ip3_dbm: Some(25.0),
        }
    }

    #[test]
    fn new_has_no_am_pm() {
        let block = test_block();
        let model = AmplifierModel::new(&block);
        assert!(model.phase_shift_at(-30.0).is_none());
    }

    #[test]
    fn with_am_pm_returns_phase_shift() {
        let block = test_block();
        let model = AmplifierModel::with_am_pm(&block, 10.0); // 10 °/dB
        // At input P1dB (-10 dBm), delta = 0 → phase = 0
        let phase = model.phase_shift_at(-10.0).unwrap();
        assert!((phase - 0.0).abs() < 1e-10, "Phase at P1dB should be 0, got {}", phase);
        // At 5 dB above input P1dB (-5 dBm): phase = 10 * 5 = 50°
        let phase = model.phase_shift_at(-5.0).unwrap();
        assert!((phase - 50.0).abs() < 1e-10, "Expected 50°, got {}", phase);
    }

    #[test]
    fn phase_shift_zero_at_deep_backoff() {
        let block = test_block();
        let model = AmplifierModel::with_am_pm(&block, 10.0);
        // At -50 dBm, well below input P1dB of -10 dBm
        let phase = model.phase_shift_at(-50.0).unwrap();
        assert!((phase - 0.0).abs() < 1e-10, "Phase at deep backoff should be 0");
    }

    #[test]
    fn phase_shift_increases_toward_p1db() {
        let block = test_block();
        let model = AmplifierModel::with_am_pm(&block, 10.0);
        let phase_low = model.phase_shift_at(-15.0).unwrap(); // below P1dB → 0
        let phase_high = model.phase_shift_at(-5.0).unwrap(); // above P1dB → positive
        assert!(
            phase_high > phase_low,
            "Phase should increase toward P1dB: low={}, high={}",
            phase_low,
            phase_high
        );
    }

    #[test]
    fn am_am_am_pm_sweep_count() {
        let block = test_block();
        let model = AmplifierModel::with_am_pm(&block, 10.0);
        let sweep = model.am_am_am_pm_sweep(-40.0, -20.0, 5.0);
        // -40, -35, -30, -25, -20 → 5 points
        assert_eq!(sweep.len(), 5);
    }

    #[test]
    fn backoff_for_target_phase_reasonable() {
        let block = test_block();
        let model = AmplifierModel::with_am_pm(&block, 10.0); // 10 °/dB
        // For max 5°: allowed_above = 5/10 = 0.5 dB → backoff = -0.5 (can be above P1dB)
        let backoff = model.backoff_for_target_phase(5.0).unwrap();
        assert!(
            (backoff - (-0.5)).abs() < 1e-10,
            "Expected backoff of -0.5, got {}",
            backoff
        );
    }

    #[test]
    fn evm_from_am_pm_zero_at_backoff() {
        let block = test_block();
        let model = AmplifierModel::with_am_pm(&block, 10.0);
        // At deep backoff, phase = 0, EVM = sin(0) = 0
        let evm = model.evm_from_am_pm(-50.0).unwrap();
        assert!((evm - 0.0).abs() < 1e-10, "EVM at deep backoff should be 0");
    }

    #[test]
    fn builder_pattern_works() {
        let block = test_block();
        let model = AmplifierModel::builder(&block)
            .am_pm_coefficient(10.0)
            .saturation_power(25.0)
            .build();
        assert_eq!(model.am_pm_coefficient_deg_per_db, Some(10.0));
        assert_eq!(model.saturation_power_dbm, Some(25.0));
        // Phase shift should work
        let phase = model.phase_shift_at(-5.0).unwrap();
        assert!((phase - 50.0).abs() < 1e-10);
    }

    #[test]
    fn with_saturation_constructor() {
        let block = test_block();
        let model = AmplifierModel::with_saturation(&block, 25.0);
        assert_eq!(model.saturation_power_dbm, Some(25.0));
        assert!(model.am_pm_coefficient_deg_per_db.is_none());
    }
}
