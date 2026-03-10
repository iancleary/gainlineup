use gainlineup::{cascade_vector_return_output, Block, Input};
use montycarlo::{MonteCarloEngine, Simulation};
use rand::Rng;
use std::fs::{create_dir_all, File};
use std::io::Write;

/// Monte Carlo analysis of output SNR margin vs design target in a cascade lineup.
///
/// The first two blocks have randomized gain and noise figure to emulate part-to-part
/// variation and operating condition spread.
struct CascadeSnrMarginSim {
    target_snr_db: f64,
}

impl Simulation for CascadeSnrMarginSim {
    // (b1_gain, b1_nf, b2_gain, b2_nf, input_power_dbm, input_temp_k)
    type Sample = (f64, f64, f64, f64, f64, f64);
    // Output SNR margin relative to target.
    type Output = f64;

    fn sample(&self, rng: &mut impl Rng) -> Self::Sample {
        let b1_gain_db = rng.gen_range(16.0..=21.0); // LNA-ish stage
        let b1_nf_db = rng.gen_range(1.0..=2.2);

        let b2_gain_db = rng.gen_range(7.0..=12.0); // IF/VGA-ish stage
        let b2_nf_db = rng.gen_range(2.0..=4.5);

        let input_power_dbm = rng.gen_range(-95.0..=-88.0);
        let input_temp_k = rng.gen_range(265.0..=330.0);

        (
            b1_gain_db,
            b1_nf_db,
            b2_gain_db,
            b2_nf_db,
            input_power_dbm,
            input_temp_k,
        )
    }

    fn evaluate(&self, s: &Self::Sample) -> Self::Output {
        let input = Input::new(12.0e9, 10.0e6, s.4, Some(s.5));

        let blocks = vec![
            Block {
                name: "LNA".to_string(),
                gain_db: s.0,
                noise_figure_db: s.1,
                output_p1db_dbm: None,
                output_ip3_dbm: None,
            },
            Block {
                name: "IF Amp".to_string(),
                gain_db: s.2,
                noise_figure_db: s.3,
                output_p1db_dbm: None,
                output_ip3_dbm: None,
            },
            Block {
                name: "Filter".to_string(),
                gain_db: -2.0,
                noise_figure_db: 2.0,
                output_p1db_dbm: None,
                output_ip3_dbm: None,
            },
        ];

        let out = cascade_vector_return_output(input, blocks);
        let output_snr_db = out.signal_to_noise_ratio_db();
        output_snr_db - self.target_snr_db
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let trials = 50_000;
    let target_snr_db = 12.0;

    let sim = CascadeSnrMarginSim { target_snr_db };
    let result = MonteCarloEngine::new(sim, trials).with_seed(42).run();

    create_dir_all("output")?;

    let mut csv = File::create("output/cascade_snr_margin_samples.csv")?;
    writeln!(csv, "snr_margin_db")?;
    for v in result.sorted_values() {
        writeln!(csv, "{v:.6}")?;
    }

    let mut txt = File::create("output/cascade_snr_margin_summary.txt")?;
    writeln!(txt, "trials={}", result.len())?;
    writeln!(txt, "target_snr_db={target_snr_db}")?;
    writeln!(txt, "mean_margin_db={:.4}", result.mean())?;
    writeln!(txt, "p05_margin_db={:.4}", result.percentile(5.0))?;
    writeln!(txt, "p50_margin_db={:.4}", result.percentile(50.0))?;
    writeln!(txt, "p95_margin_db={:.4}", result.percentile(95.0))?;
    writeln!(txt, "prob_meeting_target={:.4}", 1.0 - result.cdf(0.0))?;

    println!("Wrote output/cascade_snr_margin_samples.csv");
    println!("Wrote output/cascade_snr_margin_summary.txt");
    Ok(())
}
