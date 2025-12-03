use std::path::PathBuf;

use crate::display::{Document, Line};
use crate::highlight::SearchState;
use crate::input::FollowReader;
use crate::theme::ThemeColors;

use super::search::InteractiveSearch;

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
    /// Original document (for restoring after search cancel)
    pub original_document: Option<Document>,
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
    /// Whether case-insensitive search is enabled
    pub ignore_case: bool,
    /// Whether follow mode is active
    pub follow_mode: bool,
    /// Follow reader for tailing files
    pub follow_reader: Option<FollowReader>,
    /// Path to the file being viewed (for follow mode)
    pub file_path: Option<PathBuf>,
}

impl App {
    /// Create a new App with the given document
    pub fn new(
        document: Document,
        show_line_numbers: bool,
        search_state: Option<SearchState>,
        theme_colors: ThemeColors,
        ignore_case: bool,
        file_path: Option<PathBuf>,
    ) -> Self {
        Self {
            document,
            original_document: None,
            scroll_line: 0,
            scroll_col: 0,
            mode: Mode::Normal,
            should_quit: false,
            terminal_size: (80, 24),
            show_line_numbers,
            search_state,
            theme_colors,
            interactive_search: None,
            ignore_case,
            follow_mode: false,
            follow_reader: None,
            file_path,
        }
    }

    /// Toggle follow mode
    pub fn toggle_follow(&mut self) {
        // Only allow follow mode for files
        if let Some(ref path) = self.file_path {
            if self.follow_mode {
                // Disable follow mode
                self.follow_mode = false;
                self.follow_reader = None;
            } else {
                // Enable follow mode
                if let Ok(reader) = FollowReader::new(path.clone(), true) {
                    self.follow_mode = true;
                    self.follow_reader = Some(reader);
                    // Scroll to bottom when entering follow mode
                    self.go_to_bottom();
                }
            }
        }
    }

    /// Check for new content in follow mode and append to document
    pub fn check_follow_updates(&mut self) {
        if !self.follow_mode {
            return;
        }

        if let Some(ref mut reader) = self.follow_reader {
            if let Ok(new_lines) = reader.check_for_new_content() {
                if !new_lines.is_empty() {
                    let start_number = self.document.lines.len() + 1;
                    for (i, text) in new_lines.into_iter().enumerate() {
                        let line = Line::plain(start_number + i, &text);
                        let width = line.width();
                        self.document.lines.push(line);
                        if width > self.document.max_line_width {
                            self.document.max_line_width = width;
                        }
                    }
                    // Auto-scroll to bottom
                    self.go_to_bottom();
                }
            }
        }
    }

    /// Enter search mode
    pub fn enter_search_mode(&mut self) {
        // Save original document for potential cancellation
        self.original_document = Some(self.document.clone());
        self.interactive_search = Some(InteractiveSearch::new(self.ignore_case));
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
        // Restore original document first
        if let Some(ref original) = self.original_document {
            self.document = original.clone();
        }

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
        self.original_document = None;
    }

    /// Cancel the search and restore original document
    pub fn cancel_search(&mut self) {
        // Restore original document
        if let Some(original) = self.original_document.take() {
            self.document = original;
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
        // Try to center the line in the viewport
        let target = line_idx.saturating_sub(height / 2);
        let max_scroll = self.document.line_count().saturating_sub(height);
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
        let max_line = self.document.line_count();
        if max_line == 0 {
            3 // Minimum " 1 "
        } else {
            let digits = (max_line as f64).log10().floor() as usize + 1;
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
        let max_scroll = self.document.line_count().saturating_sub(self.content_height());
        self.scroll_line = (self.scroll_line + n).min(max_scroll);
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_line = self.scroll_line.saturating_sub(n);
    }

    /// Scroll left by n columns
    pub fn scroll_left(&mut self, n: usize) {
        self.scroll_col = self.scroll_col.saturating_sub(n);
    }

    /// Scroll right by n columns
    pub fn scroll_right(&mut self, n: usize) {
        let max_scroll = self.document.max_line_width.saturating_sub(self.content_width());
        self.scroll_col = (self.scroll_col + n).min(max_scroll);
    }

    /// Scroll to the start of the current line
    pub fn scroll_to_line_start(&mut self) {
        self.scroll_col = 0;
    }

    /// Scroll to the end of the longest visible line
    pub fn scroll_to_line_end(&mut self) {
        let max_scroll = self.document.max_line_width.saturating_sub(self.content_width());
        self.scroll_col = max_scroll;
    }

    /// Go to the top of the document
    pub fn go_to_top(&mut self) {
        self.scroll_line = 0;
    }

    /// Go to the bottom of the document
    pub fn go_to_bottom(&mut self) {
        let max_scroll = self.document.line_count().saturating_sub(self.content_height());
        self.scroll_line = max_scroll;
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

    /// Get current line number for status bar (1-indexed)
    pub fn current_line_display(&self) -> usize {
        self.scroll_line + 1
    }

    /// Get total line count for status bar
    pub fn total_lines(&self) -> usize {
        self.document.line_count()
    }

    /// Check if we're at the end of the document
    pub fn at_bottom(&self) -> bool {
        self.scroll_line + self.content_height() >= self.document.line_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    fn create_test_doc(lines: usize) -> Document {
        let text: String = (1..=lines).map(|i| format!("Line {}\n", i)).collect();
        Document::from_text(&text.trim_end(), "test.txt".to_string(), "UTF-8".to_string())
    }

    fn test_theme_colors() -> ThemeColors {
        ThemeColors::for_theme(Theme::Dark)
    }

    #[test]
    fn test_scroll_down() {
        let doc = create_test_doc(100);
        let mut app = App::new(doc, false, None, test_theme_colors(), false, None);
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
        let mut app = App::new(doc, false, None, test_theme_colors(), false, None);
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
        let mut app = App::new(doc, false, None, test_theme_colors(), false, None);
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
        let app = App::new(doc, true, None, test_theme_colors(), false, None);
        assert_eq!(app.gutter_width(), 3); // " 9 "

        let doc = create_test_doc(99);
        let app = App::new(doc, true, None, test_theme_colors(), false, None);
        assert_eq!(app.gutter_width(), 4); // " 99 "

        let doc = create_test_doc(999);
        let app = App::new(doc, true, None, test_theme_colors(), false, None);
        assert_eq!(app.gutter_width(), 5); // " 999 "
    }
}
