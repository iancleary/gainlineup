---
name: rf-literature-review
description: Evaluate published RF modeling methods against gainlineup's actual capabilities and produce a sourced, bounded productionization proposal. Use for literature discovery or refreshing the modeling shortlist, not automatic feature implementation.
---

# RF literature to production proposal

## Prerequisites and boundaries

- Work in the gainlineup checkout; read its repository instructions and check
  Git status before editing. Preserve existing work.
- Read-only access to publication metadata and available papers is sufficient.
  Do not install dependencies, purchase papers, publish, or delegate without
  the applicable authorization.
- Research does not authorize changing RF behavior. Separate proposed methods
  from implemented and experimentally validated functionality.

## Working sequence

1. Read `src/lib.rs` and the relevant README section, then the owning module
   and public example. Inventory capabilities before suggesting additions.
   Frequency sweeps and a seeded Monte Carlo example already exist.
2. For each candidate, record a concrete user question, the current model gap,
   and the minimum additional characterization data needed.
3. Verify title, authors, venue, year, and DOI through a publisher, institutional
   repository, or Crossref. Use the original paper or an open author copy when
   available. Mark metadata-only, abstract-only, and secondary evidence honestly.
4. Inspect the relevant equations and assumptions, not just the abstract.
   For PDFs, use the installed `read-pdf` skill if available; image-only PDFs
   require visual inspection or an authorized OCR tool. Extraction success
   does not imply that text or equations were actually readable.
5. Map the method to existing ownership: simple stages in `Block`, richer PA
   behavior beside `AmplifierModel`, ordered frequency/system paths in a
   companion layer. Leave full network mismatch to `touchstone` and full link
   performance to `linkbudget`.
6. Specify units, reference planes, PSD conventions, validity regime, missing
   data behavior, analytic limits, and an independent real-system acceptance
   case. Check input/output P1dB references and power versus amplitude gain.
7. Rank by user value, data availability, integration cost, and validation cost.
   Identify prerequisites and explicitly defer features needing unavailable
   measurements. Do not claim numerical speedups without measurements.
8. When `docs/literature-opportunities.md` exists, extend it rather than starting
   another roadmap; keep its README link current when publishing that document.
   Otherwise, begin with the tracked README and `docs/performance.md`, and add
   research documentation only for a substantive review. The skill and bundled
   calculator work without a companion research page. Capture only reviewed
   evidence and reusable conclusions, not downloaded papers or raw research logs.

## Reference calculations and verification

Resolve `scripts/reference_cases.py` relative to this skill directory. Run it
with `uv run --no-project`; it uses only the Python standard library. It
reproduces analytic reference numbers, not gainlineup behavior. Update it when
changing the associated examples in the research document.

Check local Markdown links, inspect the complete diff, and run `git diff --check`.
For research-only changes, state that no RF implementation was tested. For any
separately authorized implementation, use the repository's required Rust gates
and relevant end-to-end acceptance cases; the reference calculator is not a
substitute.

For proposals about integrated noise, first check the implemented baseline in
`docs/spectral-noise.md` and `src/noise.rs`. Run `cargo run --quiet --example
spectral_noise` to inspect source/stage attribution and grid refinement, and
`cargo test --test spectral_noise --test readme_13_spectral_noise` for the
analytic and public-example checks. Keep quadrature convergence, characterization
density, and measured hardware validation separate; the demo uses illustrative
tables. For device-temperature proposals, check `Block::passive_loss` and run
`cargo test --test passive_loss --test readme_14_passive_loss` to verify the
matched-loss scalar/spectral budgets. Distinguish source, device, and 290 K NF
reference temperatures. These baseline checks do not authorize model changes.

## Common failures and recovery

- **Unavailable paper or blocked publisher:** use an authorized institutional
  copy, or mark the evidence incomplete. Never present a remembered equation
  as newly verified full-text evidence.
- **Source describes a different regime:** narrow the proposal; do not silently
  apply small-signal noise math in compression or infer wideband EVM from CW IP3.
- **Numbers differ by 3 dB or 1 dB:** inspect sideband/PSD and compressed versus
  extrapolated reference conventions before altering formulas.
- **Monte Carlo looks precise:** distinguish sampling error from input/model
  uncertainty; preserve joint conditions across frequencies and stages.
- **Scope grows into a simulator:** retain one useful characterized path/model
  and defer the general graph, optimizer, or waveform engine.

Rollback boundaries are the review's own uncommitted documentation and bundled
helper changes. Do not reset unrelated work or modify published API behavior.
Research alone does not authorize commits, pushes, or releases; obtain separate
authorization for publication. Use the repository's README for current behavior
and `docs/literature-opportunities.md` for the source list when that page exists.
