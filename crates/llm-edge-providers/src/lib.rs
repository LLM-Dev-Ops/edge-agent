//! LLM Provider adapters
//!
//! Provides unified interface to multiple LLM providers:
//! - OpenAI (GPT-4, GPT-3.5, o1)
//! - Anthropic (Claude 3.5 Sonnet, Claude 3 Opus/Haiku)
//! - Google (Gemini Pro, Gemini Ultra)
//! - AWS Bedrock
//! - Azure OpenAI

pub mod types;
pub mod error;
pub mod openai;
pub mod anthropic;
pub mod adapter;

pub use error::{ProviderError, ProviderResult};
pub use types::{UnifiedRequest, UnifiedResponse, Message, Usage};
pub use adapter::LLMProvider;

#[cfg(test)]
mod tests {
    #[test]
    fn test_placeholder() {
        assert_eq!(2 + 2, 4);
    }
}
