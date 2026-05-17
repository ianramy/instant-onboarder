//! Application state management for the interactive TUI environment.
//!
//! Because `ratatui` operates on an immediate-mode rendering paradigm, the UI is entirely
//! redrawn on every tick of the event loop. Therefore, it requires a single "source of truth"
//! to reference when deciding what to render. The `AppState` struct fulfills this role,
//! holding the current state of file selections, network responses, and loading indicators.

use std::path::PathBuf;

/// The central state machine dictating the behavior and display of the TUI.
///
/// `AppState` holds all mutable data required during the runtime of the interface.
/// As the user presses keys (like Up, Down, or Enter), the event loop modifies these
/// fields, and the next UI render pass immediately reflects those changes.
pub struct AppState {
    /// A collection of valid source code file paths discovered in the target directory.
    /// This list determines what is displayed in the Left-Hand "File Explorer" pane.
    pub files: Vec<PathBuf>,

    /// The numeric index of the currently highlighted file in the `files` array.
    /// This is used to visually track the user's cursor position in the file list.
    pub selected_index: usize,

    /// The raw markdown text representing the architectural breakdown of the currently
    /// selected file. This is rendered in the Right-Hand "Deep Dive" pane.
    pub explanation: String,

    /// A boolean flag indicating whether the application is currently waiting on a
    /// background task (like an AI network request or local parse). When `true`,
    /// the UI will render a "Loading..." spinner or prompt.
    pub loading: bool,

    /// An optional string designed to capture and surface errors to the user gracefully.
    /// If an API key is invalid or the network drops, this field is populated and
    /// displayed instead of crashing the program.
    pub error_message: Option<String>,
}

impl AppState {
    /// Creates a fresh instance of `AppState` with the provided dataset of target files.
    ///
    /// Upon initialization, the cursor is automatically placed at index `0` (the first file),
    /// and a placeholder instruction is loaded into the explanation buffer to guide the user.
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self {
            files,
            selected_index: 0,
            explanation: "Press Enter to analyze file...".to_string(),
            loading: false,
            error_message: None,
        }
    }

    /// Advances the selection cursor downward through the file list.
    ///
    /// It implements a wrap-around behavior using modulo arithmetic. If the user presses
    /// the "Down" key while at the very bottom of the list, the cursor will seamlessly
    /// loop back to the top item (index 0), preventing out-of-bounds panics.
    pub fn select_next(&mut self) {
        if !self.files.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.files.len();
        }
    }

    /// Moves the selection cursor upward through the file list.
    ///
    /// Similar to `select_next`, this implements wrap-around logic. If the user is at
    /// the top of the list (index 0) and presses the "Up" key, the cursor will instantly
    /// jump to the very last file in the vector.
    pub fn select_previous(&mut self) {
        if !self.files.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.files.len() - 1;
            }
        }
    }

    /// Safely retrieves a reference to the `PathBuf` of the currently selected file.
    ///
    /// This ensures that when the user presses 'Enter' to analyze a file, the application
    /// can securely extract the exact file path corresponding to the visual cursor position.
    /// Returns `None` if the file list is completely empty.
    pub fn get_selected_file(&self) -> Option<&PathBuf> {
        self.files.get(self.selected_index)
    }
}
