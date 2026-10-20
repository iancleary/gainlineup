//! Executable mirror of the README spectral-noise example.
use gainlineup::{integrate_small_signal_noise, Block, FrequencyBlock, FrequencySweep};

#[test]
fn readme_spectral_noise() -> Result<(), Box<dyn std::error::Error>> {
    let lna = FrequencyBlock::constant(Block {
        name: "LNA".into(),
        gain_db: 20.0,
        noise_figure_db: 10.0 * 2.0_f64.log10(), // noise factor 2, referenced to 290 K
        output_p1db_dbm: None,
        output_ip3_dbm: None,
    })?;
    let grid = FrequencySweep::linear(990e6, 991e6, 41)?;
    let noise = integrate_small_signal_noise(75.0, &[lna], &grid)?;
    assert_eq!(noise.frequency_points, 41);
    assert_eq!(noise.contributions[0].name, "LNA");
    assert!((noise.total_noise_watts / 5.03936885e-13 - 1.0).abs() < 1e-12);
    println!(
        "Source: {:.6e} W; LNA: {:.6e} W; total: {:.6e} W",
        noise.source_noise_watts, noise.contributions[0].noise_power_watts, noise.total_noise_watts
    );
    Ok(())
}
