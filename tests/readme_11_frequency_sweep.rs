use gainlineup::{
    cascade_frequency_sweep, Block, FrequencyBlock, FrequencySample, FrequencySweep, Input,
};

#[test]
fn readme_frequency_sweep() -> Result<(), Box<dyn std::error::Error>> {
    let switch = FrequencyBlock::tabulated(vec![
        FrequencySample {
            frequency_hz: 2.2e9,
            block: Block {
                name: "RX switch".into(),
                gain_db: -1.0,
                noise_figure_db: 1.0,
                ..Block::default()
            },
        },
        FrequencySample {
            frequency_hz: 2.29e9,
            block: Block {
                name: "RX switch".into(),
                gain_db: -2.0,
                noise_figure_db: 2.0,
                ..Block::default()
            },
        },
    ])?;
    let lna = FrequencyBlock::constant(Block {
        name: "LNA".into(),
        gain_db: 20.0,
        noise_figure_db: 1.5,
        ..Block::default()
    })?;
    let grid = FrequencySweep::linear(2.2e9, 2.29e9, 3)?;
    let input = Input::new(0.0, 1e6, -80.0, Some(290.0));
    let points = cascade_frequency_sweep(&input, &[switch, lna], &grid)?;

    let midband = &points[1].nodes[1];
    assert_eq!(points[1].frequency_hz, 2.245e9);
    assert_eq!(midband.signal_power_dbm, -61.5);
    assert!((midband.cumulative_noise_figure_db - 3.0).abs() < 1e-9);
    Ok(())
}
