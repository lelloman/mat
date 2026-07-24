mod ansi;
mod binary;
mod encoding;
mod file;
mod stdin;

use std::path::PathBuf;

use crate::cli::Args;
use crate::error::MatError;

pub use ansi::parse_ansi;
pub use binary::is_binary;
pub use encoding::{decode_bytes, detect_encoding};
pub use file::{detect_extension, is_markdown_extension, read_file};
pub use stdin::{is_stdin_piped, read_stdin};

/// Represents the source of input
#[derive(Debug, Clone)]
pub enum InputSource {
    /// Read from a file
    File(PathBuf),
    /// Read from stdin
    Stdin,
}

/// Holds the loaded content with metadata
#[derive(Debug)]
pub struct Content {
    /// The text content
    pub text: String,
    /// Name of the source (filename or "stdin")
    pub source_name: String,
    /// Whether this should be treated as markdown
    pub is_markdown: bool,
    /// Detected or assumed encoding
    pub encoding: String,
}

/// Expand tabs to spaces with proper alignment
pub fn expand_tabs(text: &str, tab_width: usize) -> String {
    let mut result = String::with_capacity(text.len());
    let mut column = 0;

    for ch in text.chars() {
        match ch {
            '\t' => {
                // Calculate spaces needed to reach next tab stop
                let spaces_needed = tab_width - (column % tab_width);
                for _ in 0..spaces_needed {
                    result.push(' ');
                }
                column += spaces_needed;
            }
            '\n' => {
                result.push('\n');
                column = 0;
            }
            '\r' => {
                result.push('\r');
                // Don't reset column for CR (will be followed by LF)
            }
            _ => {
                result.push(ch);
                // Handle wide characters
                let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
                column += width;
            }
        }
    }

    result
}

/// Strips ANSI escape sequences from text
pub fn strip_ansi(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.next() {
                Some('[') => {
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&next) {
                            break;
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
        } else if !c.is_control() || matches!(c, '\n' | '\r' | '\t') {
            result.push(c);
        }
    }

    result
}

/// Load content from the given input source
pub fn load_content(source: InputSource, args: &Args) -> Result<Content, MatError> {
    let (raw_bytes, source_name, extension) = match &source {
        InputSource::File(path) => {
            let bytes = read_file(path)?;
            let name = path.display().to_string();
            let ext = detect_extension(path);
            (bytes, name, ext)
        }
        InputSource::Stdin => {
            let bytes = read_stdin()?;
            (bytes, "stdin".to_string(), None)
        }
    };

    // BOM detection must precede binary classification because UTF-16 text
    // normally contains NUL bytes.
    let encoding_name = detect_encoding(&raw_bytes);
    let bom_marked_text = matches!(encoding_name, "UTF-8-BOM" | "UTF-16LE" | "UTF-16BE");
    if !args.force_binary && !bom_marked_text && is_binary(&raw_bytes) {
        let path = match source {
            InputSource::File(p) => p,
            InputSource::Stdin => PathBuf::from("stdin"),
        };
        return Err(MatError::BinaryFile { path });
    }

    // Detect and decode encoding
    let text = decode_bytes(raw_bytes, encoding_name)?;

    // Determine if markdown
    let is_markdown = if args.no_markdown {
        false
    } else if args.markdown {
        true
    } else {
        extension
            .as_ref()
            .map(|e| is_markdown_extension(e))
            .unwrap_or(false)
    };

    Ok(Content {
        text,
        source_name,
        is_markdown,
        encoding: encoding_name.to_string(),
    })
}

/// Determine the input source from CLI args
pub fn determine_input_source(args: &Args) -> Option<InputSource> {
    match &args.file {
        Some(path) if path.as_os_str() == "-" => Some(InputSource::Stdin),
        Some(path) => Some(InputSource::File(path.clone())),
        None if is_stdin_piped() => Some(InputSource::Stdin),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tabs_basic() {
        assert_eq!(expand_tabs("a\tb", 4), "a   b");
        assert_eq!(expand_tabs("ab\tc", 4), "ab  c");
        assert_eq!(expand_tabs("abc\td", 4), "abc d");
        assert_eq!(expand_tabs("abcd\te", 4), "abcd    e");
    }

    #[test]
    fn test_expand_tabs_multiline() {
        assert_eq!(expand_tabs("a\tb\nc\td", 4), "a   b\nc   d");
    }

    #[test]
    fn test_expand_tabs_custom_width() {
        assert_eq!(expand_tabs("a\tb", 2), "a b");
        assert_eq!(expand_tabs("ab\tc", 2), "ab  c");
    }

    #[test]
    fn test_strip_ansi_basic() {
        // Color code (red)
        assert_eq!(strip_ansi("\x1b[31mHello\x1b[0m"), "Hello");
        // Bold
        assert_eq!(strip_ansi("\x1b[1mBold\x1b[0m"), "Bold");
        // Multiple codes
        assert_eq!(strip_ansi("\x1b[1;31mRed Bold\x1b[0m"), "Red Bold");
    }

    #[test]
    fn test_strip_ansi_preserves_normal_text() {
        assert_eq!(strip_ansi("Hello World"), "Hello World");
        assert_eq!(strip_ansi("No escape codes here"), "No escape codes here");
    }

    #[test]
    fn test_strip_ansi_discards_osc_and_cursor_controls() {
        assert_eq!(
            strip_ansi("before\x1b]0;owned\x07after\x1b[2J"),
            "beforeafter"
        );
    }
}
