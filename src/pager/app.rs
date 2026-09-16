use std::path::PathBuf;

use crate::cli::WrapMode;
use crate::display::{wrap_ranges, Document};
use crate::error::MatError;
use crate::highlight::SearchState;
use crate::input::{decode_bytes, detect_encoding, is_binary, Content};
use crate::pipeline::{process, ProcessingConfig};
use crate::theme::ThemeColors;

use super::search::InteractiveSearch;
/// Configuration needed for file reload
#[derive(Clone)]
pub struct ReloadConfig {
    pub processing: ProcessingConfig,
    pub force_binary: bool,
}

/// Construction-time pager settings.
pub struct AppConfig {
    pub styling: bool,
    pub show_line_numbers: bool,
    pub search_state: Option<SearchState>,
    pub theme_colors: ThemeColors,
    pub file_path: Option<PathBuf>,
    pub wrap_mode: WrapMode,
    pub max_width: usize,
    pub reload_config: Option<ReloadConfig>,
}

/// Pager mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// Normal viewing mode
    Normal,
    /// Search mode with query input
    Search { query: String },
}

/// Main pager application state
pub struct App {
    /// Whether generated styling, including search overlays, is enabled.
    pub styling: bool,
    /// The document being viewed
    pub document: Document,
    /// Canonical document before search overlays.
    pub base_document: Document,
    /// Current scroll line (0-indexed, top of viewport)
    pub scroll_line: usize,
    /// Current horizontal scroll offset (0-indexed)
    pub scroll_col: usize,
    /// Current mode
    pub mode: Mode,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Terminal size (width, height)
    pub terminal_size: (u16, u16),
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    /// Search state (if any)
    pub search_state: Option<SearchState>,
    /// Theme colors for UI rendering
    pub theme_colors: ThemeColors,
    /// Interactive search state
    pub interactive_search: Option<InteractiveSearch>,
    /// Whether follow mode is active
    pub follow_mode: bool,
    /// Most recent watcher or reload failure, displayed in the status bar.
    pub watch_error: Option<String>,
    pub reload_error: Option<String>,
    /// Path to the file being viewed (for follow mode)
    pub file_path: Option<PathBuf>,
    /// Line wrapping mode
    pub wrap_mode: WrapMode,
    /// Max width for truncation mode
    pub max_width: usize,
    /// Cached wrapped lines (invalidated on resize or wrap mode change)
    pub wrapped_lines: Option<Vec<WrappedLine>>,
    /// Whether the file has changed since last loaded/reloaded
    pub file_changed: bool,
    /// Configuration for file reload
    pub reload_config: Option<ReloadConfig>,
}

/// A single display row, which may be part of a wrapped line
#[derive(Debug, Clone)]
pub struct WrappedLine {
    /// Original line index in the document (0-indexed)
    pub line_idx: usize,
    /// Original line number (1-indexed, for display)
    pub line_number: usize,
    /// Whether this is the first row of the original line
    pub is_first_row: bool,
    /// Byte range in the original line for this display row.
    pub byte_start: usize,
    pub byte_end: usize,
}

impl App {
    /// Create a new App with the given document
    pub fn new(document: Document, base_document: Document, config: AppConfig) -> Self {
        Self {
            styling: config.styling,
            document,
            base_document,
            scroll_line: 0,
            scroll_col: 0,
            mode: Mode::Normal,
            should_quit: false,
            terminal_size: (80, 24),
            show_line_numbers: config.show_line_numbers,
            search_state: config.search_state,
            theme_colors: config.theme_colors,
            interactive_search: None,
            follow_mode: false,
            watch_error: None,
            reload_error: None,
            file_path: config.file_path,
            wrap_mode: config.wrap_mode,
            max_width: config.max_width,
            wrapped_lines: None,
            file_changed: false,
            reload_config: config.reload_config,
        }
    }

    /// Establish the viewport before positioning follow mode at the end.
    pub fn initialize_view(&mut self, width: u16, height: u16, follow: bool) {
        self.set_terminal_size(width, height);
        if follow {
            self.toggle_follow();
        }
    }

    /// Toggle follow mode
    pub fn toggle_follow(&mut self) {
        // Only allow follow mode for files
        if self.file_path.is_some() && self.watch_error.is_none() {
            if self.follow_mode {
                // Disable follow mode
                self.follow_mode = false;
            } else {
                self.follow_mode = true;
                self.go_to_bottom();
            }
        }
    }

    pub fn report_watch_error(&mut self, error: String) {
        self.watch_error = Some(error);
        self.follow_mode = false;
    }

    /// Reload without losing the last good document if reading or processing fails.
    pub fn reload_file(&mut self) -> bool {
        match self.try_reload_file() {
            Ok(()) => {
                self.reload_error = None;
                true
            }
            Err(error) => {
                self.reload_error = Some(error.to_string());
                false
            }
        }
    }

    fn try_reload_file(&mut self) -> Result<(), MatError> {
        let path = self
            .file_path
            .clone()
            .ok_or_else(|| MatError::InvalidArguments("Cannot reload standard input".into()))?;
        let config = self.reload_config.clone().ok_or_else(|| {
            MatError::InvalidArguments("Reload configuration is unavailable".into())
        })?;
        let bytes = std::fs::read(&path).map_err(|source| MatError::Io {
            source,
            path: path.clone(),
        })?;

        let encoding_name = detect_encoding(&bytes);
        let bom_marked = matches!(encoding_name, "UTF-8-BOM" | "UTF-16LE" | "UTF-16BE");
        if !config.force_binary && !bom_marked && is_binary(&bytes) {
            return Err(MatError::BinaryFile { path });
        }
        let text = decode_bytes(bytes, encoding_name)?;

        let content = Content {
            text,
            source_name: self.document.source_name.clone(),
            is_markdown: config.processing.render_markdown,
            encoding: encoding_name.to_string(),
        };
        let new_base = process(content, &config.processing)?;
        let mut new_doc = new_base.clone();
        if self.styling {
            if let Some(state) = &self.search_state {
                crate::highlight::apply_search_highlight(&mut new_doc, &state.pattern);
            }
        }

        // Update document
        self.base_document = new_base;
        self.document = new_doc;

        // Clear file changed flag
        self.file_changed = false;

        // Invalidate wrapped lines cache
        self.wrapped_lines = None;

        // Clamp the existing display-row offset only after rebuilding layout.
        self.build_wrapped_lines();

        // Update search state matches for new document
        if let Some(ref mut state) = self.search_state {
            state.find_matches(&self.document);
        }

        Ok(())
    }

    /// Enter search mode
    /// If `case_insensitive` is true, search will ignore case
    pub fn enter_search_mode(&mut self, case_insensitive: bool) {
        self.interactive_search = Some(InteractiveSearch::new(case_insensitive));
        self.mode = Mode::Search {
            query: String::new(),
        };
    }

    /// Add a character to the search query
    pub fn search_add_char(&mut self, c: char) {
        if let Some(ref mut search) = self.interactive_search {
            search.push_char(c);

            // Update mode with new query
            self.mode = Mode::Search {
                query: search.query.clone(),
            };

            // Apply incremental highlighting
            self.apply_incremental_search();
        }
    }

    /// Remove the last character from the search query
    pub fn search_backspace(&mut self) {
        if let Some(ref mut search) = self.interactive_search {
            search.pop_char();

            // Update mode with new query
            self.mode = Mode::Search {
                query: search.query.clone(),
            };

            // Apply incremental highlighting
            self.apply_incremental_search();
        }
    }

    /// Apply incremental search highlighting
    fn apply_incremental_search(&mut self) {
        self.document = self.base_document.clone();

        // Apply highlighting
        if self.styling {
            if let Some(ref search) = self.interactive_search {
                search.apply_highlighting(&mut self.document);
            }
        }
    }

    /// Confirm the search and exit search mode
    pub fn confirm_search(&mut self) {
        if let Some(ref search) = self.interactive_search {
            if !search.is_empty() {
                // Create a proper SearchState for navigation
                if let Some(pattern) = search.compile_pattern() {
                    let mut state = SearchState {
                        pattern,
                        matches: Vec::new(),
                        current_match: None,
                    };
                    state.find_matches(&self.document);
                    self.search_state = Some(state);
                }
            }
        }

        self.mode = Mode::Normal;
        self.interactive_search = None;
    }

    /// Cancel the search and restore original document
    pub fn cancel_search(&mut self) {
        self.document = self.base_document.clone();
        if self.styling {
            if let Some(state) = &self.search_state {
                crate::highlight::apply_search_highlight(&mut self.document, &state.pattern);
            }
        }

        self.mode = Mode::Normal;
        self.interactive_search = None;
    }

    /// Navigate to next search match
    pub fn next_match(&mut self) {
        if let Some(ref mut state) = self.search_state {
            if let Some(line_idx) = state.next_match() {
                self.scroll_to_line(line_idx);
            }
        }
    }

    /// Navigate to previous search match
    pub fn prev_match(&mut self) {
        if let Some(ref mut state) = self.search_state {
            if let Some(line_idx) = state.prev_match() {
                self.scroll_to_line(line_idx);
            }
        }
    }

    /// Scroll to show a specific line in the viewport
    fn scroll_to_line(&mut self, line_idx: usize) {
        let height = self.content_height();
        let row = if self.wrap_mode == WrapMode::Wrap {
            self.wrapped_lines
                .as_ref()
                .and_then(|rows| rows.iter().position(|row| row.line_idx == line_idx))
                .unwrap_or(line_idx)
        } else {
            line_idx
        };
        let target = row.saturating_sub(height / 2);
        let max_scroll = self.max_scroll();
        self.scroll_line = target.min(max_scroll);
    }

    /// Get search info for status bar
    pub fn search_info(&self) -> Option<(usize, usize)> {
        self.search_state.as_ref().and_then(|state| {
            let total = state.match_count();
            if total > 0 {
                let current = state.current_match_display().unwrap_or(0);
                Some((current, total))
            } else {
                None
            }
        })
    }

    /// Update terminal size
    pub fn set_terminal_size(&mut self, width: u16, height: u16) {
        self.terminal_size = (width, height);
        self.build_wrapped_lines();
    }

    /// Get the content area height (excluding status bar)
    pub fn content_height(&self) -> usize {
        self.terminal_size.1.saturating_sub(1) as usize
    }

    /// Get the content area width
    pub fn content_width(&self) -> usize {
        let gutter_width = if self.show_line_numbers {
            self.gutter_width()
        } else {
            0
        };
        (self.terminal_size.0 as usize).saturating_sub(gutter_width)
    }

    /// Get the gutter (line number) width
    pub fn gutter_width(&self) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        // Calculate width based on max line number
        let max_line = self
            .document
            .lines
            .iter()
            .map(|line| line.number)
            .max()
            .unwrap_or(0);
        if max_line == 0 {
            3 // Minimum " 1 "
        } else {
            let digits = max_line.to_string().len();
            digits + 2 // Space before and after number
        }
    }

    /// Get the range of visible lines
    pub fn visible_line_range(&self) -> (usize, usize) {
        let start = self.scroll_line;
        let end = (start + self.content_height()).min(self.document.line_count());
        (start, end)
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        let max_scroll = self.max_scroll();
        self.scroll_line = (self.scroll_line + n).min(max_scroll);
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_line = self.scroll_line.saturating_sub(n);
    }

    /// Scroll left by n columns (disabled in wrap mode)
    pub fn scroll_left(&mut self, n: usize) {
        if self.wrap_mode == WrapMode::Wrap {
            return; // No horizontal scroll in wrap mode
        }
        self.scroll_col = self.scroll_col.saturating_sub(n);
    }

    /// Scroll right by n columns (disabled in wrap mode)
    pub fn scroll_right(&mut self, n: usize) {
        if self.wrap_mode == WrapMode::Wrap {
            return; // No horizontal scroll in wrap mode
        }
        let max_scroll = self
            .document
            .max_line_width
            .saturating_sub(self.content_width());
        self.scroll_col = (self.scroll_col + n).min(max_scroll);
    }

    /// Scroll to the start of the current line (disabled in wrap mode)
    pub fn scroll_to_line_start(&mut self) {
        if self.wrap_mode != WrapMode::Wrap {
            self.scroll_col = 0;
        }
    }

    /// Scroll to the end of the longest visible line (disabled in wrap mode)
    pub fn scroll_to_line_end(&mut self) {
        if self.wrap_mode != WrapMode::Wrap {
            let max_scroll = self
                .document
                .max_line_width
                .saturating_sub(self.content_width());
            self.scroll_col = max_scroll;
        }
    }

    /// Go to the top of the document
    pub fn go_to_top(&mut self) {
        self.scroll_line = 0;
    }

    /// Go to the bottom of the document
    pub fn go_to_bottom(&mut self) {
        self.scroll_line = self.max_scroll();
    }

    /// Get maximum scroll position
    fn max_scroll(&self) -> usize {
        match self.wrap_mode {
            WrapMode::None | WrapMode::Truncate => self
                .document
                .line_count()
                .saturating_sub(self.content_height()),
            WrapMode::Wrap => self
                .total_wrapped_lines()
                .saturating_sub(self.content_height()),
        }
    }

    /// Scroll down half a page
    pub fn scroll_half_page_down(&mut self) {
        let half_page = self.content_height() / 2;
        self.scroll_down(half_page);
    }

    /// Scroll up half a page
    pub fn scroll_half_page_up(&mut self) {
        let half_page = self.content_height() / 2;
        self.scroll_up(half_page);
    }

    /// Get total line count for status bar
    pub fn total_lines(&self) -> usize {
        self.document.line_count()
    }

    /// Check if we're at the end of the document
    pub fn at_bottom(&self) -> bool {
        match self.wrap_mode {
            WrapMode::None | WrapMode::Truncate => {
                self.scroll_line + self.content_height() >= self.document.line_count()
            }
            WrapMode::Wrap => {
                let total_wrapped = self.total_wrapped_lines();
                self.scroll_line + self.content_height() >= total_wrapped
            }
        }
    }

    /// Get total number of wrapped lines (for wrap mode)
    pub fn total_wrapped_lines(&self) -> usize {
        if self.wrap_mode != WrapMode::Wrap {
            return self.document.line_count();
        }
        self.wrapped_lines
            .as_ref()
            .map_or(self.document.line_count(), Vec::len)
    }

    /// Build wrapped line indices for efficient lookup
    pub fn build_wrapped_lines(&mut self) {
        if self.wrap_mode != WrapMode::Wrap {
            self.wrapped_lines = None;
            self.clamp_scroll();
            return;
        }

        let width = self.content_width();
        if width == 0 {
            self.wrapped_lines = None;
            self.clamp_scroll();
            return;
        }

        let mut wrapped = Vec::new();

        for (line_idx, line) in self.document.lines.iter().enumerate() {
            let line_text = line.text();
            for (row_idx, range) in wrap_ranges(&line_text, width).into_iter().enumerate() {
                wrapped.push(WrappedLine {
                    line_idx,
                    line_number: line.number,
                    is_first_row: row_idx == 0,
                    byte_start: range.start,
                    byte_end: range.end,
                });
            }
        }

        self.wrapped_lines = Some(wrapped);
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        self.scroll_line = self.scroll_line.min(self.max_scroll());
        self.scroll_col = self.scroll_col.min(
            self.document
                .max_line_width
                .saturating_sub(self.content_width()),
        );
    }

    /// Invalidate wrapped lines cache (call when document changes)
    pub fn invalidate_wrap_cache(&mut self) {
        self.wrapped_lines = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::render_markdown;
    use crate::theme::Theme;

    fn create_test_doc(lines: usize) -> Document {
        let text: String = (1..=lines).map(|i| format!("Line {}\n", i)).collect();
        Document::from_text(text.trim_end(), "test.txt".to_string(), "UTF-8".to_string())
    }

    fn test_theme_colors() -> ThemeColors {
        ThemeColors::for_theme(Theme::Dark)
    }

    fn test_app(document: Document, show_line_numbers: bool, wrap_mode: WrapMode) -> App {
        let base_document = document.clone();
        App::new(
            document,
            base_document,
            AppConfig {
                styling: true,
                show_line_numbers,
                search_state: None,
                theme_colors: test_theme_colors(),
                file_path: None,
                wrap_mode,
                max_width: 200,
                reload_config: None,
            },
        )
    }

    #[test]
    fn layout_changes_keep_scrolling_and_rendering_in_bounds() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        use ratatui::{backend::TestBackend, Terminal};

        let doc = Document::from_text(&"x".repeat(1000), "test".into(), "UTF-8".into());
        let mut app = test_app(doc, true, WrapMode::Wrap);
        app.set_terminal_size(20, 5);
        app.go_to_bottom();
        super::super::input::handle_key(
            KeyEvent::new(KeyCode::Char('#'), KeyModifiers::NONE),
            &mut app,
        );
        assert!(app.at_bottom());
        assert_eq!(app.scroll_line, 46);
        for (width, height) in [(200, 5), (0, 0), (80, 24)] {
            app.set_terminal_size(width, height);
            assert!(app.scroll_line <= app.max_scroll());
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| super::super::ui::render(frame, &app))
                .unwrap();
        }
    }

    #[test]
    fn follow_starts_at_the_end_of_the_actual_viewport() {
        for mode in [WrapMode::None, WrapMode::Wrap] {
            let mut app = test_app(create_test_doc(50), false, mode);
            app.file_path = Some("test.txt".into());
            app.initialize_view(5, 5, true);
            assert!(app.follow_mode);
            assert!(app.at_bottom());
            assert_eq!(
                app.scroll_line + app.content_height(),
                app.total_wrapped_lines()
            );
        }
    }

    #[test]
    fn plain_interactive_search_keeps_navigation_without_styling() {
        let mut app = test_app(create_test_doc(50), false, WrapMode::None);
        app.styling = false;
        app.enter_search_mode(false);
        for c in "Line 40".chars() {
            app.search_add_char(c);
        }
        assert!(app
            .document
            .lines
            .iter()
            .flat_map(|line| &line.spans)
            .all(|span| span.style.is_plain()));
        app.confirm_search();
        app.next_match();
        assert_eq!(app.search_info(), Some((1, 1)));
        assert!(app.scroll_line > 0);
        app.enter_search_mode(false);
        app.search_add_char('x');
        app.cancel_search();
        assert!(app
            .document
            .lines
            .iter()
            .flat_map(|line| &line.spans)
            .all(|span| span.style.is_plain()));
    }

    #[test]
    fn test_scroll_down() {
        let doc = create_test_doc(100);
        let mut app = test_app(doc, false, WrapMode::None);
        app.set_terminal_size(80, 24); // 23 content lines

        assert_eq!(app.scroll_line, 0);
        app.scroll_down(5);
        assert_eq!(app.scroll_line, 5);

        // Can't scroll past the end
        app.scroll_down(1000);
        assert_eq!(app.scroll_line, 77); // 100 - 23 = 77
    }

    #[test]
    fn test_scroll_up() {
        let doc = create_test_doc(100);
        let mut app = test_app(doc, false, WrapMode::None);
        app.scroll_line = 50;

        app.scroll_up(10);
        assert_eq!(app.scroll_line, 40);

        // Can't scroll past the start
        app.scroll_up(1000);
        assert_eq!(app.scroll_line, 0);
    }

    #[test]
    fn test_go_to_top_bottom() {
        let doc = create_test_doc(100);
        let mut app = test_app(doc, false, WrapMode::None);
        app.set_terminal_size(80, 24);
        app.scroll_line = 50;

        app.go_to_top();
        assert_eq!(app.scroll_line, 0);

        app.go_to_bottom();
        assert_eq!(app.scroll_line, 77);
    }

    #[test]
    fn test_gutter_width() {
        let doc = create_test_doc(9);
        let app = test_app(doc, true, WrapMode::None);
        assert_eq!(app.gutter_width(), 3); // " 9 "

        let doc = create_test_doc(99);
        let app = test_app(doc, true, WrapMode::None);
        assert_eq!(app.gutter_width(), 4); // " 99 "

        let doc = create_test_doc(999);
        let app = test_app(doc, true, WrapMode::None);
        assert_eq!(app.gutter_width(), 5); // " 999 "
    }

    #[test]
    fn test_wrap_mode_scroll() {
        // Create a document with lines that will wrap
        let text = "Short\nThis is a much longer line that should wrap at width 20\nAnother";
        let doc = Document::from_text(text, "test.txt".to_string(), "UTF-8".to_string());
        let mut app = test_app(doc, false, WrapMode::Wrap);
        app.set_terminal_size(20, 10); // narrow width to force wrapping

        // Build wrapped lines
        app.build_wrapped_lines();

        // Total wrapped lines should be more than original 3 lines
        let total = app.total_wrapped_lines();
        assert!(
            total > 3,
            "Expected wrapping to increase line count, got {}",
            total
        );
    }

    #[test]
    fn test_wrap_mode_no_horizontal_scroll() {
        let doc = create_test_doc(10);
        let mut app = test_app(doc, false, WrapMode::Wrap);
        app.set_terminal_size(80, 24);

        // Horizontal scroll should be disabled in wrap mode
        app.scroll_right(10);
        assert_eq!(app.scroll_col, 0);

        app.scroll_left(10);
        assert_eq!(app.scroll_col, 0);
    }

    #[test]
    fn wrapping_keeps_grapheme_clusters_together() {
        let doc = Document::from_text("👨‍👩‍👧‍👦x", "test".into(), "UTF-8".into());
        let mut app = test_app(doc, false, WrapMode::Wrap);
        app.set_terminal_size(2, 5);
        app.build_wrapped_lines();
        let rows = app.wrapped_lines.as_ref().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            &app.document.lines[0].text()[rows[0].byte_start..rows[0].byte_end],
            "👨‍👩‍👧‍👦"
        );
    }

    #[test]
    fn tiny_terminal_dimensions_saturate() {
        let doc = create_test_doc(2);
        let mut app = test_app(doc, true, WrapMode::Wrap);
        app.set_terminal_size(0, 0);
        assert_eq!(app.content_width(), 0);
        assert_eq!(app.content_height(), 0);
        app.build_wrapped_lines();
    }

    #[test]
    fn gutter_uses_largest_original_line_number() {
        let mut doc = create_test_doc(2);
        doc.lines[1].number = 12_345;
        let app = test_app(doc, true, WrapMode::None);
        assert_eq!(app.gutter_width(), 7);
    }

    fn reloadable_app(text: &str, mode: WrapMode) -> (tempfile::NamedTempFile, App) {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), text).unwrap();
        let doc = Document::from_text(text, file.path().display().to_string(), "UTF-8".into());
        let mut app = test_app(doc, false, mode);
        app.file_path = Some(file.path().to_owned());
        app.reload_config = Some(ReloadConfig {
            processing: ProcessingConfig {
                render_markdown: false,
                preserve_ansi: false,
                styling: true,
                syntax_highlight: false,
                language: None,
                theme: Theme::Dark,
                line_range: None,
                grep: None,
                search: None,
            },
            force_binary: false,
        });
        (file, app)
    }

    #[test]
    fn reload_errors_preserve_content_and_clear_after_recovery() {
        let (file, mut app) = reloadable_app("original", WrapMode::None);
        std::fs::write(file.path(), b"\0binary").unwrap();
        assert!(!app.reload_file());
        assert_eq!(app.document.lines[0].text(), "original");
        assert!(app.reload_error.as_ref().unwrap().contains("Binary file"));
        std::fs::remove_file(file.path()).unwrap();
        assert!(!app.reload_file());
        assert!(app.reload_error.as_ref().unwrap().contains("I/O error"));
        std::fs::write(file.path(), "recovered").unwrap();
        assert!(app.reload_file());
        assert_eq!(app.document.lines[0].text(), "recovered");
        assert!(app.reload_error.is_none());
    }

    #[test]
    fn watcher_errors_disable_follow_and_remain_visible() {
        use ratatui::{backend::TestBackend, Terminal};
        let (_file, mut app) = reloadable_app("original", WrapMode::None);
        app.initialize_view(80, 5, true);
        app.report_watch_error("watch unavailable".into());
        app.toggle_follow();
        assert!(!app.follow_mode);
        assert!(app.reload_file());
        let mut terminal = Terminal::new(TestBackend::new(80, 5)).unwrap();
        terminal
            .draw(|frame| super::super::ui::render(frame, &app))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let status: String = (0..80).map(|x| buffer[(x, 4)].symbol()).collect();
        assert!(
            status.contains("Watch failed: watch unavailable"),
            "{status}"
        );
    }

    #[test]
    fn reload_preserves_wrapped_row_offset_and_clamps_after_truncation() {
        let (file, mut app) = reloadable_app(&"x".repeat(1000), WrapMode::Wrap);
        app.set_terminal_size(20, 5);
        app.scroll_down(20);
        assert!(app.reload_file());
        assert_eq!(app.scroll_line, 20);
        std::fs::write(file.path(), "x".repeat(200)).unwrap();
        assert!(app.reload_file());
        assert_eq!(app.scroll_line, 6);
        assert!(app.at_bottom());
    }

    #[test]
    fn test_reload_preserves_markdown_rendering() {
        let temp = tempfile::NamedTempFile::with_suffix(".md").unwrap();
        let initial_text = "# Initial\n\nBody text\n";
        let updated_text = "# Updated\n\nMore body text\n";
        std::fs::write(temp.path(), initial_text).unwrap();

        let source_name = temp.path().display().to_string();
        let doc = render_markdown(
            initial_text,
            source_name.clone(),
            "UTF-8".into(),
            Theme::Dark,
        );
        let expected = render_markdown(updated_text, source_name, "UTF-8".into(), Theme::Dark);
        let mut app = App::new(
            doc.clone(),
            doc,
            AppConfig {
                styling: true,
                show_line_numbers: false,
                search_state: None,
                theme_colors: test_theme_colors(),
                file_path: Some(temp.path().to_path_buf()),
                wrap_mode: WrapMode::None,
                max_width: 200,
                reload_config: Some(ReloadConfig {
                    processing: ProcessingConfig {
                        render_markdown: true,
                        preserve_ansi: false,
                        styling: true,
                        syntax_highlight: false,
                        language: None,
                        theme: Theme::Dark,
                        line_range: None,
                        grep: None,
                        search: None,
                    },
                    force_binary: false,
                }),
            },
        );

        std::fs::write(temp.path(), updated_text).unwrap();

        assert!(app.reload_file());
        assert_eq!(app.document.lines.len(), expected.lines.len());
        assert_eq!(app.document.lines[0].spans, expected.lines[0].spans);
        assert!(!app.document.lines[0].text().contains('#'));
    }
}
