//! README example: 1. Define Your Input Signal

use gainlineup::Input;

#[test]
fn input_signal_construction() {
    let input = Input {
        power_dbm: -80.0,                // received signal level
        frequency_hz: 6.0e9,             // 6 GHz C-band
        bandwidth_hz: 1.0e6,             // 1 MHz channel
        noise_temperature_k: Some(50.0), // cool sky
    };

    assert_eq!(input.power_dbm, -80.0);
    assert_eq!(input.frequency_hz, 6.0e9);
    assert_eq!(input.bandwidth_hz, 1.0e6);
    assert_eq!(input.noise_temperature_k, Some(50.0));
}
