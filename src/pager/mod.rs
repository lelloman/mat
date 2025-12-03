mod app;
mod input;
mod search;
mod ui;

use std::io::{self, stdout, Write};
use std::panic;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::cli::Args;
use crate::display::Document;
use crate::error::MatError;
use crate::highlight::SearchState;
use crate::theme::{get_theme, ThemeColors};

pub use app::{App, WrappedLine};

/// Parse line range from --lines argument
pub fn parse_line_range(range: &str, total_lines: usize) -> Result<(usize, usize), MatError> {
    let range = range.trim();

    if range.is_empty() {
        return Err(MatError::InvalidLineRange {
            range: range.to_string(),
        });
    }

    // Handle different formats: X:Y, :Y, X:, X
    if let Some((start_str, end_str)) = range.split_once(':') {
        let start = if start_str.is_empty() {
            1
        } else {
            start_str.parse::<usize>().map_err(|_| MatError::InvalidLineRange {
                range: range.to_string(),
            })?
        };

        let end = if end_str.is_empty() {
            total_lines
        } else {
            end_str.parse::<usize>().map_err(|_| MatError::InvalidLineRange {
                range: range.to_string(),
            })?
        };

        if start == 0 || end == 0 || start > end {
            return Err(MatError::InvalidLineRange {
                range: range.to_string(),
            });
        }

        Ok((start, end.min(total_lines)))
    } else {
        // Single line number
        let line = range.parse::<usize>().map_err(|_| MatError::InvalidLineRange {
            range: range.to_string(),
        })?;

        if line == 0 || line > total_lines {
            return Err(MatError::InvalidLineRange {
                range: range.to_string(),
            });
        }

        Ok((line, line))
    }
}

/// Filter document to only include lines in the given range
pub fn filter_line_range(document: &mut Document, start: usize, end: usize) {
    document.lines = document
        .lines
        .drain(..)
        .filter(|line| line.number >= start && line.number <= end)
        .collect();
    document.recalculate_max_width();
}

/// Print document directly to stdout (no-pager mode)
pub fn print_document(document: &Document, show_line_numbers: bool) -> io::Result<()> {
    let gutter_width = if show_line_numbers {
        let max_line = document.line_count();
        if max_line == 0 {
            3
        } else {
            let digits = (max_line as f64).log10().floor() as usize + 1;
            digits + 2
        }
    } else {
        0
    };

    for line in &document.lines {
        if show_line_numbers {
            print!("{:>width$} ", line.number, width = gutter_width - 2);
        }
        println!("{}", line.text());
    }

    stdout().flush()?;
    Ok(())
}

/// Run the pager TUI
pub fn run_pager(
    document: Document,
    args: &Args,
    search_state: Option<SearchState>,
    file_path: Option<std::path::PathBuf>,
) -> Result<(), MatError> {
    // Set up panic hook to restore terminal on panic
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    // Setup terminal
    enable_raw_mode().map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;

    // Determine theme and create colors
    let theme = get_theme(args.theme.as_deref());
    let theme_colors = ThemeColors::for_theme(theme);

    // Create app with search state and theme
    let mut app = App::new(
        document,
        args.line_numbers,
        search_state,
        theme_colors,
        args.ignore_case,
        file_path,
        args.wrap,
        args.max_width,
    );

    // Find all matches if search is active
    if let Some(ref mut state) = app.search_state {
        state.find_matches(&app.document);
    }

    // Enable follow mode if requested
    if args.follow {
        app.toggle_follow();
    }

    // Get initial terminal size
    let size = terminal.size().map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;
    app.set_terminal_size(size.width, size.height);

    // Build wrapped lines if in wrap mode
    app.build_wrapped_lines();

    // Main loop
    loop {
        // Render
        terminal
            .draw(|frame| {
                ui::render(frame, &app);
            })
            .map_err(|e| MatError::Io {
                source: e,
                path: std::path::PathBuf::from("terminal"),
            })?;

        // Handle events
        if event::poll(Duration::from_millis(100)).map_err(|e| MatError::Io {
            source: e,
            path: std::path::PathBuf::from("terminal"),
        })? {
            match event::read().map_err(|e| MatError::Io {
                source: e,
                path: std::path::PathBuf::from("terminal"),
            })? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if input::handle_key(key, &mut app) {
                        break;
                    }
                }
                Event::Resize(width, height) => {
                    app.set_terminal_size(width, height);
                    // Rebuild wrapped lines on resize
                    app.build_wrapped_lines();
                }
                _ => {}
            }
        }

        // Check for follow mode updates
        app.check_follow_updates();

        if app.should_quit {
            break;
        }
    }

    // Cleanup
    disable_raw_mode().map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen).map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_line_range_full() {
        assert_eq!(parse_line_range("10:20", 100).unwrap(), (10, 20));
    }

    #[test]
    fn test_parse_line_range_from_start() {
        assert_eq!(parse_line_range(":50", 100).unwrap(), (1, 50));
    }

    #[test]
    fn test_parse_line_range_to_end() {
        assert_eq!(parse_line_range("50:", 100).unwrap(), (50, 100));
    }

    #[test]
    fn test_parse_line_range_single() {
        assert_eq!(parse_line_range("42", 100).unwrap(), (42, 42));
    }

    #[test]
    fn test_parse_line_range_clamp() {
        assert_eq!(parse_line_range("50:200", 100).unwrap(), (50, 100));
    }

    #[test]
    fn test_parse_line_range_invalid() {
        assert!(parse_line_range("abc", 100).is_err());
        assert!(parse_line_range("20:10", 100).is_err());
        assert!(parse_line_range("0:10", 100).is_err());
        assert!(parse_line_range("", 100).is_err());
    }
}
