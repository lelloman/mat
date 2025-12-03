use std::fs::File;
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// Reader that follows a file for new content (tail -f style)
pub struct FollowReader {
    /// Path to the file being followed
    path: PathBuf,
    /// Current position in the file
    position: u64,
}

impl FollowReader {
    /// Create a new follow reader for the given file
    pub fn new(path: PathBuf, start_at_end: bool) -> io::Result<Self> {
        let file = File::open(&path)?;
        let position = if start_at_end {
            file.metadata()?.len()
        } else {
            0
        };

        Ok(Self { path, position })
    }

    /// Check for new content and return any new lines
    pub fn check_for_new_content(&mut self) -> io::Result<Vec<String>> {
        let file = File::open(&self.path)?;
        let metadata = file.metadata()?;
        let current_size = metadata.len();

        // Check if file has grown
        if current_size <= self.position {
            // File hasn't grown (or was truncated)
            if current_size < self.position {
                // File was truncated, reset position
                self.position = 0;
            }
            return Ok(Vec::new());
        }

        // Read new content
        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(self.position))?;

        let mut new_lines = Vec::new();
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    // Remove trailing newline
                    let trimmed = line.trim_end_matches(|c| c == '\n' || c == '\r');
                    new_lines.push(trimmed.to_string());
                }
                Err(e) => return Err(e),
            }
        }

        // Update position
        self.position = reader.stream_position()?;

        Ok(new_lines)
    }

    /// Get the current file position
    pub fn position(&self) -> u64 {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_follow_reader_new_content() {
        // Create a temp file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();
        file.flush().unwrap();

        // Create follow reader starting at current position
        let path = file.path().to_path_buf();
        let mut reader = FollowReader::new(path.clone(), true).unwrap();

        // Initially no new content
        let new_lines = reader.check_for_new_content().unwrap();
        assert!(new_lines.is_empty());

        // Append new content
        writeln!(file, "Line 3").unwrap();
        writeln!(file, "Line 4").unwrap();
        file.flush().unwrap();

        // Should now have new lines
        let new_lines = reader.check_for_new_content().unwrap();
        assert_eq!(new_lines.len(), 2);
        assert_eq!(new_lines[0], "Line 3");
        assert_eq!(new_lines[1], "Line 4");

        // No more new content
        let new_lines = reader.check_for_new_content().unwrap();
        assert!(new_lines.is_empty());
    }

    #[test]
    fn test_follow_reader_from_start() {
        // Create a temp file
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();
        file.flush().unwrap();

        // Create follow reader starting from beginning
        let path = file.path().to_path_buf();
        let mut reader = FollowReader::new(path, false).unwrap();

        // Should have existing lines
        let new_lines = reader.check_for_new_content().unwrap();
        assert_eq!(new_lines.len(), 2);
        assert_eq!(new_lines[0], "Line 1");
        assert_eq!(new_lines[1], "Line 2");
    }
}
