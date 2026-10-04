//! Dependency-free cascade performance baseline.
//!
//! Run with `cargo bench --bench cascade_performance`; set `PERF_FILTER` to a
//! case name (`scalar-vector`, `scalar-output`, `constant-sweep`,
//! `mixed-sweep`, or `dense-tabulated`) to run one case. Scalar calls take
//! owned block vectors, so cloning the prebuilt fixture is included in timing.

use std::hint::black_box;
use std::time::{Duration, Instant};

use gainlineup::{
    cascade_frequency_sweep, cascade_vector_return_output, cascade_vector_return_vector, Block,
    FrequencyBlock, FrequencySample, FrequencySweep, Input, SignalNode,
};

const SAMPLES: usize = 7;
const TARGET_SAMPLE: Duration = Duration::from_millis(120);

fn block(stage: usize, gain_db: f64) -> Block {
    Block {
        name: format!("stage-{stage:02}"),
        gain_db,
        noise_figure_db: 1.2 + stage as f64 * 0.08,
        output_p1db_dbm: Some(12.0 + stage as f64 * 0.2),
        output_ip3_dbm: Some(30.0 + stage as f64 * 0.3),
    }
}

fn input() -> Input {
    Input::new(1.0e9, 2.0e6, -18.0, Some(285.0))
}

fn scalar_blocks(count: usize) -> Vec<Block> {
    (0..count)
        .map(|stage| {
            // Includes mild passive loss and enough gain to exercise compression.
            let gain = match stage % 4 {
                0 => 8.0,
                1 => -1.0,
                2 => 7.0,
                _ => -0.5,
            };
            block(stage, gain)
        })
        .collect()
}

fn table_block(stage: usize, samples: usize) -> FrequencyBlock {
    let rows = (0..samples)
        .map(|index| {
            let fraction = index as f64 / (samples - 1) as f64;
            let mut b = block(stage, 4.0 + stage as f64 * 0.3 + fraction * 0.5);
            b.noise_figure_db += fraction * 0.15;
            b.output_p1db_dbm = Some(18.0 + stage as f64 * 0.2 + fraction * 0.5);
            b.output_ip3_dbm = Some(34.0 + stage as f64 * 0.25 + fraction * 0.5);
            FrequencySample {
                frequency_hz: 0.8e9 + fraction * 0.4e9,
                block: b,
            }
        })
        .collect();
    FrequencyBlock::tabulated(rows).expect("valid fixture table")
}

fn constant_sweep_blocks() -> Vec<FrequencyBlock> {
    scalar_blocks(8)
        .into_iter()
        .map(|b| FrequencyBlock::constant(b).expect("valid constant block"))
        .collect()
}

fn mixed_sweep_blocks() -> Vec<FrequencyBlock> {
    (0..8)
        .map(|stage| {
            if stage % 2 == 0 {
                FrequencyBlock::constant(block(stage, 5.0 + stage as f64 * 0.2))
                    .expect("valid constant block")
            } else {
                table_block(stage, 65)
            }
        })
        .collect()
}

fn dense_sweep_blocks() -> Vec<FrequencyBlock> {
    (0..16).map(|stage| table_block(stage, 257)).collect()
}

fn frequency_grid() -> FrequencySweep {
    FrequencySweep::linear(0.8e9, 1.2e9, 1001).expect("valid frequency grid")
}

fn run_sweep(
    blocks: &[FrequencyBlock],
    grid: &FrequencySweep,
) -> Vec<gainlineup::FrequencySweepPoint> {
    cascade_frequency_sweep(&input(), blocks, grid).expect("valid sweep")
}

// FNV-1a over exact field encodings; Option tags and name bytes are explicit.
struct Fingerprint(u64);

impl Fingerprint {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn bytes(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn f64(&mut self, value: f64) {
        self.bytes(&value.to_bits().to_le_bytes());
    }
    fn optional(&mut self, value: Option<f64>) {
        match value {
            Some(value) => {
                self.bytes(&[1]);
                self.f64(value);
            }
            None => self.bytes(&[0]),
        }
    }
    fn node(&mut self, node: &SignalNode) {
        self.bytes(&(node.name.len() as u64).to_le_bytes());
        self.bytes(node.name.as_bytes());
        self.f64(node.signal_frequency_hz);
        self.f64(node.signal_bandwidth_hz);
        self.f64(node.signal_power_dbm);
        self.f64(node.noise_power_dbm);
        self.f64(node.cumulative_noise_figure_db);
        self.f64(node.cumulative_gain_db);
        self.optional(node.cumulative_noise_temperature);
        self.optional(node.cumulative_oip3_dbm);
        self.optional(node.sfdr_db);
        self.optional(node.output_p1db_dbm);
    }
}

fn fingerprint_nodes<'a>(nodes: impl IntoIterator<Item = &'a SignalNode>) -> u64 {
    let mut hash = Fingerprint::new();
    for node in nodes {
        hash.node(node);
    }
    hash.0
}

fn report_fingerprint(name: &str, hash: u64) {
    println!("fingerprint {name}: {hash:016x}");
}

fn measure(name: &str, mut operation: impl FnMut()) {
    // Calibrate the batch size once, outside the seven reported measurements.
    let start = Instant::now();
    let mut iterations = 0usize;
    while start.elapsed() < Duration::from_millis(20) {
        operation();
        iterations += 1;
    }
    let batch = ((iterations as f64 * TARGET_SAMPLE.as_secs_f64() / start.elapsed().as_secs_f64())
        .ceil() as usize)
        .max(1);
    let mut times = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..batch {
            operation();
        }
        times.push(start.elapsed().as_secs_f64() * 1e9 / batch as f64);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{name}: median {:.1} ns/op ({} ops/sample)",
        times[SAMPLES / 2],
        batch
    );
}

fn selected(name: &str) -> bool {
    std::env::var("PERF_FILTER").map_or(true, |filter| filter == name)
}

fn main() {
    let grid = frequency_grid();
    let scalar = scalar_blocks(8);

    if selected("scalar-vector") {
        report_fingerprint(
            "scalar-vector",
            fingerprint_nodes(&cascade_vector_return_vector(input(), scalar.clone())),
        );
        measure("scalar-vector", || {
            black_box(cascade_vector_return_vector(input(), scalar.clone()));
        });
    }
    if selected("scalar-output") {
        let result = cascade_vector_return_output(input(), scalar.clone());
        report_fingerprint("scalar-output", fingerprint_nodes(std::iter::once(&result)));
        measure("scalar-output", || {
            black_box(cascade_vector_return_output(input(), scalar.clone()));
        });
    }
    if selected("constant-sweep") {
        let blocks = constant_sweep_blocks();
        let result = run_sweep(&blocks, &grid);
        report_fingerprint(
            "constant-sweep",
            fingerprint_nodes(result.iter().flat_map(|point| point.nodes.iter())),
        );
        measure("constant-sweep", || {
            black_box(run_sweep(&blocks, &grid));
        });
    }
    if selected("mixed-sweep") {
        let blocks = mixed_sweep_blocks();
        let result = run_sweep(&blocks, &grid);
        report_fingerprint(
            "mixed-sweep",
            fingerprint_nodes(result.iter().flat_map(|point| point.nodes.iter())),
        );
        measure("mixed-sweep", || {
            black_box(run_sweep(&blocks, &grid));
        });
    }
    if selected("dense-tabulated") {
        let blocks = dense_sweep_blocks();
        let result = run_sweep(&blocks, &grid);
        report_fingerprint(
            "dense-tabulated",
            fingerprint_nodes(result.iter().flat_map(|point| point.nodes.iter())),
        );
        measure("dense-tabulated", || {
            black_box(run_sweep(&blocks, &grid));
        });
    }
}
