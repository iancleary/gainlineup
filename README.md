# gainlineup

RF signal chain (gain lineup) analysis for receiver and transmitter design.

[![Crates.io](https://img.shields.io/crates/v/gainlineup.svg)](https://crates.io/crates/gainlineup)

## What It Does

`gainlineup` models an RF signal chain as a sequence of blocks (amplifiers, filters, attenuators, mixers) and cascades their effects on signal power, noise, and linearity. Think of it as a spreadsheet-style RF lineup — but in Rust, with proper Friis equation cascading.

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
| Noise Temperature (K) | Cascaded system temperature                          |
| OIP3 (dBm)            | Cascaded output IP3 (when blocks have IP3 set)       |
| SFDR (dB)             | Spur-free dynamic range: `2/3 × (OIP3 − noise floor)` |

---

## Compression (P1dB)

When a block has `output_p1db_dbm` set, the output power clamps at P1dB + 1 dB. Signal and noise are compressed independently — noise only compresses if it actually exceeds P1dB (rare, but handled correctly).

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

### Cascade Functions

| Function                          | Returns                              |
|-----------------------------------|--------------------------------------|
| `cascade_vector_return_output()`  | Final `SignalNode` only              |
| `cascade_vector_return_vector()`  | `Vec<SignalNode>` at every stage     |
| `cascade_am_am_sweep()`          | `Vec<(Pin, Pout)>` through full chain |
| `cascade_gain_compression_sweep()`| `Vec<(Pin, Gain)>` through full chain |

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

## Features

### Debug Output

Enable verbose debug printing during cascade calculations:

```toml
[dependencies]
gainlineup = { version = "0.18.0", features = ["debug-print"] }
```

---

## References

- Pozar, D. *Microwave Engineering* (4th ed.) — Friis equation, noise figure, IP3
- Razavi, B. *RF Microelectronics* (2nd ed.) — dynamic range, SFDR, receiver design
- [Noise Figure — Wikipedia](https://en.wikipedia.org/wiki/Noise_figure)
