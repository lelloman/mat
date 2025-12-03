use once_cell::sync::Lazy;
use ratatui::style::Color;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::display::{Document, SpanStyle, StyledSpan};
use crate::theme::Theme;

/// Lazily loaded syntax set
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

/// Lazily loaded theme set
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Get the appropriate syntect theme name for our theme
fn syntect_theme_name(theme: Theme) -> &'static str {
    match theme {
        Theme::Light => "base16-ocean.light",
        Theme::Dark => "base16-ocean.dark",
    }
}

/// Convert syntect color to ratatui color
fn syntect_to_ratatui_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

/// Convert syntect style to our span style
fn syntect_to_span_style(style: SyntectStyle) -> SpanStyle {
    SpanStyle {
        fg: Some(syntect_to_ratatui_color(style.foreground)),
        bg: None, // We don't use syntect's background
        bold: style.font_style.contains(syntect::highlighting::FontStyle::BOLD),
        italic: style.font_style.contains(syntect::highlighting::FontStyle::ITALIC),
        underline: style.font_style.contains(syntect::highlighting::FontStyle::UNDERLINE),
    }
}

/// Detect language from file extension
pub fn detect_language(filename: &str) -> Option<&'static str> {
    let extension = filename.rsplit('.').next()?;

    // Map common extensions to syntect names
    match extension.to_lowercase().as_str() {
        "rs" => Some("Rust"),
        "py" => Some("Python"),
        "js" => Some("JavaScript"),
        "ts" => Some("TypeScript"),
        "tsx" => Some("TypeScript"),
        "jsx" => Some("JavaScript"),
        "c" => Some("C"),
        "h" => Some("C"),
        "cpp" | "cc" | "cxx" => Some("C++"),
        "hpp" | "hh" | "hxx" => Some("C++"),
        "go" => Some("Go"),
        "java" => Some("Java"),
        "rb" => Some("Ruby"),
        "sh" | "bash" => Some("Bash"),
        "zsh" => Some("Bash"),
        "json" => Some("JSON"),
        "yaml" | "yml" => Some("YAML"),
        "toml" => Some("TOML"),
        "xml" => Some("XML"),
        "html" | "htm" => Some("HTML"),
        "css" => Some("CSS"),
        "sql" => Some("SQL"),
        "md" | "markdown" => Some("Markdown"),
        "php" => Some("PHP"),
        "swift" => Some("Swift"),
        "kt" | "kts" => Some("Kotlin"),
        "scala" => Some("Scala"),
        "r" => Some("R"),
        "lua" => Some("Lua"),
        "pl" | "pm" => Some("Perl"),
        "hs" => Some("Haskell"),
        "elm" => Some("Elm"),
        "erl" => Some("Erlang"),
        "ex" | "exs" => Some("Elixir"),
        "clj" | "cljs" => Some("Clojure"),
        "fs" | "fsx" => Some("F#"),
        "cs" => Some("C#"),
        "vb" => Some("Visual Basic"),
        "ps1" | "psm1" => Some("PowerShell"),
        "dockerfile" => Some("Dockerfile"),
        "makefile" | "mk" => Some("Makefile"),
        "cmake" => Some("CMake"),
        "tf" => Some("Terraform"),
        "vim" => Some("VimL"),
        "diff" | "patch" => Some("Diff"),
        "ini" | "cfg" => Some("INI"),
        "csv" => Some("CSV"),
        _ => None,
    }
}

/// Apply syntax highlighting to a document
pub fn apply_syntax_highlight(document: &mut Document, language: Option<&str>, theme: Theme) {
    let syntax_set = &*SYNTAX_SET;
    let theme_set = &*THEME_SET;

    // Try to find the syntax
    let syntax = if let Some(lang) = language {
        // Try explicit language first
        syntax_set
            .find_syntax_by_name(lang)
            .or_else(|| syntax_set.find_syntax_by_extension(lang))
    } else {
        // Try to detect from filename
        detect_language(&document.source_name)
            .and_then(|lang| syntax_set.find_syntax_by_name(lang))
            .or_else(|| {
                // Try extension directly
                let ext = document.source_name.rsplit('.').next().unwrap_or("");
                syntax_set.find_syntax_by_extension(ext)
            })
    };

    let syntax = match syntax {
        Some(s) => s,
        None => return, // No syntax found, leave document as-is
    };

    let theme_name = syntect_theme_name(theme);
    let theme = match theme_set.themes.get(theme_name) {
        Some(t) => t,
        None => return, // Theme not found
    };

    let mut highlighter = HighlightLines::new(syntax, theme);

    for line in &mut document.lines {
        let text = line.text();

        match highlighter.highlight_line(&text, syntax_set) {
            Ok(ranges) => {
                let spans: Vec<StyledSpan> = ranges
                    .into_iter()
                    .map(|(style, text)| {
                        StyledSpan::new(text, syntect_to_span_style(style))
                    })
                    .collect();

                if !spans.is_empty() {
                    line.spans = spans;
                }
            }
            Err(_) => {
                // On error, leave the line as-is
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language("main.rs"), Some("Rust"));
        assert_eq!(detect_language("script.py"), Some("Python"));
        assert_eq!(detect_language("app.js"), Some("JavaScript"));
        assert_eq!(detect_language("README.md"), Some("Markdown"));
        assert_eq!(detect_language("unknown.xyz"), None);
    }

    #[test]
    fn test_syntax_highlight_rust() {
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let mut doc = Document::from_text(code, "test.rs".to_string(), "UTF-8".to_string());

        apply_syntax_highlight(&mut doc, None, Theme::Dark);

        // After highlighting, spans should be modified
        // The exact styling depends on syntect, but we can verify spans exist
        assert!(doc.lines[0].spans.len() > 0);
    }

    #[test]
    fn test_syntax_highlight_with_explicit_language() {
        let code = "def hello():\n    print('Hi')";
        let mut doc = Document::from_text(code, "unknown.txt".to_string(), "UTF-8".to_string());

        // Without explicit language, it would not highlight
        apply_syntax_highlight(&mut doc, Some("Python"), Theme::Dark);

        // Should have been highlighted
        assert!(doc.lines[0].spans.len() > 0);
    }

    #[test]
    fn test_syntax_highlight_unknown_language() {
        let text = "Just some plain text";
        let mut doc = Document::from_text(text, "unknown.xyz".to_string(), "UTF-8".to_string());

        let original_spans_len = doc.lines[0].spans.len();
        apply_syntax_highlight(&mut doc, None, Theme::Dark);

        // Should remain unchanged
        assert_eq!(doc.lines[0].spans.len(), original_spans_len);
    }
}
