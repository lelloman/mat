use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line as RatatuiLine, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::display::Line;

use super::app::{App, Mode};

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
    let (start, end) = app.visible_line_range();
    let visible_lines = &app.document.lines[start..end];

    let gutter_width = app.gutter_width();
    let content_width = area.width as usize - gutter_width;

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

/// Render the text lines
fn render_lines(frame: &mut Frame, app: &App, lines: &[Line], width: usize, area: Rect) {
    let scroll_col = app.scroll_col;

    let display_lines: Vec<RatatuiLine> = lines
        .iter()
        .map(|line| {
            let text = line.text();
            let display_text = truncate_with_scroll(&text, scroll_col, width);

            // Convert spans to ratatui spans
            let spans: Vec<Span> = line
                .spans
                .iter()
                .map(|span| Span::styled(span.text.clone(), span.style.to_ratatui_style()))
                .collect();

            // For now, use simple text (styling will be added in later phases)
            if spans.is_empty() {
                RatatuiLine::from(Span::raw(display_text))
            } else {
                // Apply horizontal scroll to the styled content
                RatatuiLine::from(Span::raw(display_text))
            }
        })
        .collect();

    let paragraph = Paragraph::new(display_lines);
    frame.render_widget(paragraph, area);
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
            // Show search match info if available
            if let Some((current, total)) = app.search_info() {
                format!(" Match {}/{} ", current, total)
            } else {
                String::new()
            }
        }
        Mode::Search { query } => format!(" [SEARCH: {}] ", query),
    };

    // Right: column info and encoding
    let right = if app.document.encoding != "UTF-8" {
        format!(
            "Col {}/{} | {} ",
            app.scroll_col + 1,
            app.document.max_line_width,
            app.document.encoding
        )
    } else {
        format!("Col {}/{} ", app.scroll_col + 1, app.document.max_line_width)
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
}
