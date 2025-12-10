
// the definition of a block in the cascade
#[derive(Clone, Debug)]
pub struct Block {
    pub name: String,
    pub gain: f64,                                 // dB
    pub noise_figure: f64, // dB, nf would be ambiguous between noise factor and noise figure
    pub output_p1db: Option<f64>, // dBm, compression point
}

impl Block {
    pub fn default() -> Block {
        Block {
            name: String::from("default"),
            gain: 0.0,
            noise_figure: 0.0,
            output_p1db: None,
        }
    }

    pub fn noise_temperature(&self) -> f64 {
        rfconversions::noise::noise_temperature_from_noise_figure(self.noise_figure)
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
        let output_power_without_compression = input_power + self.gain;
        if let Some(op1db) = self.output_p1db {
            if output_power_without_compression > op1db + 1.0 {
                return op1db + 1.0;
            }
        }
        output_power_without_compression
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default() {
        let block = Block::default();
        assert_eq!(block.gain, 0.0);
        assert_eq!(block.noise_figure, 0.0);
        assert_eq!(block.output_p1db, None);
        assert_eq!(block.noise_temperature(), 0.0);
    }

    #[test]
    fn output_power() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_p1db: None,
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn output_power_with_compression() {
        let input_power: f64 = -30.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_p1db: Some(-20.0),
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -20.0);
    }

    #[test]
    fn output_power_with_compression_above_threshold() {
        let input_power: f64 = -25.0;
        let amplifier = super::Block {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
            output_p1db: Some(-20.0),
        };
        let output_power = amplifier.output_power(input_power);

        assert_eq!(output_power, -19.0);
    }
}
