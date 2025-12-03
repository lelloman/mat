mod cli;
mod error;
mod input;

use clap::Parser;
use std::process::ExitCode;

use cli::Args;
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

    // Print content info for testing
    eprintln!("Source: {}", content.source_name);
    eprintln!("Encoding: {}", content.encoding);
    eprintln!("Extension: {:?}", content.extension);
    eprintln!("Is Markdown: {}", content.is_markdown);
    eprintln!("Content length: {} bytes", content.text.len());
    eprintln!("---");

    // For now, just print the content
    print!("{}", content.text);

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
