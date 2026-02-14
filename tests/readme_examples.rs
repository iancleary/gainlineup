//! Integration tests matching every README code example.
//! If the README compiles, these compile. If these fail, the README is wrong.

use gainlineup::{
    cascade_am_am_sweep, cascade_gain_compression_sweep, cascade_vector_return_vector,
    Block, Imd3Point, Input, SignalNode,
};

// ----- Helper: shared blocks used across examples -----

fn lna() -> Block {
    Block {
        name: "Low Noise Amplifier".to_string(),
        gain_db: 20.0,
        noise_figure_db: 1.5,
        output_p1db_dbm: Some(5.0),
        output_ip3_dbm: Some(20.0),
    }
}

fn mixer() -> Block {
    Block {
        name: "Mixer".to_string(),
        gain_db: -8.0,
        noise_figure_db: 8.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(15.0),
    }
}

fn if_amp() -> Block {
    Block {
        name: "IF Amplifier".to_string(),
        gain_db: 25.0,
        noise_figure_db: 4.0,
        output_p1db_dbm: Some(15.0),
        output_ip3_dbm: Some(25.0),
    }
}

fn input_signal() -> Input {
    Input {
        power_dbm: -80.0,
        frequency_hz: 6.0e9,
        bandwidth_hz: 1.0e6,
        noise_temperature_k: Some(50.0),
    }
}

// =====================================================================
// 1. Define Your Input Signal
// =====================================================================

#[test]
fn input_signal_construction() {
    let input = input_signal();
    assert_eq!(input.power_dbm, -80.0);
    assert_eq!(input.frequency_hz, 6.0e9);
    assert_eq!(input.bandwidth_hz, 1.0e6);
    assert_eq!(input.noise_temperature_k, Some(50.0));
}

// =====================================================================
// 2. Define Your Blocks
// =====================================================================

#[test]
fn block_construction() {
    let l = lna();
    assert_eq!(l.gain_db, 20.0);
    assert_eq!(l.noise_figure_db, 1.5);
    assert_eq!(l.output_p1db_dbm, Some(5.0));
    assert_eq!(l.output_ip3_dbm, Some(20.0));
}

// =====================================================================
// 3. Run the Cascade
// =====================================================================

#[test]
fn cascade_three_stage_receiver() {
    let input = input_signal();
    let blocks = vec![lna(), mixer(), if_amp()];
    let nodes = cascade_vector_return_vector(input, blocks);

    assert_eq!(nodes.len(), 3);

    // Check names
    assert_eq!(nodes[0].name, "Low Noise Amplifier Output");
    assert_eq!(nodes[1].name, "Mixer Output");
    assert_eq!(nodes[2].name, "IF Amplifier Output");

    // Signal power should increase with net gain
    let output = nodes.last().unwrap();
    let total_gain = 20.0 - 8.0 + 25.0; // 37 dB small-signal
    // With compression possible, output power <= input + total_gain
    assert!(output.signal_power_dbm <= -80.0 + total_gain + 1.0);

    // Cumulative NF should be dominated by LNA (Friis)
    assert!(output.cumulative_noise_figure_db < 5.0,
        "Cascaded NF should be low due to high-gain LNA, got {:.2}",
        output.cumulative_noise_figure_db);

    // SNR should be positive for this scenario
    assert!(output.signal_to_noise_ratio_db() > 0.0);

    // OIP3 should be present (all blocks have IP3)
    assert!(output.cumulative_oip3_dbm.is_some());

    // SFDR should be present
    assert!(output.sfdr_db.is_some());
}

// =====================================================================
// 4. Compression (P1dB)
// =====================================================================

#[test]
fn compression_linear_region() {
    let pa = Block {
        name: "Power Amplifier".to_string(),
        gain_db: 30.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: Some(20.0),
        output_ip3_dbm: None,
    };

    // Linear: -20 + 30 = 10 (below P1dB)
    assert_eq!(pa.output_power(-20.0), 10.0);
    assert_eq!(pa.power_gain(-20.0), 30.0);
}

#[test]
fn compression_above_p1db() {
    let pa = Block {
        name: "Power Amplifier".to_string(),
        gain_db: 30.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: Some(20.0),
        output_ip3_dbm: None,
    };

    // Compressed: 0 + 30 = 30, clamps to P1dB + 1 = 21
    assert_eq!(pa.output_power(0.0), 21.0);
    assert_eq!(pa.power_gain(0.0), 21.0);
}

// =====================================================================
// 5. Dynamic Range
// =====================================================================

#[test]
fn dynamic_range_output_referred() {
    let l = lna();
    let dr = l.dynamic_range_db(1e6).unwrap();
    // P1dB = 5 dBm, noise floor at output is very low → DR > 90 dB
    assert!(dr > 90.0, "Expected DR > 90 dB, got {:.1}", dr);
}

#[test]
fn dynamic_range_input_referred() {
    let l = lna();
    let dr_in = l.input_dynamic_range_db(1e6).unwrap();
    assert!(dr_in > 90.0, "Expected input DR > 90 dB, got {:.1}", dr_in);
}

#[test]
fn dynamic_range_none_without_p1db() {
    let linear_block = Block {
        name: "Ideal".to_string(),
        gain_db: 10.0,
        noise_figure_db: 3.0,
        output_p1db_dbm: None,
        output_ip3_dbm: None,
    };
    assert!(linear_block.dynamic_range_db(1e6).is_none());
    assert!(linear_block.input_dynamic_range_db(1e6).is_none());
}

// =====================================================================
// 6. AM-AM Curves (Power Sweep)
// =====================================================================

#[test]
fn am_am_single_block_sweep() {
    let l = lna();
    let curve = l.am_am_sweep(-50.0, 0.0, 1.0);

    // Should have 51 points (-50 to 0 inclusive, step 1)
    assert_eq!(curve.len(), 51);

    // First point: linear
    assert_eq!(curve[0].0, -50.0);
    assert_eq!(curve[0].1, -50.0 + 20.0); // -30 dBm

    // Monotonically non-decreasing output
    for i in 1..curve.len() {
        assert!(curve[i].1 >= curve[i - 1].1,
            "AM-AM curve should be monotonically non-decreasing");
    }
}

#[test]
fn gain_compression_shows_rolloff() {
    let l = lna();
    let gc = l.gain_compression_sweep(-50.0, 0.0, 1.0);

    // At low power, gain = 20.0
    assert_eq!(gc[0].1, 20.0);

    // At high power, gain should be less
    let last_gain = gc.last().unwrap().1;
    assert!(last_gain < 20.0,
        "Gain should compress at high input, got {:.1} dB", last_gain);
}

#[test]
fn cascade_am_am_three_stage() {
    let blocks = vec![lna(), mixer(), if_amp()];
    let am_am = cascade_am_am_sweep(&blocks, -80.0, -20.0, 1.0);

    assert_eq!(am_am.len(), 61); // -80 to -20 inclusive

    // Output should be monotonically non-decreasing
    for i in 1..am_am.len() {
        assert!(am_am[i].1 >= am_am[i - 1].1);
    }
}

#[test]
fn cascade_gain_compression_three_stage() {
    let blocks = vec![lna(), mixer(), if_amp()];
    let gc = cascade_gain_compression_sweep(&blocks, -80.0, -20.0, 1.0);

    assert_eq!(gc.len(), 61);

    // At low power, gain should be close to small-signal total: 20 - 8 + 25 = 37 dB
    assert!((gc[0].1 - 37.0).abs() < 1.0,
        "Small-signal cascade gain should be ~37 dB, got {:.1}", gc[0].1);
}

// =====================================================================
// 7. IMD3 (Intermodulation from IP3)
// =====================================================================

#[test]
fn imd3_single_point() {
    let amp = Block {
        name: "Driver Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: None,
        output_ip3_dbm: Some(30.0),
    };

    // Pin = -30 → Pout = -10
    // IM3 = 3*(-10) - 2*(30) = -90 dBm
    let im3 = amp.imd3_output_power_dbm(-30.0).unwrap();
    assert!((im3 - (-90.0)).abs() < 0.01);

    // Rejection = 2*(30 - (-10)) = 80 dB
    let rejection = amp.imd3_rejection_db(-30.0).unwrap();
    assert!((rejection - 80.0).abs() < 0.01);
}

#[test]
fn imd3_3_to_1_slope() {
    let amp = Block {
        name: "Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: None,
        output_ip3_dbm: Some(30.0),
    };

    let im3_a = amp.imd3_output_power_dbm(-30.0).unwrap();
    let im3_b = amp.imd3_output_power_dbm(-29.0).unwrap();

    // 3:1 slope: 1 dB input increase → 3 dB IM3 increase
    assert!((im3_b - im3_a - 3.0).abs() < 0.01);
}

#[test]
fn imd3_sweep_structure() {
    let amp = Block {
        name: "Driver Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: None,
        output_ip3_dbm: Some(30.0),
    };

    let sweep = amp.imd3_sweep(-50.0, -10.0, 5.0);
    assert_eq!(sweep.len(), 9); // -50, -45, ..., -10

    // Rejection should decrease as input power increases
    for i in 1..sweep.len() {
        assert!(sweep[i].rejection_db <= sweep[i - 1].rejection_db + 0.01,
            "Rejection should decrease with increasing power");
    }
}

#[test]
fn imd3_none_without_ip3() {
    let block = Block::default();
    assert!(block.imd3_output_power_dbm(-30.0).is_none());
    assert!(block.imd3_rejection_db(-30.0).is_none());
    assert!(block.imd3_sweep(-50.0, -10.0, 5.0).is_empty());
}
