use std::fs::File;
use std::io::Write;
use std::path::Path;

use crate::Block;
use crate::SignalNode;
use rfconversions;

pub fn generate_html_table(
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
    writeln!(file, "table {{ border-collapse: collapse; width: 100%; }}")?;
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
    writeln!(file, "<table>")?;
    writeln!(file, "<tr>")?;
    writeln!(file, "<th>Stage</th>")?;
    writeln!(file, "<th>Name</th>")?;
    writeln!(file, "<th>Input Power (dBm)</th>")?;
    writeln!(file, "<th>Gain (dB)</th>")?;
    writeln!(file, "<th>NF (dB)</th>")?;
    writeln!(file, "<th>Cumulative Gain (dB)</th>")?;
    writeln!(file, "<th>Cumulative NF (dB)</th>")?;
    writeln!(file, "<th>Output Power (dBm)</th>")?;
    writeln!(file, "</tr>")?;

    for (i, node) in cascade.iter().enumerate() {
        writeln!(file, "<tr>")?;
        writeln!(file, "<td>{}</td>", i)?;
        writeln!(file, "<td>{}</td>", node.name)?;

        if i == 0 {
            writeln!(file, "<td>{:.2}</td>", node.power)?; // Input Power for first node is its power
            writeln!(file, "<td>-</td>")?;
            writeln!(file, "<td>-</td>")?;
            writeln!(file, "<td>-</td>")?;
            writeln!(file, "<td>-</td>")?;
            writeln!(file, "<td>{:.2}</td>", node.power)?; // Output Power is same as Input for source
        } else {
            let block = &blocks[i - 1];
            let _input_power = node.power - block.gain; // Approximation if compression happened, but strictly: P_out - Gain isn't always P_in if compressed.
                                                        // Better: cascade[i-1].power is the input power to this stage.
            let actual_input_power = cascade[i - 1].power;

            writeln!(file, "<td>{:.2}</td>", actual_input_power)?;
            writeln!(file, "<td>{:.2}</td>", block.gain)?;
            writeln!(file, "<td>{:.2}</td>", block.noise_figure)?;
            writeln!(file, "<td>{:.2}</td>", node.cumulative_gain)?;
            writeln!(
                file,
                "<td>{:.2}</td>",
                rfconversions::noise::noise_figure_from_noise_temperature(node.noise_temperature)
            )?;
            writeln!(file, "<td>{:.2}</td>", node.power)?;
        }
        writeln!(file, "</tr>")?;
    }

    writeln!(file, "</table>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;

    Ok(())
}
