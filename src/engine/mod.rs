//! Core application engine handling AI interactions, local parsing, and caching.
//!
//! The `engine` module serves as the central brain of the Instant Onboarder application.
//! It abstracts away the complexities of dealing with various language models, network
//! requests, and file system operations, providing a clean interface for the UI to consume.
//!
//! Architecture
//! This module is logically divided into three primary sub-components:
//! - Client (`client.rs`): Manages external HTTP communications with AI providers
//!   like IBM watsonx.ai and local instances like Ollama.
//! - Cache (`cache.rs`): Provides an aggressive file-system caching layer based on
//!   SHA-256 content hashing to drastically reduce API costs and response latency.
//! - Extractor (`extractor.rs`): Serves as a lightning-fast, zero-token fallback
//!   that generates structural outlines purely through lexical analysis of the source code.

pub mod cache;
pub mod client;
pub mod extractor;

// Re-export necessary components for the rest of the application
pub use cache::{CacheManager, hash_file};
pub use client::AiClient;
