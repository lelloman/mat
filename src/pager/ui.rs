use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line as RatatuiLine, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::cli::WrapMode;
use crate::display::Line;

use super::app::{App, Mode, WrappedLine};

/// Render the main UI
pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Layout: content area + status bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // Content area
            Constraint::Length(1), // Status bar
        ])
        .split(size);

    render_content(frame, app, chunks[0]);
    render_status_bar(frame, app, chunks[1]);
}

/// Render the content area (line numbers + text)
fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    let gutter_width = app.gutter_width();
    let content_width = area.width as usize - gutter_width;

    match app.wrap_mode {
        WrapMode::None => {
            render_content_normal(frame, app, area, gutter_width, content_width);
        }
        WrapMode::Wrap => {
            render_content_wrapped(frame, app, area, gutter_width, content_width);
        }
        WrapMode::Truncate => {
            render_content_truncated(frame, app, area, gutter_width, content_width);
        }
    }
}

/// Render content in normal mode (horizontal scrolling)
fn render_content_normal(frame: &mut Frame, app: &App, area: Rect, gutter_width: usize, content_width: usize) {
    let (start, end) = app.visible_line_range();
    let visible_lines = &app.document.lines[start..end];

    // Split area for gutter and content
    if app.show_line_numbers && gutter_width > 0 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(gutter_width as u16),
                Constraint::Min(1),
            ])
            .split(area);

        // Render line number gutter
        render_gutter(frame, visible_lines, gutter_width, chunks[0], app.theme_colors.line_number);

        // Render content
        render_lines(frame, app, visible_lines, content_width, chunks[1]);
    } else {
        // Render content without gutter
        render_lines(frame, app, visible_lines, content_width, area);
    }
}

/// Render content in wrap mode (soft wrapping)
fn render_content_wrapped(frame: &mut Frame, app: &App, area: Rect, gutter_width: usize, content_width: usize) {
    // Get visible wrapped lines
    let (start, end) = if let Some(ref wrapped) = app.wrapped_lines {
        let start = app.scroll_line;
        let end = (start + app.content_height()).min(wrapped.len());
        (start, end)
    } else {
        // Fallback to normal rendering if wrapped lines not built
        render_content_normal(frame, app, area, gutter_width, content_width);
        return;
    };

    let wrapped_lines = app.wrapped_lines.as_ref().unwrap();
    let visible_wrapped = &wrapped_lines[start..end];

    // Split area for gutter and content
    if app.show_line_numbers && gutter_width > 0 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(gutter_width as u16),
                Constraint::Min(1),
            ])
            .split(area);

        // Render line number gutter for wrapped lines
        render_gutter_wrapped(frame, visible_wrapped, gutter_width, chunks[0], app.theme_colors.line_number);

        // Render wrapped content
        render_wrapped_lines(frame, app, visible_wrapped, content_width, chunks[1]);
    } else {
        // Render wrapped content without gutter
        render_wrapped_lines(frame, app, visible_wrapped, content_width, area);
    }
}

/// Render content in truncate mode (hard truncation)
fn render_content_truncated(frame: &mut Frame, app: &App, area: Rect, gutter_width: usize, content_width: usize) {
    let (start, end) = app.visible_line_range();
    let visible_lines = &app.document.lines[start..end];

    // Split area for gutter and content
    if app.show_line_numbers && gutter_width > 0 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(gutter_width as u16),
                Constraint::Min(1),
            ])
            .split(area);

        // Render line number gutter
        render_gutter(frame, visible_lines, gutter_width, chunks[0], app.theme_colors.line_number);

        // Render truncated content
        render_lines_truncated(frame, app, visible_lines, content_width, chunks[1]);
    } else {
        // Render truncated content without gutter
        render_lines_truncated(frame, app, visible_lines, content_width, area);
    }
}

/// Render the line number gutter
fn render_gutter(frame: &mut Frame, lines: &[Line], gutter_width: usize, area: Rect, line_number_color: Color) {
    let gutter_style = Style::default().fg(line_number_color);

    let gutter_lines: Vec<RatatuiLine> = lines
        .iter()
        .map(|line| {
            let num_str = format!("{:>width$} ", line.number, width = gutter_width - 2);
            RatatuiLine::from(Span::styled(num_str, gutter_style))
        })
        .collect();

    let paragraph = Paragraph::new(gutter_lines);
    frame.render_widget(paragraph, area);
}

/// Render the line number gutter for wrapped lines (only show number for first row)
fn render_gutter_wrapped(frame: &mut Frame, wrapped_lines: &[WrappedLine], gutter_width: usize, area: Rect, line_number_color: Color) {
    let gutter_style = Style::default().fg(line_number_color);

    let gutter_lines: Vec<RatatuiLine> = wrapped_lines
        .iter()
        .map(|wrapped| {
            if wrapped.is_first_row {
                let num_str = format!("{:>width$} ", wrapped.line_number, width = gutter_width - 2);
                RatatuiLine::from(Span::styled(num_str, gutter_style))
            } else {
                // Continuation line - show empty gutter
                let empty_str = " ".repeat(gutter_width);
                RatatuiLine::from(Span::styled(empty_str, gutter_style))
            }
        })
        .collect();

    let paragraph = Paragraph::new(gutter_lines);
    frame.render_widget(paragraph, area);
}

/// Render wrapped lines
fn render_wrapped_lines(frame: &mut Frame, app: &App, wrapped_lines: &[WrappedLine], width: usize, area: Rect) {
    let display_lines: Vec<RatatuiLine> = wrapped_lines
        .iter()
        .map(|wrapped| {
            let line = &app.document.lines[wrapped.line_idx];
            let text = line.text();

            // Get the substring for this wrapped row
            let chars: Vec<char> = text.chars().collect();
            let row_text: String = chars
                .iter()
                .copied()
                .skip(wrapped.char_offset)
                .take_until_width(width)
                .collect();

            if line.spans.is_empty() || line.spans.len() == 1 && line.spans[0].style.is_plain() {
                // Plain text
                let padded = format!("{:width$}", row_text, width = width);
                RatatuiLine::from(Span::raw(padded))
            } else {
                // Styled text - need to extract the right portion of spans
                let ratatui_spans = extract_wrapped_spans(&line.spans, wrapped.char_offset, width);
                RatatuiLine::from(ratatui_spans)
            }
        })
        .collect();

    let paragraph = Paragraph::new(display_lines);
    frame.render_widget(paragraph, area);
}

/// Render lines with hard truncation
fn render_lines_truncated(frame: &mut Frame, app: &App, lines: &[Line], width: usize, area: Rect) {
    let scroll_col = app.scroll_col;
    let truncate_width = app.max_width.min(width);

    let display_lines: Vec<RatatuiLine> = lines
        .iter()
        .map(|line| {
            if line.spans.is_empty() || line.spans.len() == 1 && line.spans[0].style.is_plain() {
                // Simple case: plain text
                let text = line.text();
                let display_text = truncate_with_indicator(&text, scroll_col, truncate_width, width);
                RatatuiLine::from(Span::raw(display_text))
            } else {
                // Styled spans
                let ratatui_spans = truncate_spans_with_indicator(&line.spans, scroll_col, truncate_width, width);
                RatatuiLine::from(ratatui_spans)
            }
        })
        .collect();

    let paragraph = Paragraph::new(display_lines);
    frame.render_widget(paragraph, area);
}

/// Helper trait to take chars until a certain display width
trait TakeUntilWidth: Iterator<Item = char> + Sized {
    fn take_until_width(self, width: usize) -> TakeUntilWidthIter<Self> {
        TakeUntilWidthIter {
            iter: self,
            remaining_width: width,
        }
    }
}

impl<I: Iterator<Item = char>> TakeUntilWidth for I {}

struct TakeUntilWidthIter<I> {
    iter: I,
    remaining_width: usize,
}

impl<I: Iterator<Item = char>> Iterator for TakeUntilWidthIter<I> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let ch = self.iter.next()?;
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if ch_width <= self.remaining_width {
            self.remaining_width -= ch_width;
            Some(ch)
        } else {
            None
        }
    }
}

/// Extract wrapped portion of styled spans
fn extract_wrapped_spans(
    spans: &[crate::display::StyledSpan],
    char_offset: usize,
    width: usize,
) -> Vec<Span<'static>> {
    let mut result = Vec::new();
    let mut current_char = 0;
    let mut chars_taken = 0;

    for span in spans {
        if chars_taken >= width {
            break;
        }

        let mut span_text = String::new();
        let style = span.style.to_ratatui_style();

        for ch in span.text.chars() {
            let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());

            if current_char >= char_offset {
                // We're at or past the offset, start adding characters
                if chars_taken + ch_width <= width {
                    span_text.push(ch);
                    chars_taken += ch_width;
                } else {
                    break;
                }
            }
            current_char += 1;
        }

        if !span_text.is_empty() {
            result.push(Span::styled(span_text, style));
        }
    }

    // Pad with spaces if needed
    if chars_taken < width {
        result.push(Span::raw(" ".repeat(width - chars_taken)));
    }

    result
}

/// Truncate text with an indicator when content is cut off
fn truncate_with_indicator(text: &str, scroll_col: usize, max_width: usize, display_width: usize) -> String {
    let line_width = UnicodeWidthStr::width(text);

    // If the line fits within max_width, use normal truncation
    if line_width <= max_width {
        return truncate_with_scroll(text, scroll_col, display_width);
    }

    // Line exceeds max_width - show truncation indicator
    let effective_width = max_width.saturating_sub(1); // Reserve space for indicator

    let mut result = String::new();
    let mut current_col = 0;
    let mut chars_taken = 0;

    for ch in text.chars() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());

        if current_col >= scroll_col {
            if chars_taken + ch_width <= effective_width {
                result.push(ch);
                chars_taken += ch_width;
            } else {
                break;
            }
        } else if current_col + ch_width > scroll_col {
            let overlap = current_col + ch_width - scroll_col;
            for _ in 0..overlap {
                if chars_taken < effective_width {
                    result.push(' ');
                    chars_taken += 1;
                }
            }
        }

        current_col += ch_width;
    }

    // Add truncation indicator
    result.push('…');
    chars_taken += 1;

    // Pad to display width
    while chars_taken < display_width {
        result.push(' ');
        chars_taken += 1;
    }

    result
}

/// Truncate styled spans with indicator
fn truncate_spans_with_indicator(
    spans: &[crate::display::StyledSpan],
    scroll_col: usize,
    max_width: usize,
    display_width: usize,
) -> Vec<Span<'static>> {
    // Calculate total line width
    let line_width: usize = spans.iter().map(|s| s.width()).sum();

    // If the line fits within max_width, use normal truncation
    if line_width <= max_width {
        return truncate_spans_with_scroll(spans, scroll_col, display_width);
    }

    // Line exceeds max_width - show truncation indicator
    let effective_width = max_width.saturating_sub(1);

    let mut result = Vec::new();
    let mut current_col = 0;
    let mut chars_taken = 0;

    for span in spans {
        if chars_taken >= effective_width {
            break;
        }

        let mut span_text = String::new();
        let style = span.style.to_ratatui_style();

        for ch in span.text.chars() {
            let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());

            if current_col >= scroll_col {
                if chars_taken + ch_width <= effective_width {
                    span_text.push(ch);
                    chars_taken += ch_width;
                } else {
                    break;
                }
            } else if current_col + ch_width > scroll_col {
                let overlap = current_col + ch_width - scroll_col;
                for _ in 0..overlap {
                    if chars_taken < effective_width {
                        span_text.push(' ');
                        chars_taken += 1;
                    }
                }
            }

            current_col += ch_width;
        }

        if !span_text.is_empty() {
            result.push(Span::styled(span_text, style));
        }
    }

    // Add truncation indicator
    result.push(Span::styled("…", Style::default().fg(Color::DarkGray)));
    chars_taken += 1;

    // Pad to display width
    if chars_taken < display_width {
        result.push(Span::raw(" ".repeat(display_width - chars_taken)));
    }

    result
}

/// Render the text lines
fn render_lines(frame: &mut Frame, app: &App, lines: &[Line], width: usize, area: Rect) {
    let scroll_col = app.scroll_col;

    let display_lines: Vec<RatatuiLine> = lines
        .iter()
        .map(|line| {
            if line.spans.is_empty() || line.spans.len() == 1 && line.spans[0].style.is_plain() {
                // Simple case: plain text, use fast path
                let text = line.text();
                let display_text = truncate_with_scroll(&text, scroll_col, width);
                RatatuiLine::from(Span::raw(display_text))
            } else {
                // Styled spans: need to handle scrolling across span boundaries
                let ratatui_spans = truncate_spans_with_scroll(&line.spans, scroll_col, width);
                RatatuiLine::from(ratatui_spans)
            }
        })
        .collect();

    let paragraph = Paragraph::new(display_lines);
    frame.render_widget(paragraph, area);
}

/// Truncate styled spans for horizontal scrolling
fn truncate_spans_with_scroll(
    spans: &[crate::display::StyledSpan],
    scroll_col: usize,
    width: usize,
) -> Vec<Span<'static>> {
    let mut result = Vec::new();
    let mut current_col = 0;
    let mut chars_taken = 0;

    for span in spans {
        if chars_taken >= width {
            break;
        }

        let mut span_text = String::new();
        let style = span.style.to_ratatui_style();

        for ch in span.text.chars() {
            let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());

            if current_col >= scroll_col {
                // We're past the scroll offset, start adding characters
                if chars_taken + ch_width <= width {
                    span_text.push(ch);
                    chars_taken += ch_width;
                } else {
                    break;
                }
            } else if current_col + ch_width > scroll_col {
                // Character spans the scroll boundary - add spaces for partial overlap
                let overlap = current_col + ch_width - scroll_col;
                for _ in 0..overlap {
                    if chars_taken < width {
                        span_text.push(' ');
                        chars_taken += 1;
                    }
                }
            }

            current_col += ch_width;
        }

        if !span_text.is_empty() {
            result.push(Span::styled(span_text, style));
        }
    }

    // Pad with spaces if needed
    if chars_taken < width {
        result.push(Span::raw(" ".repeat(width - chars_taken)));
    }

    result
}

/// Truncate text for horizontal scrolling
fn truncate_with_scroll(text: &str, scroll_col: usize, width: usize) -> String {
    // Convert to grapheme-aware iteration
    let mut result = String::new();
    let mut current_col = 0;
    let mut chars_taken = 0;

    for ch in text.chars() {
        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());

        if current_col >= scroll_col {
            // We're past the scroll offset, start adding characters
            if chars_taken + ch_width <= width {
                result.push(ch);
                chars_taken += ch_width;
            } else {
                break;
            }
        } else if current_col + ch_width > scroll_col {
            // Character spans the scroll boundary - add spaces for partial overlap
            let overlap = current_col + ch_width - scroll_col;
            for _ in 0..overlap {
                if chars_taken < width {
                    result.push(' ');
                    chars_taken += 1;
                }
            }
        }

        current_col += ch_width;
    }

    // Pad with spaces if needed (for consistent line length)
    while result.len() < width {
        result.push(' ');
    }

    result
}

/// Render the status bar
fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let style = Style::default()
        .bg(app.theme_colors.status_bg)
        .fg(app.theme_colors.status_fg);

    // Left: file name and position
    let position = format!(
        " {} ({}/{}) ",
        app.document.source_name,
        app.current_line_display(),
        app.total_lines()
    );

    // Center: mode indicator and search info
    let mode_str = match &app.mode {
        Mode::Normal => {
            let mut indicators = Vec::new();

            // Show wrap mode indicator
            match app.wrap_mode {
                WrapMode::Wrap => indicators.push("[WRAP]".to_string()),
                WrapMode::Truncate => indicators.push("[TRUNC]".to_string()),
                WrapMode::None => {}
            }

            // Show follow mode indicator
            if app.follow_mode {
                indicators.push("[FOLLOW]".to_string());
            }

            // Show search match info if available
            if let Some((current, total)) = app.search_info() {
                indicators.push(format!("Match {}/{}", current, total));
            }

            if indicators.is_empty() {
                String::new()
            } else {
                format!(" {} ", indicators.join(" | "))
            }
        }
        Mode::Search { query } => format!(" [SEARCH: {}] ", query),
    };

    // Right: column info and encoding (only show column info when not in wrap mode)
    let right = match app.wrap_mode {
        WrapMode::Wrap => {
            // No column info in wrap mode
            if app.document.encoding != "UTF-8" {
                format!("{} ", app.document.encoding)
            } else {
                String::new()
            }
        }
        _ => {
            // Show column info in normal and truncate modes
            if app.document.encoding != "UTF-8" {
                format!(
                    "Col {}/{} | {} ",
                    app.scroll_col + 1,
                    app.document.max_line_width,
                    app.document.encoding
                )
            } else {
                format!("Col {}/{} ", app.scroll_col + 1, app.document.max_line_width)
            }
        }
    };

    // Calculate spacing
    let total_width = area.width as usize;
    let left_len = position.len();
    let mode_len = mode_str.len();
    let right_len = right.len();

    let available_space = total_width.saturating_sub(left_len + mode_len + right_len);
    let left_padding = available_space / 2;
    let right_padding = available_space - left_padding;

    let status_text = format!(
        "{}{}{}{}{}",
        position,
        " ".repeat(left_padding),
        mode_str,
        " ".repeat(right_padding),
        right
    );

    // Truncate if too long
    let status_text: String = status_text.chars().take(total_width).collect();

    // Pad if too short
    let status_text = format!("{:width$}", status_text, width = total_width);

    let paragraph = Paragraph::new(RatatuiLine::from(Span::styled(
        status_text,
        style.add_modifier(Modifier::BOLD),
    )));
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_with_scroll() {
        assert_eq!(truncate_with_scroll("Hello World", 0, 5), "Hello");
        assert_eq!(truncate_with_scroll("Hello World", 6, 5), "World");
        assert_eq!(truncate_with_scroll("Hello World", 0, 20), "Hello World         ");
    }

    #[test]
    fn test_truncate_cjk() {
        let text = "Hello世界";
        // "Hello" = 5 cols, each CJK = 2 cols
        assert_eq!(truncate_with_scroll(text, 0, 7), "Hello世"); // 5 + 2 = 7
    }

    #[test]
    fn test_truncate_with_indicator() {
        // Text fits within max_width - no indicator
        let result = truncate_with_indicator("Hello", 0, 10, 15);
        assert!(result.starts_with("Hello"));
        assert!(!result.contains('…'));

        // Text exceeds max_width - should have indicator
        let result = truncate_with_indicator("Hello World This Is Long", 0, 10, 15);
        assert!(result.contains('…'));
    }

    #[test]
    fn test_take_until_width_iterator() {
        let chars: Vec<char> = "Hello World".chars().collect();
        let result: String = chars.iter().copied().take_until_width(5).collect();
        assert_eq!(result, "Hello");

        // Test with CJK - each CJK char is 2 columns
        let chars: Vec<char> = "Hello世界".chars().collect();
        let result: String = chars.iter().copied().take_until_width(7).collect();
        assert_eq!(result, "Hello世");
    }
}
