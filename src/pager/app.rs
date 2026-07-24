use std::path::PathBuf;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::cli::WrapMode;
use crate::display::Document;
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
            file_path: config.file_path,
            wrap_mode: config.wrap_mode,
            max_width: config.max_width,
            wrapped_lines: None,
            file_changed: false,
            reload_config: config.reload_config,
        }
    }

    /// Toggle follow mode
    pub fn toggle_follow(&mut self) {
        // Only allow follow mode for files
        if self.file_path.is_some() {
            if self.follow_mode {
                // Disable follow mode
                self.follow_mode = false;
            } else {
                self.follow_mode = true;
                self.go_to_bottom();
            }
        }
    }

    /// Reload the file from disk
    /// Returns true if reload was successful
    pub fn reload_file(&mut self) -> bool {
        let path = match &self.file_path {
            Some(p) => p.clone(),
            None => return false, // Can't reload stdin
        };

        let config = match &self.reload_config {
            Some(c) => c.clone(),
            None => return false, // No reload config available
        };

        // Read the file
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(_) => return false,
        };

        let encoding_name = detect_encoding(&bytes);
        let bom_marked = matches!(encoding_name, "UTF-8-BOM" | "UTF-16LE" | "UTF-16BE");
        if !config.force_binary && !bom_marked && is_binary(&bytes) {
            return false;
        }
        let text = match decode_bytes(bytes, encoding_name) {
            Ok(t) => t,
            Err(_) => return false,
        };

        let content = Content {
            text,
            source_name: self.document.source_name.clone(),
            is_markdown: config.processing.render_markdown,
            encoding: encoding_name.to_string(),
        };
        let new_base = match process(content, &config.processing) {
            Ok(document) => document,
            Err(_) => return false,
        };
        let mut new_doc = new_base.clone();
        if let Some(state) = &self.search_state {
            crate::highlight::apply_search_highlight(&mut new_doc, &state.pattern);
        }

        // Store current scroll position
        let old_scroll = self.scroll_line;

        // Update document
        self.base_document = new_base;
        self.document = new_doc;

        // Clear file changed flag
        self.file_changed = false;

        // Invalidate wrapped lines cache
        self.wrapped_lines = None;

        // Restore scroll position (clamped to new document bounds)
        let max_scroll = self
            .document
            .line_count()
            .saturating_sub(self.content_height());
        self.scroll_line = old_scroll.min(max_scroll);

        // Rebuild wrapped lines if in wrap mode
        self.build_wrapped_lines();

        // Update search state matches for new document
        if let Some(ref mut state) = self.search_state {
            state.find_matches(&self.document);
        }

        true
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
        if let Some(ref search) = self.interactive_search {
            search.apply_highlighting(&mut self.document);
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
        if let Some(state) = &self.search_state {
            crate::highlight::apply_search_highlight(&mut self.document, &state.pattern);
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
        let old_size = self.terminal_size;
        self.terminal_size = (width, height);
        // Invalidate wrapped lines cache if size changed and we're in wrap mode
        if old_size != (width, height) && self.wrap_mode != WrapMode::None {
            self.wrapped_lines = None;
        }
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
            return;
        }

        let width = self.content_width();
        if width == 0 {
            self.wrapped_lines = None;
            return;
        }

        let mut wrapped = Vec::new();

        for (line_idx, line) in self.document.lines.iter().enumerate() {
            let line_text = line.text();
            let line_width = line.width();

            if line_width == 0 {
                // Empty line - still takes one row
                wrapped.push(WrappedLine {
                    line_idx,
                    line_number: line.number,
                    is_first_row: true,
                    byte_start: 0,
                    byte_end: 0,
                });
            } else {
                // Break line into wrapped rows
                let mut current_width = 0;
                let mut is_first = true;
                let mut row_start = 0;

                for (byte_idx, grapheme) in line_text.grapheme_indices(true) {
                    let ch_width = UnicodeWidthStr::width(grapheme);

                    if current_width + ch_width > width && current_width > 0 {
                        // Start a new row
                        wrapped.push(WrappedLine {
                            line_idx,
                            line_number: line.number,
                            is_first_row: is_first,
                            byte_start: row_start,
                            byte_end: byte_idx,
                        });
                        is_first = false;
                        row_start = byte_idx;
                        current_width = ch_width;
                    } else {
                        current_width += ch_width;
                    }
                }

                // Don't forget the last row
                if current_width > 0 || is_first {
                    wrapped.push(WrappedLine {
                        line_idx,
                        line_number: line.number,
                        is_first_row: is_first,
                        byte_start: row_start,
                        byte_end: line_text.len(),
                    });
                }
            }
        }

        self.wrapped_lines = Some(wrapped);
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
