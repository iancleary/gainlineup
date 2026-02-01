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
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(output_p1db) = self.output_p1db_dbm {
            write!(
                f,
                "Block {{ name: {}, gain: {} dB, noise_figure: {} dB, output_p1db: {} dBm }}",
                self.name, self.gain_db, self.noise_figure_db, output_p1db
            )
        } else {
            write!(
                f,
                "Block {{ name: {}, gain: {} dB, noise_figure: {} dB }}",
                self.name, self.gain_db, self.noise_figure_db
            )
        }
    }
}

impl Default for Block {
    fn default() -> Self {
        Self {
            name: String::from("default"),
            gain_db: 0.0,
            noise_figure_db: 0.0,
            output_p1db_dbm: None,
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

    // input_noise_power * power_gain = output_noise_power
    pub fn output_noise_power(&self, bandwidth: f64, input_power: f64) -> f64 {
        #[cfg(feature = "debug-print")]
        println!("START BLOCK output_noise_power");

        let input_noise_power = self.input_noise_power(bandwidth);

        #[cfg(feature = "debug-print")]
        #[cfg(feature = "debug-print")]
        println!(
            "Input Noise Power (block.input_noise_power): (dBm) {}",
            input_noise_power
        );

        let power_gain = self.power_gain(input_power);

        #[cfg(feature = "debug-print")]
        println!("Power Gain: (dB) {}", power_gain);

        let output_noise_power = input_noise_power + power_gain;

        #[cfg(feature = "debug-print")]
        println!("Output Noise Power: (dBm) {}", output_noise_power);

        #[cfg(feature = "debug-print")]
        println!("END BLOCK output_noise_power");

        output_noise_power
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
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn output_power_with_compression() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain_db: 10.0,
            noise_figure_db: 3.0,
            output_p1db_dbm: Some(-20.0),
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
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -19.0);

        let power_gain = amplifier.power_gain(input_power);
        assert_eq!(power_gain, 6.0);
    }
}
