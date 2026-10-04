# gainlineup

RF signal chain (gain lineup) analysis for receiver and transmitter design.

[![Crates.io](https://img.shields.io/crates/v/gainlineup.svg)](https://crates.io/crates/gainlineup)
[![Docs.rs](https://docs.rs/gainlineup/badge.svg)](https://docs.rs/gainlineup)

## What It Does

`gainlineup` models an RF signal chain as a sequence of blocks (amplifiers, filters, attenuators, mixers) and cascades their effects on signal power, noise, and linearity. Think of it as a spreadsheet-style RF lineup — but in Rust, with proper Friis equation cascading.

## When To Use This Crate

Use `gainlineup` for ordered RF hardware chains: LNAs, filters, attenuators,
mixers, power amplifiers, cascaded gain/noise figure, P1dB compression,
IP3/IMD3, SFDR, dynamic range, and AM-AM/AM-PM behavior.

If the task starts from `.sNp` S-parameter files or network matrices, use
`touchstone`. If it is an end-to-end communication link question involving
path loss, C/No, Eb/No, BER, margin, orbit, Doppler, PFD, or modulation, use
`linkbudget`. Use `rfconversions` for standalone scalar conversions before
building a chain.

Model each hardware stage as a `Block`. Negative `gain_db` represents loss, and
passive losses usually have matching positive `noise_figure_db`. P1dB and IP3
fields are output-referred dBm values.

## Installation

```toml
[dependencies]
gainlineup = "0.23.0"
```

## Quick Start

### 1. Define Your Input Signal

Every chain starts with an input signal: power level, frequency, bandwidth, and optionally a noise temperature (e.g., antenna sky temperature).

```rust
use gainlineup::{Input};

let input = Input {
    power_dbm: -80.0,          // received signal level
    frequency_hz: 6.0e9,       // 6 GHz C-band
    bandwidth_hz: 1.0e6,       // 1 MHz channel
    noise_temperature_k: Some(50.0), // cool sky
};
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_01_input_signal.rs)

### 2. Define Your Blocks

Each block in the chain has a name, gain, noise figure, and optionally compression (P1dB) and linearity (IP3) specs.

```rust
use gainlineup::{Block};

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
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_02_blocks.rs)

### 3. Run the Cascade

Pass the input and blocks through the cascade to get signal nodes at each stage.

```rust
use gainlineup::{Block, Input, cascade_vector_return_vector};

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

for node in &nodes {
    println!("{}: Pout={:.1} dBm, NF={:.2} dB, Gain={:.1} dB",
        node.name, node.signal_power_dbm,
        node.cumulative_noise_figure_db, node.cumulative_gain_db);
}

// Final cascade result
let output = nodes.last().unwrap();
println!("\nCascade: Gain={:.1} dB, NF={:.2} dB, SNR={:.1} dB",
    output.cumulative_gain_db,
    output.cumulative_noise_figure_db,
    output.signal_to_noise_ratio_db());
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_03_cascade.rs)

### 4. What Gets Cascaded

At each node in the chain, the cascade computes:

| Parameter             | Description                                         |
|-----------------------|-----------------------------------------------------|
| Signal Power (dBm)    | Cumulative signal level, with compression            |
| Noise Power (dBm)     | Cumulative noise from all stages                     |
| Gain (dB)             | Cumulative gain (accounts for compression)           |
| Noise Figure (dB)     | Cascaded NF via Friis equation                       |
| Noise Temperature (K) | Source plus input-referred stage noise temperature   |
| OIP3 (dBm)            | Cascaded output IP3 (when blocks have IP3 set)       |
| SFDR (dB)             | Spur-free dynamic range: `2/3 × (OIP3 − noise floor)` |

Each block adds input-referred noise `kTₑB`, where `Tₑ = 290 K × (F − 1)` and `F` is its linear noise factor. The first stage starts with the source temperature, so its cumulative input-referred temperature is `Tsource + Tₑ`. Later stages use the same cascade calculation and refer each added temperature to the chain input.

Cascaded OIP3 uses output-referred powers: `1/OIP3_total =
1/(G_stage × OIP3_previous) + 1/OIP3_stage`, with watts and linear power gain.
SFDR compares OIP3 with the calculated noise power at that same output node,
including source temperature and bandwidth. A missing block IP3 clears the
cumulative estimate; a later characterized block starts a new local estimate.
That restarted estimate does not characterize the preceding unknown stages.

---

## Compression (P1dB)

When a block has `output_p1db_dbm` set, the output power clamps at P1dB + 1 dB. The model applies this limit independently to the signal, incoming noise, and block-added noise. Friis noise figure and input-referred noise temperature are small-signal metrics; after an earlier stage compresses, the cascade uses its compressed signal gain for later stages' noise terms.

```rust
use gainlineup::{Block};

let pa = Block {
    name: "Power Amplifier".to_string(),
    gain_db: 30.0,
    noise_figure_db: 5.0,
    output_p1db_dbm: Some(20.0), // compresses above +20 dBm out
    output_ip3_dbm: None,
};

// Linear region
assert_eq!(pa.output_power(-20.0), 10.0);  // -20 + 30 = 10 (below P1dB)
assert_eq!(pa.power_gain(-20.0), 30.0);    // full gain

// Compressed
assert_eq!(pa.output_power(0.0), 21.0);    // 0 + 30 = 30, clamps to 21
assert_eq!(pa.power_gain(0.0), 21.0);      // reduced gain
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_04_compression.rs)

---

## Dynamic Range

Dynamic range tells you the usable power range of a block or chain: from the noise floor up to the compression point.

```rust
use gainlineup::{Block};

let lna = Block {
    name: "LNA".to_string(),
    gain_db: 20.0,
    noise_figure_db: 3.0,
    output_p1db_dbm: Some(10.0),
    output_ip3_dbm: None,
};

// Output-referred: P1dB_out - noise_floor_out
let dr = lna.dynamic_range_db(1e6).unwrap();
println!("Output dynamic range: {:.1} dB", dr);

// Input-referred: input_P1dB - input_noise_floor
let dr_in = lna.input_dynamic_range_db(1e6).unwrap();
println!("Input dynamic range: {:.1} dB", dr_in);
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_05_dynamic_range.rs)

Returns `None` when P1dB is not set (linear block, infinite dynamic range).

---

## AM-AM Curves (Power Sweep)

Sweep input power to see how a block or chain behaves from linear through compression. This is the classic "Pin vs Pout" curve from amplifier datasheets.

### Single Block

```rust
use gainlineup::{Block};

let lna = Block {
    name: "LNA".to_string(),
    gain_db: 20.0,
    noise_figure_db: 3.0,
    output_p1db_dbm: Some(10.0),
    output_ip3_dbm: None,
};

// Pin vs Pout
let curve = lna.am_am_sweep(-50.0, 0.0, 1.0);
for (pin, pout) in &curve {
    println!("Pin={:.0} dBm → Pout={:.1} dBm", pin, pout);
}

// Pin vs Gain (shows compression directly)
let gc = lna.gain_compression_sweep(-50.0, 0.0, 1.0);
for (pin, gain) in &gc {
    println!("Pin={:.0} dBm → Gain={:.1} dB", pin, gain);
}
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_06_am_am_single_block.rs)

### Full Cascade

```rust
use gainlineup::{Block, cascade_am_am_sweep, cascade_gain_compression_sweep};

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

// Cascade Pin vs Pout
let am_am = cascade_am_am_sweep(&blocks, -80.0, -20.0, 1.0);
for (pin, pout) in &am_am {
    println!("Pin={:.0} → Pout={:.1}", pin, pout);
}

// Cascade Pin vs Gain
let gc = cascade_gain_compression_sweep(&blocks, -80.0, -20.0, 1.0);
for (pin, gain) in &gc {
    println!("Pin={:.0} → Gain={:.1} dB", pin, gain);
}
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_07_am_am_cascade.rs)

---

## IMD3 (Intermodulation from IP3)

When a block has `output_ip3_dbm` set, you can compute third-order intermodulation products — the spurious signals that appear in a two-tone test.

```rust
use gainlineup::{Block};

let amp = Block {
    name: "Driver Amp".to_string(),
    gain_db: 20.0,
    noise_figure_db: 5.0,
    output_p1db_dbm: None,
    output_ip3_dbm: Some(30.0), // OIP3 = +30 dBm
};

// Single point
let im3 = amp.imd3_output_power_dbm(-30.0).unwrap();
println!("IM3 at Pin=-30: {:.1} dBm", im3); // -90 dBm

let rejection = amp.imd3_rejection_db(-30.0).unwrap();
println!("IM3 rejection: {:.0} dB", rejection); // 80 dB below carrier

// Full two-tone sweep
let sweep = amp.imd3_sweep(-50.0, -10.0, 5.0);
for pt in &sweep {
    println!("Pin={:.0} Pout={:.1} IM3={:.1} Rejection={:.0} dB",
        pt.input_per_tone_dbm, pt.output_per_tone_dbm,
        pt.im3_output_dbm, pt.rejection_db);
}
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_08_imd3.rs)

**Key relationships:**
- `IM3_out = 3 × Pout - 2 × OIP3` (all dBm)
- `Rejection = 2 × (OIP3 - Pout)` (dB)
- IM3 follows the **3:1 slope rule**: 3 dB increase per 1 dB input increase

---

## Node-Level Dynamic Range Summary

After running a cascade, each `SignalNode` can produce a dynamic range summary that combines P1dB, noise floor, SFDR, and input limits into one struct.

```rust
use gainlineup::{Input, Block, cascade_vector_return_output};

let input = Input::new(6.0e9, 1.0e6, -80.0, Some(50.0));
let blocks = vec![
    Block {
        name: "LNA".to_string(),
        gain_db: 20.0,
        noise_figure_db: 1.5,
        output_p1db_dbm: Some(5.0),
        output_ip3_dbm: Some(20.0),
    },
];
let node = cascade_vector_return_output(input, blocks);

// Simple linear dynamic range
if let Some(dr) = node.dynamic_range_db() {
    println!("Linear DR: {:.1} dB", dr);
}

// Full summary
if let Some(summary) = node.dynamic_range_summary() {
    println!("Linear DR: {:.1} dB", summary.linear_dr_db);
    println!("SFDR:      {:?}", summary.sfdr_db);
    println!("MDS:       {:.1} dBm", summary.mds_dbm);
    println!("Max input: {:.1} dBm", summary.max_input_dbm);
}
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_09_node_dynamic_range.rs)

Returns `None` when the node has no P1dB (e.g., a passive stage without a compression spec).

---

## AmplifierModel + AM-PM

`AmplifierModel` wraps a `Block` and adds AM-PM (phase distortion) characterization. It's a separate struct — the core `Block` stays simple for cascade analysis, while `AmplifierModel` provides richer single-amplifier modeling.

```rust
use gainlineup::{Block, AmplifierModel};

let pa = Block {
    name: "Power Amp".to_string(),
    gain_db: 20.0,
    noise_figure_db: 5.0,
    output_p1db_dbm: Some(10.0),
    output_ip3_dbm: Some(25.0),
};

// Simple: no AM-PM
let model = AmplifierModel::new(&pa);

// With AM-PM coefficient (10 °/dB near P1dB)
let model = AmplifierModel::with_am_pm(&pa, 10.0);

// Builder pattern for full configuration
let model = AmplifierModel::builder(&pa)
    .am_pm_coefficient(10.0)
    .saturation_power(25.0)
    .build();

// Phase shift at a given input power
if let Some(phase) = model.phase_shift_at(-5.0) {
    println!("Phase shift: {:.1}°", phase);
}

// Combined AM-AM + AM-PM sweep
let sweep = model.am_am_am_pm_sweep(-40.0, 0.0, 1.0);
for pt in &sweep {
    println!("Pin={:.0} Pout={:.1} Gain={:.1} Δφ={:?}",
        pt.input_dbm, pt.output_dbm, pt.gain_db, pt.phase_shift_deg);
}

// Required backoff for a phase budget
if let Some(backoff) = model.backoff_for_target_phase(5.0) {
    println!("Backoff for ≤5° phase: {:.1} dB below P1dB", backoff);
}

// EVM from AM-PM distortion
if let Some(evm) = model.evm_from_am_pm(-5.0) {
    println!("EVM from AM-PM: {:.4} ({:.2}%)", evm, evm * 100.0);
}
```

> [Full example →](https://github.com/iancleary/gainlineup/blob/main/tests/readme_10_amplifier_model.rs)

---

## CLI (TOML File Input)

The command-line tool reads a TOML file defining the input and blocks, runs the cascade, and generates an HTML table.

```bash
gainlineup files/wideband.toml
```

### TOML Format

```toml
input_power_dbm = -80.0
frequency_hz = 6.0e9
bandwidth_hz = 1.0e6

[[blocks]]
type = "explicit"
name = "Low Noise Amplifier"
gain_db = 20.0
noise_figure_db = 3.0

[[blocks]]
type = "explicit"
name = "Mixer"
gain_db = 10.0
noise_figure_db = 6.0

[[blocks]]
type = "explicit"
name = "IF Amplifier"
gain_db = 15.0
noise_figure_db = 5.0
```

### Field Aliases

For brevity, you can use short field names. The unit-suffixed names are recommended for clarity.

| Full Name            | Aliases              |
|----------------------|----------------------|
| `gain_db`            | `gain`               |
| `noise_figure_db`    | `noise_figure`, `nf` |
| `output_p1db_dbm`    | `output_p1db`, `op1db` |
| `output_ip3_dbm`     | `output_ip3`, `oip3` |
| `input_power_dbm`    | `input_power`, `pin` |
| `frequency_hz`       | `frequency`, `f`     |
| `bandwidth_hz`       | `bandwidth`, `bw`    |
| `noise_temperature_k`| `noise_temperature`  |

> **Caution:** Aliases hide unit suffixes. `pin` is always dBm, `f` is always Hz. If you assume different units, you'll get wrong results silently.

### HTML Output

The CLI generates an HTML visualization of the cascade:

[![HTML cascade output](https://github.com/iancleary/gainlineup/blob/main/files/wideband.toml.html.png?raw=true)](https://github.com/iancleary/gainlineup/tree/main/files/wideband.toml.html)

---

## Frequency Sweeps

The sweep API evaluates the existing cascade at each frequency. It returns every
stage output, so you can inspect gain, NF, signal power, noise, compression, and
linearity across a band. These additions are available in gainlineup 0.23.0.

The S-band sweep examples use the **2200–2290 MHz near-Earth S-band space-to-Earth
downlink**. NASA's [S-band allocation overview](https://explorers.larc.nasa.gov/2023ESE/pdf_files/S-Band-Overview_2-8GHz.pdf)
identifies this downlink range; the FCC's [47 CFR § 2.106](https://www.ecfr.gov/current/title-47/chapter-I/subchapter-A/part-2/subpart-B/section-2.106)
lists the allocations and applicable conditions. Band-edge points characterize
components across the allocation; they are not assigned operating channels.

```rust
use gainlineup::{
    cascade_frequency_sweep, Block, FrequencyBlock, FrequencySample, FrequencySweep, Input,
};

let switch = FrequencyBlock::tabulated(vec![
    FrequencySample {
        frequency_hz: 2.2e9,
        block: Block {
            name: "RX switch".into(),
            gain_db: -1.0,
            noise_figure_db: 1.0,
            ..Block::default()
        },
    },
    FrequencySample {
        frequency_hz: 2.29e9,
        block: Block {
            name: "RX switch".into(),
            gain_db: -2.0,
            noise_figure_db: 2.0,
            ..Block::default()
        },
    },
])?;
let lna = FrequencyBlock::constant(Block {
    name: "LNA".into(),
    gain_db: 20.0,
    noise_figure_db: 1.5,
    ..Block::default()
})?;
let grid = FrequencySweep::linear(2.2e9, 2.29e9, 3)?;
let input = Input::new(0.0, 1e6, -80.0, Some(290.0));
let points = cascade_frequency_sweep(&input, &[switch, lna], &grid)?;

let midband = &points[1].nodes[1];
assert_eq!(points[1].frequency_hz, 2.245e9);
assert_eq!(midband.signal_power_dbm, -61.5);
assert!((midband.cumulative_noise_figure_db - 3.0).abs() < 1e-9);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The executable mirror is [tests/readme_11_frequency_sweep.rs](tests/readme_11_frequency_sweep.rs).

- `FrequencySweep::linear(start_hz, stop_hz, points)` includes both endpoints.
  `logarithmic` uses the same arguments. Range grids require 2–1,000,000 points.
- `FrequencySweep::from_frequencies` accepts 1–1,000,000 strictly increasing,
  finite, positive frequencies. It does not sort or deduplicate the input.
- Tables need at least two ordered samples with the same component name.
  Gain and NF interpolate linearly in dB versus Hz; intercepts interpolate in
  dBm. Optional intercepts must be present at every sample or absent at every
  sample. Frequencies outside the table return an error.
- Input power, bandwidth, and source temperature remain fixed across the sweep.
  Only `Input::frequency_hz` is replaced. The library retains its 270 K default
  for an unspecified temperature; the CLI retains its 290 K default.
- Each point is a narrowband scalar evaluation. A sweep does not integrate
  noise across the swept band, change signal bandwidth, or translate frequency.
  Existing `Block`, `Input`, and single-frequency cascade APIs remain usable.

### CLI and Component Tables

Run from the repository root:

```bash
cargo run --quiet -- sweep files/frequency_sweep.toml 2.2e9 2.29e9 101 > sweep.csv
cargo run --quiet --example transceiver_sweeps > transceiver.csv
```

The `sweep` command writes CSV with one row per stage per frequency, including
gain, NF, signal/noise power, SNR, OIP3, and SFDR. Stage indices start at 1.
Unavailable optional metrics are empty fields. Diagnostics go to stderr.
Invalid configurations fail before any CSV rows are emitted. No HTML file or
browser tab is created. The existing single-frequency CLI still produces HTML.

Add a frequency-dependent component to a TOML lineup as follows:

```toml
[[blocks]]
type = "tabulated"
name = "RX switch"

[[blocks.samples]]
frequency_hz = 2.2e9
gain_db = -1.0
noise_figure_db = 1.0

[[blocks.samples]]
frequency_hz = 2.29e9
gain_db = -2.0
noise_figure_db = 2.0
```

Each sample can also specify `output_p1db_dbm` and `output_ip3_dbm`.
The top-level `frequency_hz` remains required for the existing file format;
the sweep command replaces it with the requested range. Explicit blocks are
constant across frequency. Includes resolve relative to their containing file.
Touchstone blocks load once per sweep and interpolate scalar `S21` gain in dB
versus Hz within the measured band. This is a matched scalar approximation,
without complex phase or mismatch effects. The single-frequency Touchstone
loader retains its exact-sample behavior. Supply NF explicitly for active
Touchstone devices; the default `NF = -gain` describes passive loss at 290 K.

### GNSS Receiver

[demos/gnss_receiver.rs](demos/gnss_receiver.rs) models separate GPS L1
and L5 receiver RF paths. Their carrier frequencies are **1575.42 MHz** and
**1176.45 MHz**, as specified in the [GPS signal interface documentation](https://archive.gps.gov/technical/icwg/IS-GPS-200N.pdf).
Each path uses:

`Antenna terminal → feed/ESD loss → preselection SAW → LNA → post-LNA SAW → receiver RF gain stage`

The SAW–LNA–SAW topology is also used in commercial GNSS front ends; see the
[u-blox NEO-F10N integration manual, section 4.3](https://content.u-blox.com/sites/default/files/documents/NEO-F10N_IntegrationManual_UBXDOC-963802114-12193.pdf).
The example's component values are illustrative and do not represent that device.
The antenna is represented by received power and source temperature at its
terminal. Its gain is already included in the assumed −130 dBm input power.

| Path | Carrier | Component sweep | Assumed analysis bandwidth |
|------|---------|-----------------|----------------------------|
| GPS L1 | 1575.42 MHz | 1573.42–1577.42 MHz | 2 MHz |
| GPS L5 | 1176.45 MHz | 1164.45–1188.45 MHz | 20 MHz |

These are chosen characterization windows around each carrier, not allocation
edges. Bandwidth is a fixed rectangular noise-analysis assumption. The sampled
filter loss does not define or integrate a measured equivalent noise bandwidth.
L1 and L5 use separate filter/LNA paths rather than interpolation across the
large gap between the two carriers.

```bash
# Both paths, every stage, including C/N0 in dB-Hz
cargo run --quiet --example gnss_receiver > gnss_receiver.csv

# Equivalent TOML lineups through the standard sweep CLI
cargo run --quiet -- sweep files/gnss_l1.toml 1573.42e6 1577.42e6 41 > gnss_l1.csv
cargo run --quiet -- sweep files/gnss_l5.toml 1164.45e6 1188.45e6 41 > gnss_l5.csv
```

The Rust example exports `cn0_db_hz` at every stage, using
`node.signal_power_dbm - node.noise_spectral_density()`.
Equivalently, `C/N0 = SNR + 10 log10(bandwidth_hz)`. The standard CLI keeps
its existing columns; calculate C/N0 from its SNR and the configured bandwidth.

At either carrier, with the assumed −130 dBm input and 290 K source, the model
gives 32 dB gain, approximately 2.365 dB NF, −98 dBm output signal, and
41.610 dB-Hz C/N0. L1 pre-correlation SNR is approximately −21.400 dB;
L5 is −31.400 dB because its analysis bandwidth is ten times larger. The
equal C/N0 values follow from equal assumed carrier powers and center-frequency
component characteristics. These values are checked in
[tests/readme_12_gnss_receiver.rs](tests/readme_12_gnss_receiver.rs).

This is an RF budget through the receiver input. Acquisition, despreading,
tracking, AGC/ADC behavior, antenna patterns, and interference rejection need
additional models. In particular, a negative pre-correlation SNR is not an
acquisition failure criterion. See ESA's [GNSS front-end overview](https://gssc.esa.int/navipedia/index.php/Front_End)
for the distinction between front-end and signal-processing performance.

### From Separate Lineups to a Transceiver

[demos/transceiver_sweeps.rs](demos/transceiver_sweeps.rs) evaluates three
illustrative lineups. Component values are examples, not measured specifications.
The spacecraft transmitter and ground receiver are the two downlink endpoints.

| Lineup | Source and selected path | Sweep |
|--------|--------------------------|-------|
| Ground RX RF section | Ground antenna → switch RX route → LNA → RF filter | 2200–2290 MHz |
| Spacecraft TX RF section | RF input → driver → PA → output filter → switch TX route → spacecraft antenna | 2200–2290 MHz |
| Ground RX LO distribution | Oscillator carrier → buffer → splitter branch → LO switch | 2100–2190 MHz |

The ground receiver uses a chosen 100 MHz IF with low-side LO injection:
`f_LO = f_RF - 100 MHz`. The LO range is internal tuning, not a radiated
allocation. The example derives its LO endpoints from the RF endpoints and IF.

The oscillator is an `Input` for its carrier-power budget. The selected switch
path is a loss block. These lineups calculate carrier levels and thermal-noise
budgets; they do not yet connect mixer LO ports or predict phase noise,
off-state isolation, or simultaneous TX leakage into RX.

The next modeling improvements, in implementation order, are:

| Priority | Addition | Acceptance case |
|----------|----------|-----------------|
| 1 | Explicit mixer stage and frequency plan, with LO reference, selected product, image frequency, inversion, and port limits | 2245 MHz RF and 2145 MHz LO produce 100 MHz IF; report the 2045 MHz image and evaluate the next filter at 100 MHz |
| 2 | Oscillator phase-noise samples versus offset, LO-drive limits, and buffer residual noise | Check drive margin at RX/TX mixer ports; integrate phase noise over stated offset limits; calculate blocker reciprocal mixing |
| 3 | Named operating states and shared component identities, with separate switch insertion-loss and isolation paths | Select RX or TX routes; +30 dBm TX and 50 dB isolation give −20 dBm leakage before downstream losses |
| 4 | Filter response and equivalent noise bandwidth, distinct from signal bandwidth | Halving rectangular noise bandwidth lowers white-noise power by 3.01 dB; integrate noise through downstream filters |
| 5 | Requirement margins and tolerance sweeps, with explicit unknown versus ideal linearity | Check sensitivity, gain ripple, compression/backoff, LO drive, and leakage across frequency and component tolerances |

Use a companion stage/system layer for these additions so existing `Block` and
`Input` literals remain compatible. Start with selected named paths before adding
a general graph solver. Mixer noise needs explicit image-sideband assumptions;
an SSB/DSB label alone is insufficient for general cascades. See Analog Devices'
[mixer noise analysis](https://www.analog.com/en/resources/technical-articles/system-noisefigure-analysis-for-modern-radio-receivers.html),
[phase-noise integration tutorial](https://www.analog.com/media/en/training-seminars/tutorials/mt-008.pdf),
and [LO reciprocal-mixing discussion](https://www.analog.com/en/resources/technical-articles/wideband-lo-noise-in-passive-transmitreceive-mixer-ics.html).

Keep full S-parameter mismatch analysis in `touchstone`. Waveform EVM/ACLR,
PA memory effects, and correlated shared-LO noise need richer models than this
scalar cascade.

---

## API Summary

### Core Types

| Type         | Description                                      |
|--------------|--------------------------------------------------|
| `Input`      | Signal entering the chain (power, freq, BW, temp)|
| `Block`      | A component: gain, NF, P1dB, IP3                 |
| `SignalNode`  | Result at each stage: power, noise, NF, gain, OIP3, SFDR |
| `Imd3Point`  | Two-tone test result: carrier + IM3 levels        |
| `DynamicRange` | Summary: linear DR, SFDR, MDS, max input        |
| `AmplifierModel` | Block wrapper with AM-PM characterization     |
| `AmplifierPoint` | Combined AM-AM + AM-PM sweep point             |
| `FrequencySweep` | Validated linear, logarithmic, or explicit frequency grid |
| `FrequencyBlock` | Constant or tabulated scalar component response |
| `FrequencySample` | A block characterization at one frequency |
| `FrequencySweepPoint` | Frequency and every stage's output node |
| `SweepError` | Invalid sweep inputs, characterization, or numerical results |

### Cascade Functions

| Function                          | Returns                              |
|-----------------------------------|--------------------------------------|
| `cascade_vector_return_output()`  | Final `SignalNode` only              |
| `cascade_vector_return_vector()`  | `Vec<SignalNode>` at every stage     |
| `cascade_am_am_sweep()`          | `Vec<(Pin, Pout)>` through full chain |
| `cascade_gain_compression_sweep()`| `Vec<(Pin, Gain)>` through full chain |
| `cascade_frequency_sweep()` | `Result<Vec<FrequencySweepPoint>, SweepError>` |

### Block Methods

| Method                        | Returns                              |
|-------------------------------|--------------------------------------|
| `output_power(pin)`           | Pout with compression                |
| `power_gain(pin)`             | Gain at a given input level          |
| `dynamic_range_db(bw)`        | Output-referred DR (P1dB - noise)    |
| `input_dynamic_range_db(bw)`  | Input-referred DR                    |
| `am_am_curve(powers)`         | `Vec<(Pin, Pout)>`                   |
| `am_am_sweep(start, stop, step)` | `Vec<(Pin, Pout)>` evenly spaced  |
| `gain_compression_curve(powers)` | `Vec<(Pin, Gain)>`                |
| `gain_compression_sweep(..)`  | `Vec<(Pin, Gain)>` evenly spaced     |
| `imd3_output_power_dbm(pin)`  | IM3 product power (dBm)             |
| `imd3_rejection_db(pin)`      | Carrier minus IM3 (dB)              |
| `imd3_sweep(start, stop, step)` | `Vec<Imd3Point>`                  |

### SignalNode Methods

| Method                      | Returns                                |
|-----------------------------|----------------------------------------|
| `signal_to_noise_ratio_db()`| SNR at this node (dB)                  |
| `noise_spectral_density()`  | Noise PSD (dBm/Hz)                     |
| `dynamic_range_db()`        | Linear DR at node: P1dB − noise (dB)   |
| `dynamic_range_summary()`   | Full `DynamicRange` summary             |

---

## Diagnostics (Tracing)

`gainlineup` uses [`tracing`](https://docs.rs/tracing) for structured, runtime-controllable diagnostics. Set the `RUST_LOG` environment variable to see intermediate cascade calculations:

```bash
# See cascade math: noise power, signal power, compression at each stage
RUST_LOG=gainlineup=debug gainlineup config.toml

# See everything including config file content
RUST_LOG=gainlineup=trace gainlineup config.toml

# Combine with dependency crates (e.g. touchstone file parsing)
RUST_LOG=gainlineup=debug,touchstone=debug gainlineup config.toml

# Only warnings and errors (quiet mode)
RUST_LOG=gainlineup=warn gainlineup config.toml
```

### Using tracing in your own application

If you use `gainlineup` as a library, install any `tracing` subscriber in your application to capture events:

```rust
use tracing_subscriber::EnvFilter;

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::from_default_env())
    .init();

// Now all gainlineup tracing events will be captured
let node = input.cascade_block(&lna);
```

Without a subscriber installed, all tracing calls are zero-cost no-ops.

---

## Performance Checks

The 0.23.1 performance pass measured 1.20–1.58× faster scalar cascades and
1.48–6.58× faster frequency sweeps on the local benchmark workloads, with
matching node fingerprints. See [benchmark results and method](docs/performance.md)
for the before/after timings, workload sizes, and verification limits.

Run `just bench` to measure scalar cascades and constant, mixed, and tabulated
frequency sweeps. The benchmark uses release optimization and reports the median
of seven samples per case. To measure one case, use
`PERF_FILTER=mixed-sweep just bench`.

Component fixtures are built before timing. Scalar cases include the block-vector
clone required by the owned API. Each case prints a fingerprint of every node's
fields, including the exact floating-point bits. Compare fingerprints and timings
between revisions on the same machine and Rust toolchain. Run benchmarks without
other builds or tests competing for CPU time.

Constant-only frequency sweeps calculate the scalar metrics once, then copy the
results with each requested frequency. Mixed and tabulated lineups are evaluated
at every frequency. The interpolation and RF equations use the same arithmetic
as individual scalar cascades. Tests compare every result field bit for bit,
including compression, source temperature, missing intercepts, and table samples.

---

## References

- Pozar, D. *Microwave Engineering* (4th ed.) — Friis equation, noise figure, IP3
- Razavi, B. *RF Microelectronics* (2nd ed.) — dynamic range, SFDR, receiver design
- [Noise Figure — Wikipedia](https://en.wikipedia.org/wiki/Noise_figure)

---

## Optional: Monte Carlo SNR Margin Example (examples crate)

This repo includes an optional standalone example crate for Monte Carlo analysis of
cascade SNR margin using `montycarlo`.

Path:
- `examples/montecarlo-gain-target/`

What it does:
- Builds a 3-block cascade lineup with `gainlineup`
- Applies random variation to **gain + noise figure** of the first two blocks
- Randomizes input signal power and input noise temperature
- Computes output-node SNR margin vs a target
- Writes CSV + summary text and a Python histogram/CDF plot

Run:

```bash
cd examples/montecarlo-gain-target
uv sync
cargo run
uv run plot_gain_margin.py
```

Output files:
- `output/cascade_snr_margin_samples.csv`
- `output/cascade_snr_margin_summary.txt`
- `output/cascade_snr_margin_plots.png`
