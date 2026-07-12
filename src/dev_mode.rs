use std::fmt;
use std::time::Duration;

/// Metrics for tracking API usage and performance in developer mode
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub duration: Duration,
    pub tokens_per_second: f64,
}

impl Metrics {
    pub fn new(
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
        duration: Duration,
    ) -> Self {
        let elapsed_secs = duration.as_secs_f64().max(0.001);
        let tokens_per_second = completion_tokens as f64 / elapsed_secs;

        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens,
            duration,
            tokens_per_second,
        }
    }

    /// Get a formatted duration string
    pub fn duration_str(&self) -> String {
        let total_ms = self.duration.as_millis();
        if total_ms < 1000 {
            format!("{}ms", total_ms)
        } else if total_ms < 60000 {
            format!("{:.1}s", total_ms as f64 / 1000.0)
        } else {
            let mins = total_ms / 60000;
            let secs = (total_ms % 60000) as f64 / 1000.0;
            format!("{}m {:.0}s", mins, secs)
        }
    }

    /// Get a formatted tokens per second string
    pub fn tokens_per_second_str(&self) -> String {
        format!("{:.1} tok/s", self.tokens_per_second)
    }

    /// Estimate cost in USD based on common pricing (rough estimates)
    pub fn estimated_cost(&self) -> String {
        // Rough pricing: ~$10/M input tokens, ~$30/M output tokens for GPT-4o
        let input_cost = self.prompt_tokens as f64 * 2.5 / 1_000_000.0;
        let output_cost = self.completion_tokens as f64 * 10.0 / 1_000_000.0;
        let total = input_cost + output_cost;
        format!("${:.6}", total)
    }
}

impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "⎿ ⎺ Tokens: {}↑ / {}↓ / {}∑ | Duration: {} | Speed: {} | Est. Cost: {}",
            self.prompt_tokens,
            self.completion_tokens,
            self.total_tokens,
            self.duration_str(),
            self.tokens_per_second_str(),
            self.estimated_cost(),
        )
    }
}

/// Compact single-line metrics display
pub fn format_metrics_line(metrics: &Metrics) -> String {
    format!(
        "⬆ {}  ⬇ {}  ∑ {}  ⏱ {}  ⚡ {}  💰 {}",
        metrics.prompt_tokens,
        metrics.completion_tokens,
        metrics.total_tokens,
        metrics.duration_str(),
        metrics.tokens_per_second_str(),
        metrics.estimated_cost(),
    )
}
