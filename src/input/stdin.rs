use std::io::{self, Read};

use crate::error::MatError;

/// Read all content from stdin into a buffer
pub fn read_stdin() -> Result<Vec<u8>, MatError> {
    let mut buffer = Vec::new();
    io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|source| MatError::Io {
            source,
            path: std::path::PathBuf::from("stdin"),
        })?;
    Ok(buffer)
}

/// Check if stdin is a pipe (not a TTY)
pub fn is_stdin_piped() -> bool {
    !atty_check()
}

/// Check if stdin is a TTY
fn atty_check() -> bool {
    use std::io::IsTerminal;
    io::stdin().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_stdin_piped_in_test_environment() {
        // In test environment, stdin is typically not a TTY
        // This just verifies the function doesn't panic
        let _ = is_stdin_piped();
    }
}
