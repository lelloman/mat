mod cli;
mod display;
mod error;
mod input;

use clap::Parser;
use std::process::ExitCode;

use cli::Args;
use display::Document;
use error::{MatError, EXIT_SUCCESS};
use input::{determine_input_source, load_content};

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

    // Create document
    let document = Document::from_text(&content.text, content.source_name, content.encoding);

    // Print document info for testing
    eprintln!("Source: {}", document.source_name);
    eprintln!("Encoding: {}", document.encoding);
    eprintln!("Lines: {}", document.line_count());
    eprintln!("Max width: {}", document.max_line_width);
    eprintln!("---");

    // For now, just print the content
    for line in &document.lines {
        println!("{}", line.text());
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
