#![cfg(feature = "cli")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!(
            "gainlineup-sweep-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        Self(directory)
    }

    fn write(&self, path: &str, content: &str) -> PathBuf {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn sweep(&self, range: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_gainlineup"))
            .current_dir(&self.0)
            .args(["sweep", "config.toml"])
            .args(range)
            .env("RUST_LOG", "gainlineup=debug")
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const INPUT: &str = "input_power_dbm = -80.0\nfrequency_hz = 1.5e9\nbandwidth_hz = 1e6\nnoise_temperature_k = 290.0\n";
const TABLE: &str = r#"
[[blocks]]
type = "tabulated"
name = "LNA"
[[blocks.samples]]
frequency_hz = 1e9
gain_db = 10.0
noise_figure_db = 1.0
output_p1db_dbm = 20.0
output_ip3_dbm = 30.0
[[blocks.samples]]
frequency_hz = 2e9
gain_db = 20.0
noise_figure_db = 3.0
output_p1db_dbm = 30.0
output_ip3_dbm = 40.0
"#;

fn success_stdout(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn assert_no_artifacts(directory: &Path) {
    assert!(!directory.join("config.toml.html").exists());
    assert!(!directory.join("js").exists());
}

#[test]
fn interpolates_table_and_emits_only_stage_csv_with_units() {
    let fixture = Fixture::new();
    fixture.write("config.toml", &format!("{INPUT}{TABLE}"));
    let output = success_stdout(fixture.sweep(&["1e9", "2e9", "3"]));
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "frequency_hz,stage_index,stage_name,signal_power_dbm,noise_power_dbm,cumulative_gain_db,cumulative_noise_figure_db,snr_db,cumulative_oip3_dbm,sfdr_db");
    let midpoint: Vec<_> = lines[2].split(',').collect();
    assert_eq!(
        &midpoint[..4],
        &["1500000000", "1", "\"LNA Output\"", "-65"]
    );
    assert_eq!(midpoint[5], "15");
    assert!((midpoint[6].parse::<f64>().unwrap() - 2.0).abs() < 1e-12);
    assert_eq!(midpoint[8], "35");
    let noise_dbm: f64 = midpoint[4].parse().unwrap();
    let snr_db: f64 = midpoint[7].parse().unwrap();
    let sfdr_db: f64 = midpoint[9].parse().unwrap();
    assert!((noise_dbm - (-96.975_188_704_107_8)).abs() < 0.01);
    assert!((snr_db - (-65.0 - noise_dbm)).abs() < 1e-10);
    assert!((sfdr_db - 2.0 / 3.0 * (35.0 - noise_dbm)).abs() < 1e-10);
    assert_no_artifacts(&fixture.0);
}

#[test]
fn fixed_blocks_preserve_stage_order_and_csv_quotes() {
    let fixture = Fixture::new();
    fixture.write(
        "config.toml",
        &format!(
            r#"{INPUT}
[[blocks]]
type = "explicit"
name = 'LNA, "primary"'
gain_db = 20.0
noise_figure_db = 2.0
[[blocks]]
type = "explicit"
name = "Pad"
gain_db = -3.0
noise_figure_db = 3.0
"#
        ),
    );
    let output = success_stdout(fixture.sweep(&["1e9", "2e9", "2"]));
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 5);
    assert!(lines[1].starts_with("1000000000,1,\"LNA, \"\"primary\"\" Output\",-60,"));
    assert!(lines[2].starts_with("1000000000,2,\"Pad Output\",-63,"));
    assert!(lines[3].starts_with("2000000000,1,"));
    assert!(lines[4].starts_with("2000000000,2,"));
    assert!(lines[2].ends_with(",,"));
    assert_no_artifacts(&fixture.0);
}

#[test]
fn nested_includes_resolve_touchstone_paths_and_interpolate_s21() {
    let fixture = Fixture::new();
    fixture.write(
        "config.toml",
        &format!("{INPUT}[[blocks]]\ntype = \"include\"\npath = \"parts/chain.toml\"\n"),
    );
    fixture.write(
        "parts/chain.toml",
        "[[blocks]]\ntype = \"include\"\npath = \"measured/filter.toml\"\n",
    );
    fixture.write(
        "parts/measured/filter.toml",
        "[[blocks]]\ntype = \"touchstone\"\nname = \"Filter\"\nfile_path = \"filter.s2p\"\n",
    );
    fixture.write(
        "parts/measured/filter.s2p",
        "# Hz S DB R 50\n1000000000 -20 0 -3 0 -3 0 -20 0\n2000000000 -20 0 -7 0 -7 0 -20 0\n",
    );
    let output = success_stdout(fixture.sweep(&["1e9", "2e9", "3"]));
    let lines: Vec<_> = output.lines().collect();
    assert_eq!(lines.len(), 4);
    let midpoint: Vec<_> = lines[2].split(',').collect();
    assert_eq!(midpoint[3], "-85");
    assert_eq!(midpoint[5], "-5");
    assert_eq!(midpoint[6], "5");
    // Legacy single-frequency loading still requires an exact Touchstone sample.
    let error =
        gainlineup::cli::load_config(fixture.0.join("config.toml").to_str().unwrap()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Frequency 1500000000 Hz not found in touchstone file filter.s2p"
    );
}

#[test]
fn existing_config_loader_accepts_tabulated_blocks() {
    let fixture = Fixture::new();
    let path = fixture.write("config.toml", &format!("{INPUT}{TABLE}"));
    let config = gainlineup::cli::load_config(path.to_str().unwrap()).unwrap();
    assert_eq!(config.blocks.len(), 1);
    assert_eq!(config.blocks[0].gain_db, 15.0);
    assert_eq!(config.blocks[0].noise_figure_db, 2.0);
    assert_eq!(config.blocks[0].output_p1db_dbm, Some(25.0));
    assert_eq!(config.blocks[0].output_ip3_dbm, Some(35.0));
}

#[test]
fn invalid_sweeps_emit_no_csv_even_when_first_frequency_is_valid() {
    let fixture = Fixture::new();
    fixture.write("config.toml", &format!("{INPUT}{TABLE}"));
    for range in [
        vec!["1e9", "3e9", "3"],
        vec!["2e9", "1e9", "3"],
        vec!["1e9", "2e9", "1"],
        vec!["1e9", "2e9", "1000001"],
        vec!["NaN", "2e9", "3"],
        vec!["1e9", "inf", "3"],
        vec!["invalid", "2e9", "3"],
        vec!["1e9", "2e9", "2.5"],
        vec!["1e9", "2e9"],
    ] {
        let output = fixture.sweep(&range);
        assert!(!output.status.success(), "accepted {range:?}");
        assert!(output.stdout.is_empty(), "emitted stdout for {range:?}");
        assert!(!output.stderr.is_empty());
    }
    assert_no_artifacts(&fixture.0);
}

#[test]
fn invalid_tables_and_input_emit_no_csv() {
    let fixture = Fixture::new();
    for config in [
        format!("{INPUT}[[blocks]]\ntype = \"tabulated\"\nname = \"Empty\"\nsamples = []"),
        format!("{INPUT}[[blocks]]\ntype = \"tabulated\"\nname = \"Missing\""),
        format!(
            "{INPUT}{}",
            TABLE.replace("frequency_hz = 2e9", "frequency_hz = 1e9")
        ),
        format!("{INPUT}{}", TABLE.replace("output_ip3_dbm = 40.0", "")),
        format!("{INPUT}{TABLE}").replace("bandwidth_hz = 1e6", "bandwidth_hz = -1.0"),
    ] {
        fixture.write("config.toml", &config);
        let output = fixture.sweep(&["1e9", "2e9", "3"]);
        assert!(!output.status.success(), "accepted {config}");
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
    }
}

#[test]
fn one_port_touchstone_is_a_clear_error_without_a_panic() {
    let fixture = Fixture::new();
    fixture.write("config.toml", &format!("{INPUT}[[blocks]]\ntype = \"touchstone\"\nname = \"One port\"\nfile_path = \"one.s1p\""));
    fixture.write(
        "one.s1p",
        "# Hz S DB R 50\n1000000000 -20 0\n2000000000 -20 0\n",
    );
    let output = fixture.sweep(&["1e9", "2e9", "3"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains("at least two ports"), "{error}");
    assert!(!error.contains("panicked"));
    assert!(
        gainlineup::cli::touchstone_file_path_and_frequency_to_struct(
            fixture.0.join("one.s1p").to_string_lossy().into_owned(),
            1e9,
        )
        .is_err()
    );
}

#[test]
fn circular_include_returns_an_error() {
    let fixture = Fixture::new();
    fixture.write(
        "config.toml",
        &format!("{INPUT}[[blocks]]\ntype = \"include\"\npath = \"config.toml\""),
    );
    let output = fixture.sweep(&["1e9", "2e9", "3"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Circular config include"));
}
