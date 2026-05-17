//! Centralized error handling and standardized diagnostic reporting.
//!
//! This module leverages the `thiserror` crate to derive standard library `Error`
//! traits automatically, reducing boilerplate. Furthermore, it utilizes the `miette`
//! crate to provide rich, colorful, and highly descriptive error diagnostics to the
//! user, often including helpful resolution steps if something goes wrong.

use miette::Diagnostic;
use thiserror::Error;

/// The central enum containing all possible failure states within the application.
///
/// By bubbling all errors up into this unified enum, functions across different
/// modules (like the network client, the file parser, and the configuration manager)
/// can all return a single, predictable `Result<T, OnboarderError>` signature.
#[derive(Error, Debug, Diagnostic)]
pub enum OnboarderError {
    /// Emitted when the application state is invalid, missing, or malformed.
    /// Often indicates that the user needs to provide credentials.
    #[error("Configuration error: {0}")]
    #[diagnostic(
        code(onboarder::config),
        help("Run `onboarder --setup` to configure your API keys and settings")
    )]
    ConfigError(String),

    /// Emitted when file system operations fail (e.g., unable to read source files,
    /// write to the cache, or access the config directory). Wraps the standard `std::io::Error`.
    #[error("I/O error: {0}")]
    #[diagnostic(
        code(onboarder::io),
        help("Check file permissions and ensure the path exists")
    )]
    IoError(#[from] std::io::Error),

    /// Emitted when external API responses or local directory structures do not conform
    /// to expected formats, preventing successful data extraction.
    #[error("Parsing error: {0}")]
    #[diagnostic(
        code(onboarder::parsing),
        help("Ensure the file format is correct and all required fields are present")
    )]
    ParsingError(String),

    /// Emitted when communication with external AI endpoints or local Daemons fails
    /// due to timeouts, invalid domains, or lack of internet access. Wraps `reqwest::Error`.
    #[error("Network error: {0}")]
    #[diagnostic(
        code(onboarder::network),
        help("Check your internet connection and API endpoint configuration")
    )]
    NetworkError(#[from] reqwest::Error),

    /// Emitted specifically when the application fails to serialize or deserialize
    /// local settings or network payloads. Wraps `serde_json::Error`.
    #[error("JSON serialization error: {0}")]
    #[diagnostic(
        code(onboarder::json),
        help("The configuration file may be corrupted. Try deleting it and running setup again")
    )]
    JsonError(#[from] serde_json::Error),
}
