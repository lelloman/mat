mod binary;
mod encoding;
mod file;
mod follow;
mod stdin;

use std::path::PathBuf;

use crate::cli::Args;
use crate::error::MatError;

pub use binary::is_binary;
pub use encoding::{decode_bytes, detect_encoding};
pub use file::{detect_extension, is_markdown_extension, read_file};
pub use follow::FollowReader;
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
    /// File extension if applicable
    pub extension: Option<String>,
    /// Whether this should be treated as markdown
    pub is_markdown: bool,
    /// Detected or assumed encoding
    pub encoding: String,
}

/// Strips ANSI escape sequences from text
pub fn strip_ansi(text: &str) -> String {
    // Match ANSI escape sequences: ESC [ ... m (SGR) and other CSI sequences
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Check for CSI sequence (ESC [)
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // Consume until we hit a letter (the command)
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // Skip other escape sequences (ESC followed by single char)
            else if chars.peek().is_some() {
                chars.next();
            }
        } else {
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

    // Check for binary content
    if !args.force_binary && is_binary(&raw_bytes) {
        let path = match source {
            InputSource::File(p) => p,
            InputSource::Stdin => PathBuf::from("stdin"),
        };
        return Err(MatError::BinaryFile { path });
    }

    // Detect and decode encoding
    let encoding_name = detect_encoding(&raw_bytes);
    let text = decode_bytes(raw_bytes, encoding_name)?;

    // Strip ANSI unless --ansi flag is set
    let text = if args.ansi { text } else { strip_ansi(&text) };

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
        extension,
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
