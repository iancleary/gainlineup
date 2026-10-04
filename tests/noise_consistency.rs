use gainlineup::{Block, Input, SignalNode};

fn block(gain_db: f64, noise_figure_db: f64) -> Block {
    Block {
        name: "Stage".to_string(),
        gain_db,
        noise_figure_db,
        output_p1db_dbm: None,
        output_ip3_dbm: None,
    }
}

fn watts(dbm: f64) -> f64 {
    1.0e-3 * 10.0_f64.powf(dbm / 10.0)
}

#[test]
fn six_db_noise_figure_adds_one_noise_factor_term() {
    let stage = block(20.0, 6.0);
    let bandwidth_hz = 1.0e6;
    let factor = 10.0_f64.powf(6.0 / 10.0);
    let expected_input_w = 1.380_649e-23 * 290.0 * (factor - 1.0) * bandwidth_hz;
    let actual_input_w = watts(stage.input_noise_power(bandwidth_hz));
    assert!((actual_input_w / expected_input_w - 1.0).abs() < 1.0e-12);

    let input = Input::new(1.0e9, bandwidth_hz, -50.0, Some(290.0));
    let output = input.cascade_block(&stage);
    let expected_output_w = 1.380_649e-23 * 290.0 * factor * bandwidth_hz * 100.0;
    assert!((watts(output.noise_power_dbm) / expected_output_w - 1.0).abs() < 1.0e-12);
    assert!((output.cumulative_noise_temperature.unwrap() - 290.0 * factor).abs() < 1.0e-9);
}

#[test]
fn first_stage_matches_equivalent_source_node_even_under_signal_compression() {
    let input = Input::new(1.0e9, 1.0e6, 0.0, Some(290.0));
    let mut stage = block(20.0, 6.0);
    stage.output_p1db_dbm = Some(10.0);
    let source = SignalNode {
        name: "Input".to_string(),
        signal_frequency_hz: input.frequency_hz,
        signal_bandwidth_hz: input.bandwidth_hz,
        signal_power_dbm: input.power_dbm,
        noise_power_dbm: input.noise_power(),
        cumulative_noise_figure_db: 0.0,
        cumulative_gain_db: 0.0,
        cumulative_noise_temperature: input.noise_temperature_k,
        cumulative_oip3_dbm: None,
        sfdr_db: None,
        output_p1db_dbm: None,
    };
    let first = input.cascade_block(&stage);
    let later = source.cascade_block(&stage);
    assert_eq!(first.signal_power_dbm, 11.0);
    assert_eq!(first.noise_power_dbm, later.noise_power_dbm);
    assert_eq!(
        first.cumulative_noise_figure_db,
        later.cumulative_noise_figure_db
    );
    assert_eq!(
        first.cumulative_noise_temperature,
        later.cumulative_noise_temperature
    );
}

#[test]
fn matched_attenuator_preserves_ambient_thermal_noise() {
    let input = Input::new(1.0e9, 1.0e6, -50.0, Some(290.0));
    let attenuator = block(-6.0, 6.0);
    let output = input.cascade_block(&attenuator);
    assert!((output.noise_power_dbm - input.noise_power()).abs() < 1.0e-12);
}

#[test]
fn two_stage_noise_matches_friis_at_small_signal_levels() {
    let input = Input::new(1.0e9, 1.0e6, -60.0, Some(290.0));
    let first = block(10.0, 6.0);
    let second = block(-6.0, 6.0);
    let output = input.cascade_block(&first).cascade_block(&second);
    let f1 = 10.0_f64.powf(6.0 / 10.0);
    let f2 = f1;
    let expected_factor = f1 + (f2 - 1.0) / 10.0;
    let expected_output_w =
        1.380_649e-23 * 290.0 * 1.0e6 * expected_factor * 10.0_f64.powf(4.0 / 10.0);
    assert!(
        (10.0_f64.powf(output.cumulative_noise_figure_db / 10.0) - expected_factor).abs() < 1.0e-12
    );
    assert!((watts(output.noise_power_dbm) / expected_output_w - 1.0).abs() < 1.0e-12);
}

#[test]
fn neutral_stage_passes_source_noise_without_added_noise() {
    let input = Input::new(1.0e9, 1.0e6, -50.0, Some(290.0));
    let neutral = block(0.0, 0.0);
    assert_eq!(
        neutral.input_noise_power(input.bandwidth_hz),
        f64::NEG_INFINITY
    );
    let output = input.cascade_block(&neutral);
    assert!((output.noise_power_dbm - input.noise_power()).abs() < 1.0e-12);
    assert_eq!(output.cumulative_noise_figure_db, 0.0);
}
