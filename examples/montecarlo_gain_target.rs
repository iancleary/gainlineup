use montycarlo::{MonteCarloEngine, Simulation};
use rand::Rng;
use std::fs::{create_dir_all, File};
use std::io::Write;

struct GainMarginSim {
    target_gain_db: f64,
}

impl Simulation for GainMarginSim {
    type Sample = (f64, f64, f64);
    type Output = f64;

    fn sample(&self, rng: &mut impl Rng) -> Self::Sample {
        let g1 = rng.gen_range(17.0..=21.0);
        let g2 = rng.gen_range(-2.5..=-0.5);
        let g3 = rng.gen_range(10.0..=14.0);
        (g1, g2, g3)
    }

    fn evaluate(&self, sample: &Self::Sample) -> Self::Output {
        let total_gain = sample.0 + sample.1 + sample.2;
        total_gain - self.target_gain_db
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let trials = 50_000;
    let target_gain_db = 30.0;
    let sim = GainMarginSim { target_gain_db };
    let result = MonteCarloEngine::new(sim, trials).with_seed(42).run();

    create_dir_all("examples/output")?;
    let mut csv = File::create("examples/output/gain_margin_samples.csv")?;
    writeln!(csv, "margin_db")?;
    for v in result.sorted_values() {
        writeln!(csv, "{v:.6}")?;
    }

    let mut txt = File::create("examples/output/gain_margin_summary.txt")?;
    writeln!(txt, "trials={}", result.len())?;
    writeln!(txt, "target_gain_db={target_gain_db}")?;
    writeln!(txt, "mean_margin_db={:.4}", result.mean())?;
    writeln!(txt, "p05_margin_db={:.4}", result.percentile(5.0))?;
    writeln!(txt, "p50_margin_db={:.4}", result.percentile(50.0))?;
    writeln!(txt, "p95_margin_db={:.4}", result.percentile(95.0))?;
    writeln!(txt, "prob_meeting_target={:.4}", 1.0 - result.cdf(0.0))?;

    println!("Wrote examples/output/gain_margin_samples.csv");
    println!("Wrote examples/output/gain_margin_summary.txt");
    Ok(())
}
