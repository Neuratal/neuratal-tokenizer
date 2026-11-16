//! Benchmarking and comparative evaluation module for Neuratal Tokenizer.

pub mod metrics;

pub use metrics::{
    CategoryMetrics, ComparativeBenchmarkReport, EvalCategory, EvalSample, StrategyBenchmarkResult,
    get_standard_evaluation_suite, run_pretokenization_comparison,
};
