//! README example: Compression (P1dB)

use gainlineup::Block;

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
