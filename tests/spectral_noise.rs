use gainlineup::{
    cascade_vector_return_vector, integrate_small_signal_noise, Block, FrequencyBlock,
    FrequencySample, FrequencySweep, Input, NoiseError,
};
use std::error::Error;

const K: f64 = 1.380_649e-23;

fn block(name: &str, gain: f64, factor: f64) -> Block {
    Block {
        name: name.into(),
        gain_db: 10.0 * gain.log10(),
        noise_figure_db: 10.0 * factor.log10(),
        output_p1db_dbm: None,
        output_ip3_dbm: None,
    }
}

fn models(blocks: &[Block]) -> Vec<FrequencyBlock> {
    blocks
        .iter()
        .cloned()
        .map(|b| FrequencyBlock::constant(b).unwrap())
        .collect()
}

fn close(actual: f64, expected: f64, relative: f64) {
    assert!(
        (actual / expected - 1.0).abs() < relative,
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn flat_chain_matches_analytic_budget_and_legacy_at_every_prefix() {
    let blocks = [
        block("LNA", 10.0, 2.0),
        block("Pad", 0.5, 2.0),
        block("Pad", 0.25, 4.0),
    ];
    let grid = FrequencySweep::linear(1e9, 1.001e9, 5).unwrap();
    // Independent literal budgets in units k*T0*B: source and input-ordered stages.
    let budgets: &[&[f64]] = &[&[10.0, 10.0], &[5.0, 5.0, 0.5], &[1.25, 1.25, 0.125, 0.75]];
    let nodes =
        cascade_vector_return_vector(Input::new(1e9, 1e6, -100.0, Some(290.0)), blocks.to_vec());
    for length in 1..=3 {
        let result =
            integrate_small_signal_noise(290.0, &models(&blocks[..length]), &grid).unwrap();
        let scale = K * 290.0 * 1e6;
        close(
            result.source_noise_watts,
            budgets[length - 1][0] * scale,
            1e-12,
        );
        for (stage, &expected) in result.contributions.iter().zip(&budgets[length - 1][1..]) {
            close(stage.noise_power_watts, expected * scale, 1e-12);
        }
        close(
            result.total_noise_watts,
            budgets[length - 1].iter().sum::<f64>() * scale,
            1e-12,
        );
        close(
            result.total_noise_watts,
            1e-3 * 10f64.powf(nodes[length - 1].noise_power_dbm / 10.0),
            1e-12,
        );
        assert_eq!(
            result.total_noise_watts,
            result.source_noise_watts
                + result
                    .contributions
                    .iter()
                    .map(|s| s.noise_power_watts)
                    .sum::<f64>()
        );
    }
    let result = integrate_small_signal_noise(290.0, &models(&blocks), &grid).unwrap();
    assert_eq!(
        result
            .contributions
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["LNA", "Pad", "Pad"]
    );
    assert_eq!(
        (
            result.start_frequency_hz,
            result.stop_frequency_hz,
            result.frequency_points
        ),
        (1e9, 1.001e9, 5)
    );
}

#[test]
fn cold_source_gain_loss_prefixes_and_reordering_change_stage_budget() {
    let blocks = [block("LNA", 10.0, 2.0), block("Loss", 0.5, 2.0)];
    let grid = FrequencySweep::linear(1.0, 2.0, 2).unwrap();
    let input = Input::new(1.5, 1.0, -100.0, Some(0.0));
    let nodes = cascade_vector_return_vector(input, blocks.to_vec());
    for (length, expected) in [(1, vec![10.0]), (2, vec![5.0, 0.5])] {
        let result = integrate_small_signal_noise(0.0, &models(&blocks[..length]), &grid).unwrap();
        assert_eq!(result.source_noise_watts, 0.0);
        for (stage, weight) in result.contributions.iter().zip(expected) {
            close(stage.noise_power_watts, K * 290.0 * weight, 1e-12);
        }
        close(
            result.total_noise_watts,
            1e-3 * 10f64.powf(nodes[length - 1].noise_power_dbm / 10.0),
            1e-12,
        );
    }
    let reordered =
        integrate_small_signal_noise(0.0, &models(&[blocks[1].clone(), blocks[0].clone()]), &grid)
            .unwrap();
    close(
        reordered.contributions[0].noise_power_watts,
        K * 290.0 * 5.0,
        1e-12,
    );
    close(
        reordered.contributions[1].noise_power_watts,
        K * 290.0 * 10.0,
        1e-12,
    );
    close(reordered.total_noise_watts, K * 290.0 * 15.0, 1e-12);
}

#[test]
fn rectangular_band_halving_and_classical_zero_limit() {
    let stages = models(&[block("Ideal", 2.0, 1.0)]);
    let full = integrate_small_signal_noise(
        100.0,
        &stages,
        &FrequencySweep::linear(10.0, 30.0, 3).unwrap(),
    )
    .unwrap();
    let half = integrate_small_signal_noise(
        100.0,
        &stages,
        &FrequencySweep::linear(10.0, 20.0, 3).unwrap(),
    )
    .unwrap();
    close(full.total_noise_watts, K * 100.0 * 2.0 * 20.0, 1e-12);
    assert!(
        (10.0 * (half.total_noise_watts / full.total_noise_watts).log10() + 3.01029995664).abs()
            < 1e-10
    );
    let zero = integrate_small_signal_noise(
        0.0,
        &stages,
        &FrequencySweep::linear(10.0, 30.0, 3).unwrap(),
    )
    .unwrap();
    assert_eq!(zero.total_noise_watts, 0.0);
    assert_eq!(zero.source_noise_watts, 0.0);
    assert_eq!(zero.contributions[0].noise_power_watts, 0.0);
}

#[test]
fn irregular_and_logarithmic_grids_integrate_in_hz_not_grid_index() {
    // dB-linear interpolation gives G(f)=10^((f-1)/99), not a flat PSD.
    let table = FrequencyBlock::tabulated(vec![
        FrequencySample {
            frequency_hz: 1.0,
            block: block("Slope", 1.0, 1.0),
        },
        FrequencySample {
            frequency_hz: 100.0,
            block: block("Slope", 10.0, 1.0),
        },
    ])
    .unwrap();
    for grid in [
        FrequencySweep::from_frequencies(vec![1.0, 2.0, 19.0, 100.0]).unwrap(),
        FrequencySweep::logarithmic(1.0, 100.0, 9).unwrap(),
    ] {
        let frequencies = grid.frequencies_hz();
        let expected_area: f64 = frequencies
            .windows(2)
            .map(|p| {
                let gain = |f: f64| 10f64.powf((f - 1.0) / 99.0);
                (p[1] - p[0]) * (gain(p[0]) + gain(p[1])) / 2.0
            })
            .sum();
        let result =
            integrate_small_signal_noise(100.0, std::slice::from_ref(&table), &grid).unwrap();
        close(result.total_noise_watts, K * 100.0 * expected_area, 1e-12);
    }
}

#[test]
fn finite_one_pole_integral_converges_with_characterization_and_quadrature() {
    // Mathematical noise-free transfer, not a physical passive filter at 290 K.
    // Positive endpoints omit both 0..f_low and f_high..infinity.
    let low = 1.0;
    let high = 1000.0;
    let corner: f64 = 100.0;
    let exact_area = corner * ((high / corner).atan() - (low / corner).atan());
    let infinite_area = std::f64::consts::FRAC_PI_2 * corner;
    assert!((infinite_area - exact_area) / infinite_area > 0.069);
    let evaluate = |samples, points| {
        let characterization = FrequencySweep::linear(low, high, samples).unwrap();
        let table = FrequencyBlock::tabulated(
            characterization
                .frequencies_hz()
                .iter()
                .map(|&f| FrequencySample {
                    frequency_hz: f,
                    block: block("One pole", 1.0 / (1.0 + (f / corner).powi(2)), 1.0),
                })
                .collect(),
        )
        .unwrap();
        integrate_small_signal_noise(
            290.0,
            &[table],
            &FrequencySweep::linear(low, high, points).unwrap(),
        )
        .unwrap()
        .total_noise_watts
            / (K * 290.0)
    };
    let coarse = evaluate(4001, 101);
    let fine = evaluate(4001, 2001);
    assert!((fine - exact_area).abs() < (coarse - exact_area).abs());
    close(fine, exact_area, 2e-6);
    // Fine quadrature alone cannot repair an inadequately characterized response.
    assert!((evaluate(11, 2001) - exact_area).abs() > (fine - exact_area).abs() * 100.0);
}

#[test]
fn invalid_conditions_coverage_and_unrepresentable_arithmetic_are_errors() {
    let grid = FrequencySweep::linear(1.0, 2.0, 2).unwrap();
    let stages = models(&[block("Ideal", 1.0, 1.0)]);
    for temperature in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(
            integrate_small_signal_noise(temperature, &stages, &grid),
            Err(NoiseError::InvalidSourceTemperature)
        );
    }
    assert_eq!(
        integrate_small_signal_noise(0.0, &[], &grid),
        Err(NoiseError::EmptyChain)
    );
    assert_eq!(
        integrate_small_signal_noise(
            0.0,
            &stages,
            &FrequencySweep::from_frequencies(vec![1.0]).unwrap()
        ),
        Err(NoiseError::InsufficientPoints)
    );
    let table = FrequencyBlock::tabulated(vec![
        FrequencySample {
            frequency_hz: 1.0,
            block: block("Short", 1.0, 1.0),
        },
        FrequencySample {
            frequency_hz: 1.5,
            block: block("Short", 1.0, 1.0),
        },
    ])
    .unwrap();
    let error = integrate_small_signal_noise(0.0, &[table], &grid).unwrap_err();
    assert!(matches!(
        error,
        NoiseError::Characterization {
            stage_index: 0,
            frequency_hz: 2.0,
            ..
        }
    ));
    assert!(error.source().is_some());
    for (gain_db, noise_figure_db) in [(4000.0, 0.0), (-4000.0, 0.0), (0.0, 4000.0), (-3200.0, 0.0)]
    {
        let stage = FrequencyBlock::constant(Block {
            name: "Extreme".into(),
            gain_db,
            noise_figure_db,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        })
        .unwrap();
        assert!(matches!(
            integrate_small_signal_noise(290.0, &[stage], &grid),
            Err(NoiseError::Numerical { .. })
        ));
    }
    let huge = models(&[block("Huge", 1e200, 1.0), block("Huge", 1e200, 1.0)]);
    assert!(matches!(
        integrate_small_signal_noise(0.0, &huge, &grid),
        Err(NoiseError::Numerical {
            stage_index: Some(0),
            ..
        })
    ));
    let tiny = models(&[block("Tiny", 1e-200, 1.0), block("Tiny", 1e-200, 1.0)]);
    assert!(matches!(
        integrate_small_signal_noise(0.0, &tiny, &grid),
        Err(NoiseError::Numerical {
            stage_index: Some(0),
            ..
        })
    ));
    let wide = FrequencySweep::linear(1.0, 1e300, 2).unwrap();
    assert!(matches!(
        integrate_small_signal_noise(1e300, &models(&[block("Gain", 1e20, 1.0)]), &wide),
        Err(NoiseError::Numerical { .. })
    ));
}

#[test]
fn intercept_metadata_and_mixer_name_do_not_change_small_signal_result() {
    let plain = block("Mixer", 10.0, 2.0);
    let extreme = Block {
        output_p1db_dbm: Some(-300.0),
        output_ip3_dbm: Some(-300.0),
        ..plain.clone()
    };
    let grid = FrequencySweep::linear(1e9, 2e9, 3).unwrap();
    assert_eq!(
        integrate_small_signal_noise(290.0, &models(&[plain]), &grid).unwrap(),
        integrate_small_signal_noise(290.0, &models(&[extreme]), &grid).unwrap()
    );
}

#[test]
fn finite_large_psd_does_not_overflow_trapezoid_average() {
    // Both endpoint PSDs are ~1.38e308 W/Hz: their sum overflows, but their
    // average and the integral over this small declared band are representable.
    let grid = FrequencySweep::linear(1e-100, 2e-100, 2).unwrap();
    let result = integrate_small_signal_noise(
        1e300,
        &models(&[block("Extreme but representable", 1e31, 1.0)]),
        &grid,
    )
    .unwrap();
    close(result.total_noise_watts, 1.380649e208, 1e-12);
}

#[test]
fn tiny_positive_nf_is_not_rounded_to_zero_added_noise() {
    let stage = FrequencyBlock::constant(Block {
        name: "Near ideal".into(),
        gain_db: 0.0,
        noise_figure_db: 1e-16,
        output_p1db_dbm: None,
        output_ip3_dbm: None,
    })
    .unwrap();
    let result =
        integrate_small_signal_noise(0.0, &[stage], &FrequencySweep::linear(1.0, 2.0, 2).unwrap())
            .unwrap();
    close(
        result.total_noise_watts,
        K * 290.0 * std::f64::consts::LN_10 * 1e-17,
        1e-12,
    );
}
