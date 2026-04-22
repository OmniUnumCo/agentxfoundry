//! Concrete `LLMClient` implementations.
//!
//! Each submodule here implements the `crate::traits::LLMClient` trait for one
//! backend. All providers accept their configuration at construction time
//! (base URL, API key, model defaults) so they can be injected into a
//! `DeterministicLLMClient` wrapper with no global state.
//!
//! E.1 ships `ollama` only. Cloud providers land in E.2/E.3.

pub mod ollama;

pub use ollama::OllamaClient;
