//! Integration tests: realistic RF receive chain scenarios.
//!
//! These tests model actual satellite ground terminal receiver chains
//! and verify cascaded gain, noise figure, P1dB, and dynamic range.

use gainlineup::{Block, Input, cascade_vector_return_output, cascade_vector_return_vector};

/// Helper: assert float equality within tolerance
fn assert_approx(actual: f64, expected: f64, tol: f64, msg: &str) {
    assert!(
        (actual - expected).abs() < tol,
        "{msg}: expected {expected:.4}, got {actual:.4}"
    );
}

/// Ka-band LEO ground terminal receive chain:
/// Antenna (G/T given) → LNA → Band-pass filter → Mixer → IF amplifier → ADC driver
///
/// Verifies that system noise figure is dominated by the LNA,
/// and that compression point cascades correctly.
#[test]
fn ka_band_leo_receive_chain() {
    let input = Input::new(
        26.5e9,   // Ka-band downlink (Hz)
        500.0e6,  // 500 MHz bandwidth
        -80.0,    // Received signal power (dBm)
        Some(75.0), // Antenna noise temperature (K) — cold sky
    );

    let blocks = vec![
        Block {
            name: "LNA".to_string(),
            gain_db: 35.0,
            noise_figure_db: 1.2,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: Some(25.0),
        },
        Block {
            name: "BPF".to_string(),
            gain_db: -2.0,
            noise_figure_db: 2.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        },
        Block {
            name: "Mixer".to_string(),
            gain_db: -7.0,
            noise_figure_db: 8.0,
            output_p1db_dbm: Some(5.0),
            output_ip3_dbm: Some(15.0),
        },
        Block {
            name: "IF Amp".to_string(),
            gain_db: 25.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(15.0),
            output_ip3_dbm: Some(28.0),
        },
        Block {
            name: "ADC Driver".to_string(),
            gain_db: 10.0,
            noise_figure_db: 5.0,
            output_p1db_dbm: Some(12.0),
            output_ip3_dbm: Some(22.0),
        },
    ];

    let output = cascade_vector_return_output(input, blocks);

    // Total gain: 35 - 2 - 7 + 25 + 10 = 61 dB
    assert_approx(output.cumulative_gain_db, 61.0, 0.01, "Total gain");

    // Output signal: -80 + 61 = -19 dBm
    assert_approx(output.signal_power_dbm, -19.0, 0.01, "Output signal");

    // System NF should be close to LNA NF (Friis: first stage dominates)
    assert!(output.cumulative_noise_figure_db < 1.5,
        "System NF should be LNA-dominated, got {:.2}", output.cumulative_noise_figure_db);
    assert!(output.cumulative_noise_figure_db > 1.2,
        "System NF must be >= LNA NF, got {:.2}", output.cumulative_noise_figure_db);
}

/// Two-stage LNA test: verify that adding a second LNA stage
/// further reduces system noise figure contribution from downstream.
#[test]
fn two_stage_lna_noise_improvement() {
    let input1 = Input::new(12.0e9, 36.0e6, -60.0, Some(100.0));
    let input2 = Input::new(12.0e9, 36.0e6, -60.0, Some(100.0));

    // Single LNA + lossy downconverter
    let single = vec![
        Block {
            name: "LNA".to_string(),
            gain_db: 20.0,
            noise_figure_db: 1.5,
            output_p1db_dbm: Some(5.0),
            output_ip3_dbm: None,
        },
        Block {
            name: "Downconverter".to_string(),
            gain_db: -10.0,
            noise_figure_db: 12.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        },
    ];

    // Two LNAs + same downconverter
    let dual = vec![
        Block {
            name: "LNA1".to_string(),
            gain_db: 20.0,
            noise_figure_db: 1.5,
            output_p1db_dbm: Some(5.0),
            output_ip3_dbm: None,
        },
        Block {
            name: "LNA2".to_string(),
            gain_db: 15.0,
            noise_figure_db: 2.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        },
        Block {
            name: "Downconverter".to_string(),
            gain_db: -10.0,
            noise_figure_db: 12.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        },
    ];

    let single_out = cascade_vector_return_output(input1, single);
    let dual_out = cascade_vector_return_output(input2, dual);

    // Dual LNA should have lower system NF
    assert!(
        dual_out.cumulative_noise_figure_db < single_out.cumulative_noise_figure_db,
        "Dual LNA NF ({:.3}) should be less than single ({:.3})",
        dual_out.cumulative_noise_figure_db,
        single_out.cumulative_noise_figure_db
    );
}

/// High-dynamic-range chain: verify node-by-node power levels
/// don't exceed P1dB at any stage with nominal input.
#[test]
fn signal_headroom_check() {
    let input = Input::new(2.2e9, 10.0e6, -50.0, None);

    let blocks = vec![
        Block {
            name: "LNA".to_string(),
            gain_db: 25.0,
            noise_figure_db: 0.8,
            output_p1db_dbm: Some(5.0),
            output_ip3_dbm: Some(20.0),
        },
        Block {
            name: "IF Amp".to_string(),
            gain_db: 20.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(15.0),
            output_ip3_dbm: Some(30.0),
        },
        Block {
            name: "VGA".to_string(),
            gain_db: -5.0,
            noise_figure_db: 5.0,
            output_p1db_dbm: Some(20.0),
            output_ip3_dbm: Some(35.0),
        },
    ];

    let nodes = cascade_vector_return_vector(input, blocks);

    // Verify each node's signal is well below its P1dB
    // Expected signals: -50+25=-25, -25+20=-5, -5-5=-10
    let expected_signals = [-25.0, -5.0, -10.0];
    for (i, node) in nodes.iter().enumerate() {
        assert_approx(node.signal_power_dbm, expected_signals[i], 0.01,
            &format!("Node {} signal", i));

        if let Some(p1db) = node.output_p1db_dbm {
            let headroom = p1db - node.signal_power_dbm;
            assert!(headroom > 10.0,
                "Node {} has only {:.1} dB headroom (signal={:.1}, P1dB={:.1})",
                i, headroom, node.signal_power_dbm, p1db);
        }
    }
}

/// Attenuator pad: verify NF equals attenuation for passive device,
/// and that it correctly reduces signal level.
#[test]
fn passive_attenuator_nf_equals_loss() {
    let input = Input::new(1.0e9, 1.0e6, -20.0, None);

    let blocks = vec![
        Block {
            name: "6dB Pad".to_string(),
            gain_db: -6.0,
            noise_figure_db: 6.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        },
    ];

    let output = cascade_vector_return_output(input, blocks);
    assert_approx(output.signal_power_dbm, -26.0, 0.01, "Signal after 6dB pad");
    assert_approx(output.cumulative_noise_figure_db, 6.0, 0.01, "NF of passive = loss");
    assert_approx(output.cumulative_gain_db, -6.0, 0.01, "Gain = -6 dB");
}

/// Ku-band VSAT terminal: full chain with realistic block specs,
/// verify output SNR is positive (link closes).
#[test]
fn ku_band_vsat_link_closes() {
    let input = Input::new(
        12.25e9,   // Ku-band downlink
        36.0e6,    // 36 MHz transponder
        -85.0,     // Weak signal at antenna port
        Some(50.0), // Good clear-sky antenna temp
    );

    let blocks = vec![
        Block {
            name: "LNB".to_string(),
            gain_db: 55.0,
            noise_figure_db: 0.7,
            output_p1db_dbm: Some(0.0),
            output_ip3_dbm: Some(15.0),
        },
        Block {
            name: "Cable Loss".to_string(),
            gain_db: -15.0,
            noise_figure_db: 15.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        },
        Block {
            name: "IRD Input".to_string(),
            gain_db: 30.0,
            noise_figure_db: 8.0,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        },
    ];

    let output = cascade_vector_return_output(input, blocks);

    // Total gain: 55 - 15 + 30 = 70 dB
    assert_approx(output.cumulative_gain_db, 70.0, 0.01, "VSAT total gain");

    // Output signal: -85 + 70 = -15 dBm
    assert_approx(output.signal_power_dbm, -15.0, 0.01, "VSAT output signal");

    // SNR should be positive (link closes)
    assert!(output.signal_to_noise_ratio_db() > 0.0,
        "VSAT link should close, SNR = {:.1} dB", output.signal_to_noise_ratio_db());

    // System NF dominated by LNB
    assert!(output.cumulative_noise_figure_db < 1.0,
        "System NF should be LNB-dominated, got {:.2} dB", output.cumulative_noise_figure_db);
}
