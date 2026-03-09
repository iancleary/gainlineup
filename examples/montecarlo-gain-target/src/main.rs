use montycarlo::{MonteCarloEngine, Simulation};
use rand::Rng;
use std::fs::{create_dir_all, File};
use std::io::Write;

struct GainMarginSim { target_gain_db: f64 }

impl Simulation for GainMarginSim {
    type Sample = (f64, f64, f64);
    type Output = f64;

    fn sample(&self, rng: &mut impl Rng) -> Self::Sample {
        (rng.gen_range(17.0..=21.0), rng.gen_range(-2.5..=-0.5), rng.gen_range(10.0..=14.0))
    }

    fn evaluate(&self, s: &Self::Sample) -> Self::Output {
        (s.0 + s.1 + s.2) - self.target_gain_db
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let result = MonteCarloEngine::new(GainMarginSim { target_gain_db: 30.0 }, 50_000)
        .with_seed(42)
        .run();

    create_dir_all("output")?;
    let mut csv = File::create("output/gain_margin_samples.csv")?;
    writeln!(csv, "margin_db")?;
    for v in result.sorted_values() { writeln!(csv, "{v:.6}")?; }

    let mut txt = File::create("output/gain_margin_summary.txt")?;
    writeln!(txt, "trials={}", result.len())?;
    writeln!(txt, "mean_margin_db={:.4}", result.mean())?;
    writeln!(txt, "p05_margin_db={:.4}", result.percentile(5.0))?;
    writeln!(txt, "p50_margin_db={:.4}", result.percentile(50.0))?;
    writeln!(txt, "p95_margin_db={:.4}", result.percentile(95.0))?;
    writeln!(txt, "prob_meeting_target={:.4}", 1.0 - result.cdf(0.0))?;
    Ok(())
}
