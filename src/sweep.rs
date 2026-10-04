use std::fmt;

use crate::{cascade_vector_return_vector, Block, Input, SignalNode};

/// Invalid sweep grid, characterization data, or input conditions.
#[derive(Clone, Debug, PartialEq)]
pub struct SweepError(String);

impl fmt::Display for SweepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for SweepError {}

fn invalid(message: impl Into<String>) -> SweepError {
    SweepError(message.into())
}

fn validate_frequency(frequency_hz: f64) -> Result<(), SweepError> {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return Err(invalid("frequency_hz must be finite and positive"));
    }
    Ok(())
}

/// A validated, strictly increasing grid of positive frequencies in Hz.
///
/// Range constructors include both endpoints. A grid holds at most one million
/// points. Use [`Self::from_frequencies`] for a single point or a measured grid.
#[derive(Clone, Debug)]
pub struct FrequencySweep {
    frequencies_hz: Vec<f64>,
}

impl FrequencySweep {
    /// Create an inclusive, linearly spaced range with at least two points.
    pub fn linear(start_hz: f64, stop_hz: f64, points: usize) -> Result<Self, SweepError> {
        Self::range(start_hz, stop_hz, points, false)
    }

    /// Create an inclusive, logarithmically spaced range with at least two points.
    pub fn logarithmic(start_hz: f64, stop_hz: f64, points: usize) -> Result<Self, SweepError> {
        Self::range(start_hz, stop_hz, points, true)
    }

    fn range(
        start_hz: f64,
        stop_hz: f64,
        points: usize,
        logarithmic: bool,
    ) -> Result<Self, SweepError> {
        validate_frequency(start_hz)?;
        validate_frequency(stop_hz)?;
        if stop_hz <= start_hz {
            return Err(invalid("stop_hz must be greater than start_hz"));
        }
        if !(2..=1_000_000).contains(&points) {
            return Err(invalid("range sweeps require 2 to 1000000 points"));
        }
        let mut frequencies = Vec::with_capacity(points);
        for i in 0..points {
            let fraction = i as f64 / (points - 1) as f64;
            let frequency = if i == 0 {
                start_hz
            } else if i == points - 1 {
                stop_hz
            } else if logarithmic {
                (start_hz.ln() * (1.0 - fraction) + stop_hz.ln() * fraction).exp()
            } else {
                start_hz + (stop_hz - start_hz) * fraction
            };
            frequencies.push(frequency);
        }
        Self::from_frequencies(frequencies)
    }

    /// Use an explicit grid without sorting, deduplicating, or rounding it.
    ///
    /// Rejects empty, nonfinite, nonpositive, repeated, or descending frequencies,
    /// including range grids whose distinct points cannot be represented by `f64`.
    pub fn from_frequencies(frequencies_hz: Vec<f64>) -> Result<Self, SweepError> {
        if frequencies_hz.is_empty() || frequencies_hz.len() > 1_000_000 {
            return Err(invalid("sweeps require 1 to 1000000 frequencies"));
        }
        for &frequency in &frequencies_hz {
            validate_frequency(frequency)?;
        }
        if frequencies_hz.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(invalid("sweep frequencies must be strictly increasing"));
        }
        Ok(Self { frequencies_hz })
    }

    /// Frequencies in ascending order, including the exact requested endpoints.
    #[must_use]
    pub fn frequencies_hz(&self) -> &[f64] {
        &self.frequencies_hz
    }
}

/// Characterization of one component at one frequency.
#[derive(Clone, Debug)]
pub struct FrequencySample {
    /// Measurement frequency in Hz.
    pub frequency_hz: f64,
    /// Gain, NF, and optional output-referred P1dB and IP3 at this frequency.
    pub block: Block,
}

#[derive(Clone, Debug)]
enum Response {
    Constant(Block),
    Tabulated(Vec<FrequencySample>),
}

/// A scalar block with either constant or tabulated frequency response.
///
/// Tabulated gain and NF interpolate linearly in dB versus Hz. P1dB and IP3
/// interpolate linearly in dBm. This is a scalar characterization model, not a
/// complex S-parameter interpolation or an impedance-mismatch calculation.
#[derive(Clone, Debug)]
pub struct FrequencyBlock {
    response: Response,
}

fn validate_block(block: &Block) -> Result<(), SweepError> {
    if !block.gain_db.is_finite()
        || !block.noise_figure_db.is_finite()
        || block.noise_figure_db < 0.0
        || block.output_p1db_dbm.is_some_and(|x| !x.is_finite())
        || block.output_ip3_dbm.is_some_and(|x| !x.is_finite())
    {
        return Err(invalid(format!(
            "block {:?}: gain and intercepts must be finite; noise figure must be finite and nonnegative",
            block.name
        )));
    }
    Ok(())
}

impl FrequencyBlock {
    /// Use the same block parameters at every frequency.
    ///
    /// Rejects nonfinite parameters and negative noise figures.
    pub fn constant(block: Block) -> Result<Self, SweepError> {
        validate_block(&block)?;
        Ok(Self {
            response: Response::Constant(block),
        })
    }

    /// Build a bounded response table from at least two ordered samples.
    ///
    /// Frequencies must be finite, positive, and strictly increasing. Samples
    /// must share a block name. Each optional intercept must be supplied at all
    /// samples or at none; missing characterization is never invented.
    /// Evaluation outside the table fails instead of extrapolating or clamping.
    pub fn tabulated(samples: Vec<FrequencySample>) -> Result<Self, SweepError> {
        if samples.len() < 2 {
            return Err(invalid("a response table requires at least two samples"));
        }
        let first = &samples[0].block;
        for sample in &samples {
            validate_frequency(sample.frequency_hz)?;
            validate_block(&sample.block)?;
            if sample.block.name != first.name
                || sample.block.output_p1db_dbm.is_some() != first.output_p1db_dbm.is_some()
                || sample.block.output_ip3_dbm.is_some() != first.output_ip3_dbm.is_some()
            {
                return Err(invalid(
                    "response samples must share a name and consistent optional intercept fields",
                ));
            }
        }
        if samples
            .windows(2)
            .any(|pair| pair[0].frequency_hz >= pair[1].frequency_hz)
        {
            return Err(invalid("response frequencies must be strictly increasing"));
        }
        Ok(Self {
            response: Response::Tabulated(samples),
        })
    }

    /// Evaluate this component at a positive, finite frequency in Hz.
    ///
    /// Exact sample frequencies return the original parameters. Interpolation
    /// does not change the stage name or optional-parameter availability.
    pub fn at_frequency(&self, frequency_hz: f64) -> Result<Block, SweepError> {
        validate_frequency(frequency_hz)?;
        let samples = match &self.response {
            Response::Constant(block) => return Ok(block.clone()),
            Response::Tabulated(samples) => samples,
        };
        let first = &samples[0];
        let last = &samples[samples.len() - 1];
        if frequency_hz < first.frequency_hz || frequency_hz > last.frequency_hz {
            return Err(invalid(format!(
                "block {:?}: frequency {frequency_hz} Hz is outside characterized range {}..={} Hz",
                first.block.name, first.frequency_hz, last.frequency_hz
            )));
        }
        let upper = samples.partition_point(|sample| sample.frequency_hz < frequency_hz);
        if samples[upper].frequency_hz == frequency_hz {
            return Ok(samples[upper].block.clone());
        }
        let low = &samples[upper - 1];
        let high = &samples[upper];
        let fraction = (frequency_hz - low.frequency_hz) / (high.frequency_hz - low.frequency_hz);
        let interpolate = |a: f64, b: f64| a * (1.0 - fraction) + b * fraction;
        let optional = |a: Option<f64>, b: Option<f64>| a.zip(b).map(|(a, b)| interpolate(a, b));
        Ok(Block {
            name: low.block.name.clone(),
            gain_db: interpolate(low.block.gain_db, high.block.gain_db),
            noise_figure_db: interpolate(low.block.noise_figure_db, high.block.noise_figure_db),
            output_p1db_dbm: optional(low.block.output_p1db_dbm, high.block.output_p1db_dbm),
            output_ip3_dbm: optional(low.block.output_ip3_dbm, high.block.output_ip3_dbm),
        })
    }
}

/// Stage-by-stage results at one input frequency.
#[derive(Clone, Debug)]
pub struct FrequencySweepPoint {
    /// Input center frequency in Hz.
    pub frequency_hz: f64,
    /// Output of every stage, in the same order as the supplied blocks.
    pub nodes: Vec<SignalNode>,
}

/// Evaluate a lineup over a frequency grid with constant input power and bandwidth.
///
/// Replaces `input.frequency_hz` with each grid frequency and preserves the source
/// temperature (including the existing 270 K default for `None`). Each point uses
/// the existing cascade gain, noise, compression, and linearity models. This is a
/// sequence of narrowband evaluations, not an integral over the swept bandwidth.
/// No frequency conversion, LO phase noise, or switch isolation is implied.
///
/// Returns an error for an empty lineup, invalid input power, nonpositive or
/// nonfinite bandwidth/temperature, or a point outside any component's table.
/// Nonfinite calculated metrics (for example, from numerical overflow) also fail.
/// No partial sweep is returned on failure. The supplied input and blocks are
/// not modified.
pub fn cascade_frequency_sweep(
    input: &Input,
    blocks: &[FrequencyBlock],
    sweep: &FrequencySweep,
) -> Result<Vec<FrequencySweepPoint>, SweepError> {
    if blocks.is_empty() {
        return Err(invalid("a frequency sweep requires at least one block"));
    }
    if !input.power_dbm.is_finite()
        || !input.bandwidth_hz.is_finite()
        || input.bandwidth_hz <= 0.0
        || input
            .noise_temperature_k
            .is_some_and(|temperature| !temperature.is_finite() || temperature <= 0.0)
    {
        return Err(invalid(
            "input power must be finite; bandwidth and source temperature must be finite and positive",
        ));
    }
    // Validate coverage before calculating any nodes.
    for block in blocks {
        block.at_frequency(sweep.frequencies_hz[0])?;
        block.at_frequency(sweep.frequencies_hz[sweep.frequencies_hz.len() - 1])?;
    }
    sweep
        .frequencies_hz
        .iter()
        .map(|&frequency_hz| {
            let evaluated = blocks
                .iter()
                .map(|block| block.at_frequency(frequency_hz))
                .collect::<Result<Vec<_>, _>>()?;
            let mut point_input = input.clone();
            point_input.frequency_hz = frequency_hz;
            let nodes = cascade_vector_return_vector(point_input, evaluated);
            for node in &nodes {
                let required = [
                    node.signal_power_dbm,
                    node.noise_power_dbm,
                    node.cumulative_gain_db,
                    node.cumulative_noise_figure_db,
                ];
                let optional = [
                    node.cumulative_noise_temperature,
                    node.cumulative_oip3_dbm,
                    node.sfdr_db,
                    node.output_p1db_dbm,
                ];
                if required.iter().any(|value| !value.is_finite())
                    || optional.iter().flatten().any(|value| !value.is_finite())
                {
                    return Err(invalid(format!(
                        "nonfinite cascade result at {frequency_hz} Hz, stage {:?}; check parameter magnitudes",
                        node.name
                    )));
                }
            }
            Ok(FrequencySweepPoint { frequency_hz, nodes })
        })
        .collect()
}
