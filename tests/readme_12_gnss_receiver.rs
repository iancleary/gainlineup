use gainlineup::{cascade_vector_return_vector, cli::load_config, Input};
use std::process::Command;

#[test]
fn readme_gnss_center_budgets_and_cn0() {
    for (path, center_hz, bandwidth_hz, expected_snr_db) in [
        ("files/gnss_l1.toml", 1575.42e6, 2e6, -21.40030650190265),
        ("files/gnss_l5.toml", 1176.45e6, 20e6, -31.40030650190265),
    ] {
        let config = load_config(path).unwrap();
        assert_eq!(config.frequency_hz, center_hz);
        assert_eq!(config.bandwidth_hz, Some(bandwidth_hz));
        let input = Input::new(
            config.frequency_hz,
            config.bandwidth_hz.unwrap(),
            config.input_power_dbm,
            config.noise_temperature_k,
        );
        let nodes = cascade_vector_return_vector(input, config.blocks);
        assert_eq!(nodes.len(), 5);
        // Feed loss and the preselector add 1.5 dB NF before the LNA.
        assert!((nodes[1].cumulative_noise_figure_db - 1.5).abs() < 1e-10);
        let node = nodes.last().unwrap();
        // Center gains: -0.5 -1 +20 -1.5 +15 = 32 dB.
        assert_eq!(node.cumulative_gain_db, 32.0);
        assert_eq!(node.signal_power_dbm, -98.0);
        // Independent Friis budget with gains above and NFs 0.5, 1, 0.8, 1.5, 3 dB.
        assert!((node.cumulative_noise_figure_db - 2.3651937394909424).abs() < 1e-10);
        let cn0_db_hz = node.signal_power_dbm - node.noise_spectral_density();
        assert!((cn0_db_hz - 41.60999345473715).abs() < 1e-10);
        assert!((node.signal_to_noise_ratio_db() - expected_snr_db).abs() < 1e-10);
        assert!(
            (cn0_db_hz - (node.signal_to_noise_ratio_db() + 10.0 * bandwidth_hz.log10())).abs()
                < 1e-10
        );
    }
}

#[test]
fn gnss_cli_examples_resolve_both_filter_edges_and_the_carrier() {
    for (path, start_hz, center_hz, stop_hz) in [
        ("files/gnss_l1.toml", 1573.42e6, 1575.42e6, 1577.42e6),
        ("files/gnss_l5.toml", 1164.45e6, 1176.45e6, 1188.45e6),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_gainlineup"))
            .args([
                "sweep",
                path,
                &start_hz.to_string(),
                &stop_hz.to_string(),
                "3",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let csv = String::from_utf8(output.stdout).unwrap();
        let rows: Vec<Vec<&str>> = csv
            .lines()
            .skip(1)
            .map(|row| row.split(',').collect())
            .collect();
        assert_eq!(rows.len(), 15);
        for (row_index, frequency_hz, gain_db, signal_dbm) in [
            (4, start_hz, 27.0, -103.0),
            (9, center_hz, 32.0, -98.0),
            (14, stop_hz, 27.0, -103.0),
        ] {
            let row = &rows[row_index];
            assert_eq!(row[0].parse::<f64>().unwrap(), frequency_hz);
            assert_eq!(row[1], "5");
            assert_eq!(row[3].parse::<f64>().unwrap(), signal_dbm);
            assert_eq!(row[5].parse::<f64>().unwrap(), gain_db);
        }
    }
}
