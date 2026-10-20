# Small-signal spectral noise

This evaluator is implemented **on this branch, not yet released**. The public
entry point is [`integrate_small_signal_noise`](../src/noise.rs), reusing
[`FrequencyBlock` and `FrequencySweep`](../src/sweep.rs). Start with the
[README example](../README.md#spectral-noise-integration-on-this-branch-not-released)
or the complete [illustrative receiver](../demos/spectral_noise.rs):

```bash
cargo run --quiet --example spectral_noise
cargo test --test spectral_noise --test readme_13_spectral_noise
```

## Mental model: several noise sources, one output reference plane

Treat the chain as an ordered list of matched scalar stages. Gain `G_i(f)` is
linear **power** gain, not voltage gain; `F_i(f)` is linear noise factor referenced
to `T0 = 290 K`. NF and gain in dB convert with `10^(dB/10)`. Each stage has
an independent equivalent input noise temperature `T_e,i = T0*(F_i - 1)`.
The source has an explicitly supplied, band-constant temperature `Ts` in kelvin.
With Boltzmann's constant `k = 1.380649e-23 J/K`:

```text
D_i(f)      = product of G_j(f), j = i through last stage
S_source(f) = k * Ts * D_0(f)                       [W/Hz]
S_stage,i(f)= k * 290 * (F_i(f) - 1) * D_i(f)       [W/Hz]
N_source   = integral from f_start to f_stop of S_source(f) df   [W]
N_stage,i  = integral from f_start to f_stop of S_stage,i(f) df  [W]
N_total    = N_source + sum_i N_stage,i                         [W]
```

`D_i` includes the current stage because its equivalent noise temperature is
input-referred. Every returned contribution is instead **final-output-referred**.
Later loss therefore attenuates earlier noise as well as the source. Reordering
stages can preserve net gain while changing the noise budget. To study an
intermediate output, call the function with that prefix of the chain; do not
read a final-output contribution as power at its originating stage.

`IntegratedNoise` records exact band endpoints, grid point count, propagated
source power, input-ordered `NoiseContribution` entries, and their total.
Duplicate stage names remain separate entries: use position to distinguish them.
Watts are canonical, including zero noise. Only convert positive results with
`10*log10(watts/1e-3)` for dBm; zero watts has no finite dBm representation.
Zero kelvin and zero NF are accepted as classical ideal limits, not claims of
physically realizable hardware or a quantum-noise model.

## Signal bandwidth is not the integration band

The older `cascade_frequency_sweep` evaluates independent lineups with fixed
`Input::bandwidth_hz`. It does **not** integrate across its sweep grid. Averaging
its dBm values cannot give an integrated noise budget.

The new API takes no `Input`, carrier power, or occupied signal bandwidth.
Its band is exactly the first through last frequency in the supplied grid;
trapezoids use actual Hz intervals, including for irregular/logarithmic grids.
It returns noise power only, not filtered modulated signal power, SNR, or ENBW.
An equivalent noise bandwidth requires a specified reference transfer/gain;
each noise source sees a different remaining transfer path here.

## Characterization and convergence are separate questions

Tables interpolate gain and NF linearly **in dB versus Hz**, as in the existing
frequency evaluator. Both band endpoints are checked against every table before
integration. Nothing extrapolates; constants apply throughout the declared band.

1. Choose a finite band supported by every component table and justified by the
   receiver measurement/model. Grid frequencies must be positive; zero is not
   implicitly included.
2. Hold tables and endpoints fixed. Increase quadrature points, preferably
   retaining table breakpoints in the grids. Compare total **and individual**
   contributions against a tolerance meaningful for the design.
3. Independently assess whether characterization is dense enough around steep
   features. Fine quadrature on a sparse table is not additional measured data.
4. Assess omitted tails separately. Extending the band needs additional
   characterization; numerical convergence in one band proves nothing about
   noise outside it. There is no automatic adaptive integration.

The receiver demo uses illustrative preselector → LNA → post-LNA filter tables,
not measured hardware or a data-acquisition workflow. Passive stages have
`NF_dB = -gain_db`, appropriate to matched losses at 290 K; the source is 75 K.
Observed totals with the native Rust toolchain:

| Points | Total W | Total dBm | Change from previous |
| ---: | ---: | ---: | ---: |
| 41 | 2.338083045e-12 | -86.311401 | — |
| 81 | 2.335396210e-12 | -86.316394 | -0.114916% |
| 161 | 2.334724482e-12 | -86.317644 | -0.028763% |
| 321 | 2.334556550e-12 | -86.317956 | -0.007193% |

All integrate **990000000..=1010000000 Hz**. At 321 points, source power is
`3.544392406e-13 W`, preselector-added noise `9.158370738e-13 W`, LNA-added
noise `1.008666042e-12 W`, and post-filter-added noise `5.561419291e-14 W`.
The LNA contribution is the largest in this illustrative final-output budget.
The shrinking change demonstrates quadrature refinement for these tables,
not hardware accuracy or recovery of out-of-band tails.

An independent check integrates the piecewise tables analytically. On each
shared table interval, each summed dB gain is affine in Hz, so its linear
transfer is exponential. For endpoint log gains `a` and `a + d` and width `w`,
the exact area is `w * exp(a) * expm1(d) / d`, with limit `w * exp(a)` at `d=0`.
Each stage's `(F-1)*G` term is the difference of two such integrals. This gives
`2.334500572294e-12 W` for the same receiver tables, with the 321-point result
about **0.002398% above** that answer. This checks quadrature error independently
of the production trapezoid calculation, not physical accuracy of the tables.

[Public regression workflows](../tests/spectral_noise.rs) compare flat budgets
and every prefix with analytic expectations and the legacy small-signal cascade.
They also integrate a mathematical noise-free one-pole transfer
`G(f)=1/(1+(f/fc)^2)` against the finite-band analytic area
`fc*(atan(f_stop/fc)-atan(f_start/fc))`. With `fc=100 Hz` and `1..1000 Hz`,
more than 6.9% of the `0..infinity` area `pi*fc/2` is omitted. Dense
characterization plus quadrature refinement approaches the **finite-band**
answer; sparse characterization plus fine quadrature does not fix model error.
That mathematical transfer is not a physical ambient passive filter: a real
lossy filter also adds thermal noise.

## Validity and troubleshooting

- **Small signal only:** nominal gain is always used. P1dB/IP3 metadata have no
  effect. The API cannot check compression or hardware operating point because
  it takes no signal/blocker powers. Establish linear operation separately.
- **Frequency preserving only:** a block named `Mixer` still uses the same
  frequency at input and output. No image noise, LO noise, conversion, folding,
  or sideband model is inferred. Mismatch, correlation and complex network
  effects are outside this scalar model.
- **Temperature:** source temperature must be finite and nonnegative, with no
  implicit default. Stage NF is referenced to 290 K, not the source temperature.
  A physical-temperature-aware passive constructor remains a separate follow-up.
- **Coverage errors:** `NoiseError::Characterization` reports the zero-based
  stage index and frequency and wraps `SweepError` as its error source. Narrow
  the band or supply defensible additional characterization; do not silently
  clamp to the table edge.
- **Grid/chain errors:** at least two grid points and one stage are required.
  Existing `FrequencySweep` validation remains unchanged.
- **Numerical errors:** finite dB values can overflow or underflow on conversion.
  `NoiseError::Numerical` reports stage/frequency context (`None` means source
  or total arithmetic; interval errors identify the upper endpoint). Linear
  gains/products, PSDs, integrals and sums are checked. A gain that underflows
  to zero is an error, not perfect isolation. No partial successful result is
  returned. Rescale unrealistic models or reduce extreme bands/parameters;
  do not replace errors with zeros.
- **Floating point limits:** this is ordinary `f64` quadrature, not arbitrary
  precision. Subnormal precision loss and tiny terms lost when added to much
  larger totals remain possible. Trapezoid averages avoid needless overflow;
  near-zero NF uses `exp_m1` to avoid subtractive cancellation.

The implementation streams two PSD vectors and accumulated powers, using
O(stages) auxiliary storage independent of grid length. Noise arithmetic costs
O(points × stages); table binary searches additionally cost
O(points × sum(log(samples per table))). No dependency, CLI, or legacy
signal/noise behavior changes are needed.

For the evidence and broader proposals, see
[literature-backed modeling opportunities, section 1](literature-opportunities.md#1-integrate-noise-spectra-not-a-single-center-frequency-noise-floor).
