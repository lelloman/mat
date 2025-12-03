mod cli;
mod error;

use clap::Parser;
use std::process::ExitCode;

use cli::Args;
use error::{MatError, EXIT_SUCCESS};

fn run(args: Args) -> Result<(), MatError> {
    // Stub: just print args for now
    eprintln!("mat v{}", env!("CARGO_PKG_VERSION"));
    eprintln!("Args: {:?}", args);

    // Check if file exists (placeholder logic)
    if let Some(ref path) = args.file {
        if path.as_os_str() != "-" && !path.exists() {
            return Err(MatError::Io {
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
                path: path.clone(),
            });
        }
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
