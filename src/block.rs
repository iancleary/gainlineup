use std::default::Default;
use std::fmt;

use crate::constants;

// the definition of a block in the cascade
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub gain_db: f64,                 // dB
    pub noise_figure_db: f64, // dB, nf would be ambiguous between noise factor and noise figure
    pub output_p1db_dbm: Option<f64>, // dBm, output 1 dB compression point
    pub output_ip3_dbm: Option<f64>,  // dBm, output third-order intercept point
    pub isolation_db: Option<f64>,    // dB, out-of-band rejection/isolation
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Block {{ name: {}, gain: {} dB, noise_figure: {} dB",
            self.name, self.gain_db, self.noise_figure_db
        )?;
        if let Some(output_p1db) = self.output_p1db_dbm {
            write!(f, ", output_p1db: {} dBm", output_p1db)?;
        }
        if let Some(output_ip3) = self.output_ip3_dbm {
            write!(f, ", output_ip3: {} dBm", output_ip3)?;
        }
        if let Some(isolation) = self.isolation_db {
            write!(f, ", isolation: {} dB", isolation)?;
        }
        write!(f, " }}")
    }
}

impl Default for Block {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            gain_db: 0.0,
            noise_figure_db: 0.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
            isolation_db: None,
        }
    }
}
impl Block {
    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.noise_figure_db)
    }

    pub fn noise_factor(&self) -> f64 {
        rfconversions::noise::noise_factor_from_noise_figure(self.noise_figure_db)
    }

    // input noise power (F-1)*kTB
    pub fn input_noise_power(&self, bandwidth: f64) -> f64 {
        let noise_factor = self.noise_factor();
        let noise_temperature = self.noise_temperature();

        let f_minus_1 = noise_factor - 1.0;

        let ktb = constants::BOLTZMANN * noise_temperature * bandwidth;

        rfconversions::power::watts_to_dbm(f_minus_1 * ktb)
    }

    // input_noise_power + power_gain (for noise level not signal level) = output_noise_power
    pub fn output_noise_power(&self, bandwidth: f64) -> f64 {
        #[cfg(feature = "debug-print")]
        println!("START BLOCK output_noise_power");

        let input_noise_power = self.input_noise_power(bandwidth);

        #[cfg(feature = "debug-print")]
        println!(
            "Input Noise Power (block.input_noise_power): (dBm) {}",
            input_noise_power
        );

        let output_noise_power_without_compression = input_noise_power + self.gain_db;

        #[cfg(feature = "debug-print")]
        println!(
            "Output Noise Power without compression: (dBm) {}",
            output_noise_power_without_compression
        );

        let output_noise_power_dbm = if let Some(output_p1db_dbm) = self.output_p1db_dbm {
            if output_noise_power_without_compression > output_p1db_dbm + 1.0 {
                output_p1db_dbm + 1.0
            } else {
                output_noise_power_without_compression
            }
        } else {
            output_noise_power_without_compression
        };

        #[cfg(feature = "debug-print")]
        let noise_power_gain = output_noise_power_dbm - input_noise_power;
        #[cfg(feature = "debug-print")]
        println!("Noise Power Gain: (dB) {}", noise_power_gain);

        #[cfg(feature = "debug-print")]
        println!("Output Noise Power: (dBm) {}", output_noise_power_dbm);

        #[cfg(feature = "debug-print")]
        println!("END BLOCK output_noise_power");

        output_noise_power_dbm
    }

    pub fn output_power(&self, input_power: f64) -> f64 {
        // this is a simple calculation, which could be upgrade to
        // use the compression curve of a block, if present,
        // or the compression parameters from a piecewise amplifier model
        // where there is a linear region, a compression region, and a saturation region
        // those regions are defined by the compression point, a curve fit within te region of compression, and the saturation point
        // the curve fit is a simple linear fit between the compression point and the saturation point
        // the compression point is the point where the compression begins
        // the saturation point is the point where the compression ends
        // this probably would be a polynomial fit, but for initial documentation of the idea,
        // it's a simple linear fit
        let output_power_without_compression = input_power + self.gain_db;
        if let Some(op1db) = self.output_p1db_dbm {
            if output_power_without_compression > op1db + 1.0 {
                return op1db + 1.0;
            }
        }
        output_power_without_compression
    }

    pub fn power_gain(&self, input_power: f64) -> f64 {
        self.output_power(input_power) - input_power
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let block = Block::default();
        assert_eq!(block.gain_db, 0.0);
        assert_eq!(block.noise_figure_db, 0.0);
        assert_eq!(block.output_p1db_dbm, None);
        assert_eq!(block.noise_temperature(), 0.0);
    }

    #[test]
    fn output_power() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
            isolation_db: None,
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn output_power_with_compression_below_threshold() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-20.0),
            output_ip3_dbm: None,
            isolation_db: None,
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -20.0);

        let power_gain = amplifier.power_gain(input_power);
        assert_eq!(power_gain, 10.0);
    }

    #[test]
    fn output_power_with_compression_above_threshold() {
        let input_power: f64 = -25.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-20.0),
            output_ip3_dbm: None,
            isolation_db: None,
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -19.0);

        let power_gain = amplifier.power_gain(input_power);
        assert_eq!(power_gain, 6.0);
    }

    #[test]
    fn output_noise_power_without_compression() {
        let bandwidth: f64 = 1.0e6;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: None,
            output_ip3_dbm: None,
            isolation_db: None,
        };
        let output_noise_power = amplifier.output_noise_power(bandwidth);

        // With 1 MHz bandwidth, 3 dB NF (290K), thermal noise ~= -114 dBm
        // After 10 dB gain: -114 + 10 = -104 dBm
        assert!(
            (output_noise_power - (-104.02)).abs() < 0.01,
            "Expected output noise power around -104.02 dBm, got {}",
            output_noise_power
        );
    }

    #[test]
    fn output_noise_power_with_compression_below_threshold() {
        let bandwidth: f64 = 1.0e6;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-20.0), // P1dB well above noise floor
            output_ip3_dbm: None,
            isolation_db: None,
        };
        let output_noise_power = amplifier.output_noise_power(bandwidth);

        // Noise is -104 dBm, well below P1dB of -20 dBm, so no compression
        assert!(
            (output_noise_power - (-104.02)).abs() < 0.01,
            "Noise should not compress when well below P1dB. Expected -104.02 dBm, got {}",
            output_noise_power
        );
    }

    #[test]
    fn output_noise_power_with_compression_above_threshold() {
        // To test noise compression, we need noise that actually exceeds P1dB
        // Use very high bandwidth to increase noise power
        let bandwidth: f64 = 1.0e9; // Very high bandwidth
        let amplifier = super::Block {
            name: "High Noise Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-80.0), // P1dB that noise will exceed
            output_ip3_dbm: None,
            isolation_db: None,
        };
        let output_noise_power = amplifier.output_noise_power(bandwidth);

        // With very high bandwidth, noise exceeds P1dB, should compress to P1dB + 1 dB
        assert_eq!(
            output_noise_power, -79.0,
            "Noise should compress to P1dB + 1 dB when above threshold"
        );
    }
}
