use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Line wrapping mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum WrapMode {
    /// No wrapping, horizontal scrolling enabled
    #[default]
    None,
    /// Soft wrap at terminal width
    Wrap,
    /// Hard truncate at max-width
    Truncate,
}

/// mat - A CLI tool combining cat, less, grep functionality with markdown rendering and syntax highlighting
#[derive(Parser, Debug, Default)]
#[command(name = "mat")]
#[command(version)]
#[command(about = "A CLI tool combining cat, less, grep with markdown rendering and syntax highlighting")]
#[command(long_about = None)]
pub struct Args {
    /// Input file (use - for stdin)
    #[arg(value_name = "FILE")]
    pub file: Option<PathBuf>,

    /// Show line numbers
    #[arg(short = 'n', long = "line-numbers")]
    pub line_numbers: bool,

    /// Disable syntax highlighting
    #[arg(short = 'N', long = "no-highlight")]
    pub no_highlight: bool,

    /// Force markdown rendering
    #[arg(short = 'm', long = "markdown")]
    pub markdown: bool,

    /// Disable markdown auto-detection
    #[arg(short = 'M', long = "no-markdown")]
    pub no_markdown: bool,

    /// Follow mode (tail -f style)
    #[arg(short = 'f', long = "follow")]
    pub follow: bool,

    /// Highlight pattern matches
    #[arg(short = 's', long = "search", value_name = "PAT")]
    pub search: Option<String>,

    /// Filter to matching lines
    #[arg(short = 'g', long = "grep", value_name = "PAT")]
    pub grep: Option<String>,

    /// Case-insensitive for search/grep
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Treat pattern as literal string, not regex
    #[arg(short = 'F', long = "fixed-strings")]
    pub fixed_strings: bool,

    /// Match whole words only
    #[arg(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    /// Match whole lines only
    #[arg(short = 'x', long = "line-regexp")]
    pub line_regexp: bool,

    /// Lines after grep match
    #[arg(short = 'A', long = "after", value_name = "N")]
    pub after: Option<usize>,

    /// Lines before grep match
    #[arg(short = 'B', long = "before", value_name = "N")]
    pub before: Option<usize>,

    /// Lines before and after grep match
    #[arg(short = 'C', long = "context", value_name = "N")]
    pub context: Option<usize>,

    /// Line wrap mode: none, wrap, truncate
    #[arg(long = "wrap", value_enum, default_value = "none")]
    pub wrap: WrapMode,

    /// Max line width before truncation
    #[arg(short = 'W', long = "max-width", value_name = "N", default_value = "200")]
    pub max_width: usize,

    /// Force syntax highlighting language
    #[arg(short = 'l', long = "language", value_name = "LANG")]
    pub language: Option<String>,

    /// Select color theme
    #[arg(short = 't', long = "theme", value_name = "NAME")]
    pub theme: Option<String>,

    /// Show line range: 50:100, :100, 50:, or 50
    #[arg(short = 'L', long = "lines", value_name = "RANGE")]
    pub lines: Option<String>,

    /// Direct output, skip TUI pager
    #[arg(short = 'P', long = "no-pager")]
    pub no_pager: bool,

    /// Preserve ANSI escape codes in input
    #[arg(long = "ansi")]
    pub ansi: bool,

    /// Force display of binary files
    #[arg(long = "force-binary")]
    pub force_binary: bool,
}
