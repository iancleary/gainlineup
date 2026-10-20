use std::{error::Error, fmt};

use crate::{constants::BOLTZMANN, FrequencyBlock, FrequencySweep, SweepError};

/// One stage's added noise, referred to the final output of the supplied chain.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseContribution {
    /// Stage name; duplicate names still identify separate, input-ordered entries.
    pub name: String,
    /// Band-integrated, final-output-referred added noise in watts.
    pub noise_power_watts: f64,
}

/// Small-signal noise integrated only over the declared, finite frequency band.
#[derive(Clone, Debug, PartialEq)]
pub struct IntegratedNoise {
    /// Exact first grid frequency in Hz (no implicit extension to zero).
    pub start_frequency_hz: f64,
    /// Exact last grid frequency in Hz (no extrapolated filter tails).
    pub stop_frequency_hz: f64,
    /// Number of quadrature points used, including both endpoints.
    pub frequency_points: usize,
    /// Source noise propagated to the final output, in watts.
    pub source_noise_watts: f64,
    /// Stage-added noise at the final output, in input order.
    pub contributions: Vec<NoiseContribution>,
    /// Source plus all stage contributions, summed in watts.
    pub total_noise_watts: f64,
}

/// Invalid integration conditions or unrepresentable noise arithmetic.
#[derive(Clone, Debug, PartialEq)]
pub enum NoiseError {
    /// Source temperature must be finite and nonnegative.
    InvalidSourceTemperature,
    /// At least one stage is required.
    EmptyChain,
    /// Integration requires at least two distinct grid frequencies.
    InsufficientPoints,
    /// A stage cannot be evaluated at a requested frequency.
    Characterization {
        /// Zero-based input stage index.
        stage_index: usize,
        /// Requested frequency in Hz.
        frequency_hz: f64,
        /// Underlying characterization error.
        source: SweepError,
    },
    /// A conversion, product, integral, or sum overflowed or underflowed to zero.
    Numerical {
        /// Zero-based stage index, or `None` for source/total arithmetic.
        stage_index: Option<usize>,
        /// Frequency in Hz; interval errors identify the upper endpoint.
        frequency_hz: f64,
    },
}

impl fmt::Display for NoiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSourceTemperature => f.write_str("source temperature must be finite and nonnegative"),
            Self::EmptyChain => f.write_str("noise integration requires at least one stage"),
            Self::InsufficientPoints => f.write_str("noise integration requires at least two frequencies"),
            Self::Characterization { stage_index, frequency_hz, source } => write!(f, "stage {stage_index} at {frequency_hz} Hz: {source}"),
            Self::Numerical { stage_index, frequency_hz } => write!(f, "unrepresentable noise arithmetic at {frequency_hz} Hz, stage {stage_index:?}; check parameter magnitudes"),
        }
    }
}

impl Error for NoiseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Characterization { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn checked(value: f64, positive: bool, error: &NoiseError) -> Result<f64, NoiseError> {
    if !value.is_finite() || value < 0.0 || (positive && value == 0.0) {
        Err(error.clone())
    } else {
        Ok(value)
    }
}

fn product(a: f64, b: f64, error: &NoiseError) -> Result<f64, NoiseError> {
    checked(a * b, a > 0.0 && b > 0.0, error)
}

/// Integrate noise for matched, linear, frequency-preserving stages with
/// independent added noise and a constant source temperature across the band.
///
/// Always uses nominal small-signal power gain. P1dB and IP3 metadata have no
/// effect: without signal power this API cannot validate a hardware operating
/// point. Even a stage named "Mixer" does not imply frequency conversion.
/// The caller must establish that these modeling assumptions are appropriate.
///
/// At each frequency, stage `i` contributes `k * 290 * (F_i - 1)` W/Hz at its
/// input, multiplied by all gains from stage `i` through the final output.
/// The source contributes `k * source_temperature_k` times the whole-chain gain.
/// Each contribution is trapezoid-integrated in watts using actual Hz spacing.
/// Gain and NF use [`FrequencyBlock`]'s dB-versus-Hz interpolation. Refining the
/// quadrature grid does not add characterization data. No ENBW is inferred.
///
/// Zero kelvin and zero NF are accepted classical ideal limits; watts avoid
/// negative-infinite dBm for zero noise. Rejects invalid temperature, empty
/// chains, single-point grids, missing endpoint coverage, and unrepresentable
/// arithmetic, with no partial successful result. Gain underflow to zero is
/// rejected, not interpreted as perfect isolation. Ordinary `f64` rounding,
/// subnormal precision loss, and loss of tiny terms in sums remain possible.
///
/// Uses O(stages) auxiliary storage independent of grid length. Noise arithmetic
/// is O(points * stages); tabulated evaluation additionally requires binary
/// searches, O(points * sum(log(table samples per stage))).
pub fn integrate_small_signal_noise(
    source_temperature_k: f64,
    blocks: &[FrequencyBlock],
    grid: &FrequencySweep,
) -> Result<IntegratedNoise, NoiseError> {
    if !source_temperature_k.is_finite() || source_temperature_k < 0.0 {
        return Err(NoiseError::InvalidSourceTemperature);
    }
    if blocks.is_empty() {
        return Err(NoiseError::EmptyChain);
    }
    let frequencies = grid.frequencies_hz();
    if frequencies.len() < 2 {
        return Err(NoiseError::InsufficientPoints);
    }
    let start = frequencies[0];
    let stop = frequencies[frequencies.len() - 1];
    let evaluate = |index: usize, frequency_hz: f64| {
        blocks[index]
            .evaluate(frequency_hz)
            .map_err(|source| NoiseError::Characterization {
                stage_index: index,
                frequency_hz,
                source,
            })
    };
    let mut contributions = Vec::with_capacity(blocks.len());
    for index in 0..blocks.len() {
        let first = evaluate(index, start)?;
        evaluate(index, stop)?;
        contributions.push(NoiseContribution {
            name: first.name.clone(),
            noise_power_watts: 0.0,
        });
    }
    // The final slot carries source PSD; the preceding slots retain input order.
    let mut previous = vec![0.0; blocks.len() + 1];
    let mut current = previous.clone();
    let mut source_noise_watts = 0.0;
    for (point, &frequency_hz) in frequencies.iter().enumerate() {
        let mut downstream_gain = 1.0;
        for index in (0..blocks.len()).rev() {
            let block = evaluate(index, frequency_hz)?;
            let error = NoiseError::Numerical {
                stage_index: Some(index),
                frequency_hz,
            };
            let gain = checked(10.0_f64.powf(block.gain_db / 10.0), true, &error)?;
            downstream_gain = product(downstream_gain, gain, &error)?;
            let excess_factor = checked(
                (block.noise_figure_db * (std::f64::consts::LN_10 / 10.0)).exp_m1(),
                block.noise_figure_db > 0.0,
                &error,
            )?;
            current[index] = product(
                product(BOLTZMANN * 290.0, excess_factor, &error)?,
                downstream_gain,
                &error,
            )?;
        }
        let error = NoiseError::Numerical {
            stage_index: None,
            frequency_hz,
        };
        current[blocks.len()] = product(
            product(BOLTZMANN, source_temperature_k, &error)?,
            downstream_gain,
            &error,
        )?;
        if point > 0 {
            let width_hz = checked(frequency_hz - frequencies[point - 1], true, &error)?;
            for index in 0..current.len() {
                let error = NoiseError::Numerical {
                    stage_index: (index < blocks.len()).then_some(index),
                    frequency_hz,
                };
                let low = previous[index].min(current[index]);
                let high = previous[index].max(current[index]);
                // Avoid overflow from adding two large PSDs before dividing by two.
                let average = checked(low + (high - low) / 2.0, high > 0.0, &error)?;
                let area = product(average, width_hz, &error)?;
                let accumulated = if index == blocks.len() {
                    &mut source_noise_watts
                } else {
                    &mut contributions[index].noise_power_watts
                };
                *accumulated = checked(*accumulated + area, false, &error)?;
            }
        }
        std::mem::swap(&mut current, &mut previous);
    }
    let error = NoiseError::Numerical {
        stage_index: None,
        frequency_hz: stop,
    };
    let total_noise_watts = contributions
        .iter()
        .try_fold(source_noise_watts, |sum, stage| {
            checked(sum + stage.noise_power_watts, false, &error)
        })?;
    Ok(IntegratedNoise {
        start_frequency_hz: start,
        stop_frequency_hz: stop,
        frequency_points: frequencies.len(),
        source_noise_watts,
        contributions,
        total_noise_watts,
    })
}
