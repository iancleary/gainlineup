//! README example: AM-AM Curves - Single Block

use gainlineup::Block;

#[test]
fn am_am_single_block_sweep() {
    let lna = Block {
        name: "LNA".to_string(),
        gain_db: 20.0,
        noise_figure_db: 3.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: None,
    };

    // Pin vs Pout
    let curve = lna.am_am_sweep(-50.0, 0.0, 1.0);

    // Should have 51 points (-50 to 0 inclusive, step 1)
    assert_eq!(curve.len(), 51);

    // First point: linear
    assert_eq!(curve[0].0, -50.0);
    assert_eq!(curve[0].1, -50.0 + 20.0); // -30 dBm

    // Monotonically non-decreasing output
    for i in 1..curve.len() {
        assert!(
            curve[i].1 >= curve[i - 1].1,
            "AM-AM curve should be monotonically non-decreasing"
        );
    }
}

#[test]
fn gain_compression_shows_rolloff() {
    let lna = Block {
        name: "LNA".to_string(),
        gain_db: 20.0,
        noise_figure_db: 3.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: None,
    };

    // Pin vs Gain (shows compression directly)
    let gc = lna.gain_compression_sweep(-50.0, 0.0, 1.0);

    // At low power, gain = 20.0
    assert_eq!(gc[0].1, 20.0);

    // At high power, gain should be less
    let last_gain = gc.last().unwrap().1;
    assert!(
        last_gain < 20.0,
        "Gain should compress at high input, got {:.1} dB",
        last_gain
    );
}
