use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum OnboarderError {
    #[error("Configuration error: {0}")]
    #[diagnostic(
        code(onboarder::config),
        help("Run `onboarder --setup` to configure your API keys and settings")
    )]
    ConfigError(String),

    #[error("I/O error: {0}")]
    #[diagnostic(
        code(onboarder::io),
        help("Check file permissions and ensure the path exists")
    )]
    IoError(#[from] std::io::Error),

    #[error("Parsing error: {0}")]
    #[diagnostic(
        code(onboarder::parsing),
        help("Ensure the file format is correct and all required fields are present")
    )]
    ParsingError(String),

    #[error("Network error: {0}")]
    #[diagnostic(
        code(onboarder::network),
        help("Check your internet connection and API endpoint configuration")
    )]
    NetworkError(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    #[diagnostic(
        code(onboarder::json),
        help("The configuration file may be corrupted. Try deleting it and running setup again")
    )]
    JsonError(#[from] serde_json::Error),
}

// Made with Bob
