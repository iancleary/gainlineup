// the input to our `create_user` handler
#[derive(Debug)]
pub struct GainBlock {
    name: String,
    gain: f64, // dB
    noise_figure: f64, // dB, nf would be ambiguous between noise factor and noise figure
}

fn cascade(input_power: f64, block1: GainBlock) -> f64 {
    input_power + block1.gain
}

#[cfg(test)]
mod tests {
    use crate::cascade::cascade;
    use crate::cascade::GainBlock;

    #[test]
    fn one_part() {
        let input_power: f64 = -30.0;
        let amplifier = GainBlock {
            name: "Simple Amplifier".to_string(),
            gain: 10.0,
            noise_figure: 3.0,
        };
        let output_power = cascade(input_power, amplifier);

        assert_eq!(output_power, -20.0);
    }
}
