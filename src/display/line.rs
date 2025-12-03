use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthStr;

/// Style for a span of text
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpanStyle {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl SpanStyle {
    /// Create a new default style
    pub fn new() -> Self {
        Self::default()
    }

    /// Set foreground color
    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    /// Set background color
    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set bold
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Set italic
    #[allow(dead_code)]
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Set underline
    pub fn underline(mut self) -> Self {
        self.underline = true;
        self
    }

    /// Convert to ratatui Style
    pub fn to_ratatui_style(&self) -> Style {
        let mut style = Style::default();

        if let Some(fg) = self.fg {
            style = style.fg(fg);
        }
        if let Some(bg) = self.bg {
            style = style.bg(bg);
        }

        let mut modifiers = Modifier::empty();
        if self.bold {
            modifiers |= Modifier::BOLD;
        }
        if self.italic {
            modifiers |= Modifier::ITALIC;
        }
        if self.underline {
            modifiers |= Modifier::UNDERLINED;
        }

        if !modifiers.is_empty() {
            style = style.add_modifier(modifiers);
        }

        style
    }

    /// Check if this is the default plain style (no styling)
    pub fn is_plain(&self) -> bool {
        self.fg.is_none()
            && self.bg.is_none()
            && !self.bold
            && !self.italic
            && !self.underline
    }
}

/// A span of styled text
#[derive(Debug, Clone, PartialEq)]
pub struct StyledSpan {
    pub text: String,
    pub style: SpanStyle,
}

impl StyledSpan {
    /// Create a new styled span
    pub fn new(text: impl Into<String>, style: SpanStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    /// Create a plain (unstyled) span
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: SpanStyle::default(),
        }
    }

    /// Get the display width of this span
    pub fn width(&self) -> usize {
        UnicodeWidthStr::width(self.text.as_str())
    }
}

/// A line of styled text with metadata
#[derive(Debug, Clone)]
pub struct Line {
    /// Original line number (1-indexed)
    pub number: usize,
    /// Styled spans making up the line
    pub spans: Vec<StyledSpan>,
    /// Whether this line is a grep match
    pub is_match: bool,
    /// Whether this line is grep context (for future use with context styling)
    #[allow(dead_code)]
    pub is_context: bool,
}

impl Line {
    /// Create a plain (unstyled) line
    pub fn plain(number: usize, text: &str) -> Self {
        Self {
            number,
            spans: vec![StyledSpan::plain(text)],
            is_match: false,
            is_context: false,
        }
    }

    /// Create a separator line (used between grep groups)
    pub fn separator() -> Self {
        Self {
            number: 0,
            spans: vec![StyledSpan::new(
                "--",
                SpanStyle::new().fg(Color::DarkGray),
            )],
            is_match: false,
            is_context: false,
        }
    }

    /// Get the display width of this line
    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| s.width()).sum()
    }

    /// Get the raw text content of this line
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

/// A document containing multiple lines
#[derive(Debug, Clone)]
pub struct Document {
    /// All lines in the document
    pub lines: Vec<Line>,
    /// Maximum line width in the document
    pub max_line_width: usize,
    /// Source name (filename or "stdin")
    pub source_name: String,
    /// Detected encoding
    pub encoding: String,
}

impl Document {
    /// Create a document from text content
    pub fn from_text(text: &str, source_name: String, encoding: String) -> Self {
        let lines: Vec<Line> = text
            .lines()
            .enumerate()
            .map(|(i, line_text)| Line::plain(i + 1, line_text))
            .collect();

        let max_line_width = lines.iter().map(|l| l.width()).max().unwrap_or(0);

        Self {
            lines,
            max_line_width,
            source_name,
            encoding,
        }
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Recalculate max line width
    pub fn recalculate_max_width(&mut self) {
        self.max_line_width = self.lines.iter().map(|l| l.width()).max().unwrap_or(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_style_builder() {
        let style = SpanStyle::new().fg(Color::Red).bold().underline();
        assert_eq!(style.fg, Some(Color::Red));
        assert!(style.bold);
        assert!(style.underline);
        assert!(!style.italic);
    }

    #[test]
    fn test_styled_span_width() {
        let span = StyledSpan::plain("Hello");
        assert_eq!(span.width(), 5);

        let cjk = StyledSpan::plain("世界");
        assert_eq!(cjk.width(), 4); // Each CJK char is 2 columns wide
    }

    #[test]
    fn test_line_width() {
        let line = Line::plain(1, "Hello, World!");
        assert_eq!(line.width(), 13);
    }

    #[test]
    fn test_line_text() {
        let line = Line {
            number: 1,
            spans: vec![
                StyledSpan::plain("Hello"),
                StyledSpan::plain(", "),
                StyledSpan::plain("World!"),
            ],
            is_match: false,
            is_context: false,
        };
        assert_eq!(line.text(), "Hello, World!");
    }

    #[test]
    fn test_document_from_text() {
        let text = "Line 1\nLine 2\nLine 3";
        let doc = Document::from_text(text, "test.txt".to_string(), "UTF-8".to_string());

        assert_eq!(doc.line_count(), 3);
        assert_eq!(doc.lines[0].number, 1);
        assert_eq!(doc.lines[1].number, 2);
        assert_eq!(doc.lines[2].number, 3);
        assert_eq!(doc.max_line_width, 6);
    }

    #[test]
    fn test_empty_document() {
        let doc = Document::from_text("", "test.txt".to_string(), "UTF-8".to_string());
        assert_eq!(doc.line_count(), 0);
        assert_eq!(doc.max_line_width, 0);
    }
}
