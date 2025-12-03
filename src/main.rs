mod cli;
mod display;
mod error;
mod filter;
mod highlight;
mod input;
mod markdown;
mod pager;
mod theme;

use clap::Parser;
use std::process::ExitCode;

use cli::Args;
use display::Document;
use error::{MatError, EXIT_SUCCESS};
use filter::{grep_filter, GrepOptions};
use highlight::{apply_search_highlight, apply_syntax_highlight, SearchState};
use input::{determine_input_source, load_content};
use markdown::render_markdown;
use pager::{filter_line_range, parse_line_range, print_document, run_pager};
use theme::get_theme;

fn run(args: Args) -> Result<(), MatError> {
    // Determine input source
    let source = match determine_input_source(&args) {
        Some(s) => s,
        None => {
            eprintln!("mat: No input file specified. Use 'mat <file>' or pipe data to stdin.");
            return Ok(());
        }
    };

    // Load content
    let content = load_content(source, &args)?;

    // Determine if we should render as markdown
    let should_render_markdown = if args.no_markdown {
        false
    } else if args.markdown {
        true
    } else {
        // Auto-detect based on extension
        content.is_markdown
    };

    // Create document (with or without markdown rendering)
    let mut document = if should_render_markdown {
        render_markdown(&content.text, content.source_name)
    } else {
        Document::from_text(&content.text, content.source_name, content.encoding)
    };

    // Apply line range filter if specified
    if let Some(ref range) = args.lines {
        let (start, end) = parse_line_range(range, document.line_count())?;
        filter_line_range(&mut document, start, end);
    }

    // Apply grep filter if specified
    if let Some(grep_options) = GrepOptions::from_args(&args)? {
        document = grep_filter(&document, &grep_options);
    }

    // Determine theme for highlighting
    let theme = get_theme(args.theme.as_deref());

    // Apply syntax highlighting if not disabled
    if !args.no_highlight {
        apply_syntax_highlight(&mut document, args.language.as_deref(), theme);
    }

    // Apply search highlighting if specified
    let search_state = SearchState::from_args(&args)?;
    if let Some(ref state) = search_state {
        apply_search_highlight(&mut document, &state.pattern);
    }

    // Run pager or print directly
    if args.no_pager {
        print_document(&document, args.line_numbers).map_err(|e| MatError::Io {
            source: e,
            path: std::path::PathBuf::from("stdout"),
        })?;
    } else {
        run_pager(document, &args, search_state)?;
    }

    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();

    match run(args) {
        Ok(()) => ExitCode::from(EXIT_SUCCESS as u8),
        Err(e) => {
            eprintln!("mat: {}", e);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}
