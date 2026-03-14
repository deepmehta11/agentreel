/// Model cost estimation (USD per 1M tokens, as of March 2026).

pub struct ModelCost {
    pub input: f64,
    pub output: f64,
}

/// Estimate cost in USD for a given model and token count.
pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
    let costs = match_model_cost(model);
    (input_tokens as f64 * costs.input / 1_000_000.0)
        + (output_tokens as f64 * costs.output / 1_000_000.0)
}

fn match_model_cost(model: &str) -> ModelCost {
    let m = model.to_lowercase();

    // Anthropic
    if m.contains("opus") {
        return ModelCost { input: 15.0, output: 75.0 };
    }
    if m.contains("sonnet") {
        return ModelCost { input: 3.0, output: 15.0 };
    }
    if m.contains("haiku") {
        return ModelCost { input: 0.80, output: 4.0 };
    }

    // OpenAI
    if m.contains("gpt-4.5") {
        return ModelCost { input: 5.0, output: 15.0 };
    }
    if m.contains("gpt-4o-mini") {
        return ModelCost { input: 0.15, output: 0.60 };
    }
    if m.contains("gpt-4o") {
        return ModelCost { input: 2.50, output: 10.0 };
    }
    if m.contains("o3-mini") || m.contains("o4-mini") {
        return ModelCost { input: 1.10, output: 4.40 };
    }
    if m.contains("o3") {
        return ModelCost { input: 10.0, output: 40.0 };
    }

    // Google
    if m.contains("gemini-2.5-pro") {
        return ModelCost { input: 1.25, output: 10.0 };
    }
    if m.contains("gemini") {
        return ModelCost { input: 0.15, output: 0.60 };
    }

    // DeepSeek
    if m.contains("deepseek-r1") {
        return ModelCost { input: 0.55, output: 2.19 };
    }
    if m.contains("deepseek") {
        return ModelCost { input: 0.27, output: 1.10 };
    }

    // Unknown — assume mid-range
    ModelCost { input: 3.0, output: 15.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_opus_cost() {
        let cost = estimate_cost("claude-opus-4-6", 1000, 500);
        assert!((cost - 0.0525).abs() < 0.0001);
    }

    #[test]
    fn test_claude_sonnet_cost() {
        let cost = estimate_cost("claude-sonnet-4-20250514", 1000, 500);
        assert!((cost - 0.0105).abs() < 0.0001);
    }

    #[test]
    fn test_gpt4o_cost() {
        let cost = estimate_cost("gpt-4o", 5000, 2000);
        assert!((cost - 0.0325).abs() < 0.0001);
    }

    #[test]
    fn test_unknown_model_uses_default() {
        let cost = estimate_cost("some-unknown-model", 1000, 1000);
        assert!(cost > 0.0);
    }
}
