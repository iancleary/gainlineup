//! README example: AmplifierModel + AM-PM

use gainlineup::{AmplifierModel, Block};

#[test]
fn amplifier_model_basic() {
    let pa = Block {
        name: "Power Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(25.0),
    };

    // Simple: no AM-PM
    let model = AmplifierModel::new(&pa);
    assert!(model.phase_shift_at(-5.0).is_none());
}

#[test]
fn amplifier_model_with_am_pm() {
    let pa = Block {
        name: "Power Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(25.0),
    };

    // With AM-PM coefficient (10 deg/dB near P1dB)
    let model = AmplifierModel::with_am_pm(&pa, 10.0);
    assert!(model.phase_shift_at(-5.0).is_some());
}

#[test]
fn amplifier_model_builder() {
    let pa = Block {
        name: "Power Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(25.0),
    };

    // Builder pattern for full configuration
    let model = AmplifierModel::builder(&pa)
        .am_pm_coefficient(10.0)
        .saturation_power(25.0)
        .build();

    // Phase shift at a given input power
    let phase = model.phase_shift_at(-5.0);
    assert!(phase.is_some());
}

#[test]
fn amplifier_model_sweep() {
    let pa = Block {
        name: "Power Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(25.0),
    };

    let model = AmplifierModel::with_am_pm(&pa, 10.0);

    // Combined AM-AM + AM-PM sweep
    let sweep = model.am_am_am_pm_sweep(-40.0, 0.0, 1.0);
    assert_eq!(sweep.len(), 41);

    // Each point should have phase shift data
    for pt in &sweep {
        assert!(pt.phase_shift_deg.is_some());
    }
}

#[test]
fn amplifier_model_backoff_and_evm() {
    let pa = Block {
        name: "Power Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(25.0),
    };

    let model = AmplifierModel::with_am_pm(&pa, 10.0);

    // Required backoff for a phase budget
    // Negative backoff means you can exceed P1dB by that amount
    let backoff = model.backoff_for_target_phase(5.0);
    assert!(backoff.is_some());
    assert!((backoff.unwrap() - (-0.5)).abs() < 1e-10);

    // EVM from AM-PM distortion
    let evm = model.evm_from_am_pm(-5.0);
    assert!(evm.is_some());
    assert!(evm.unwrap() > 0.0);
}
