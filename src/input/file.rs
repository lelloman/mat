use std::fs;
use std::path::Path;

use crate::error::MatError;

/// Read file contents as raw bytes
pub fn read_file(path: &Path) -> Result<Vec<u8>, MatError> {
    fs::read(path).map_err(|source| MatError::Io {
        source,
        path: path.to_path_buf(),
    })
}

/// Detect file extension from path
pub fn detect_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
}

/// Check if extension indicates a markdown file
pub fn is_markdown_extension(ext: &str) -> bool {
    matches!(ext, "md" | "markdown" | "mdown" | "mkd" | "mkdn")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_extension() {
        assert_eq!(
            detect_extension(Path::new("foo.rs")),
            Some("rs".to_string())
        );
        assert_eq!(
            detect_extension(Path::new("foo.MD")),
            Some("md".to_string())
        );
        assert_eq!(detect_extension(Path::new("foo")), None);
        assert_eq!(
            detect_extension(Path::new("/path/to/file.txt")),
            Some("txt".to_string())
        );
    }

    #[test]
    fn test_is_markdown_extension() {
        assert!(is_markdown_extension("md"));
        assert!(is_markdown_extension("markdown"));
        assert!(is_markdown_extension("mdown"));
        assert!(!is_markdown_extension("txt"));
        assert!(!is_markdown_extension("rs"));
    }
}
