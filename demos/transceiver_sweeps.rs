//! Illustrative matched scalar budgets, not measured hardware specifications.
//! Near-Earth S-band downlink: 2200-2290 MHz, space-to-Earth.
//! RX is on the ground; TX is on the spacecraft. See README allocation sources.
//! LO is the ground receiver's carrier distribution budget; phase noise,
//! mixer conversion, off-state isolation, and simultaneous TX leakage need
//! additional models. The RX and TX switch responses represent selected paths.

use gainlineup::{
    cascade_frequency_sweep, Block, FrequencyBlock, FrequencySample, FrequencySweep, Input,
    SweepError,
};

const DOWNLINK_START_HZ: f64 = 2.2e9;
const DOWNLINK_STOP_HZ: f64 = 2.29e9;
const IF_HZ: f64 = 100e6;

fn constant(name: &str, gain_db: f64, noise_figure_db: f64) -> Result<FrequencyBlock, SweepError> {
    FrequencyBlock::constant(Block {
        name: name.into(),
        gain_db,
        noise_figure_db,
        ..Block::default()
    })
}

fn switch_path(name: &str) -> Result<FrequencyBlock, SweepError> {
    FrequencyBlock::tabulated(
        [(DOWNLINK_START_HZ, 1.0), (DOWNLINK_STOP_HZ, 1.5)]
            .into_iter()
            .map(|(frequency_hz, loss)| FrequencySample {
                frequency_hz,
                block: Block {
                    name: name.into(),
                    gain_db: -loss,
                    noise_figure_db: loss,
                    ..Block::default()
                },
            })
            .collect(),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Separate downlink endpoints, each with its own antenna switch.
    let rx = vec![
        switch_path("Ground antenna switch RX route")?,
        constant("LNA", 20.0, 1.5)?,
        constant("RF filter", -2.0, 2.0)?,
    ];
    let tx = vec![
        constant("RF driver", 15.0, 4.0)?,
        FrequencyBlock::constant(Block {
            name: "PA".into(),
            gain_db: 25.0,
            noise_figure_db: 5.0,
            output_p1db_dbm: Some(30.0),
            output_ip3_dbm: Some(40.0),
        })?,
        constant("Output filter", -2.0, 2.0)?,
        switch_path("Spacecraft antenna switch TX route")?,
    ];
    let lo = vec![
        constant("LO buffer", 10.0, 5.0)?,
        constant("Two-way splitter selected branch", -3.5, 3.5)?,
        constant("LO switch selected route", -1.5, 1.5)?,
    ];
    let rf_grid = FrequencySweep::linear(DOWNLINK_START_HZ, DOWNLINK_STOP_HZ, 11)?;
    // Low-side LO: 2100-2190 MHz gives a chosen 100 MHz IF at the ground RX.
    // LO tuning is a design choice, not a radiated frequency allocation.
    let lo_grid = FrequencySweep::linear(DOWNLINK_START_HZ - IF_HZ, DOWNLINK_STOP_HZ - IF_HZ, 11)?;
    let lineups = [
        (
            "ground_rx",
            Input::new(0.0, 1e6, -80.0, Some(75.0)),
            rx,
            &rf_grid,
        ),
        (
            "spacecraft_tx",
            Input::new(0.0, 1e6, -15.0, Some(290.0)),
            tx,
            &rf_grid,
        ),
        // Oscillator carrier power only. Source thermal temperature is not phase noise.
        (
            "ground_rx_lo",
            Input::new(0.0, 1e6, 0.0, Some(290.0)),
            lo,
            &lo_grid,
        ),
    ];
    println!("lineup,frequency_hz,output_power_dbm,gain_db,noise_figure_db");
    for (name, input, blocks, grid) in lineups {
        for point in cascade_frequency_sweep(&input, &blocks, grid)? {
            let output = point.nodes.last().expect("nonempty lineup");
            println!(
                "{name},{},{},{},{}",
                point.frequency_hz,
                output.signal_power_dbm,
                output.cumulative_gain_db,
                output.cumulative_noise_figure_db
            );
        }
    }
    Ok(())
}
