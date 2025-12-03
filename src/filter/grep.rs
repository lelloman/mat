use regex::Regex;

use crate::cli::Args;
use crate::display::{Document, Line, SpanStyle, StyledSpan};
use crate::error::MatError;

/// Options for grep filtering
#[derive(Debug)]
pub struct GrepOptions {
    /// Compiled regex pattern
    pub pattern: Regex,
    /// Lines to show before match
    pub before: usize,
    /// Lines to show after match
    pub after: usize,
}

impl GrepOptions {
    /// Create GrepOptions from CLI args
    pub fn from_args(args: &Args) -> Result<Option<Self>, MatError> {
        let pattern_str = match &args.grep {
            Some(p) => p,
            None => return Ok(None),
        };

        if pattern_str.is_empty() {
            return Err(MatError::EmptyPattern);
        }

        let pattern = build_regex(pattern_str, args)?;

        // Determine context lines
        let (before, after) = if let Some(c) = args.context {
            (c, c)
        } else {
            (args.before.unwrap_or(0), args.after.unwrap_or(0))
        };

        Ok(Some(Self {
            pattern,
            before,
            after,
        }))
    }
}

/// Build a regex pattern string with the given options
pub fn build_regex_pattern(
    pattern: &str,
    ignore_case: bool,
    fixed_strings: bool,
    word_regexp: bool,
    line_regexp: bool,
) -> String {
    let mut pattern_str = if fixed_strings {
        // Escape all regex metacharacters
        regex::escape(pattern)
    } else {
        pattern.to_string()
    };

    // Word boundary matching
    if word_regexp {
        pattern_str = format!(r"\b{}\b", pattern_str);
    }

    // Line matching
    if line_regexp {
        pattern_str = format!(r"^{}$", pattern_str);
    }

    // Add case-insensitive flag if needed
    if ignore_case {
        pattern_str = format!("(?i){}", pattern_str);
    }

    pattern_str
}

/// Build a regex pattern with the given CLI options
pub fn build_regex(pattern: &str, args: &Args) -> Result<Regex, MatError> {
    let pattern_str = build_regex_pattern(
        pattern,
        args.ignore_case,
        args.fixed_strings,
        args.word_regexp,
        args.line_regexp,
    );

    Regex::new(&pattern_str).map_err(|e| MatError::InvalidRegex {
        source: e,
        pattern: pattern.to_string(),
    })
}

/// Filter a document to only include matching lines and context
pub fn grep_filter(document: &Document, options: &GrepOptions) -> Document {
    let total_lines = document.lines.len();
    if total_lines == 0 {
        return Document {
            lines: vec![],
            max_line_width: 0,
            source_name: document.source_name.clone(),
            encoding: document.encoding.clone(),
        };
    }

    // First pass: find all matching line indices
    let mut match_indices: Vec<usize> = Vec::new();
    for (i, line) in document.lines.iter().enumerate() {
        let text = line.text();
        if options.pattern.is_match(&text) {
            match_indices.push(i);
        }
    }

    if match_indices.is_empty() {
        return Document {
            lines: vec![],
            max_line_width: 0,
            source_name: document.source_name.clone(),
            encoding: document.encoding.clone(),
        };
    }

    // Second pass: build ranges including context
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for &match_idx in &match_indices {
        let start = match_idx.saturating_sub(options.before);
        let end = (match_idx + options.after + 1).min(total_lines);
        ranges.push((start, end));
    }

    // Merge overlapping ranges
    let merged_ranges = merge_ranges(ranges);

    // Collect lines with proper flags
    let mut result_lines: Vec<Line> = Vec::new();
    let mut last_end = 0;

    for (start, end) in merged_ranges {
        // Add separator if there's a gap
        if !result_lines.is_empty() && start > last_end {
            result_lines.push(Line::separator());
        }

        for i in start..end {
            let original_line = &document.lines[i];
            let text = original_line.text();
            let is_match = match_indices.contains(&i);

            let mut line = Line {
                number: original_line.number,
                spans: original_line.spans.clone(),
                is_match,
                is_context: !is_match,
            };

            // Apply dim style to context lines
            if !is_match {
                line.spans = vec![StyledSpan::new(
                    text,
                    SpanStyle::default().fg(ratatui::style::Color::DarkGray),
                )];
            }

            result_lines.push(line);
        }

        last_end = end;
    }

    let max_line_width = result_lines.iter().map(|l| l.width()).max().unwrap_or(0);

    Document {
        lines: result_lines,
        max_line_width,
        source_name: document.source_name.clone(),
        encoding: document.encoding.clone(),
    }
}

/// Merge overlapping ranges
fn merge_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if ranges.is_empty() {
        return vec![];
    }

    ranges.sort_by_key(|r| r.0);

    let mut merged: Vec<(usize, usize)> = vec![ranges[0]];

    for (start, end) in ranges.into_iter().skip(1) {
        let last = merged.last_mut().unwrap();
        if start <= last.1 {
            // Overlapping or adjacent, extend
            last.1 = last.1.max(end);
        } else {
            // Gap, add new range
            merged.push((start, end));
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_doc() -> Document {
        let text = "apple\nbanana\ncherry\napricot\nblueberry\ncoconut\navocado";
        Document::from_text(text, "test.txt".to_string(), "UTF-8".to_string())
    }

    #[test]
    fn test_basic_grep() {
        let doc = create_test_doc();
        let options = GrepOptions {
            pattern: Regex::new("a").unwrap(),
            before: 0,
            after: 0,
        };

        let filtered = grep_filter(&doc, &options);

        // apple, banana (contiguous), separator, apricot, separator, avocado
        // = 2 matches + separator + 1 match + separator + 1 match = 6 total lines
        // Match lines: apple, banana, apricot, avocado
        let match_lines: Vec<_> = filtered.lines.iter().filter(|l| l.is_match).collect();
        assert_eq!(match_lines.len(), 4);
    }

    #[test]
    fn test_grep_with_context() {
        let doc = create_test_doc();
        let options = GrepOptions {
            pattern: Regex::new("cherry").unwrap(),
            before: 1,
            after: 1,
        };

        let filtered = grep_filter(&doc, &options);

        // banana (before), cherry (match), apricot (after)
        assert_eq!(filtered.lines.len(), 3);
        assert_eq!(filtered.lines[0].number, 2); // banana
        assert!(filtered.lines[0].is_context);
        assert_eq!(filtered.lines[1].number, 3); // cherry
        assert!(filtered.lines[1].is_match);
        assert_eq!(filtered.lines[2].number, 4); // apricot
        assert!(filtered.lines[2].is_context);
    }

    #[test]
    fn test_grep_with_separator() {
        let doc = create_test_doc();
        let options = GrepOptions {
            pattern: Regex::new("^(apple|coconut)$").unwrap(),
            before: 0,
            after: 0,
        };

        let filtered = grep_filter(&doc, &options);

        // apple, separator, coconut
        assert_eq!(filtered.lines.len(), 3);
        assert_eq!(filtered.lines[1].number, 0); // separator has number 0
    }

    #[test]
    fn test_merge_ranges() {
        let ranges = vec![(0, 3), (2, 5), (7, 10)];
        let merged = merge_ranges(ranges);
        assert_eq!(merged, vec![(0, 5), (7, 10)]);
    }

    #[test]
    fn test_build_regex_case_insensitive() {
        let args = Args {
            ignore_case: true,
            fixed_strings: false,
            word_regexp: false,
            line_regexp: false,
            ..Default::default()
        };

        let regex = build_regex("ABC", &args).unwrap();
        assert!(regex.is_match("abc"));
        assert!(regex.is_match("ABC"));
    }

    #[test]
    fn test_build_regex_fixed_strings() {
        let args = Args {
            ignore_case: false,
            fixed_strings: true,
            word_regexp: false,
            line_regexp: false,
            ..Default::default()
        };

        let regex = build_regex("[a-z]", &args).unwrap();
        assert!(regex.is_match("[a-z]")); // literal match
        assert!(!regex.is_match("abc")); // not a character class
    }

    #[test]
    fn test_build_regex_word_boundary() {
        let args = Args {
            ignore_case: false,
            fixed_strings: false,
            word_regexp: true,
            line_regexp: false,
            ..Default::default()
        };

        let regex = build_regex("test", &args).unwrap();
        assert!(regex.is_match("test"));
        assert!(regex.is_match("a test here"));
        assert!(!regex.is_match("testing"));
    }
}
