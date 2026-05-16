use crate::engine::{AiClient, CacheManager, hash_file};
use crate::errors::OnboarderError;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::io;
use std::path::PathBuf;

/// Application state for the TUI
struct AppState {
    files: Vec<PathBuf>,
    selected_index: usize,
    explanation: String,
    loading: bool,
    error_message: Option<String>,
}

impl AppState {
    fn new(files: Vec<PathBuf>) -> Self {
        Self {
            files,
            selected_index: 0,
            explanation: "Press Enter to analyze file...".to_string(),
            loading: false,
            error_message: None,
        }
    }

    fn select_next(&mut self) {
        if !self.files.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.files.len();
        }
    }

    fn select_previous(&mut self) {
        if !self.files.is_empty() {
            if self.selected_index > 0 {
                self.selected_index -= 1;
            } else {
                self.selected_index = self.files.len() - 1;
            }
        }
    }

    fn get_selected_file(&self) -> Option<&PathBuf> {
        self.files.get(self.selected_index)
    }
}

/// Terminal wrapper that ensures proper cleanup
struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    fn new() -> Result<Self, OnboarderError> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    fn get_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore terminal
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

/// Run the TUI application
pub async fn run_tui(
    files: Vec<PathBuf>,
    ai_client: AiClient,
    cache: CacheManager,
) -> Result<(), OnboarderError> {
    if files.is_empty() {
        return Err(OnboarderError::ConfigError(
            "No files to analyze. Please provide a directory with source files.".to_string(),
        ));
    }

    let mut terminal_guard = TerminalGuard::new()?;
    let mut state = AppState::new(files);

    loop {
        // Draw UI
        terminal_guard.get_mut().draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(f.area());

            // Left pane: File Explorer
            let file_items: Vec<ListItem> = state
                .files
                .iter()
                .enumerate()
                .map(|(i, path)| {
                    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("???");
                    let style = if i == state.selected_index {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    ListItem::new(filename).style(style)
                })
                .collect();

            let mut list_state = ListState::default();
            list_state.select(Some(state.selected_index));

            let file_list = List::new(file_items)
                .block(
                    Block::default()
                        .title("📁 File Explorer (↑/↓ to navigate, Enter to analyze)")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(file_list, chunks[0], &mut list_state);

            // Right pane: Deep Dive Explanation
            let title = if state.loading {
                "Deep Dive (Loading...)"
            } else if state.error_message.is_some() {
                "Deep Dive (Error)"
            } else {
                "Deep Dive"
            };

            let content = if let Some(ref error) = state.error_message {
                format!(
                    "Error: {}\n\nPress Enter to retry or select another file.",
                    error
                )
            } else {
                state.explanation.clone()
            };

            let explanation_text = Paragraph::new(content)
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .wrap(Wrap { trim: true })
                .style(Style::default().fg(Color::White));

            f.render_widget(explanation_text, chunks[1]);

            // Footer with instructions
            let footer_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.area());

            let footer = Paragraph::new("Press 'q' or 'Esc' to quit")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer, footer_chunks[1]);
        })?;

        // Handle events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        break;
                    }
                    KeyCode::Down => {
                        state.select_next();
                        state.error_message = None;
                    }
                    KeyCode::Up => {
                        state.select_previous();
                        state.error_message = None;
                    }
                    KeyCode::Enter => {
                        if let Some(file_path) = state.get_selected_file().cloned() {
                            state.loading = true;
                            state.error_message = None;

                            // Force a redraw to show loading state
                            terminal_guard.get_mut().draw(|f| {
                                let chunks = Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Percentage(30),
                                        Constraint::Percentage(70),
                                    ])
                                    .split(f.area());

                                let loading_text =
                                    Paragraph::new("🔄 Analyzing file...\n\nPlease wait...")
                                        .block(
                                            Block::default()
                                                .title("🔄 Deep Dive (Loading...)")
                                                .borders(Borders::ALL)
                                                .border_style(Style::default().fg(Color::Yellow)),
                                        )
                                        .style(Style::default().fg(Color::Yellow));

                                f.render_widget(loading_text, chunks[1]);
                            })?;

                            // Analyze the file
                            match analyze_file(&file_path, &ai_client, &cache).await {
                                Ok(explanation) => {
                                    state.explanation = explanation;
                                    state.loading = false;
                                }
                                Err(e) => {
                                    state.error_message = Some(e.to_string());
                                    state.loading = false;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

/// Analyze a file: check cache first, then call AI if needed
async fn analyze_file(
    file_path: &PathBuf,
    ai_client: &AiClient,
    cache: &CacheManager,
) -> Result<String, OnboarderError> {
    // Generate file hash
    let file_hash = hash_file(file_path)?;

    // Check cache first
    if let Some(cached) = cache.get_cached_explanation(&file_hash) {
        return Ok(format!(
            "# {} (cached)\n\n{}",
            file_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("???"),
            cached
        ));
    }

    // Read file content
    let content = std::fs::read_to_string(file_path)?;

    // Call AI to generate explanation
    let explanation = ai_client.explain_file(file_path, &content).await?;

    // Save to cache
    cache.save_explanation(&file_hash, &explanation)?;

    Ok(format!(
        "# {}\n\n{}",
        file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("???"),
        explanation
    ))
}

// Made with Bob
