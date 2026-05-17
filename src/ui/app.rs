//! Core application loop, layout orchestration, and rendering engine.
//!
//! This module houses the primary `run_tui` loop, which serves as the beating heart
//! of the interface. It utilizes `ratatui`'s constraint-based layout engine to split
//! the terminal into responsive panes, capturing keyboard events via `crossterm` and
//! piping them into the `AppState` mutations to create an interactive experience.

use crate::engine::{AiClient, CacheManager, hash_file};
use crate::errors::OnboarderError;
use crate::ui::state::AppState;
use crate::ui::utils::{detect_project_stack, format_markdown};
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

/// A Resource Acquisition Is Initialization (RAII) guard for terminal state.
///
/// Terminal interfaces rely on placing the user's terminal into "Raw Mode" (disabling
/// line buffering and local echo) and moving to an Alternate Screen buffer so the user's
/// prior bash history isn't overwritten. If the application crashes unexpectedly, leaving
/// the terminal in this raw state severely degrades the user's experience.
///
/// This struct guarantees that the terminal is gracefully restored to its default
/// cooked state when it goes out of scope (via the `Drop` trait), ensuring safety even
/// during a panic.
pub struct TerminalGuard {
    /// The initialized Ratatui Terminal instance tied to a Crossterm backend.
    pub terminal: Terminal<CrosstermBackend<io::Stdout>>,
}

impl TerminalGuard {
    /// Instantiates a new terminal guard, enabling raw mode and setting up the alternate screen.
    pub fn new() -> Result<Self, OnboarderError> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Provides mutable access to the underlying `ratatui::Terminal` instance.
    ///
    /// This is strictly used to call the `.draw()` method on the terminal instance
    /// during the main event loop to execute the rendering instructions.
    pub fn get_mut(&mut self) -> &mut Terminal<CrosstermBackend<io::Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Automatically restore the user's terminal upon program exit or crash
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

/// Asynchronously orchestrates the analysis of a selected file.
///
/// When the user hits 'Enter' on a specific file, this function creates a hash of that
/// file's content and queries the `CacheManager`. If a hit is found, it instantly returns
/// the cached markdown. If the cache misses, it delegates the raw file content to the
/// `AiClient` for generation, awaits the result, saves the new markdown to disk, and
/// returns it to be displayed in the UI.
pub async fn analyze_file(
    file_path: &PathBuf,
    ai_client: &AiClient,
    cache: &CacheManager,
) -> Result<String, OnboarderError> {
    let file_hash = hash_file(file_path)?;

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

    let content = std::fs::read_to_string(file_path)?;
    let explanation = ai_client.explain_file(file_path, &content).await?;
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

/// The primary application execution loop and UI rendering orchestration point.
///
/// This function acts as an infinite loop. On every tick, it clears the terminal,
/// calculates structural layout constraints, renders all widgets based on the current
/// `AppState`, and then pauses to poll `crossterm` for any new keystrokes.
///
/// It splits the UI into a 30% left column (housing Meta data and the File Explorer)
/// and a 70% right column (housing the massive Deep Dive Markdown output).
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
        // Draw the UI based on current State
        terminal_guard.get_mut().draw(|f| {
            // 1. Establish the main vertical division: 30% Left / 70% Right
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(f.area());

            // 2. Establish horizontal division on the Left side: Fixed header, fluid list below
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(7), Constraint::Min(0)])
                .split(main_chunks[0]);

            // 3. Render Project Meta Pane (Top Left Area)
            let stack_info = detect_project_stack(&state.files);
            let meta_text = format!(
                "Project Type:\n{}\n\nFiles Scanned: {}",
                stack_info,
                state.files.len()
            );
            let meta_pane = Paragraph::new(meta_text)
                .block(
                    Block::default()
                        .title("Project Meta")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Green)),
                )
                .style(Style::default().fg(Color::White));
            f.render_widget(meta_pane, left_chunks[0]);

            // 4. Render File Explorer (Bottom Left Area)
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
                        .title("📁 File Explorer (Use Arrow keys ↑/↓ to navigate)")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(file_list, left_chunks[1], &mut list_state);

            // 5. Render Right Pane: Deep Dive Explanation Content
            let title = if state.loading {
                "Deep Dive (Loading...)"
            } else if state.error_message.is_some() {
                "Deep Dive (Error)"
            } else {
                "Deep Dive"
            };

            let content = if let Some(ref error) = state.error_message {
                format!("Error: {}\n\nPress Enter to retry.", error)
            } else {
                state.explanation.clone()
            };

            let explanation_text = Paragraph::new(format_markdown(&content))
                .block(
                    Block::default()
                        .title(title)
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .wrap(Wrap { trim: true }) // Automatically handle line-breaks
                .style(Style::default().fg(Color::White));

            f.render_widget(explanation_text, main_chunks[1]);

            // 6. Render the sticky instruction footer at the absolute bottom
            let footer_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(1)])
                .split(f.area());

            let footer = Paragraph::new("Press 'q' or 'Esc' to quit")
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(footer, footer_chunks[1]);
        })?;

        // Await user keyboard interactions for up to 100 milliseconds per tick
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break, // Quit Application
                    KeyCode::Down => {
                        state.select_next();
                        state.error_message = None;
                    }
                    KeyCode::Up => {
                        state.select_previous();
                        state.error_message = None;
                    }
                    KeyCode::Enter => {
                        // Analyze file when user hits enter on the list
                        if let Some(file_path) = state.get_selected_file().cloned() {
                            state.loading = true;
                            state.error_message = None;

                            // Immediately force a terminal redraw to display the loading spinner
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

                            // Execute asynchronous external AI call or read from Local Extractor
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
