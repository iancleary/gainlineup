//! Cross-crate validation: gainlineup cascade vs rfconversions Friis functions.
//!
//! These tests verify that gainlineup's internal cascade noise figure computation
//! produces results consistent with rfconversions' standalone `cascade_noise_figure`
//! function.
//!
//! Bug #55 (incorrect Friis denominator) has been fixed. All tests now pass.

use gainlineup::{cascade_vector_return_output, cascade_vector_return_vector, Block, Input};
use rfconversions::noise::cascade_noise_figure;

/// Helper: assert float equality within tolerance
fn assert_approx(actual: f64, expected: f64, tol: f64, msg: &str) {
    assert!(
        (actual - expected).abs() < tol,
        "{msg}: expected {expected:.4}, got {actual:.4} (diff {:.6})",
        (actual - expected).abs()
    );
}

/// Build rfconversions stage tuples from parallel NF/gain arrays.
fn stages(nfs: &[f64], gains: &[f64]) -> Vec<(f64, f64)> {
    nfs.iter().zip(gains.iter()).map(|(&n, &g)| (n, g)).collect()
}

/// Two-stage LNA + mixer: compare gainlineup cascade NF against rfconversions.
/// This passes because LNA's high gain makes the denominator bug negligible.
#[test]
fn two_stage_lna_mixer_nf_consistency() {
    let nf_lna = 1.5;
    let gain_lna = 25.0;
    let nf_mixer = 8.0;
    let gain_mixer = -6.0;

    let input = Input::new(12.0e9, 36.0e6, -70.0, Some(290.0));
    let blocks = vec![
        Block {
            name: "LNA".to_string(),
            gain_db: gain_lna,
            noise_figure_db: nf_lna,
            output_p1db_dbm: Some(5.0),
            output_ip3_dbm: Some(20.0),
        },
        Block {
            name: "Mixer".to_string(),
            gain_db: gain_mixer,
            noise_figure_db: nf_mixer,
            output_p1db_dbm: Some(10.0),
            output_ip3_dbm: None,
        },
    ];

    let output = cascade_vector_return_output(input, blocks);
    let expected_nf = cascade_noise_figure(&stages(&[nf_lna, nf_mixer], &[gain_lna, gain_mixer]));

    assert_approx(
        output.cumulative_noise_figure_db,
        expected_nf,
        0.01,
        "Two-stage cascade NF",
    );
}

/// Four-stage satellite receive chain: LNA → BPF → Mixer → IF Amp.
#[test]
fn four_stage_rx_chain_nf() {
    let nfs = [1.2, 0.5, 7.0, 3.0];
    let gains = [30.0, -0.5, -6.0, 20.0];

    let input = Input::new(20.0e9, 500.0e6, -85.0, Some(75.0));
    let blocks: Vec<Block> = ["LNA", "BPF", "Mixer", "IF Amp"]
        .iter()
        .enumerate()
        .map(|(i, name)| Block {
            name: name.to_string(),
            gain_db: gains[i],
            noise_figure_db: nfs[i],
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        })
        .collect();

    let output = cascade_vector_return_output(input, blocks);
    let expected_nf = cascade_noise_figure(&stages(&nfs, &gains));

    assert_approx(
        output.cumulative_noise_figure_db,
        expected_nf,
        0.01,
        "Four-stage cascade NF",
    );
}

/// Passive-only chain (attenuators): NF should equal total loss.
#[test]
fn passive_chain_nf_equals_loss() {
    let losses = [3.0, 1.0, 2.0];
    let total_loss: f64 = losses.iter().sum();

    let input = Input::new(1.0e9, 1.0e6, -30.0, Some(290.0));
    let blocks: Vec<Block> = losses
        .iter()
        .enumerate()
        .map(|(i, &loss)| Block {
            name: format!("Atten{}", i + 1),
            gain_db: -loss,
            noise_figure_db: loss,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        })
        .collect();

    let output = cascade_vector_return_output(input, blocks);
    let nfs: Vec<f64> = losses.to_vec();
    let gains: Vec<f64> = losses.iter().map(|l| -l).collect();
    let expected_nf = cascade_noise_figure(&stages(&nfs, &gains));

    assert_approx(
        output.cumulative_noise_figure_db,
        expected_nf,
        0.01,
        "Passive chain NF (gainlineup vs rfconversions)",
    );
    assert_approx(
        output.cumulative_noise_figure_db,
        total_loss,
        0.01,
        "Passive chain NF should equal total loss",
    );
}

/// Vector cascade: verify each intermediate node's NF matches rfconversions.
#[test]
fn vector_cascade_intermediate_nf_consistency() {
    let nfs = [2.0, 6.0, 10.0, 3.0];
    let gains = [20.0, -8.0, 15.0, 10.0];

    let input = Input::new(5.8e9, 20.0e6, -50.0, Some(290.0));
    let blocks: Vec<Block> = (0..4)
        .map(|i| Block {
            name: format!("Stage{}", i + 1),
            gain_db: gains[i],
            noise_figure_db: nfs[i],
            output_p1db_dbm: None,
            output_ip3_dbm: None,
        })
        .collect();

    let nodes = cascade_vector_return_vector(input, blocks);

    for n in 1..=4 {
        let expected_nf = cascade_noise_figure(&stages(&nfs[..n], &gains[..n]));
        assert_approx(
            nodes[n - 1].cumulative_noise_figure_db,
            expected_nf,
            0.01,
            &format!("Intermediate NF at stage {n}"),
        );
    }
}

/// Single block: cascade NF should equal the block's own NF.
#[test]
fn single_block_nf_identity() {
    let nf = 4.5;
    let gain = 12.0;

    let input = Input::new(2.4e9, 10.0e6, -40.0, Some(290.0));
    let blocks = vec![Block {
        name: "Amp".to_string(),
        gain_db: gain,
        noise_figure_db: nf,
        output_p1db_dbm: Some(20.0),
        output_ip3_dbm: Some(35.0),
    }];

    let output = cascade_vector_return_output(input, blocks);
    let expected_nf = cascade_noise_figure(&[(nf, gain)]);

    assert_approx(output.cumulative_noise_figure_db, nf, 0.001, "Single block NF");
    assert_approx(expected_nf, nf, 0.001, "rfconversions single-stage NF");
}
