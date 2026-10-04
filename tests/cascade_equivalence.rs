use gainlineup::{
    cascade_frequency_sweep, cascade_vector_return_output, cascade_vector_return_vector, Block,
    FrequencyBlock, FrequencySample, FrequencySweep, Input, SignalNode,
};

fn assert_option_bits(actual: Option<f64>, expected: Option<f64>, field: &str) {
    match (actual, expected) {
        (None, None) => {}
        (Some(actual), Some(expected)) => assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "{field}: {actual:?} != {expected:?}"
        ),
        (actual, expected) => panic!("{field} option mismatch: {actual:?} != {expected:?}"),
    }
}

fn assert_node_bits(actual: &SignalNode, expected: &SignalNode) {
    assert_eq!(actual.name, expected.name, "name");
    assert_eq!(
        actual.signal_frequency_hz.to_bits(),
        expected.signal_frequency_hz.to_bits(),
        "frequency"
    );
    assert_eq!(
        actual.signal_bandwidth_hz.to_bits(),
        expected.signal_bandwidth_hz.to_bits(),
        "bandwidth"
    );
    assert_eq!(
        actual.signal_power_dbm.to_bits(),
        expected.signal_power_dbm.to_bits(),
        "signal power"
    );
    assert_eq!(
        actual.noise_power_dbm.to_bits(),
        expected.noise_power_dbm.to_bits(),
        "noise power"
    );
    assert_eq!(
        actual.cumulative_noise_figure_db.to_bits(),
        expected.cumulative_noise_figure_db.to_bits(),
        "cumulative NF"
    );
    assert_eq!(
        actual.cumulative_gain_db.to_bits(),
        expected.cumulative_gain_db.to_bits(),
        "cumulative gain"
    );
    assert_option_bits(
        actual.cumulative_noise_temperature,
        expected.cumulative_noise_temperature,
        "noise temperature",
    );
    assert_option_bits(
        actual.cumulative_oip3_dbm,
        expected.cumulative_oip3_dbm,
        "cumulative OIP3",
    );
    assert_option_bits(actual.sfdr_db, expected.sfdr_db, "SFDR");
    assert_option_bits(
        actual.output_p1db_dbm,
        expected.output_p1db_dbm,
        "output P1dB",
    );
}

fn chained(input: &Input, blocks: &[Block]) -> Vec<SignalNode> {
    let mut nodes: Vec<SignalNode> = Vec::with_capacity(blocks.len());
    for block in blocks {
        let node = match nodes.last() {
            Some(previous) => previous.cascade_block(block),
            None => input.cascade_block(block),
        };
        nodes.push(node);
    }
    nodes
}

fn block(
    name: &str,
    gain_db: f64,
    noise_figure_db: f64,
    p1: Option<f64>,
    ip3: Option<f64>,
) -> Block {
    Block {
        name: name.to_owned(),
        gain_db,
        noise_figure_db,
        output_p1db_dbm: p1,
        output_ip3_dbm: ip3,
    }
}

#[test]
fn scalar_vector_and_output_apis_match_explicit_chaining_bit_for_bit() {
    let cases = [
        (Input::new(915e6, 200e3, -35.0, None), vec![]),
        (
            Input::new(2.4e9, 20e6, -25.0, Some(75.0)),
            vec![block("单段", 18.25, 1.2, Some(-4.0), Some(15.0))],
        ),
        (
            Input::new(5.8e9, 40e6, 8.0, Some(290.0)),
            vec![
                block("hot PA", 24.0, 3.1, Some(20.0), Some(32.0)),
                block("attenuator", -12.0, 12.0, None, None),
                block("", 17.0, 2.0, Some(-30.0), Some(8.0)),
            ],
        ),
        (
            Input::new(1.2e9, 1e6, -20.0, None),
            vec![
                block("IP3 disappears", 5.0, 0.4, None, None),
                block("later intercept", 9.0, 1.0, Some(18.0), Some(27.0)),
                block("compressed", 12.0, 2.3, Some(-1.0), Some(24.0)),
            ],
        ),
    ];

    for (input, blocks) in cases {
        let expected = chained(&input, &blocks);
        let actual = cascade_vector_return_vector(input.clone(), blocks.clone());
        assert_eq!(actual.len(), expected.len());
        for (actual, expected) in actual.iter().zip(&expected) {
            assert_node_bits(actual, expected);
        }
        let output = cascade_vector_return_output(input, blocks);
        if let Some(expected) = expected.last() {
            assert_node_bits(&output, expected);
        } else {
            assert_node_bits(&output, &SignalNode::default());
        }
    }
}

fn sample(
    frequency_hz: f64,
    name: &str,
    gain: f64,
    nf: f64,
    p1: Option<f64>,
    ip3: Option<f64>,
) -> FrequencySample {
    FrequencySample {
        frequency_hz,
        block: block(name, gain, nf, p1, ip3),
    }
}

#[test]
fn frequency_sweep_matches_public_resolution_then_scalar_chaining_bit_for_bit() {
    let constant_only = vec![
        FrequencyBlock::constant(block("fixed", 20.0, 1.0, Some(5.0), Some(28.0))).unwrap(),
        FrequencyBlock::constant(block("loss", -3.0, 3.0, None, None)).unwrap(),
    ];
    let mixed = vec![
        FrequencyBlock::tabulated(vec![
            sample(1e9, "таблица", 22.0, 0.7, Some(8.0), Some(35.0)),
            sample(10e9, "таблица", 16.0, 2.1, Some(2.0), Some(29.0)),
            sample(100e9, "таблица", 11.0, 3.5, Some(-4.0), Some(23.0)),
        ])
        .unwrap(),
        FrequencyBlock::constant(block("compressor", 18.0, 2.0, Some(-10.0), Some(25.0))).unwrap(),
        FrequencyBlock::tabulated(vec![
            sample(1e9, "no intercepts", -4.0, 4.0, None, None),
            sample(100e9, "no intercepts", -7.0, 6.0, None, None),
        ])
        .unwrap(),
    ];

    let mut points = vec![1e9, 10e9, 100e9]; // exact endpoints and interior knots
    points.extend((1..=510).map(|index| 10f64.powf(9.0 + 2.0 * f64::from(index) / 511.0)));
    points.sort_by(f64::total_cmp);
    points.dedup_by(|a, b| a.to_bits() == b.to_bits());

    for (models, grid) in [
        (&constant_only, vec![2e9]),      // single point, all blocks constant
        (&constant_only, points.clone()), // many points, exercising constant reuse
        (&mixed, points),                 // logarithmically distributed, irregular spacing
    ] {
        let input = Input::new(7e9, 5e6, 6.0, Some(75.0));
        let sweep = FrequencySweep::from_frequencies(grid.clone()).unwrap();
        let actual = cascade_frequency_sweep(&input, models, &sweep).unwrap();
        assert_eq!(actual.len(), grid.len());
        for (point, &frequency_hz) in actual.iter().zip(&grid) {
            assert_eq!(point.frequency_hz.to_bits(), frequency_hz.to_bits());
            let resolved: Vec<_> = models
                .iter()
                .map(|model| model.at_frequency(frequency_hz).unwrap())
                .collect();
            let mut point_input = input.clone();
            point_input.frequency_hz = frequency_hz;
            let expected = chained(&point_input, &resolved);
            assert_eq!(point.nodes.len(), expected.len());
            for (actual, expected) in point.nodes.iter().zip(&expected) {
                assert_node_bits(actual, expected);
            }
        }
    }
}

#[test]
fn all_constant_sweep_still_rejects_nonfinite_cascade_results() {
    let input = Input::new(0.0, 1e6, -80.0, Some(290.0));
    let blocks = [FrequencyBlock::constant(block("overflow", 4000.0, 3.0, None, None)).unwrap()];
    let grid = FrequencySweep::linear(1e9, 2e9, 3).unwrap();
    let error = cascade_frequency_sweep(&input, &blocks, &grid).unwrap_err();
    assert!(error.to_string().contains("nonfinite cascade result"));
}
