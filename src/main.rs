mod cli;
mod display;
mod error;
mod filter;
mod highlight;
mod input;
mod markdown;
mod pager;
mod pipeline;
mod theme;

use clap::Parser;
use std::{io::IsTerminal, process::ExitCode};

use cli::{Args, ColorMode};
use error::{MatError, EXIT_SUCCESS};
use filter::GrepOptions;
use highlight::SearchState;
use input::{determine_input_source, load_content};
use pager::{print_document, run_pager};
use pipeline::{process, ProcessingConfig};
use theme::get_theme;

fn run(args: Args) -> Result<(), MatError> {
    let source = determine_input_source(&args).ok_or(MatError::MissingInput)?;
    let stdout_is_tty = std::io::stdout().is_terminal();
    let direct_output = args.no_pager || !stdout_is_tty;
    if args.follow && direct_output {
        return Err(MatError::InvalidArguments(
            "--follow requires the interactive pager and cannot be used with direct output"
                .to_string(),
        ));
    }

    if args.follow && matches!(source, input::InputSource::Stdin) {
        return Err(MatError::FollowModeStdin);
    }

    let content = load_content(source.clone(), &args)?;
    if args.file.is_none() && content.text.is_empty() {
        return Err(MatError::MissingInput);
    }
    let render_markdown = if args.no_markdown {
        false
    } else if args.markdown {
        true
    } else {
        content.is_markdown
    };
    let grep_options = GrepOptions::from_args(&args)?;
    let search_state = SearchState::from_args(&args)?;
    let theme = get_theme(args.theme);
    let styling = match args.color {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => stdout_is_tty && std::env::var_os("NO_COLOR").is_none(),
    };
    let processing = ProcessingConfig {
        render_markdown,
        preserve_ansi: args.ansi,
        styling,
        syntax_highlight: !args.no_highlight,
        language: args.language.clone(),
        theme,
        line_range: args.lines.clone(),
        grep: grep_options,
        search: search_state.as_ref().map(|state| state.pattern.clone()),
    };
    let mut base_processing = processing.clone();
    base_processing.search = None;
    let base_document = process(content, &base_processing)?;
    let mut document = base_document.clone();
    if let Some(state) = &search_state {
        highlight::apply_search_highlight(&mut document, &state.pattern);
    }

    // Get file path for follow mode (only for file inputs)
    let file_path = match &source {
        input::InputSource::File(p) => Some(p.clone()),
        input::InputSource::Stdin => None,
    };

    if direct_output {
        print_document(&document, args.line_numbers, styling).map_err(|e| MatError::Io {
            source: e,
            path: std::path::PathBuf::from("stdout"),
        })?;
    } else {
        run_pager(
            document,
            base_document,
            &args,
            search_state,
            file_path,
            base_processing,
        )?;
    }

    Ok(())
}

fn main() -> ExitCode {
    let args = Args::parse();

    match run(args) {
        Ok(()) => ExitCode::from(EXIT_SUCCESS as u8),
        Err(e) => {
            if matches!(&e, MatError::Io { source, .. } if source.kind() == std::io::ErrorKind::BrokenPipe)
            {
                return ExitCode::SUCCESS;
            }
            eprintln!("mat: {}", e);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}
