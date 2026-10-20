"""Independent analytic references for docs/literature-opportunities.md.

Run with uv run --no-project. No dependencies or gainlineup imports are needed.
These are illustrative reference calculations, not tests of the Rust crate.
"""

import math


def main() -> None:
    """Print the research shortlist's analytic acceptance references."""
    print("Spectral noise (classical, matched, small-signal)")
    print(f"  Half rectangular bandwidth: {10 * math.log10(0.5):.6f} dB")
    enbw_ratio = math.pi / 2
    print(f"  One-sided one-pole ENBW / f_3dB: {enbw_ratio:.6f}")
    print(
        "  One-pole noise above rectangular f_3dB: "
        f"{10 * math.log10(enbw_ratio):.6f} dB"
    )
    loss_linear = 2.0
    for temperature_k in (150.0, 290.0):
        noise_factor = 1 + (loss_linear - 1) * temperature_k / 290.0
        noise_figure_db = 10 * math.log10(noise_factor)
        print(f"  L=2 at {temperature_k:.0f} K: NF={noise_figure_db:.6f} dB")

    print("\nRapp calibration (amplitudes in square-root watts)")
    gain_db = 30.0
    output_p1db_dbm = 20.0
    shape = 2.0
    input_p1db_dbm = output_p1db_dbm - gain_db + 1.0
    amplitude_gain = 10 ** (gain_db / 20)
    input_amplitude = math.sqrt(10 ** ((input_p1db_dbm - 30) / 10))
    # Solve the Rapp law for 1 dB compression at this input amplitude.
    saturation_amplitude = (
        amplitude_gain
        * input_amplitude
        / (10 ** (shape / 10) - 1) ** (1 / (2 * shape))
    )
    output_amplitude = amplitude_gain * input_amplitude / (
        1 + (amplitude_gain * input_amplitude / saturation_amplitude) ** (2 * shape)
    ) ** (1 / (2 * shape))
    output_dbm = 10 * math.log10(output_amplitude**2) + 30
    print(f"  True input P1dB: {input_p1db_dbm:.6f} dBm")
    print(f"  Output at that input: {output_dbm:.6f} dBm")

    print("\nUncertainty references")
    # Var(X + Y) = Var(X) + Var(Y) + 2 Cov(X, Y); both sigmas are 1 dB.
    for correlation in (0.0, 0.8):
        sigma_db = math.sqrt(2 + 2 * correlation)
        print(
            f"  Two 1 dB sigma gains, rho={correlation}: "
            f"summed sigma={sigma_db:.6f} dB"
        )
    # Fixed-count independent Bernoulli trials: (1 - p_upper)^N = alpha.
    failure_upper = -math.expm1(math.log(0.05) / 50_000)
    print(
        "  Zero failures / 50000 IID trials: "
        f"95% upper bound={failure_upper * 1e6:.6f} ppm"
    )

    print("\nOne-blocker, flat-SSB-noise reciprocal mixing")
    blocker_dbm = -30.0
    lo_noise_dbc_per_hz = -140.0
    bandwidth_hz = 1e6
    noise_dbm = blocker_dbm + lo_noise_dbc_per_hz + 10 * math.log10(bandwidth_hz)
    print(f"  Mixer-input-referred reciprocal noise: {noise_dbm:.6f} dBm")

    print("\nFixed phase rotation (no amplitude error)")
    phase_rad = math.pi
    raw_evm = 2 * abs(math.sin(phase_rad / 2))
    print(f"  Uncompensated vector EVM at 180 degrees: {raw_evm:.6f} ratio")
    print("  After exact constant-phase compensation: 0 ratio")


if __name__ == "__main__":
    main()
