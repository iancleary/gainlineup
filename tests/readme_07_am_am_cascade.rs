//! README example: AM-AM Curves - Full Cascade

use gainlineup::{cascade_am_am_sweep, cascade_gain_compression_sweep, Block};

#[test]
fn cascade_am_am_three_stage() {
    let lna = Block {
        name: "Low Noise Amplifier".to_string(),
        gain_db: 20.0,
        noise_figure_db: 1.5,
        output_p1db_dbm: Some(5.0),
        output_ip3_dbm: Some(20.0),
    };

    let mixer = Block {
        name: "Mixer".to_string(),
        gain_db: -8.0,
        noise_figure_db: 8.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(15.0),
    };

    let if_amp = Block {
        name: "IF Amplifier".to_string(),
        gain_db: 25.0,
        noise_figure_db: 4.0,
        output_p1db_dbm: Some(15.0),
        output_ip3_dbm: Some(25.0),
    };

    let blocks = vec![lna.clone(), mixer.clone(), if_amp.clone()];

    // Cascade Pin vs Pout
    let am_am = cascade_am_am_sweep(&blocks, -80.0, -20.0, 1.0);
    assert_eq!(am_am.len(), 61); // -80 to -20 inclusive

    // Output should be monotonically non-decreasing
    for i in 1..am_am.len() {
        assert!(am_am[i].1 >= am_am[i - 1].1);
    }
}

#[test]
fn cascade_gain_compression_three_stage() {
    let lna = Block {
        name: "Low Noise Amplifier".to_string(),
        gain_db: 20.0,
        noise_figure_db: 1.5,
        output_p1db_dbm: Some(5.0),
        output_ip3_dbm: Some(20.0),
    };

    let mixer = Block {
        name: "Mixer".to_string(),
        gain_db: -8.0,
        noise_figure_db: 8.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: Some(15.0),
    };

    let if_amp = Block {
        name: "IF Amplifier".to_string(),
        gain_db: 25.0,
        noise_figure_db: 4.0,
        output_p1db_dbm: Some(15.0),
        output_ip3_dbm: Some(25.0),
    };

    let blocks = vec![lna.clone(), mixer.clone(), if_amp.clone()];

    // Cascade Pin vs Gain
    let gc = cascade_gain_compression_sweep(&blocks, -80.0, -20.0, 1.0);
    assert_eq!(gc.len(), 61);

    // At low power, gain should be close to small-signal total: 20 - 8 + 25 = 37 dB
    assert!(
        (gc[0].1 - 37.0).abs() < 1.0,
        "Small-signal cascade gain should be ~37 dB, got {:.1}",
        gc[0].1
    );
}
