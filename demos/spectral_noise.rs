//! Illustrative scalar receiver data, not measured hardware characterization.
use gainlineup::{
    integrate_small_signal_noise, Block, FrequencyBlock, FrequencySample, FrequencySweep,
};

fn response(
    name: &str,
    data: &[(f64, f64, f64)],
) -> Result<FrequencyBlock, gainlineup::SweepError> {
    FrequencyBlock::tabulated(
        data.iter()
            .map(|&(mhz, gain_db, noise_figure_db)| FrequencySample {
                frequency_hz: mhz * 1e6,
                block: Block {
                    name: name.into(),
                    gain_db,
                    noise_figure_db,
                    output_p1db_dbm: None,
                    output_ip3_dbm: None,
                },
            })
            .collect(),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let blocks = [
        response(
            "Preselector",
            &[
                (990.0, -12.0, 12.0),
                (995.0, -3.0, 3.0),
                (1000.0, -1.0, 1.0),
                (1005.0, -3.0, 3.0),
                (1010.0, -12.0, 12.0),
            ],
        )?,
        response(
            "LNA",
            &[(990.0, 19.0, 1.8), (1000.0, 20.0, 1.5), (1010.0, 19.0, 1.8)],
        )?,
        response(
            "Post-LNA filter",
            &[
                (990.0, -20.0, 20.0),
                (995.0, -4.0, 4.0),
                (1000.0, -2.0, 2.0),
                (1005.0, -4.0, 4.0),
                (1010.0, -20.0, 20.0),
            ],
        )?,
    ];
    println!("Illustrative matched, linear, frequency-preserving receiver; source 75 K");
    println!("Tables are illustrative, not measured; no operating-point validation.");
    let mut previous: Option<f64> = None;
    for points in [41, 81, 161, 321] {
        let grid = FrequencySweep::linear(990e6, 1010e6, points)?;
        let result = integrate_small_signal_noise(75.0, &blocks, &grid)?;
        println!(
            "\n{points} points; band {:.0}..={:.0} Hz",
            result.start_frequency_hz, result.stop_frequency_hz
        );
        println!("  Source             {:.9e} W", result.source_noise_watts);
        for stage in &result.contributions {
            println!("  {:<18} {:.9e} W", stage.name, stage.noise_power_watts);
        }
        println!(
            "  Total              {:.9e} W ({:.6} dBm)",
            result.total_noise_watts,
            10.0 * (result.total_noise_watts / 1e-3).log10()
        );
        if let Some(prior) = previous {
            println!(
                "  Change from prior  {:.6}%",
                100.0 * (result.total_noise_watts / prior - 1.0)
            );
        }
        previous = Some(result.total_noise_watts);
    }
    Ok(())
}
