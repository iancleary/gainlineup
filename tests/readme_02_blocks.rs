//! README example: 2. Define Your Blocks

use gainlineup::Block;

#[test]
fn block_construction() {
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

    assert_eq!(lna.gain_db, 20.0);
    assert_eq!(lna.noise_figure_db, 1.5);
    assert_eq!(lna.output_p1db_dbm, Some(5.0));
    assert_eq!(lna.output_ip3_dbm, Some(20.0));

    assert_eq!(mixer.gain_db, -8.0);
    assert_eq!(mixer.noise_figure_db, 8.0);

    assert_eq!(if_amp.gain_db, 25.0);
    assert_eq!(if_amp.noise_figure_db, 4.0);
}
