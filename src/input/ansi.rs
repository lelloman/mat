use ratatui::style::Color;
use unicode_width::UnicodeWidthChar;

use crate::display::{Document, Line, SpanStyle, StyledSpan};

/// Parse only Select Graphic Rendition (SGR) sequences. All other terminal
/// controls are discarded so file content cannot take control of the terminal.
pub fn parse_ansi(text: &str, source_name: String, encoding: String) -> Document {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut style = SpanStyle::default();
    let mut column = 0;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\x1b' => {
                flush(&mut spans, &mut buffer, &style);
                consume_escape(&mut chars, &mut style);
            }
            '\n' => {
                flush(&mut spans, &mut buffer, &style);
                lines.push(Line {
                    number: lines.len() + 1,
                    spans: std::mem::take(&mut spans),
                    is_match: false,
                    is_context: false,
                });
                column = 0;
            }
            '\t' => {
                let spaces = 4 - column % 4;
                buffer.extend(std::iter::repeat_n(' ', spaces));
                column += spaces;
            }
            '\r' => {}
            c if c.is_control() => {}
            c => {
                buffer.push(c);
                column += UnicodeWidthChar::width(c).unwrap_or(0);
            }
        }
    }
    flush(&mut spans, &mut buffer, &style);
    if !spans.is_empty() || (!text.is_empty() && !text.ends_with('\n')) {
        lines.push(Line {
            number: lines.len() + 1,
            spans,
            is_match: false,
            is_context: false,
        });
    }

    let max_line_width = lines.iter().map(Line::width).max().unwrap_or(0);
    Document {
        lines,
        max_line_width,
        source_name,
        encoding,
    }
}

fn flush(spans: &mut Vec<StyledSpan>, buffer: &mut String, style: &SpanStyle) {
    if !buffer.is_empty() {
        spans.push(StyledSpan::new(std::mem::take(buffer), style.clone()));
    }
}

fn consume_escape<I>(chars: &mut std::iter::Peekable<I>, style: &mut SpanStyle)
where
    I: Iterator<Item = char>,
{
    match chars.next() {
        Some('[') => {
            let mut params = String::new();
            while let Some(&next) = chars.peek() {
                chars.next();
                if ('@'..='~').contains(&next) {
                    if next == 'm' {
                        apply_sgr(&params, style);
                    }
                    break;
                }
                if params.len() < 128 {
                    params.push(next);
                }
            }
        }
        Some(']') => {
            let mut previous_escape = false;
            for next in chars.by_ref() {
                if next == '\x07' || (previous_escape && next == '\\') {
                    break;
                }
                previous_escape = next == '\x1b';
            }
        }
        Some(_) | None => {}
    }
}

fn apply_sgr(params: &str, style: &mut SpanStyle) {
    let values: Vec<u16> = if params.is_empty() {
        vec![0]
    } else {
        params
            .split(';')
            .filter_map(|part| part.parse().ok())
            .collect()
    };
    let mut index = 0;
    while index < values.len() {
        let code = values[index];
        match code {
            0 => *style = SpanStyle::default(),
            1 => style.bold = true,
            2 => style.dim = true,
            3 => style.italic = true,
            4 => style.underline = true,
            22 => {
                style.bold = false;
                style.dim = false;
            }
            23 => style.italic = false,
            24 => style.underline = false,
            30..=37 => style.fg = Some(basic_color(code - 30, false)),
            39 => style.fg = None,
            40..=47 => style.bg = Some(basic_color(code - 40, false)),
            49 => style.bg = None,
            90..=97 => style.fg = Some(basic_color(code - 90, true)),
            100..=107 => style.bg = Some(basic_color(code - 100, true)),
            38 | 48 => {
                let target = if code == 38 {
                    &mut style.fg
                } else {
                    &mut style.bg
                };
                if values.get(index + 1) == Some(&5) {
                    if let Some(&value) = values.get(index + 2) {
                        *target = Some(Color::Indexed(value.min(255) as u8));
                        index += 2;
                    }
                } else if values.get(index + 1) == Some(&2) {
                    if let (Some(&r), Some(&g), Some(&b)) = (
                        values.get(index + 2),
                        values.get(index + 3),
                        values.get(index + 4),
                    ) {
                        *target = Some(Color::Rgb(
                            r.min(255) as u8,
                            g.min(255) as u8,
                            b.min(255) as u8,
                        ));
                        index += 4;
                    }
                }
            }
            _ => {}
        }
        index += 1;
    }
}

fn basic_color(value: u16, bright: bool) -> Color {
    const NORMAL: [Color; 8] = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::Gray,
    ];
    const BRIGHT: [Color; 8] = [
        Color::DarkGray,
        Color::LightRed,
        Color::LightGreen,
        Color::LightYellow,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];
    if bright {
        BRIGHT[value as usize]
    } else {
        NORMAL[value as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_sgr_and_discards_terminal_controls() {
        let doc = parse_ansi(
            "\x1b[31mred\x1b[0m\x1b[2J\x1b]0;owned\x07safe",
            "test".into(),
            "UTF-8".into(),
        );
        assert_eq!(doc.lines[0].text(), "redsafe");
        assert_eq!(doc.lines[0].spans[0].style.fg, Some(Color::Red));
    }
}
