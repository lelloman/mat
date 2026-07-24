use regex::Regex;

use crate::display::Document;
use crate::error::MatError;
use crate::filter::{apply_grep_highlight, grep_filter, GrepOptions};
use crate::highlight::{apply_search_highlight, apply_syntax_highlight};
use crate::input::{expand_tabs, parse_ansi, strip_ansi, Content};
use crate::markdown::render_markdown;
use crate::pager::{filter_line_range, parse_line_range};
use crate::theme::Theme;

/// Immutable processing settings reused by initial load and every reload.
#[derive(Clone)]
pub struct ProcessingConfig {
    pub render_markdown: bool,
    pub preserve_ansi: bool,
    pub styling: bool,
    pub syntax_highlight: bool,
    pub language: Option<String>,
    pub theme: Theme,
    pub line_range: Option<String>,
    pub grep: Option<GrepOptions>,
    pub search: Option<Regex>,
}

/// Build a display document through the single canonical processing pipeline.
pub fn process(content: Content, config: &ProcessingConfig) -> Result<Document, MatError> {
    let source_name = content.source_name;
    let encoding = content.encoding;
    let mut document = if config.render_markdown {
        let text = expand_tabs(&strip_ansi(&content.text), 4);
        render_markdown(&text, source_name, encoding, config.theme)
    } else if config.preserve_ansi && config.styling {
        parse_ansi(&content.text, source_name, encoding)
    } else {
        let text = expand_tabs(&strip_ansi(&content.text), 4);
        Document::from_text(&text, source_name, encoding)
    };

    if let Some(range) = &config.line_range {
        let (start, end) = parse_line_range(range, document.line_count())?;
        filter_line_range(&mut document, start, end);
    }
    if config.syntax_highlight && !config.render_markdown && !config.preserve_ansi {
        apply_syntax_highlight(&mut document, config.language.as_deref(), config.theme);
    }
    if let Some(grep) = &config.grep {
        document = grep_filter(&document, grep);
        apply_grep_highlight(&mut document, &grep.pattern);
    }
    if let Some(search) = &config.search {
        apply_search_highlight(&mut document, search);
    }
    if !config.styling {
        for span in document.lines.iter_mut().flat_map(|line| &mut line.spans) {
            span.style = Default::default();
        }
    }
    document.recalculate_max_width();
    Ok(document)
}
