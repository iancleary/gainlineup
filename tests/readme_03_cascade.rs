//! README example: 3. Run the Cascade

use gainlineup::{cascade_vector_return_vector, Block, Input};

#[test]
fn cascade_three_stage_receiver() {
    let input = Input {
        power_dbm: -80.0,
        frequency_hz: 6.0e9,
        bandwidth_hz: 1.0e6,
        noise_temperature_k: Some(50.0),
    };

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
    let nodes = cascade_vector_return_vector(input, blocks);

    assert_eq!(nodes.len(), 3);

    // Check names
    assert_eq!(nodes[0].name, "Low Noise Amplifier Output");
    assert_eq!(nodes[1].name, "Mixer Output");
    assert_eq!(nodes[2].name, "IF Amplifier Output");

    // Signal power should increase with net gain
    let output = nodes.last().unwrap();
    let total_gain = 20.0 - 8.0 + 25.0; // 37 dB small-signal
    assert!(output.signal_power_dbm <= -80.0 + total_gain + 1.0);

    // Cumulative NF should be dominated by LNA (Friis)
    assert!(
        output.cumulative_noise_figure_db < 5.0,
        "Cascaded NF should be low due to high-gain LNA, got {:.2}",
        output.cumulative_noise_figure_db
    );

    // SNR should be positive for this scenario
    assert!(output.signal_to_noise_ratio_db() > 0.0);

    // OIP3 should be present (all blocks have IP3)
    assert!(output.cumulative_oip3_dbm.is_some());

    // SFDR should be present
    assert!(output.sfdr_db.is_some());
}
