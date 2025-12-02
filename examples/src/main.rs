extern crate gainlineup;
use rfconversions;

fn main() {
    println!("Hello, World!");

    let input_power: f64 = -30.0;

    println!("Input Power: {} dBm", input_power);
    let input_node = gainlineup::SignalNode {
        name: "Input".to_string(),
        power: input_power,
        noise_temperature: 290.0, // 290 K is room temperature
        cumulative_gain: 0.0,     // starting/initial/input node of cascade
    };

    println!("Input Node: {:#?}", input_node);
    let amplifier = gainlineup::Block {
        name: "Low Noise Amplifier".to_string(),
        gain: 30.0,        // dB
        noise_figure: 3.0, // dB
    };

    println!("Amplifier: {:#?}", amplifier);
    let output_node = gainlineup::cascade_node(input_node, amplifier);

    assert_eq!(output_node.power, 0.0);
    assert_eq!(output_node.name, "Low Noise Amplifier Output");
    // assert_eq!(output_node.noise_temperature, rfconversions::noise::noise_temperature_from_noise_figure(3.0));
    let output_noise_figure =
        rfconversions::noise::noise_figure_from_noise_temperature(output_node.noise_temperature);
    assert_eq!(output_noise_figure, 3.0124584457866126);

    println!("Output Node: {:#?}", output_node);
    println!("Output Power: {} dBm", output_node.power);
    println!("Output Noise Figure: {} dB", output_noise_figure);
}
