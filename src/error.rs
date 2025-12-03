use std::path::PathBuf;
use thiserror::Error;

/// Exit code for successful execution
pub const EXIT_SUCCESS: i32 = 0;

/// Exit code for general errors (file not found, permission denied, I/O error)
pub const EXIT_ERROR: i32 = 1;

/// Exit code for invalid arguments (bad regex, invalid flags, invalid line range)
pub const EXIT_INVALID_ARGS: i32 = 2;

/// Custom error type for mat
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum MatError {
    /// File I/O errors
    #[error("I/O error for '{path}': {source}")]
    Io {
        #[source]
        source: std::io::Error,
        path: PathBuf,
    },

    /// Invalid regex pattern
    #[error("Invalid regex pattern '{pattern}': {source}")]
    InvalidRegex {
        #[source]
        source: regex::Error,
        pattern: String,
    },

    /// Empty search/grep pattern
    #[error("Empty pattern provided. Did you mean to omit -s/-g?")]
    EmptyPattern,

    /// Binary file detected
    #[error("Binary file detected: '{path}'. Use --force-binary to view anyway")]
    BinaryFile { path: PathBuf },

    /// Invalid line range format
    #[error("Invalid line range format: '{range}'. Expected formats: X:Y, :Y, X:, or X")]
    InvalidLineRange { range: String },

    /// Encoding detection/conversion failed
    #[error("Failed to detect or convert encoding for '{path}'")]
    EncodingError { path: PathBuf },
}

impl MatError {
    /// Returns the appropriate exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            MatError::InvalidRegex { .. } | MatError::InvalidLineRange { .. } => EXIT_INVALID_ARGS,
            _ => EXIT_ERROR,
        }
    }
}

impl From<std::io::Error> for MatError {
    fn from(source: std::io::Error) -> Self {
        MatError::Io {
            source,
            path: PathBuf::new(),
        }
    }
}
