mod app;
mod input;
mod search;
mod ui;
mod watcher;

use std::io::{self, stdout, BufWriter, Write};
use std::panic::{self, PanicHookInfo};
use std::sync::Arc;
use std::time::Duration;

use crossterm::{
    cursor::Show,
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::cli::Args;
use crate::display::Document;
use crate::error::MatError;
use crate::highlight::SearchState;
use crate::pipeline::ProcessingConfig;
use crate::theme::{get_theme, ThemeColors};

pub use app::App;
use app::{AppConfig, ReloadConfig};
use watcher::FileWatcher;

type PanicHook = dyn for<'a> Fn(&PanicHookInfo<'a>) + Send + Sync + 'static;

/// Owns every process-global terminal mutation made by the pager.
struct TerminalGuard {
    original_hook: Arc<PanicHook>,
    raw_mode: bool,
    alternate_screen: bool,
}

impl TerminalGuard {
    fn install() -> Self {
        let original_hook: Arc<PanicHook> = Arc::from(panic::take_hook());
        let panic_hook = Arc::clone(&original_hook);
        panic::set_hook(Box::new(move |info| {
            let _ = disable_raw_mode();
            let _ = execute!(stdout(), Show, LeaveAlternateScreen);
            panic_hook(info);
        }));
        Self {
            original_hook,
            raw_mode: false,
            alternate_screen: false,
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.raw_mode {
            let _ = disable_raw_mode();
        }
        if self.alternate_screen {
            let _ = execute!(stdout(), Show, LeaveAlternateScreen);
        }
        let original = Arc::clone(&self.original_hook);
        let _ = panic::take_hook();
        panic::set_hook(Box::new(move |info| original(info)));
    }
}

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
            start_str
                .parse::<usize>()
                .map_err(|_| MatError::InvalidLineRange {
                    range: range.to_string(),
                })?
        };

        let end = if end_str.is_empty() {
            total_lines
        } else {
            end_str
                .parse::<usize>()
                .map_err(|_| MatError::InvalidLineRange {
                    range: range.to_string(),
                })?
        };

        if start == 0 || end == 0 || start > end || start > total_lines {
            return Err(MatError::InvalidLineRange {
                range: range.to_string(),
            });
        }

        Ok((start, end.min(total_lines)))
    } else {
        // Single line number
        let line = range
            .parse::<usize>()
            .map_err(|_| MatError::InvalidLineRange {
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
pub fn print_document(
    document: &Document,
    show_line_numbers: bool,
    styled: bool,
) -> io::Result<()> {
    let gutter_width = if show_line_numbers {
        let max_line = document
            .lines
            .iter()
            .map(|line| line.number)
            .max()
            .unwrap_or(0);
        if max_line == 0 {
            3
        } else {
            let digits = max_line.to_string().len();
            digits + 2
        }
    } else {
        0
    };

    let stdout = stdout();
    let mut output = BufWriter::new(stdout.lock());
    for line in &document.lines {
        if show_line_numbers {
            if line.number == 0 {
                write!(output, "{:width$}", "", width = gutter_width)?;
            } else {
                write!(output, "{:>width$} ", line.number, width = gutter_width - 2)?;
            }
        }
        if styled {
            for span in &line.spans {
                write!(output, "{}{}", style_sequence(&span.style), span.text)?;
                if !span.style.is_plain() {
                    write!(output, "\x1b[0m")?;
                }
            }
        } else {
            write!(output, "{}", line.text())?;
        }
        writeln!(output)?;
    }
    output.flush()?;
    Ok(())
}

fn style_sequence(style: &crate::display::SpanStyle) -> String {
    let mut codes = Vec::new();
    if style.bold {
        codes.push("1".to_string());
    }
    if style.dim {
        codes.push("2".to_string());
    }
    if style.italic {
        codes.push("3".to_string());
    }
    if style.underline {
        codes.push("4".to_string());
    }
    if let Some(color) = style.fg {
        codes.push(color_code(color, false));
    }
    if let Some(color) = style.bg {
        codes.push(color_code(color, true));
    }
    if codes.is_empty() {
        String::new()
    } else {
        format!("\x1b[{}m", codes.join(";"))
    }
}

fn color_code(color: ratatui::style::Color, background: bool) -> String {
    use ratatui::style::Color;
    let base = if background { 40 } else { 30 };
    let bright = if background { 100 } else { 90 };
    match color {
        Color::Black => base.to_string(),
        Color::Red => (base + 1).to_string(),
        Color::Green => (base + 2).to_string(),
        Color::Yellow => (base + 3).to_string(),
        Color::Blue => (base + 4).to_string(),
        Color::Magenta => (base + 5).to_string(),
        Color::Cyan => (base + 6).to_string(),
        Color::Gray => (base + 7).to_string(),
        Color::DarkGray => bright.to_string(),
        Color::LightRed => (bright + 1).to_string(),
        Color::LightGreen => (bright + 2).to_string(),
        Color::LightYellow => (bright + 3).to_string(),
        Color::LightBlue => (bright + 4).to_string(),
        Color::LightMagenta => (bright + 5).to_string(),
        Color::LightCyan => (bright + 6).to_string(),
        Color::White => (bright + 7).to_string(),
        Color::Indexed(index) => format!("{};5;{index}", if background { 48 } else { 38 }),
        Color::Rgb(r, g, b) => format!("{};2;{r};{g};{b}", if background { 48 } else { 38 }),
        Color::Reset => {
            if background {
                "49".to_string()
            } else {
                "39".to_string()
            }
        }
    }
}

/// Run the pager TUI
pub fn run_pager(
    document: Document,
    base_document: Document,
    args: &Args,
    search_state: Option<SearchState>,
    file_path: Option<std::path::PathBuf>,
    processing: ProcessingConfig,
) -> Result<(), MatError> {
    let mut guard = TerminalGuard::install();
    enable_raw_mode().map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;
    guard.raw_mode = true;

    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen).map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;
    guard.alternate_screen = true;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(|e| MatError::Io {
        source: e,
        path: std::path::PathBuf::from("terminal"),
    })?;

    // Determine theme and create colors
    let theme = get_theme(args.theme);
    let theme_colors = if processing.styling {
        ThemeColors::for_theme(theme)
    } else {
        ThemeColors::plain()
    };

    // Create reload config if viewing a file
    let reload_config = file_path.as_ref().map(|_| ReloadConfig {
        processing,
        force_binary: args.force_binary,
    });

    // Create app with search state and theme
    let mut app = App::new(
        document,
        base_document,
        AppConfig {
            show_line_numbers: args.line_numbers,
            search_state,
            theme_colors,
            file_path: file_path.clone(),
            wrap_mode: args.wrap,
            max_width: args.max_width,
            reload_config,
        },
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

    // Set up file watcher if viewing a file (not stdin)
    let file_watcher = file_path
        .as_ref()
        .and_then(|path| FileWatcher::new(path).ok());

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

        if let Some(ref watcher) = file_watcher {
            if watcher.check_changed() {
                if app.follow_mode {
                    let was_at_bottom = app.at_bottom();
                    if app.reload_file() && was_at_bottom {
                        app.go_to_bottom();
                    }
                } else {
                    app.file_changed = true;
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

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
