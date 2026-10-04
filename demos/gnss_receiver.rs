//! Illustrative GPS L1/L5 RF receiver paths, evaluated separately.
//! Carrier frequencies follow GPS interface specifications; sweep windows,
//! rectangular noise bandwidths, and component values are design assumptions.
//! Antenna gain is already included in the source power at the antenna terminal.
//! No despreading, tracking, ADC, mixer, or blocker model is implied.

use gainlineup::{
    cascade_frequency_sweep, Block, FrequencyBlock, FrequencySample, FrequencySweep, Input,
    SweepError,
};

fn response(
    name: &str,
    center_hz: f64,
    half_span_hz: f64,
    gain_nf: [(f64, f64); 3],
    output_p1db_dbm: Option<f64>,
) -> Result<FrequencyBlock, SweepError> {
    FrequencyBlock::tabulated(
        [-1.0, 0.0, 1.0]
            .into_iter()
            .zip(gain_nf)
            .map(|(offset, (gain_db, noise_figure_db))| FrequencySample {
                frequency_hz: center_hz + offset * half_span_hz,
                block: Block {
                    name: name.into(),
                    gain_db,
                    noise_figure_db,
                    output_p1db_dbm,
                    output_ip3_dbm: None,
                },
            })
            .collect(),
    )
}

fn receiver(center_hz: f64, half_span_hz: f64) -> Result<Vec<FrequencyBlock>, SweepError> {
    Ok(vec![
        FrequencyBlock::constant(Block {
            name: "Antenna feed and ESD loss".into(),
            gain_db: -0.5,
            noise_figure_db: 0.5,
            ..Block::default()
        })?,
        response(
            "Preselection SAW filter",
            center_hz,
            half_span_hz,
            [(-2.5, 2.5), (-1.0, 1.0), (-2.5, 2.5)],
            None,
        )?,
        response(
            "LNA",
            center_hz,
            half_span_hz,
            [(18.0, 1.2), (20.0, 0.8), (18.0, 1.2)],
            Some(0.0),
        )?,
        response(
            "Post-LNA SAW filter",
            center_hz,
            half_span_hz,
            [(-3.0, 3.0), (-1.5, 1.5), (-3.0, 3.0)],
            None,
        )?,
        FrequencyBlock::constant(Block {
            name: "Receiver RF gain stage".into(),
            gain_db: 15.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        })?,
    ])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("band,frequency_hz,bandwidth_hz,stage_index,stage_name,signal_power_dbm,noise_power_dbm,cumulative_gain_db,cumulative_noise_figure_db,snr_db,cn0_db_hz");
    // Separate filter/LNA paths. These windows are not GNSS allocation edges
    // or claims about a measured filter's equivalent noise bandwidth.
    for (band, center_hz, half_span_hz, bandwidth_hz) in [
        ("gps_l1", 1575.42e6, 2.0e6, 2.0e6),
        ("gps_l5", 1176.45e6, 12.0e6, 20.0e6),
    ] {
        let input = Input::new(center_hz, bandwidth_hz, -130.0, Some(290.0));
        let grid = FrequencySweep::linear(center_hz - half_span_hz, center_hz + half_span_hz, 41)?;
        let points = cascade_frequency_sweep(&input, &receiver(center_hz, half_span_hz)?, &grid)?;
        for point in points {
            for (index, node) in point.nodes.iter().enumerate() {
                // C/N0 = carrier dBm - output noise density dBm/Hz.
                let cn0_db_hz = node.signal_power_dbm - node.noise_spectral_density();
                println!(
                    "{band},{},{},{},\"{}\",{},{},{},{},{},{}",
                    point.frequency_hz,
                    bandwidth_hz,
                    index + 1,
                    node.name,
                    node.signal_power_dbm,
                    node.noise_power_dbm,
                    node.cumulative_gain_db,
                    node.cumulative_noise_figure_db,
                    node.signal_to_noise_ratio_db(),
                    cn0_db_hz,
                );
            }
        }
    }
    Ok(())
}
