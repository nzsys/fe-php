pub mod collector;
pub mod exporter;

pub use collector::MetricsCollector;
pub use exporter::export_metrics;

pub fn init_metrics() {
    // Initialize metrics (simplified version)
    // In a real implementation, this would set up Prometheus metrics
}
