//! README example: IMD3 (Intermodulation from IP3)

use gainlineup::Block;

#[test]
fn imd3_single_point() {
    let amp = Block {
        name: "Driver Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: None,
        output_ip3_dbm: Some(30.0), // OIP3 = +30 dBm
    };

    // Pin = -30 -> Pout = -10
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
        name: "Driver Amp".to_string(),
        gain_db: 20.0,
        noise_figure_db: 5.0,
        output_p1db_dbm: None,
        output_ip3_dbm: Some(30.0),
    };

    let im3_a = amp.imd3_output_power_dbm(-30.0).unwrap();
    let im3_b = amp.imd3_output_power_dbm(-29.0).unwrap();

    // 3:1 slope: 1 dB input increase -> 3 dB IM3 increase
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
        assert!(
            sweep[i].rejection_db <= sweep[i - 1].rejection_db + 0.01,
            "Rejection should decrease with increasing power"
        );
    }
}

#[test]
fn imd3_none_without_ip3() {
    let block = Block::default();
    assert!(block.imd3_output_power_dbm(-30.0).is_none());
    assert!(block.imd3_rejection_db(-30.0).is_none());
    assert!(block.imd3_sweep(-50.0, -10.0, 5.0).is_empty());
}
