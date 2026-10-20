use gainlineup::{
    cascade_vector_return_vector, integrate_small_signal_noise, Block, FrequencyBlock,
    FrequencySweep, Input, PassiveLossError,
};

const K: f64 = 1.380_649e-23;
const BANDWIDTH_HZ: f64 = 1e6;

fn close(actual: f64, expected: f64) {
    assert!(
        (actual / expected - 1.0).abs() < 1e-12,
        "{actual:e} != {expected:e}"
    );
}

fn pad(name: &str, linear_loss: f64, temperature_k: f64) -> Block {
    Block::passive_loss(name, 10.0 * linear_loss.log10(), temperature_k).unwrap()
}

#[test]
fn matched_pad_noise_figure_and_thermal_equilibrium() {
    let cold = pad("Cold pad", 2.0, 150.0);
    close(cold.noise_figure_db, 1.8105467858723132);
    close(cold.noise_temperature(), 150.0);
    for temperature_k in [0.0, 150.0, 290.0, 300.0] {
        let stage = pad("Pad", 2.0, temperature_k);
        assert_eq!(stage.output_p1db_dbm, None);
        assert_eq!(stage.output_ip3_dbm, None);
        if temperature_k == 0.0 {
            assert_eq!(stage.noise_figure_db, 0.0);
        } else {
            let input = Input::new(1e9, BANDWIDTH_HZ, -100.0, Some(temperature_k));
            let node = input.cascade_block(&stage);
            close(
                1e-3 * 10f64.powf(node.noise_power_dbm / 10.0),
                K * temperature_k * BANDWIDTH_HZ,
            );
            let result = integrate_small_signal_noise(
                temperature_k,
                &[FrequencyBlock::constant(stage.clone()).unwrap()],
                &FrequencySweep::linear(1e9, 1.001e9, 3).unwrap(),
            )
            .unwrap();
            close(result.total_noise_watts, K * temperature_k * BANDWIDTH_HZ);
        }
        if temperature_k == 290.0 {
            close(stage.noise_figure_db, -stage.gain_db);
        }
    }
}

#[test]
fn mixed_temperatures_preserve_order_and_prefix_budgets_across_public_apis() {
    let forward = [pad("150 K pad", 2.0, 150.0), pad("300 K pad", 4.0, 300.0)];
    let reverse = [forward[1].clone(), forward[0].clone()];
    // Independent output-temperature budgets: source, then added noise in stage order.
    for (blocks, budgets) in [
        (&forward, [[145.0, 75.0, 0.0], [36.25, 18.75, 225.0]]),
        (&reverse, [[72.5, 225.0, 0.0], [36.25, 112.5, 75.0]]),
    ] {
        let nodes = cascade_vector_return_vector(
            Input::new(1e9, BANDWIDTH_HZ, -100.0, Some(290.0)),
            blocks.to_vec(),
        );
        for length in 1..=2 {
            let models: Vec<_> = blocks[..length]
                .iter()
                .cloned()
                .map(|b| FrequencyBlock::constant(b).unwrap())
                .collect();
            let result = integrate_small_signal_noise(
                290.0,
                &models,
                &FrequencySweep::linear(1e9, 1.001e9, 5).unwrap(),
            )
            .unwrap();
            let scale = K * BANDWIDTH_HZ;
            let budget = budgets[length - 1];
            close(result.source_noise_watts, budget[0] * scale);
            for (index, contribution) in result.contributions.iter().enumerate() {
                assert_eq!(contribution.name, blocks[index].name);
                close(contribution.noise_power_watts, budget[index + 1] * scale);
            }
            let expected_temperature_k: f64 = budget.iter().sum();
            close(result.total_noise_watts, expected_temperature_k * scale);
            let node = &nodes[length - 1];
            close(
                1e-3 * 10f64.powf(node.noise_power_dbm / 10.0),
                expected_temperature_k * scale,
            );
            // This scalar field is input-referred, unlike the output noise budget.
            close(
                node.cumulative_noise_temperature.unwrap()
                    * 10f64.powf(node.cumulative_gain_db / 10.0),
                expected_temperature_k,
            );
            assert_eq!(node.cumulative_oip3_dbm, None);
            assert_eq!(node.sfdr_db, None);
            assert_eq!(node.output_p1db_dbm, None);
        }
    }
}

#[test]
fn invalid_inputs_and_conservative_numerical_limits_are_typed_errors() {
    for loss_db in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            Block::passive_loss("Bad loss", loss_db, 290.0).unwrap_err(),
            PassiveLossError::InvalidLoss
        );
    }
    for temperature_k in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            Block::passive_loss("Bad temperature", 0.0, temperature_k).unwrap_err(),
            PassiveLossError::InvalidTemperature
        );
    }
    for (loss_db, temperature_k) in [
        (4000.0, 0.0),              // Gain underflows, even without added noise.
        (3100.0, 290.0),            // L - 1 overflows.
        (100.0, f64::MAX),          // Excess noise factor overflows.
        (1e-300, 1e-300),           // Positive excess factor underflows.
        (f64::from_bits(1), 290.0), // Loss exponent underflows.
    ] {
        assert_eq!(
            Block::passive_loss("Extreme", loss_db, temperature_k).unwrap_err(),
            PassiveLossError::Numerical
        );
    }
    assert!(PassiveLossError::Numerical
        .to_string()
        .contains("magnitudes"));
}

#[test]
fn zero_limits_and_small_positive_noise_do_not_lose_simple_cases() {
    for temperature_k in [0.0, 290.0, f64::MAX, f64::from_bits(1)] {
        let stage = Block::passive_loss("No loss", 0.0, temperature_k).unwrap();
        assert_eq!(stage.noise_figure_db, 0.0);
        assert_eq!(stage.gain_db, 0.0);
    }
    for loss_db in [f64::from_bits(1), 3.0, 3200.0] {
        assert_eq!(
            Block::passive_loss("Classically cold", loss_db, 0.0)
                .unwrap()
                .noise_figure_db,
            0.0
        );
    }
    let tiny = Block::passive_loss("Tiny loss", 1e-20, 150.0).unwrap();
    close(tiny.noise_figure_db, 5.172413793103448e-21);
    // Avoid overflow of Te when the dimensionless factor is representable.
    assert!(pad("Hot", 2.0, f64::MAX).noise_figure_db.is_finite());
    // Do not erase a subnormal physical temperature before multiplying by loss.
    assert!(
        Block::passive_loss("Tiny temperature", 100.0, f64::from_bits(1))
            .unwrap()
            .noise_figure_db
            > 0.0
    );
}
