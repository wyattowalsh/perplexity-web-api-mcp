use crate::types::{Model, SearchMode};

pub const API_BASE_URL: &str = "https://www.perplexity.ai";
pub const API_VERSION: &str = "2.18";

pub const ENDPOINT_AUTH_SESSION: &str = "/api/auth/session";
pub const ENDPOINT_SSE_ASK: &str = "/rest/sse/perplexity_ask";
pub const ENDPOINT_UPLOAD_URL: &str = "/rest/uploads/create_upload_url";

pub const API_MODE_CONCISE: &str = "concise";
pub const API_MODE_COPILOT: &str = "copilot";

pub const MODEL_PREFERENCE_TURBO: &str = "turbo";
pub const MODEL_PREFERENCE_PPLX_PRO: &str = "pplx_pro";
pub const MODEL_PREFERENCE_PPLX_REASONING: &str = "pplx_reasoning";
pub const MODEL_PREFERENCE_PPLX_ALPHA: &str = "pplx_alpha";

pub const MODEL_NAME_SONAR: &str = "sonar";
pub const MODEL_PREFERENCE_SONAR: &str = "experimental";

pub const MODEL_NAME_GPT52: &str = "gpt-5.2";
pub const MODEL_PREFERENCE_GPT52: &str = "gpt52";

pub const MODEL_NAME_CLAUDE45SONNET: &str = "claude-4.5-sonnet";
pub const MODEL_PREFERENCE_CLAUDE45SONNET: &str = "claude45sonnet";

pub const MODEL_NAME_GROK41: &str = "grok-4.1";
pub const MODEL_PREFERENCE_GROK41: &str = "grok41nonreasoning";

pub const MODEL_NAME_GPT52_THINKING: &str = "gpt-5.2-thinking";
pub const MODEL_PREFERENCE_GPT52_THINKING: &str = "gpt52_thinking";

pub const MODEL_NAME_CLAUDE45SONNET_THINKING: &str = "claude-4.5-sonnet-thinking";
pub const MODEL_PREFERENCE_CLAUDE45SONNET_THINKING: &str = "claude45sonnetthinking";

pub const MODEL_NAME_GEMINI30PRO: &str = "gemini-3.0-pro";
pub const MODEL_PREFERENCE_GEMINI30PRO: &str = "gemini30pro";

pub const MODEL_NAME_KIMIK2THINKING: &str = "kimi-k2-thinking";
pub const MODEL_PREFERENCE_KIMIK2THINKING: &str = "kimik2thinking";

pub const MODEL_NAME_GROK41_REASONING: &str = "grok-4.1-reasoning";
pub const MODEL_PREFERENCE_GROK41_REASONING: &str = "grok41reasoning";

/// Returns the model preference string for the API payload.
///
/// Returns `Some(preference)` if the mode+model combination is valid,
/// or `None` if the model is incompatible with the given mode.
pub fn model_preference(mode: SearchMode, model: Option<Model>) -> Option<&'static str> {
    match (mode, model) {
        // Auto mode - only default model
        (SearchMode::Auto, None) => Some(MODEL_PREFERENCE_TURBO),
        (SearchMode::Auto, Some(_)) => None,

        // Pro mode models
        (SearchMode::Pro, None) => Some(MODEL_PREFERENCE_PPLX_PRO),
        (SearchMode::Pro, Some(Model::Sonar)) => Some(MODEL_PREFERENCE_SONAR),
        (SearchMode::Pro, Some(Model::Gpt52)) => Some(MODEL_PREFERENCE_GPT52),
        (SearchMode::Pro, Some(Model::Claude45Sonnet)) => {
            Some(MODEL_PREFERENCE_CLAUDE45SONNET)
        }
        (SearchMode::Pro, Some(Model::Grok41)) => Some(MODEL_PREFERENCE_GROK41),
        (SearchMode::Pro, Some(_)) => None, // Other models not valid for Pro

        // Reasoning mode models
        (SearchMode::Reasoning, None) => Some(MODEL_PREFERENCE_PPLX_REASONING),
        (SearchMode::Reasoning, Some(Model::Gpt52Thinking)) => {
            Some(MODEL_PREFERENCE_GPT52_THINKING)
        }
        (SearchMode::Reasoning, Some(Model::Claude45SonnetThinking)) => {
            Some(MODEL_PREFERENCE_CLAUDE45SONNET_THINKING)
        }
        (SearchMode::Reasoning, Some(Model::Gemini30Pro)) => {
            Some(MODEL_PREFERENCE_GEMINI30PRO)
        }
        (SearchMode::Reasoning, Some(Model::KimiK2Thinking)) => {
            Some(MODEL_PREFERENCE_KIMIK2THINKING)
        }
        (SearchMode::Reasoning, Some(Model::Grok41Reasoning)) => {
            Some(MODEL_PREFERENCE_GROK41_REASONING)
        }
        (SearchMode::Reasoning, Some(_)) => None, // Other models not valid for Reasoning

        // Deep Research mode - only default model
        (SearchMode::DeepResearch, None) => Some(MODEL_PREFERENCE_PPLX_ALPHA),
        (SearchMode::DeepResearch, Some(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_mode_defaults() {
        assert_eq!(model_preference(SearchMode::Auto, None), Some(MODEL_PREFERENCE_TURBO));
    }

    #[test]
    fn test_auto_mode_rejects_models() {
        assert_eq!(model_preference(SearchMode::Auto, Some(Model::Gpt52)), None);
        assert_eq!(model_preference(SearchMode::Auto, Some(Model::Sonar)), None);
    }

    #[test]
    fn test_pro_mode_defaults() {
        assert_eq!(model_preference(SearchMode::Pro, None), Some(MODEL_PREFERENCE_PPLX_PRO));
    }

    #[test]
    fn test_pro_mode_models() {
        assert_eq!(
            model_preference(SearchMode::Pro, Some(Model::Sonar)),
            Some(MODEL_PREFERENCE_SONAR)
        );
        assert_eq!(
            model_preference(SearchMode::Pro, Some(Model::Gpt52)),
            Some(MODEL_PREFERENCE_GPT52)
        );
        assert_eq!(
            model_preference(SearchMode::Pro, Some(Model::Claude45Sonnet)),
            Some(MODEL_PREFERENCE_CLAUDE45SONNET)
        );
        assert_eq!(
            model_preference(SearchMode::Pro, Some(Model::Grok41)),
            Some(MODEL_PREFERENCE_GROK41)
        );
    }

    #[test]
    fn test_pro_mode_rejects_reasoning_models() {
        assert_eq!(model_preference(SearchMode::Pro, Some(Model::Gpt52Thinking)), None);
        assert_eq!(
            model_preference(SearchMode::Pro, Some(Model::Claude45SonnetThinking)),
            None
        );
    }

    #[test]
    fn test_reasoning_mode_defaults() {
        assert_eq!(
            model_preference(SearchMode::Reasoning, None),
            Some(MODEL_PREFERENCE_PPLX_REASONING)
        );
    }

    #[test]
    fn test_reasoning_mode_models() {
        assert_eq!(
            model_preference(SearchMode::Reasoning, Some(Model::Gpt52Thinking)),
            Some(MODEL_PREFERENCE_GPT52_THINKING)
        );
        assert_eq!(
            model_preference(SearchMode::Reasoning, Some(Model::Claude45SonnetThinking)),
            Some(MODEL_PREFERENCE_CLAUDE45SONNET_THINKING)
        );
        assert_eq!(
            model_preference(SearchMode::Reasoning, Some(Model::Gemini30Pro)),
            Some(MODEL_PREFERENCE_GEMINI30PRO)
        );
        assert_eq!(
            model_preference(SearchMode::Reasoning, Some(Model::KimiK2Thinking)),
            Some(MODEL_PREFERENCE_KIMIK2THINKING)
        );
        assert_eq!(
            model_preference(SearchMode::Reasoning, Some(Model::Grok41Reasoning)),
            Some(MODEL_PREFERENCE_GROK41_REASONING)
        );
    }

    #[test]
    fn test_reasoning_mode_rejects_pro_models() {
        assert_eq!(model_preference(SearchMode::Reasoning, Some(Model::Gpt52)), None);
        assert_eq!(model_preference(SearchMode::Reasoning, Some(Model::Sonar)), None);
    }

    #[test]
    fn test_deep_research_mode_defaults() {
        assert_eq!(
            model_preference(SearchMode::DeepResearch, None),
            Some(MODEL_PREFERENCE_PPLX_ALPHA)
        );
    }

    #[test]
    fn test_deep_research_mode_rejects_models() {
        assert_eq!(model_preference(SearchMode::DeepResearch, Some(Model::Gpt52)), None);
        assert_eq!(
            model_preference(SearchMode::DeepResearch, Some(Model::Gpt52Thinking)),
            None
        );
    }
}
