# Literature-backed modeling opportunities

This is a research shortlist, **not a replacement roadmap**. The first
small-signal spectral-noise evaluator and its temperature-aware passive-loss
constructor follow-up are now implemented on this branch, **not yet released**;
the other model families remain proposals. The original
shortlist was checked against gainlineup 0.23.1, including the frequency
sweeps, GNSS/transceiver examples, and standalone Monte Carlo example. The
recommendations favor established models that can produce useful, verifiable
results with modest inputs—not novelty for its own sake.

Implementation is tracked in [issue #79](https://github.com/iancleary/gainlineup/issues/79).
The documentation kickoff did not implement these models. The first code
milestone below does not complete all proposals or close that issue.
Coordinate correctness and API decisions with the existing
[1.0 readiness review, #78](https://github.com/iancleary/gainlineup/issues/78);
the proposed extensions are not all prerequisites for 1.0.

## Recommendation

Start with **frequency-integrated noise and per-stage noise contributions**.
It directly improves the existing receiver examples without requiring a waveform
simulator. In parallel, if measured amplifier curves are available, build an
opt-in **calibrated memoryless PA model**. Do not change the legacy clamp silently.

| Order | Opportunity | User-visible improvement | Relative scope and main prerequisite |
| --- | --- | --- | --- |
| 1 | Spectral noise integration and attribution | Actual filtered noise power; identify which component dominates | Medium; characterized gain/NF over the integration band |
| 2 | Smooth, calibrated AM-AM/AM-PM | Meaningful compression and backoff curves | Medium; shape parameters or measured curves |
| 3 | Correlation-aware uncertainty and yield | Know whether a lineup meets requirements across tolerances | Medium; defensible distributions and dependence assumptions |
| 4 | Image-aware mixers, then reciprocal mixing | Predict sensitivity loss from image noise and nearby blockers | Large; explicit frequency plan, sideband gains, LO characterization |
| 5 | Bussgang distortion estimates | Estimate useful signal versus nonlinear distortion without a full waveform engine | Exploratory; validated memoryless PA and explicit signal statistics |

These are relative engineering judgments, not measured schedules or speedups.
Items 1, 3, and 4 deepen features already mentioned in the README roadmap.
Items 2 and 5 address limitations of the current amplifier approximation.

### First implementation milestone

Implemented on this branch: a library-first, small-signal spectral-noise evaluator
for one characterized receiver path; see the [spectral-noise guide](spectral-noise.md)
and [runnable receiver](../demos/spectral_noise.rs). It reuses `FrequencyBlock`
characterization and returns band-integrated output noise, separate source/stage
contributions, and explicit integration limits. Analytic limits, node-by-node
attribution, and illustrative grid refinement are verified through public APIs.
The detailed delivery checklist lives in [issue #79](https://github.com/iancleary/gainlineup/issues/79).

Keep frequency translation, large-signal noise, mismatch, and waveform metrics
out of this first delivery. The small temperature-aware passive-loss constructor
follow-up is also implemented on this branch, not released. The existing scalar
and frequency-sweep APIs retain their current behavior.

## 1. Integrate noise spectra, not a single center-frequency noise floor

**Literature.** Friis [1] supplies the cascade foundation already used here.
Kerr and Randa [2], especially “Noise Temperature as a Measure of Noise Power”
and “Thermal Noise in Networks and Attenuators,” make the important distinction
between spectral noise temperature and physical temperature. The integration
and attribution below are an application of those principles to this crate.

**Original gap (addressed by the separate evaluator on this branch).**
[Frequency sweeps](../src/sweep.rs) still evaluate independent
narrowband lineups with a fixed `Input::bandwidth_hz`.
[Node noise](../src/node.rs) is already integrated over that assumed bandwidth.
The GNSS example explicitly does not integrate its SAW response. Averaging the
existing dBm results would not produce a correct band-integrated answer.

**Implemented first slice (not released).**
[`integrate_small_signal_noise`](../src/noise.rs) is the separate small-signal
spectral-noise evaluator using the existing characterized components. At each
frequency, for matched scalar stages with independent added noise:

```text
G_i(f)        = linear power gain of stage i
T_e,i(f)      = 290 K × (F_i(f) - 1)
S_i(f)       = G_i(f) × [S_(i-1)(f) + k × T_e,i(f)]    [W/Hz]
N_out        = integral over the declared band of S_out(f) df  [W]
```

Carry the source contribution and each stage's added contribution through the
remaining gains. Their integrated powers must sum to the final noise power.
Report these contributions, integration limits, and convergence—not just NF.
This answers “is the preselector, LNA, or following stage worth improving?”
without introducing an optimizer.

Equivalent noise bandwidth is a summary of a **particular transfer path**:
`B_eq = integral G(f) df / G_ref`. Each stage's added noise sees a different
remaining path, so one filter ENBW cannot generally stand in for the whole
chain. Signal occupied bandwidth, integration bandwidth, and ENBW must remain
distinct. For a modulated signal, filtered signal power also needs its spectrum;
a single carrier can simply use the gain at its frequency.

**Implemented follow-up (on this branch, not released):**
[`Block::passive_loss`](../src/block.rs) is a checked temperature-aware passive-loss
constructor. For a matched attenuator with linear loss `L >= 1` in the classical
regime:

```text
T_e = (L - 1) × T_physical
F   = 1 + T_e / 290 K
```

Thus `NF_dB = loss_dB` holds at 290 K, not at every physical temperature. This
constructs an ordinary `Block`; no new field on every block is required.
P1dB/IP3 remain unspecified, not characterized ideal linearity. Invalid inputs
and conservative numerical limits return `PassiveLossError`. See the
[passive-loss guide](spectral-noise.md#temperature-aware-passive-loss) for reference
planes, zero-K limits, and mixed-temperature source/stage budgets.

**Acceptance cases:** Analytic spectral cases are covered by
[public regression workflows](../tests/spectral_noise.rs), and grid refinement is
demonstrated with illustrative tables. The passive-temperature constructor and
mixed-temperature scalar/spectral prefix cases are covered by
[public passive-loss workflows](../tests/passive_loss.rs). Validation against
measured filter data and hardware remains pending.

- A flat, uncompressed lineup reproduces the existing `kTB` result.
- Halving a rectangular integration bandwidth changes white-noise power by
  −3.0103 dB.
- A mathematical one-sided, first-order low-pass reference has
  `B_eq = (pi/2) × f_3dB`, 1.9612 dB more noise than treating `f_3dB` as ENBW.
  This is an analytic low-pass reference, not a claim about a measured SAW or
  the current positive-RF-frequency grid accepting zero.
- Loss `L = 2` at 150 K has NF approximately 1.8105 dB; at 290 K it has
  NF approximately 3.0103 dB.
- A real filter table produces stable integrated noise when the frequency grid
  is refined, and the reported contributions sum in watts.

**Limits.** The implemented evaluator requires linear, matched,
frequency-preserving paths with independent added noise and constant source
temperature. It always uses nominal small-signal gain: P1dB/IP3 metadata have
no effect, and without signal power it cannot validate the hardware operating
point. The caller must establish validity; a stage named Mixer does not imply
frequency conversion. Integrate in linear units, use actual Hz spacing on
irregular/log grids, and do not extrapolate beyond characterization. Finite measurement spans omit filter tails;
report a band-limited integral rather than pretending it is an infinite-band
ENBW. Keep mismatch/network analysis in `touchstone` and quantum-noise extensions
out of the initial feature.

## 2. Make amplifier curves smooth and calibrated

**Literature.** Saleh's TWT models [3] and Rapp's amplifier model [4] are practical
memoryless nonlinear models. The accessible MathWorks reference [5] documents
Saleh, modified Rapp, and measured lookup-table implementations. These are
behavioral model families, not universal semiconductor laws.

**Current gap.** `Block::output_power` clamps at output P1dB + 1 dB.
[`AmplifierModel`](../src/amplifier_model.rs) adds a phase ramp, but its combined
sweep still calls that clamp. `saturation_power_dbm` is stored by constructors
and the builder but is not used in the AM-AM calculation. There is no measured
curve or smooth compression knee.

**Smallest useful feature.** An opt-in response model alongside `AmplifierModel`:

- Rapp AM-AM for a simple smooth saturation curve; require an explicit shape
  parameter rather than inventing one from P1dB.
- A bounded measured `Pin_dBm, Pout_dBm, phase_deg` table for components whose
  behavior does not fit that model. Start with transparent interpolation, not
  an automatic fitting framework.
- Add Saleh only for devices/measurement ranges where its joint AM-AM/AM-PM
  behavior fits. Its amplitude curve can turn over; do not impose it as a
  generic asymptotic saturation law.

For amplitude `a` in square-root watts, amplitude gain `g`, output saturation
amplitude `A_sat`, and positive shape parameter `p`, the Rapp amplitude law is:

```text
A_out(a) = g*a / [1 + (g*a/A_sat)^(2*p)]^(1/(2*p))
```

Explicit amplitude normalization avoids confusing power gain (`10^(G_dB/10)`)
with amplitude gain (`10^(G_dB/20)`). Phase models need their own characterization;
Rapp AM-AM alone does not supply AM-PM or measured IP3.

**Important calibration decision:** for a physical output-referred P1dB,
`Pin_1dB = Pout_1dB - G_small_signal + 1 dB`. A 30 dB amplifier with output
P1dB of +20 dBm reaches that point at −9 dBm input. The current helper uses
`Pout_1dB - G_small_signal` as its phase-ramp reference. Preserve that legacy
behavior while making the new model's definition explicit.

**Acceptance cases before shipping:**

- Recover the specified small-signal gain at deep backoff and the supplied
  output P1dB at its correctly referred input power.
- Changing saturation or shape parameters visibly changes the computed curve.
- Reproduce held-out measured AM-AM/AM-PM points, with error limits chosen from
  measurement uncertainty and the intended engineering use.
- Existing `Block` examples and clamp results remain unchanged.

**Limits.** Gain, P1dB, and IP3 do not uniquely determine a nonlinear waveform
model. Do not claim simultaneous agreement with all three unless verified.
Measured curves need frequency, temperature, bias, load, and reference-plane
metadata. Start with standalone amplifiers: the legacy cascade independently
clamps signal and noise and is not a statistically valid large-signal noise
propagator. A smooth signal curve alone does not fix that assumption. PA memory,
hysteresis, and wideband spectral regrowth remain outside this first feature.

## 3. Upgrade Monte Carlo into defensible uncertainty and yield analysis

**Literature.** JCGM 101:2008 [6] describes propagation of input distributions,
joint distributions for dependent inputs, coverage intervals, and adaptive
Monte Carlo convergence (§§5.1, 6.4.8, 7.2, 7.9). McKay, Beckman, and Conover [7]
provide Latin hypercube sampling as a potential variance-reduction method—not
a guarantee of faster convergence for every RF metric.

**Current gap.** The [existing example](../examples/montecarlo-gain-target/src/main.rs)
already uses seed 42 and 50,000 trials. It samples six independent uniform
variables and reports SNR-margin percentiles and yield. The opportunity is
not “add randomness”; it is represent dependencies, requirement semantics,
and numerical confidence.

**Smallest useful feature.** Extend that example and its existing simulation
runner first, keeping sampling machinery out of the core cascade:

1. Distinguish manufacturing spread, shared environmental conditions, and
   uncertainty in measured specifications. A shared temperature or lot offset
   should affect the relevant stages together.
2. Sample one device realization, then evaluate that same realization across
   the frequency band. Independent redraws at each frequency create artificial
   ripple and the wrong probability of meeting a whole-band requirement.
3. Report named requirement margins and the probability that **all** required
   points pass, alongside per-point results. Do not multiply per-point yields
   as if they were independent.
4. Preserve seed/configuration/model provenance and report convergence of the
   requested quantiles separately from uncertainty in estimated pass rates.
5. Only then compare Latin hypercube with ordinary Monte Carlo across repeated
   runs at equal evaluation counts. Retain it only if the target estimates
   actually become more stable.

**Acceptance cases before shipping:**

- Zero spread reproduces the deterministic lineup at every frequency.
- In an uncompressed gain-only check, two 1 dB-standard-deviation gain errors
  have a summed standard deviation of 1.4142 dB when independent, but 1.8974 dB
  when their correlation is 0.8. This checks a meaningful correlation effect.
- Identical seeds/configurations reproduce results; independent repeat runs
  establish the stability of reported quantiles/yield.
- No failures in 50,000 fixed-count IID trials does **not** prove zero failure
  probability: the one-sided 95% binomial upper bound is about 59.91 ppm.
  This particular bound is not automatically valid for Latin hypercube samples
  or an adaptively stopped experiment.

**Limits.** More trials do not repair invented distributions or an inaccurate RF
model. Coverage intervals of modeled output quantities are not the same thing
as confidence intervals for estimated yield. Datasheet min/typ/max values do
not establish a probability distribution. Correlation assumptions and the
finite frequency grid must be visible in the report. Do not claim compliance
between sampled frequencies without additional bounds or refinement.

## 4. Model image noise and blocker-driven reciprocal mixing

**Literature.** Kerr and Randa [2], “Noise in Mixers” and Figures 3–5, explain why
signal and image sidebands need separate treatment. Leeson [8] is background
for oscillator-noise spectra, not a complete reciprocal-mixing model. The
README already links an Analog Devices reciprocal-mixing application article;
that page was unavailable during this review, so it is not treated as reviewed
evidence here.

**Current gap.** A mixer is currently a scalar `Block`; `SignalNode::cascade_block`
retains the input frequency. The existing LO-distribution example budgets carrier
power, not phase noise. Merely adding an SSB/DSB label or an oscillator NF would
not solve either problem.

**Smallest useful feature, in dependency order:**

1. Implement the README's explicit mixer/frequency-plan stage, preserving
   existing scalar `Block` literals. Track the selected product, image, and
   separate signal/image conversion gains.
2. Propagate independently characterized signal-sideband noise, image-sideband
   noise, and mixer-added noise to the IF. State how any measured mixer NF was
   defined so image noise is not counted twice.
3. Add measured LO single-sideband phase-noise samples versus offset and a
   blocker at the mixer input after preselection. Predict reciprocal noise over
   the actual receive filter band. Prefer measured spectra to estimating an
   oscillator from Leeson parameters that the user does not have.

For one blocker, approximately flat SSB LO noise over a narrow receive band,
and powers referred back to the mixer RF input, the usual engineering estimate
is:

```text
N_reciprocal_dBm ≈ P_blocker_dBm + L_LO_dBc_per_Hz(offset)
                  + 10*log10(B_noise_Hz)
```

A −30 dBm blocker, −140 dBc/Hz LO noise, and 1 MHz noise bandwidth give
approximately −110 dBm input-referred reciprocal noise. This is a derived
reference example under those assumptions, not a hardware measurement. For
shaped spectra or filters, integrate linear spectral power instead.

**Acceptance cases before shipping:**

- The README's 2245 MHz RF / 2145 MHz LO example produces 100 MHz IF and a
  2045 MHz image; the following filter is evaluated at IF, not RF.
- Two equal independent sideband-noise contributions double IF noise power
  relative to one, with conversion gains and mixer-added noise held fixed.
- The flat-spectrum reciprocal-mixing calculation matches the example above;
  10 dB less blocker power gives 10 dB less reciprocal noise in this regime.
- Compare a characterized mixer/LO/filter setup against an independent
  calculation or measurement before treating the feature as production-ready.

**Limits.** Define SSB versus double-sided PSD conventions and reference planes
before implementation; these are common factor-of-two errors. The narrowband
approximation does not describe LO spurs, compression under strong blockers,
image rejection without image-path data, or shared-LO correlations. Build a
selected path, not a general transceiver graph solver.

## 5. Estimate nonlinear distortion with Bussgang decomposition

**Literature.** Demir and Björnson [9], §§III–IV and equation (8), explain the
complex Bussgang decomposition and its signal-to-distortion ratio. The key
lesson is that nonlinear distortion can be **uncorrelated** with the input
without being independent of it.

**Current gap.** `AmplifierModel::evm_from_am_pm` returns `abs(sin(phase))`,
documented as a small-angle approximation. It neither averages over a signal's
amplitude distribution nor accounts for AM-AM. It is not general waveform EVM.
Even for a fixed uncorrected phase rotation, the exact vector error is
`2*abs(sin(phase/2))`; at 180 degrees the legacy approximation returns essentially
zero.
A receiver that removes a constant phase rotation has a different metric again.

**Smallest useful experiment.** After item 2, use an explicit zero-mean circular
complex Gaussian input and a memoryless response `y = U(x)`. Compute:

```text
b              = E[y * conjugate(x)] / E[|x|^2]
P_useful       = |b|^2 * E[|x|^2]
P_distortion   = E[|y|^2] - P_useful
SDR            = P_useful / P_distortion
residual_ratio = sqrt(P_distortion / P_useful)
```

For a radial AM-AM/AM-PM model, these expectations reduce to one-dimensional
amplitude integrals over a Rayleigh distribution. This provides a plausible
fast route to power-backoff versus distortion curves without running an OFDM
modulator or FFT. Runtime and accuracy still need measurement.

Expose `residual_ratio` as a precisely defined gain/phase-compensated,
sample-domain model metric, **not** automatically as standards-compliant EVM.
Gaussian-like many-carrier signals are a potential application; arbitrary
modulation and operating conditions must not be silently assumed Gaussian.

**Acceptance cases before shipping:** a linear complex gain has zero distortion;
a constant phase rotation has zero compensated residual; nonlinear model
integrals agree with an independently sampled complex-Gaussian experiment over
several backoff values and grid refinements. Specify behavior for zero useful
power and cancellation near the linear limit.

**Limits.** These moments do not predict the distortion spectrum, in-band EVM
after channel filtering, or ACLR. Do not simply add distortion powers from
successive nonlinear stages: their distortions need not be independent, and
the next stage's input is no longer Gaussian. Start with one amplifier or a
justified composite memoryless mapping. Memory-polynomial PA models and digital
predistortion should wait for measured wideband I/Q data and a clear owning
package.

## Production gates shared by the proposals

- Preserve the current scalar APIs and documented clamp while introducing
  explicitly selected richer models.
- Keep unknown characterization distinct from ideal behavior. In particular,
  the current OIP3 estimate can restart after an unknown stage; a production
  requirement report must not present that local estimate as a characterized
  full-chain SFDR. Include model-validity and characterized-span information.
- Choose data/reference planes first; then equations; then numerical methods;
  then CLI output. Do not build a general solver before a real acceptance case.
- Validate analytic limits, then a complete characterized chain. Literature
  agreement is not hardware validation, and synthetic fitted data are not
  independent evidence that a device model is accurate.
- For implementation, follow repository gates: format, all-target/all-feature
  Clippy, all-feature tests, rustdoc for public changes, and `git diff --check`.
  Update node-by-node and README-mirrored coverage where repository policy
  requires it; do not introduce a new test framework for this research.

## Sources and evidence depth

1. H. T. Friis, **“Noise Figures of Radio Receivers,”** *Proceedings of the IRE*
   32(7), 419–422 (1944). [DOI](https://doi.org/10.1109/JRPROC.1944.232049).
   Bibliographic metadata verified through Crossref; this review does not claim
   to have inspected the original full text. The crate already implements Friis.
2. A. R. Kerr and J. Randa, **“Thermal Noise and Noise Measurements—A 2010
   Update,”** *IEEE Microwave Magazine* 11(6), 40–52 (2010).
   [DOI](https://doi.org/10.1109/MMM.2010.937732),
   [NIST copy](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=904009).
   Relevant pages visually reviewed: printed pp. 41, 45, 47–49. The PDF is
   image-only; text extraction alone was insufficient.
3. A. A. M. Saleh, **“Frequency-Independent and Frequency-Dependent Nonlinear
   Models of TWT Amplifiers,”** *IEEE Transactions on Communications* 29(11),
   1715–1720 (1981). [DOI](https://doi.org/10.1109/TCOM.1981.1094911).
   Bibliographic metadata verified; model equations cross-checked against [5],
   not an inspected original full text.
4. C. Rapp, **“Effects of HPA-Nonlinearity on a 4-DPSK/OFDM-Signal for a Digital
   Sound Broadcasting System,”** *Second European Conference on Satellite
   Communications*, ESA SP-332, 179–184 (1991).
   [DLR record and abstract](https://elib.dlr.de/33776/).
   Record/abstract reviewed; DLR explicitly does not supply the full text.
5. MathWorks, **“Memoryless Nonlinearity,”** especially “Saleh Model Method”
   and “Modified Rapp Model Method.”
   [Implementation reference](https://www.mathworks.com/help/comm/ref/memorylessnonlinearity.html).
   Relevant equations reviewed. This is secondary implementation documentation,
   not an independent experimental validation or the original Rapp paper.
6. JCGM, **Evaluation of measurement data—Supplement 1 to the GUM—Propagation
   of distributions using a Monte Carlo method**, JCGM 101:2008.
   [Official text](https://www.bipm.org/documents/20126/2071204/JCGM_101_2008_E.pdf).
   Relevant sections reviewed; its uncertainty framework is being adapted here
   to design tolerances, not claimed as a manufacturing-yield certification.
7. M. D. McKay, R. J. Beckman, and W. J. Conover, **“A Comparison of Three
   Methods for Selecting Values of Input Variables in the Analysis of Output
   from a Computer Code,”** *Technometrics* 21(2), 239–245 (1979).
   [DOI](https://doi.org/10.2307/1268522).
   Bibliographic metadata verified; full text not inspected. Latin hypercube is
   a candidate for a controlled comparison, not an established speedup here.
8. D. B. Leeson, **“A Simple Model of Feedback Oscillator Noise Spectrum,”**
   *Proceedings of the IEEE* 54(2), 329–330 (1966).
   [DOI](https://doi.org/10.1109/PROC.1966.4682).
   Bibliographic metadata verified; full text not inspected. Background only;
   it should not be cited as validating the complete proposed receiver model.
9. Ö. T. Demir and E. Björnson, **“The Bussgang Decomposition of Nonlinear
   Systems: Basic Theory and MIMO Extensions,”** *IEEE Signal Processing
   Magazine* 38(1), 131–136 (2021).
   [DOI](https://doi.org/10.1109/MSP.2020.3025538),
   [open author preprint](https://arxiv.org/abs/2005.01597).
   Preprint §§III–IV and the uncorrelated-versus-independent distinction reviewed;
   publication metadata verified separately.

## Reproducing and extending the review

The repository-local [RF literature review skill](../.agents/skills/rf-literature-review/SKILL.md)
records the source-to-proposal procedure. Its dependency-free reference
calculator reproduces the analytic numbers above:

```bash
uv run --no-project .agents/skills/rf-literature-review/scripts/reference_cases.py
```

The calculator does **not** call gainlineup, validate a new implementation, or
establish hardware accuracy. It supplies independent numerical acceptance
references, not evidence that the Rust implementation works. The original
research changed no RF behavior; verification of the new evaluator is recorded
in the [spectral-noise guide](spectral-noise.md).
