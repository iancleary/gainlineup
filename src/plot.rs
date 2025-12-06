use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::Block;
use crate::SignalNode;
use rfconversions;

pub fn generate_html_table(
    input_power: f64,
    frequency: f64,
    input_noise_temperature: Option<f64>,
    cascade: &Vec<SignalNode>,
    blocks: &Vec<Block>,
    output_path_str: &str,
) -> Result<(), std::io::Error> {
    let path = Path::new(output_path_str);
    let mut file = File::create(path)?;

    writeln!(file, "<!DOCTYPE html>")?;
    writeln!(file, "<html>")?;
    writeln!(file, "<head>")?;
    writeln!(file, "<title>Gain Lineup Cascade</title>")?;
    writeln!(file, "<style>")?;
    writeln!(file, "table {{ border-collapse: collapse; }}")?;
    writeln!(file, ".cascade {{ width: 100%; }}")?;
    writeln!(file, ".parameters {{ width: auto; }}")?;
    writeln!(file, ".parameters td:nth-child(2) {{ text-align: right; }}")?;
    writeln!(
        file,
        "th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}"
    )?;
    writeln!(file, "th {{ background-color: #f2f2f2; }}")?;
    writeln!(file, "tr:nth-child(even) {{ background-color: #f9f9f9; }}")?;
    writeln!(file, "</style>")?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;
    writeln!(file, "<h1>Gain Lineup Cascade</h1>")?;

    writeln!(file, "<h2>Input Parameters</h2>")?;
    writeln!(file, "<table class=\"parameters\">")?;
    writeln!(file, "<tr>")?;
    writeln!(file, "<th>Parameter</th>")?;
    writeln!(file, "<th>Value</th>")?;
    writeln!(file, "<th>Unit</th>")?;
    writeln!(file, "</tr>")?;
    writeln!(file, "<tr>")?;
    writeln!(file, "<td>Input Power</td>")?;
    writeln!(file, "<td>{:.2}</td>", input_power)?;
    writeln!(file, "<td>dBm</td>")?;
    writeln!(file, "</tr>")?;
    writeln!(file, "<tr>")?;
    writeln!(file, "<td>Frequency</td>")?;
    let (freq_val, freq_unit) = if frequency >= 1e12 {
        (frequency / 1e12, "THz")
    } else if frequency >= 1e9 {
        (frequency / 1e9, "GHz")
    } else if frequency >= 1e6 {
        (frequency / 1e6, "MHz")
    } else if frequency >= 1e3 {
        (frequency / 1e3, "kHz")
    } else {
        (frequency, "Hz")
    };
    writeln!(file, "<td>{:.2}</td>", freq_val)?;
    writeln!(file, "<td>{}</td>", freq_unit)?;
    writeln!(file, "</tr>")?;
    writeln!(file, "<tr>")?;
    let noise_temp_label = if input_noise_temperature.is_some() {
        "Input Noise Temperature"
    } else {
        "Input Noise Temperature (Default)"
    };
    writeln!(file, "<td>{}</td>", noise_temp_label)?;
    writeln!(
        file,
        "<td>{:.2}</td>",
        input_noise_temperature.unwrap_or(290.0)
    )?;
    writeln!(file, "<td>K</td>")?;
    writeln!(file, "</tr>")?;
    writeln!(file, "</table>")?;
    writeln!(file, "<br>")?;

    writeln!(file, "<h2>Signal Cascade</h2>")?;
    writeln!(file, "<table class=\"cascade\">")?;
    writeln!(file, "<tr>")?;
    writeln!(file, "<th>Stage</th>")?;
    writeln!(file, "<th>Name</th>")?;
    writeln!(file, "<th>Gain (dB)</th>")?;
    writeln!(file, "<th>NF (dB)</th>")?;
    writeln!(file, "<th>Input Power (dBm)</th>")?;
    writeln!(file, "<th>Output Power (dBm)</th>")?;
    writeln!(file, "<th>Output P1dB (dBm)</th>")?;
    writeln!(file, "<th>Cumulative Gain (dB)</th>")?;
    writeln!(file, "<th>Cumulative NF (dB)</th>")?;
    writeln!(file, "</tr>")?;

    for (i, node) in cascade.iter().enumerate() {
        writeln!(file, "<tr>")?;
        writeln!(file, "<td>{}</td>", i)?;
        writeln!(file, "<td>{}</td>", node.name)?;

        if i == 0 {
            writeln!(file, "<td>-</td>")?;
            writeln!(file, "<td>-</td>")?;
            writeln!(file, "<td>{:.2}</td>", node.power)?; // Input Power
            writeln!(file, "<td>{:.2}</td>", node.power)?; // Output Power
            writeln!(file, "<td>-</td>")?; // P1dB
            writeln!(file, "<td>-</td>")?; // Cumulative Gain
            writeln!(file, "<td>-</td>")?; // Cumulative NF
        } else {
            let block = &blocks[i - 1];
            let actual_input_power = cascade[i - 1].power;

            writeln!(file, "<td>{:.2}</td>", block.gain)?;
            writeln!(file, "<td>{:.2}</td>", block.noise_figure)?;
            writeln!(file, "<td>{:.2}</td>", actual_input_power)?;
            writeln!(file, "<td>{:.2}</td>", node.power)?;
            if let Some(p1db) = block.output_1db_compression_point {
                writeln!(file, "<td>{:.2}</td>", p1db)?;
            } else {
                writeln!(file, "<td>-</td>")?;
            }
            writeln!(file, "<td>{:.2}</td>", node.cumulative_gain)?;
            writeln!(
                file,
                "<td>{:.2}</td>",
                rfconversions::noise::noise_figure_from_noise_temperature(node.noise_temperature)
            )?;
        }
        writeln!(file, "</tr>")?;
    }

    writeln!(file, "</table>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;

    Ok(())
}
