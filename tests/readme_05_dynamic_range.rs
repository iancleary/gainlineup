//! README example: Dynamic Range

use gainlineup::Block;

#[test]
fn dynamic_range_output_referred() {
    let lna = Block {
        name: "LNA".to_string(),
        gain_db: 20.0,
        noise_figure_db: 3.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: None,
    };

    // Output-referred: P1dB_out - noise_floor_out
    let dr = lna.dynamic_range_db(1e6).unwrap();
    assert!(dr > 90.0, "Expected DR > 90 dB, got {:.1}", dr);
}

#[test]
fn dynamic_range_input_referred() {
    let lna = Block {
        name: "LNA".to_string(),
        gain_db: 20.0,
        noise_figure_db: 3.0,
        output_p1db_dbm: Some(10.0),
        output_ip3_dbm: None,
    };

    // Input-referred: input_P1dB - input_noise_floor
    let dr_in = lna.input_dynamic_range_db(1e6).unwrap();
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
