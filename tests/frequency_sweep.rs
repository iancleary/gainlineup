use gainlineup::{
    cascade_frequency_sweep, cascade_vector_return_vector, Block, FrequencyBlock, FrequencySample,
    FrequencySweep, Input,
};

fn sample(frequency_hz: f64, gain_db: f64, noise_figure_db: f64) -> FrequencySample {
    FrequencySample {
        frequency_hz,
        block: Block {
            name: "LNA".into(),
            gain_db,
            noise_figure_db,
            output_p1db_dbm: Some(gain_db - 10.0),
            output_ip3_dbm: Some(gain_db + 5.0),
        },
    }
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

#[test]
fn inclusive_linear_logarithmic_and_measured_grids() {
    let linear = FrequencySweep::linear(2.2e9, 2.29e9, 3).unwrap();
    assert_eq!(linear.frequencies_hz(), &[2.2e9, 2.245e9, 2.29e9]);
    let log = FrequencySweep::logarithmic(1.0, 100.0, 3).unwrap();
    assert_eq!(log.frequencies_hz()[0], 1.0);
    close(log.frequencies_hz()[1], 10.0);
    assert_eq!(log.frequencies_hz()[2], 100.0);
    assert_eq!(
        FrequencySweep::from_frequencies(vec![2.245e9])
            .unwrap()
            .frequencies_hz(),
        &[2.245e9]
    );
}

#[test]
fn invalid_or_unrepresentable_grids_fail_instead_of_looping() {
    for (start, stop, points) in [
        (0.0, 10.0, 2),
        (-1.0, 10.0, 2),
        (1.0, f64::INFINITY, 2),
        (f64::NAN, 10.0, 2),
        (10.0, 1.0, 2),
        (1.0, 1.0, 2),
        (1.0, 10.0, 0),
        (1.0, 10.0, 1),
        (1.0, 10.0, 1_000_001),
        (1e16, 1e16 + 2.0, 3),
    ] {
        assert!(FrequencySweep::linear(start, stop, points).is_err());
        assert!(FrequencySweep::logarithmic(start, stop, points).is_err());
    }
    for frequencies in [vec![], vec![2.0, 1.0], vec![1.0, 1.0], vec![f64::NAN]] {
        assert!(FrequencySweep::from_frequencies(frequencies).is_err());
    }
}

#[test]
fn tables_preserve_samples_and_interpolate_db_and_dbm() {
    let table =
        FrequencyBlock::tabulated(vec![sample(2.2e9, 20.0, 1.0), sample(2.29e9, 16.0, 3.0)])
            .unwrap();
    let low = table.at_frequency(2.2e9).unwrap();
    let mid = table.at_frequency(2.2225e9).unwrap();
    let high = table.at_frequency(2.29e9).unwrap();
    assert_eq!(low.gain_db, 20.0);
    assert_eq!(high.gain_db, 16.0);
    assert_eq!(mid.name, "LNA");
    close(mid.gain_db, 19.0);
    close(mid.noise_figure_db, 1.5);
    close(mid.output_p1db_dbm.unwrap(), 9.0);
    close(mid.output_ip3_dbm.unwrap(), 24.0);
    for frequency in [2.19e9, 2.30e9, f64::NAN, f64::INFINITY, 0.0] {
        assert!(table.at_frequency(frequency).is_err());
    }
}

#[test]
fn tables_reject_missing_or_ambiguous_characterization() {
    let first = sample(1.0, 10.0, 2.0);
    let last = sample(2.0, 12.0, 3.0);
    assert!(FrequencyBlock::tabulated(vec![]).is_err());
    assert!(FrequencyBlock::tabulated(vec![first.clone()]).is_err());
    assert!(FrequencyBlock::tabulated(vec![first.clone(), first.clone()]).is_err());
    assert!(FrequencyBlock::tabulated(vec![last.clone(), first.clone()]).is_err());
    for changed in [
        FrequencySample {
            frequency_hz: f64::NAN,
            ..last.clone()
        },
        FrequencySample {
            block: Block {
                name: "Different device".into(),
                ..last.block.clone()
            },
            ..last.clone()
        },
        FrequencySample {
            block: Block {
                output_ip3_dbm: None,
                ..last.block.clone()
            },
            ..last.clone()
        },
        FrequencySample {
            block: Block {
                output_p1db_dbm: None,
                ..last.block.clone()
            },
            ..last.clone()
        },
        FrequencySample {
            block: Block {
                noise_figure_db: -1.0,
                ..last.block.clone()
            },
            ..last.clone()
        },
        FrequencySample {
            block: Block {
                gain_db: f64::INFINITY,
                ..last.block.clone()
            },
            ..last.clone()
        },
    ] {
        assert!(FrequencyBlock::tabulated(vec![first.clone(), changed]).is_err());
    }
    for block in [
        Block {
            gain_db: f64::NAN,
            ..Block::default()
        },
        Block {
            noise_figure_db: f64::NAN,
            ..Block::default()
        },
        Block {
            output_ip3_dbm: Some(f64::INFINITY),
            ..Block::default()
        },
    ] {
        assert!(FrequencyBlock::constant(block).is_err());
    }
}

#[test]
fn constant_sweep_preserves_every_legacy_node_metric_and_input() {
    let input = Input::new(1.0, 1.0e6, -80.0, Some(75.0));
    let blocks = vec![
        sample(1.0, 20.0, 1.5).block,
        Block {
            name: "Pad".into(),
            gain_db: -3.0,
            noise_figure_db: 3.0,
            ..Block::default()
        },
    ];
    let models = blocks
        .iter()
        .cloned()
        .map(FrequencyBlock::constant)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let grid = FrequencySweep::linear(1e9, 2e9, 3).unwrap();
    let sweep = cascade_frequency_sweep(&input, &models, &grid).unwrap();
    assert_eq!(input.frequency_hz, 1.0);
    for point in sweep {
        let legacy = cascade_vector_return_vector(
            Input {
                frequency_hz: point.frequency_hz,
                ..input.clone()
            },
            blocks.clone(),
        );
        for (actual, expected) in point.nodes.iter().zip(legacy.iter()) {
            assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        }
    }
}

#[test]
fn passive_sweep_follows_loss_nf_and_thermal_noise() {
    let input = Input::new(0.0, 1e6, -80.0, Some(290.0));
    let mut low = sample(1e9, -1.0, 1.0);
    low.block.output_p1db_dbm = None;
    low.block.output_ip3_dbm = None;
    let mut high = low.clone();
    high.frequency_hz = 2e9;
    high.block.gain_db = -3.0;
    high.block.noise_figure_db = 3.0;
    let models = [FrequencyBlock::tabulated(vec![low, high]).unwrap()];
    let points = cascade_frequency_sweep(
        &input,
        &models,
        &FrequencySweep::linear(1e9, 2e9, 3).unwrap(),
    )
    .unwrap();
    for (index, point) in points.iter().enumerate() {
        let node = &point.nodes[0];
        let loss = (index + 1) as f64;
        close(node.signal_power_dbm, -80.0 - loss);
        close(node.cumulative_gain_db, -loss);
        close(node.cumulative_noise_figure_db, loss);
        close(node.noise_power_dbm, input.noise_power());
        assert_eq!(node.cumulative_oip3_dbm, None);
    }
}

#[test]
fn midpoint_friis_and_compression_follow_resolved_characterization() {
    let input = Input::new(0.0, 1e6, -5.0, Some(290.0));
    let lna =
        FrequencyBlock::tabulated(vec![sample(1e9, 20.0, 1.0), sample(2e9, 16.0, 3.0)]).unwrap();
    let point = cascade_frequency_sweep(
        &input,
        &[lna],
        &FrequencySweep::from_frequencies(vec![1.5e9]).unwrap(),
    )
    .unwrap();
    let node = &point[0].nodes[0];
    close(node.signal_power_dbm, 9.0); // midpoint OP1dB 8 dBm + 1 dB clamp
    close(node.cumulative_noise_figure_db, 2.0);

    let input = Input {
        power_dbm: -80.0,
        ..input
    };
    let lna = FrequencyBlock::constant(sample(1.0, 20.0, 2.0).block).unwrap();
    let mixer_budget = FrequencyBlock::constant(sample(1.0, -6.0, 6.0).block).unwrap();
    let point = cascade_frequency_sweep(
        &input,
        &[lna, mixer_budget],
        &FrequencySweep::from_frequencies(vec![1.5e9]).unwrap(),
    )
    .unwrap();
    let expected_nf = 10.0 * (10f64.powf(0.2) + (10f64.powf(0.6) - 1.0) / 100.0).log10();
    close(point[0].nodes[1].cumulative_noise_figure_db, expected_nf);
}

#[test]
fn rejects_invalid_inputs_empty_lineups_and_incomplete_coverage() {
    let input = Input::new(0.0, 1e6, -80.0, None);
    let grid = FrequencySweep::linear(1e9, 2e9, 3).unwrap();
    let blocks = [FrequencyBlock::constant(Block::default()).unwrap()];
    assert!(cascade_frequency_sweep(&input, &[], &grid).is_err());
    for changed in [
        Input {
            bandwidth_hz: 0.0,
            ..input.clone()
        },
        Input {
            bandwidth_hz: f64::NAN,
            ..input.clone()
        },
        Input {
            power_dbm: f64::INFINITY,
            ..input.clone()
        },
        Input {
            noise_temperature_k: Some(-1.0),
            ..input.clone()
        },
    ] {
        assert!(cascade_frequency_sweep(&changed, &blocks, &grid).is_err());
    }
    let short = [
        FrequencyBlock::tabulated(vec![sample(1e9, 10.0, 1.0), sample(1.5e9, 9.0, 2.0)]).unwrap(),
    ];
    let error = cascade_frequency_sweep(&input, &short, &grid).unwrap_err();
    assert!(error.to_string().contains("outside characterized range"));
    assert!(cascade_frequency_sweep(&input, &blocks, &grid).is_ok());
}

#[test]
fn finite_parameters_that_overflow_cascade_math_return_an_error() {
    let input = Input::new(0.0, 1e6, -80.0, Some(290.0));
    let blocks = [FrequencyBlock::constant(Block {
        gain_db: 4000.0,
        noise_figure_db: 3.0,
        ..Block::default()
    })
    .unwrap()];
    let grid = FrequencySweep::linear(1e9, 2e9, 3).unwrap();
    let error = cascade_frequency_sweep(&input, &blocks, &grid).unwrap_err();
    assert!(error.to_string().contains("nonfinite cascade result"));
}
