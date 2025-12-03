use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::style::Color;

use crate::display::{Document, Line, SpanStyle, StyledSpan};

/// Render markdown text to a styled document
pub fn render_markdown(text: &str, source_name: String) -> Document {
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(text, options);

    let mut renderer = MarkdownRenderer::new();
    renderer.render(parser);

    let lines = renderer.into_lines();
    let max_width = lines.iter().map(|l| l.width()).max().unwrap_or(0);

    Document {
        lines,
        max_line_width: max_width,
        source_name,
        encoding: "UTF-8".to_string(),
    }
}

/// Internal renderer state
struct MarkdownRenderer {
    /// Accumulated lines
    lines: Vec<Line>,
    /// Current line being built
    current_line: Vec<StyledSpan>,
    /// Current line number
    line_number: usize,
    /// Current style stack (for nested formatting)
    style_stack: Vec<SpanStyle>,
    /// Whether we're in a code block
    in_code_block: bool,
    /// Whether we're in a blockquote
    in_blockquote: bool,
    /// Current list depth
    list_depth: usize,
    /// List item counters for ordered lists (per depth)
    list_counters: Vec<usize>,
    /// Whether current list item at each depth is ordered
    list_ordered: Vec<bool>,
    /// Whether we just started a list item (for bullet/number prefix)
    needs_list_prefix: bool,
    /// Current heading level (for adding underlines)
    current_heading: Option<HeadingLevel>,
}

impl MarkdownRenderer {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            current_line: Vec::new(),
            line_number: 1,
            style_stack: vec![SpanStyle::default()],
            in_code_block: false,
            in_blockquote: false,
            list_depth: 0,
            list_counters: Vec::new(),
            list_ordered: Vec::new(),
            needs_list_prefix: false,
            current_heading: None,
        }
    }

    fn render<'a>(&mut self, parser: Parser<'a>) {
        for event in parser {
            self.handle_event(event);
        }

        // Flush any remaining content (only if there's content)
        if !self.current_line.is_empty() {
            self.flush_line();
        }
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start_tag(tag),
            Event::End(tag_end) => self.end_tag(tag_end),
            Event::Text(text) => self.add_text(&text),
            Event::Code(code) => self.add_inline_code(&code),
            Event::SoftBreak => self.add_text(" "),
            Event::HardBreak => self.new_line(),
            Event::Rule => self.add_horizontal_rule(),
            Event::TaskListMarker(checked) => self.add_task_marker(checked),
            Event::FootnoteReference(_) => {} // Skip footnotes for now
            Event::Html(_) => {}              // Skip raw HTML
            Event::InlineHtml(_) => {}        // Skip inline HTML
            Event::InlineMath(_) => {}        // Skip math for now
            Event::DisplayMath(_) => {}       // Skip math for now
        }
    }

    fn start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Heading { level, .. } => {
                // Only flush if there's content (to avoid empty lines at start)
                if !self.current_line.is_empty() || !self.lines.is_empty() {
                    self.flush_line();
                }
                // Store heading level for decorations in end_tag
                self.current_heading = Some(level);

                // Add top border for H1
                if level == HeadingLevel::H1 {
                    let border_style = SpanStyle::new().fg(Color::Yellow);
                    self.add_styled_text("╔", border_style.clone());
                    self.add_styled_text(&"═".repeat(50), border_style.clone());
                    self.add_styled_text("╗", border_style);
                    self.flush_line();
                    // Add side border prefix
                    let side_style = SpanStyle::new().fg(Color::Yellow);
                    self.add_styled_text("║  ", side_style);
                } else if level == HeadingLevel::H2 {
                    // H2 gets inline prefix decoration
                    let decor_style = SpanStyle::new().fg(Color::Blue);
                    self.add_styled_text("──◈ ", decor_style);
                } else {
                    // Other levels get simple prefix
                    let (prefix, prefix_style) = self.heading_prefix(level);
                    if !prefix.is_empty() {
                        self.add_styled_text(prefix, prefix_style);
                    }
                }
                // Apply heading style
                let style = self.heading_style(level);
                self.push_style(style);
            }
            Tag::Paragraph => {
                // Add blank line before paragraph (unless at start or in list)
                if !self.lines.is_empty() && self.list_depth == 0 && !self.in_blockquote {
                    self.flush_line();
                }
            }
            Tag::BlockQuote(_) => {
                self.flush_line();
                self.in_blockquote = true;
                self.add_blockquote_prefix();
            }
            Tag::CodeBlock(kind) => {
                self.flush_line();
                self.in_code_block = true;

                // Add a visual indicator for code blocks (a subtle box top)
                let style = SpanStyle::new().fg(Color::DarkGray);
                if let CodeBlockKind::Fenced(lang) = kind {
                    if !lang.is_empty() {
                        self.add_styled_text(&format!("─── {} ", lang), style.clone());
                        // Fill to make it look like a box
                        self.add_styled_text(&"─".repeat(30), style);
                    } else {
                        self.add_styled_text(&"─".repeat(40), style);
                    }
                } else {
                    self.add_styled_text(&"─".repeat(40), style);
                }
                self.flush_line();
            }
            Tag::List(start) => {
                if self.list_depth == 0 && (!self.current_line.is_empty() || !self.lines.is_empty()) {
                    self.flush_line();
                }
                self.list_depth += 1;
                self.list_ordered.push(start.is_some());
                self.list_counters.push(start.unwrap_or(1) as usize);
            }
            Tag::Item => {
                // Only flush if there's content
                if !self.current_line.is_empty() || !self.lines.is_empty() {
                    self.flush_line();
                }
                self.needs_list_prefix = true;
            }
            Tag::Emphasis => {
                let style = SpanStyle::new().fg(Color::Yellow);
                self.push_style(style);
            }
            Tag::Strong => {
                let mut style = SpanStyle::new();
                style.bold = true;
                self.push_style(style);
            }
            Tag::Strikethrough => {
                let style = SpanStyle::new().fg(Color::DarkGray);
                self.push_style(style);
            }
            Tag::Link { .. } => {
                // Style the link text with blue underline, no brackets
                let style = SpanStyle::new().fg(Color::Blue).underline();
                self.push_style(style);
            }
            Tag::Image { .. } => {
                let style = SpanStyle::new().fg(Color::Magenta);
                self.add_styled_text("[Image: ", style.clone());
                self.push_style(style);
            }
            Tag::Table(_) => {
                self.flush_line();
            }
            Tag::TableHead | Tag::TableRow | Tag::TableCell => {}
            Tag::FootnoteDefinition(_) => {}
            Tag::MetadataBlock(_) => {}
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition => {}
            Tag::HtmlBlock => {}
        }
    }

    fn end_tag(&mut self, tag_end: TagEnd) {
        match tag_end {
            TagEnd::Heading(_) => {
                self.pop_style();
                // Add decorations based on heading level
                if let Some(level) = self.current_heading.take() {
                    match level {
                        HeadingLevel::H1 => {
                            self.flush_line();
                            // Bottom border for the frame
                            let border_style = SpanStyle::new().fg(Color::Yellow);
                            self.add_styled_text("╚", border_style.clone());
                            self.add_styled_text(&"═".repeat(50), border_style.clone());
                            self.add_styled_text("╝", border_style);
                            self.flush_line();
                        }
                        HeadingLevel::H2 => {
                            // Trailing decoration on same line
                            let decor_style = SpanStyle::new().fg(Color::Blue);
                            self.add_styled_text(" ◈", decor_style.clone());
                            self.add_styled_text(&"─".repeat(30), decor_style);
                            self.flush_line();
                        }
                        _ => {
                            self.flush_line();
                        }
                    }
                } else {
                    self.flush_line();
                }
                // Add blank line after heading
                self.lines.push(Line::plain(self.line_number, ""));
                self.line_number += 1;
            }
            TagEnd::Paragraph => {
                self.flush_line();
            }
            TagEnd::BlockQuote(_) => {
                self.in_blockquote = false;
                self.flush_line();
            }
            TagEnd::CodeBlock => {
                self.in_code_block = false;
                // Add bottom border for code block
                let style = SpanStyle::new().fg(Color::DarkGray);
                self.add_styled_text(&"─".repeat(40), style);
                self.flush_line();
            }
            TagEnd::List(_) => {
                self.list_depth = self.list_depth.saturating_sub(1);
                self.list_counters.pop();
                self.list_ordered.pop();
                if self.list_depth == 0 {
                    self.flush_line();
                }
            }
            TagEnd::Item => {}
            TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => {
                self.pop_style();
            }
            TagEnd::Link => {
                self.pop_style();
            }
            TagEnd::Image => {
                self.pop_style();
                self.current_line.push(StyledSpan::new("]", SpanStyle::new().fg(Color::Magenta)));
            }
            TagEnd::Table => {}
            TagEnd::TableHead | TagEnd::TableRow => {
                self.flush_line();
            }
            TagEnd::TableCell => {
                self.add_text(" | ");
            }
            TagEnd::FootnoteDefinition => {}
            TagEnd::MetadataBlock(_) => {}
            TagEnd::DefinitionList
            | TagEnd::DefinitionListTitle
            | TagEnd::DefinitionListDefinition => {}
            TagEnd::HtmlBlock => {}
        }
    }

    fn add_text(&mut self, text: &str) {
        // Handle list prefix if needed
        if self.needs_list_prefix {
            self.add_list_prefix();
            self.needs_list_prefix = false;
        }

        if self.in_code_block {
            // Code block: preserve formatting with monospace style
            let style = SpanStyle::new().fg(Color::Green);
            for line in text.split('\n') {
                if !self.current_line.is_empty() {
                    self.flush_line();
                }
                self.add_styled_text(line, style.clone());
            }
        } else if self.in_blockquote {
            // Handle blockquote text (may contain newlines)
            for (i, line) in text.split('\n').enumerate() {
                if i > 0 {
                    self.flush_line();
                    self.add_blockquote_prefix();
                }
                self.add_styled_text(line, self.current_style());
            }
        } else {
            // Normal text
            self.add_styled_text(text, self.current_style());
        }
    }

    fn add_inline_code(&mut self, code: &str) {
        // Show inline code with cyan color, no backticks
        let style = SpanStyle::new().fg(Color::Cyan);
        self.current_line.push(StyledSpan::new(code, style));
    }

    fn add_horizontal_rule(&mut self) {
        self.flush_line();
        let style = SpanStyle::new().fg(Color::DarkGray);
        self.add_styled_text("─".repeat(40).as_str(), style);
        self.flush_line();
    }

    fn add_task_marker(&mut self, checked: bool) {
        let marker = if checked { "[x] " } else { "[ ] " };
        let style = SpanStyle::new().fg(Color::Magenta);
        self.add_styled_text(marker, style);
    }

    fn add_list_prefix(&mut self) {
        let indent = "  ".repeat(self.list_depth.saturating_sub(1));

        if let Some(&ordered) = self.list_ordered.last() {
            if ordered {
                // Ordered list
                let counter = self.list_counters.last().copied().unwrap_or(1);
                let prefix = format!("{}{}. ", indent, counter);
                let style = SpanStyle::new().fg(Color::Yellow);
                self.add_styled_text(&prefix, style);

                // Increment counter
                if let Some(c) = self.list_counters.last_mut() {
                    *c += 1;
                }
            } else {
                // Unordered list
                let bullet = match self.list_depth {
                    1 => "• ",
                    2 => "◦ ",
                    _ => "▪ ",
                };
                let prefix = format!("{}{}", indent, bullet);
                let style = SpanStyle::new().fg(Color::Yellow);
                self.add_styled_text(&prefix, style);
            }
        }
    }

    fn add_blockquote_prefix(&mut self) {
        let style = SpanStyle::new().fg(Color::DarkGray);
        self.current_line.push(StyledSpan::new("│ ", style));
    }

    fn add_styled_text(&mut self, text: &str, style: SpanStyle) {
        if !text.is_empty() {
            self.current_line.push(StyledSpan::new(text, style));
        }
    }

    fn heading_style(&self, level: HeadingLevel) -> SpanStyle {
        match level {
            HeadingLevel::H1 => SpanStyle::new().fg(Color::White).bold(),
            HeadingLevel::H2 => SpanStyle::new().fg(Color::Cyan).bold(),
            HeadingLevel::H3 => SpanStyle::new().fg(Color::Green).bold(),
            HeadingLevel::H4 => SpanStyle::new().fg(Color::Magenta).bold(),
            HeadingLevel::H5 => SpanStyle::new().fg(Color::Yellow).bold(),
            HeadingLevel::H6 => SpanStyle::new().fg(Color::DarkGray).bold(),
        }
    }

    fn heading_prefix(&self, level: HeadingLevel) -> (&'static str, SpanStyle) {
        // H1 and H2 are handled separately with frames/decorations
        match level {
            HeadingLevel::H1 => ("", SpanStyle::default()),
            HeadingLevel::H2 => ("", SpanStyle::default()),
            HeadingLevel::H3 => ("▸ ", SpanStyle::new().fg(Color::Green).bold()),
            HeadingLevel::H4 => ("◆ ", SpanStyle::new().fg(Color::Magenta).bold()),
            HeadingLevel::H5 => ("◇ ", SpanStyle::new().fg(Color::Yellow).bold()),
            HeadingLevel::H6 => ("· ", SpanStyle::new().fg(Color::DarkGray).bold()),
        }
    }

    fn current_style(&self) -> SpanStyle {
        self.style_stack.last().cloned().unwrap_or_default()
    }

    fn push_style(&mut self, style: SpanStyle) {
        // Merge with current style
        let current = self.current_style();
        let merged = SpanStyle {
            fg: style.fg.or(current.fg),
            bg: style.bg.or(current.bg),
            bold: style.bold || current.bold,
            italic: style.italic || current.italic,
            underline: style.underline || current.underline,
        };
        self.style_stack.push(merged);
    }

    fn pop_style(&mut self) {
        if self.style_stack.len() > 1 {
            self.style_stack.pop();
        }
    }

    fn flush_line(&mut self) {
        if self.current_line.is_empty() {
            // Empty line
            self.lines.push(Line::plain(self.line_number, ""));
        } else {
            let spans = std::mem::take(&mut self.current_line);
            self.lines.push(Line {
                number: self.line_number,
                spans,
                is_match: false,
                is_context: false,
            });
        }
        self.line_number += 1;
    }

    fn new_line(&mut self) {
        self.flush_line();
        if self.in_blockquote {
            self.add_blockquote_prefix();
        }
    }

    fn into_lines(self) -> Vec<Line> {
        self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_heading() {
        let md = "# Hello World";
        let doc = render_markdown(md, "test.md".to_string());

        assert!(!doc.lines.is_empty(), "Document should have lines");
        // H1 now has a frame, so "Hello World" is on line 1 (after top border)
        let all_text: String = doc.lines.iter().map(|l| l.text()).collect();
        assert!(all_text.contains("Hello World"), "Expected 'Hello World' in document");
    }

    #[test]
    fn test_render_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let doc = render_markdown(md, "test.md".to_string());

        // Should have code block markers and content
        assert!(doc.lines.len() >= 3);
    }

    #[test]
    fn test_render_list() {
        let md = "- Item 1\n- Item 2\n- Item 3";
        let doc = render_markdown(md, "test.md".to_string());

        assert!(doc.lines.len() >= 3);
        let text = doc.lines[0].text();
        assert!(text.contains("Item 1"));
    }

    #[test]
    fn test_render_inline_code() {
        let md = "Use `println!` to print";
        let doc = render_markdown(md, "test.md".to_string());

        let text = doc.lines[0].text();
        assert!(text.contains("println!"));
    }

    #[test]
    fn test_render_emphasis() {
        let md = "This is *italic* and **bold**";
        let doc = render_markdown(md, "test.md".to_string());

        let text = doc.lines[0].text();
        assert!(text.contains("italic"));
        assert!(text.contains("bold"));
    }
}
