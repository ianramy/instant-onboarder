//! User Interface module for rendering the Terminal User Interface (TUI).
//!
//! The `ui` module is responsible for all visual presentation and user interaction
//! within the terminal environment. It leverages the `ratatui` crate for drawing
//! layout grids, widgets, and styled text, while relying on `crossterm` to handle
//! low-level terminal manipulation (like capturing raw keyboard input and entering
//! alternate screen modes).
//!
//! Architecture
//! This module is logically divided into three sub-components:
//! - State (`state.rs`): Manages the mutable data layer that dictates what the UI
//!   currently displays (selected files, loaded markdown, error messages).
//! - Utils (`utils.rs`): Provides helper functions to transform raw data, such as
//!   parsing markdown strings into colored `ratatui` text blocks.
//! - App (`app.rs`): Contains the core synchronous event loop that continuously
//!   draws the interface and polls for user input.

pub mod app;
pub mod state;
pub mod utils;

// Re-export the main run loop for use in main.rs so the application entry point
// remains clean and avoids deeply nested path resolutions.
pub use app::run_tui;
