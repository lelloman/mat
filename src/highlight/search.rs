use ratatui::style::Color;
use regex::Regex;

use crate::cli::Args;
use crate::display::{Document, SpanStyle, StyledSpan};
use crate::error::MatError;
use crate::filter::build_regex;

/// Position of a match in the document
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchPosition {
    /// Line index (0-indexed)
    pub line_idx: usize,
    /// Start column (0-indexed, in characters)
    pub start_col: usize,
    /// End column (0-indexed, exclusive)
    pub end_col: usize,
}

/// Search state for the pager
#[derive(Debug)]
pub struct SearchState {
    /// Compiled regex pattern
    pub pattern: Regex,
    /// All match positions
    pub matches: Vec<MatchPosition>,
    /// Current match index (None if no navigation yet)
    pub current_match: Option<usize>,
}

impl SearchState {
    /// Create search state from CLI args
    pub fn from_args(args: &Args) -> Result<Option<Self>, MatError> {
        let pattern_str = match &args.search {
            Some(p) => p,
            None => return Ok(None),
        };

        if pattern_str.is_empty() {
            return Err(MatError::EmptyPattern);
        }

        let pattern = build_regex(pattern_str, args)?;

        Ok(Some(Self {
            pattern,
            matches: Vec::new(),
            current_match: None,
        }))
    }

    /// Find all matches in the document and store positions
    pub fn find_matches(&mut self, document: &Document) {
        self.matches.clear();

        for (line_idx, line) in document.lines.iter().enumerate() {
            let text = line.text();
            for mat in self.pattern.find_iter(&text) {
                self.matches.push(MatchPosition {
                    line_idx,
                    start_col: mat.start(),
                    end_col: mat.end(),
                });
            }
        }
    }

    /// Get total number of matches
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Get current match position (1-indexed for display)
    pub fn current_match_display(&self) -> Option<usize> {
        self.current_match.map(|i| i + 1)
    }

    /// Navigate to next match, returns the line index to scroll to
    pub fn next_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }

        let next = match self.current_match {
            Some(i) => (i + 1) % self.matches.len(),
            None => 0,
        };

        self.current_match = Some(next);
        Some(self.matches[next].line_idx)
    }

    /// Navigate to previous match, returns the line index to scroll to
    pub fn prev_match(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }

        let prev = match self.current_match {
            Some(i) => {
                if i == 0 {
                    self.matches.len() - 1
                } else {
                    i - 1
                }
            }
            None => self.matches.len() - 1,
        };

        self.current_match = Some(prev);
        Some(self.matches[prev].line_idx)
    }
}

/// Style for search highlighting
pub fn highlight_style() -> SpanStyle {
    SpanStyle {
        fg: Some(Color::Black),
        bg: Some(Color::Yellow),
        bold: true,
        italic: false,
        underline: false,
    }
}

/// Apply search highlighting to a document
/// This overlays search highlights on top of existing styles (preserving grep highlights etc)
pub fn apply_search_highlight(document: &mut Document, pattern: &Regex) {
    let search_style = highlight_style();

    for line in &mut document.lines {
        let text = line.text();
        let matches: Vec<_> = pattern.find_iter(&text).collect();

        if matches.is_empty() {
            continue;
        }

        // Build new spans that preserve existing styles but overlay search highlights
        let mut new_spans = Vec::new();
        let mut char_offset = 0;

        for span in &line.spans {
            let span_start = char_offset;
            let span_end = char_offset + span.text.len();

            // Find matches that overlap with this span
            let mut last_pos = 0;
            for mat in &matches {
                // Skip matches that don't overlap with this span
                if mat.end() <= span_start || mat.start() >= span_end {
                    continue;
                }

                // Calculate overlap within this span
                let overlap_start = mat.start().saturating_sub(span_start).min(span.text.len());
                let overlap_end = (mat.end() - span_start).min(span.text.len());

                // Add text before the match (with original style)
                if overlap_start > last_pos {
                    new_spans.push(StyledSpan::new(
                        &span.text[last_pos..overlap_start],
                        span.style.clone(),
                    ));
                }

                // Add matched text (with search highlight)
                if overlap_end > overlap_start {
                    new_spans.push(StyledSpan::new(
                        &span.text[overlap_start..overlap_end],
                        search_style.clone(),
                    ));
                }

                last_pos = overlap_end;
            }

            // Add remaining text after last match (with original style)
            if last_pos < span.text.len() {
                new_spans.push(StyledSpan::new(
                    &span.text[last_pos..],
                    span.style.clone(),
                ));
            }

            char_offset = span_end;
        }

        if !new_spans.is_empty() {
            line.spans = new_spans;
        }
    }
}

/// Highlight matches in a single line
fn highlight_line(text: &str, pattern: &Regex, style: &SpanStyle) -> Vec<StyledSpan> {
    let matches: Vec<_> = pattern.find_iter(text).collect();

    if matches.is_empty() {
        // No matches, return as-is
        return vec![StyledSpan::plain(text)];
    }

    let mut spans = Vec::new();
    let mut last_end = 0;

    for mat in matches {
        // Add non-matching portion before this match
        if mat.start() > last_end {
            spans.push(StyledSpan::plain(&text[last_end..mat.start()]));
        }

        // Add matching portion with highlight
        spans.push(StyledSpan::new(&text[mat.start()..mat.end()], style.clone()));

        last_end = mat.end();
    }

    // Add remaining text after last match
    if last_end < text.len() {
        spans.push(StyledSpan::plain(&text[last_end..]));
    }

    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_line_single() {
        let pattern = Regex::new("world").unwrap();
        let style = highlight_style();

        let spans = highlight_line("Hello world!", &pattern, &style);

        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "Hello ");
        assert_eq!(spans[1].text, "world");
        assert_eq!(spans[2].text, "!");
        assert_eq!(spans[1].style.bg, Some(Color::Yellow));
    }

    #[test]
    fn test_highlight_line_multiple() {
        let pattern = Regex::new("a").unwrap();
        let style = highlight_style();

        let spans = highlight_line("banana", &pattern, &style);

        // b, a, n, a, n, a = 6 spans
        assert_eq!(spans.len(), 6);
        assert_eq!(spans[0].text, "b");
        assert_eq!(spans[1].text, "a");
        assert_eq!(spans[2].text, "n");
        assert_eq!(spans[3].text, "a");
        assert_eq!(spans[4].text, "n");
        assert_eq!(spans[5].text, "a");
    }

    #[test]
    fn test_highlight_line_no_match() {
        let pattern = Regex::new("xyz").unwrap();
        let style = highlight_style();

        let spans = highlight_line("Hello world", &pattern, &style);

        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "Hello world");
    }

    #[test]
    fn test_search_state_navigation() {
        let pattern = Regex::new("a").unwrap();
        let mut state = SearchState {
            pattern,
            matches: vec![
                MatchPosition {
                    line_idx: 0,
                    start_col: 0,
                    end_col: 1,
                },
                MatchPosition {
                    line_idx: 2,
                    start_col: 3,
                    end_col: 4,
                },
                MatchPosition {
                    line_idx: 5,
                    start_col: 1,
                    end_col: 2,
                },
            ],
            current_match: None,
        };

        // First next goes to first match
        assert_eq!(state.next_match(), Some(0));
        assert_eq!(state.current_match, Some(0));

        // Second next goes to second match
        assert_eq!(state.next_match(), Some(2));
        assert_eq!(state.current_match, Some(1));

        // Third next goes to third match
        assert_eq!(state.next_match(), Some(5));
        assert_eq!(state.current_match, Some(2));

        // Fourth next wraps to first match
        assert_eq!(state.next_match(), Some(0));
        assert_eq!(state.current_match, Some(0));

        // Prev goes back to last
        assert_eq!(state.prev_match(), Some(5));
        assert_eq!(state.current_match, Some(2));
    }
}
