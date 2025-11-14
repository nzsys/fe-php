use clap::Args;
use anyhow::Result;
use hdrhistogram::Histogram;
use std::time::{Duration, Instant};
use tracing::info;

#[derive(Args)]
pub struct BenchArgs {
    #[arg(short, long, default_value = "http://localhost:8080")]
    pub url: String,

    #[arg(short, long, default_value = "60")]
    pub duration: u64,

    #[arg(short, long, default_value = "100")]
    pub rps: u64,

    #[arg(short = 'c', long, default_value = "10")]
    pub concurrency: usize,
}

pub async fn run(args: BenchArgs) -> Result<()> {
    info!("Starting benchmark...");
    println!("=== Benchmark Configuration ===");
    println!("URL: {}", args.url);
    println!("Duration: {}s", args.duration);
    println!("Target RPS: {}", args.rps);
    println!("Concurrency: {}", args.concurrency);
    println!();

    let client = reqwest::Client::new();
    let start_time = Instant::now();
    let duration = Duration::from_secs(args.duration);

    let mut histogram = Histogram::<u64>::new(3)?;
    let mut total_requests = 0u64;
    let mut successful_requests = 0u64;
    let mut failed_requests = 0u64;

    while start_time.elapsed() < duration {
        let req_start = Instant::now();

        match client.get(&args.url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    successful_requests += 1;
                } else {
                    failed_requests += 1;
                }
            }
            Err(_) => {
                failed_requests += 1;
            }
        }

        let latency = req_start.elapsed().as_millis() as u64;
        histogram.record(latency)?;
        total_requests += 1;

        let target_interval = Duration::from_millis(1000 / args.rps);
        if let Some(sleep_time) = target_interval.checked_sub(req_start.elapsed()) {
            tokio::time::sleep(sleep_time).await;
        }
    }

    let actual_duration = start_time.elapsed().as_secs_f64();
    let actual_rps = total_requests as f64 / actual_duration;

    println!("=== Benchmark Results ===");
    println!("Duration: {:.2}s", actual_duration);
    println!("Target RPS: {}", args.rps);
    println!("Actual RPS: {:.2}", actual_rps);
    println!();
    println!("Requests:");
    println!("  Total: {}", total_requests);
    println!("  Successful: {} ({:.2}%)", successful_requests, (successful_requests as f64 / total_requests as f64) * 100.0);
    println!("  Failed: {} ({:.2}%)", failed_requests, (failed_requests as f64 / total_requests as f64) * 100.0);
    println!();
    println!("Response Times:");
    println!("  p50:  {}ms", histogram.value_at_quantile(0.50));
    println!("  p75:  {}ms", histogram.value_at_quantile(0.75));
    println!("  p95:  {}ms", histogram.value_at_quantile(0.95));
    println!("  p99:  {}ms", histogram.value_at_quantile(0.99));
    println!("  max:  {}ms", histogram.max());

    Ok(())
}
