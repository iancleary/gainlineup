//! Executable mirror of README's temperature-aware passive-loss example.
use gainlineup::{Block, Input, PassiveLossError};

#[test]
fn readme_passive_loss() -> Result<(), PassiveLossError> {
    let pad = Block::passive_loss("150 K pad", 10.0 * 2.0_f64.log10(), 150.0)?;
    assert!((pad.noise_figure_db - 1.8105467858723132).abs() < 1e-12);
    let output = Input::new(1e9, 1e6, -100.0, Some(290.0)).cascade_block(&pad);
    // Output temperature: 290/2 + 150*(1 - 1/2) = 220 K.
    let output_watts = 1e-3 * 10.0_f64.powf(output.noise_power_dbm / 10.0);
    assert!((output_watts / (1.380649e-23 * 220.0 * 1e6) - 1.0).abs() < 1e-12);
    assert_eq!(output.cumulative_oip3_dbm, None); // Unspecified, not ideal linearity.
    Ok(())
}
