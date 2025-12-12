use std::default::Default;
use std::fmt;

use crate::block::Block;
use crate::node::SignalNode;

// the input is the signal that enters the cascade, which is different than the nodes
// that are the outputs of each block, see block.rs for more details
#[derive(Clone, Debug)]
pub struct Input {
    pub frequency: f64, // Hz, center frequency of signal
    pub bandwidth: f64, // Hz, width of signal
    pub power: f64,     // dBm, power of signal
}

impl fmt::Display for Input {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Input {{ frequency: {}, bandwidth: {}, power: {} }}",
            self.frequency, self.bandwidth, self.power
        )
    }
}

impl Default for Input {
    fn default() -> Self {
        Self {
            frequency: 0.0,   // placeholder value, you should change this (0 Hz)
            bandwidth: 100.0, // placeholder value, you should change this (100 Hz)
            // https://www.w8ji.com/cw_bandwidth_described.htm describes how CW signals are generally made, which require non-zero bandwidth
            power: 0.0, // placeholder value, you should change this (0 dBm)
        }
    }
}

impl Input {
    pub fn new(frequency: f64, bandwidth: f64, power: f64) -> Input {
        Input {
            frequency,
            bandwidth,
            power,
        }
    }

    pub fn cascade_block(&self, block: &Block) -> SignalNode {
        let output_node_name = block.name.clone() + " Output";

        let block_noise_factor =
            rfconversions::noise::noise_factor_from_noise_figure(block.noise_figure);

        // handle compression point
        let output_power_without_compression = self.power + block.gain;
        let output_power = if let Some(output_p1db) = block.output_p1db {
            if output_power_without_compression > output_p1db + 1.0 {
                output_p1db + 1.0
            } else {
                output_power_without_compression
            }
        } else {
            output_power_without_compression
        };

        let stage_gain = output_power - self.power;

        let cumulative_noise_factor = block_noise_factor;

        let cumulative_noise_figure =
            rfconversions::noise::noise_figure_from_noise_factor(cumulative_noise_factor);

        SignalNode {
            name: output_node_name,
            power: output_power,
            frequency: self.frequency,
            bandwidth: self.bandwidth,
            noise_figure: cumulative_noise_figure,
            cumulative_gain: stage_gain,
        }
    }
}
