//! Large file support using memory mapping and lazy loading.
//! This module is prepared for future integration but not yet used in the main flow.

#![allow(dead_code)]

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::num::NonZeroUsize;
use std::path::PathBuf;

use lru::LruCache;
use memmap2::Mmap;

use crate::display::{Line, SpanStyle, StyledSpan};

/// Threshold for using lazy loading (10MB)
pub const LARGE_FILE_THRESHOLD: u64 = 10 * 1024 * 1024;

/// Check if a file should use lazy loading
pub fn should_use_lazy_loading(path: &std::path::Path) -> io::Result<bool> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len() >= LARGE_FILE_THRESHOLD)
}

/// A lazily-loaded document using memory mapping
pub struct LazyDocument {
    /// Memory-mapped file
    mmap: Mmap,
    /// Byte offsets of each line start
    line_offsets: Vec<u64>,
    /// Cache of recently accessed lines
    line_cache: LruCache<usize, Line>,
    /// Total number of lines
    pub total_lines: usize,
    /// Source name for display
    pub source_name: String,
    /// Detected encoding
    pub encoding: String,
    /// Maximum line width encountered (updated as lines are accessed)
    pub max_line_width: usize,
    /// Path to the file
    pub path: PathBuf,
}

impl LazyDocument {
    /// Create a new LazyDocument from a file path
    pub fn new(path: PathBuf) -> io::Result<Self> {
        let file = File::open(&path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Build line offset index
        let line_offsets = build_line_offsets(&mmap);
        let total_lines = line_offsets.len().saturating_sub(1);

        let source_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Cache size: keep ~100 lines in cache
        let cache_size = NonZeroUsize::new(100).unwrap();

        Ok(Self {
            mmap,
            line_offsets,
            line_cache: LruCache::new(cache_size),
            total_lines,
            source_name,
            encoding: "UTF-8".to_string(),
            max_line_width: 0,
            path,
        })
    }

    /// Get the total number of lines
    pub fn line_count(&self) -> usize {
        self.total_lines
    }

    /// Get a line by index (0-indexed)
    pub fn get_line(&mut self, idx: usize) -> Option<&Line> {
        if idx >= self.total_lines {
            return None;
        }

        // Check cache first
        if self.line_cache.contains(&idx) {
            return self.line_cache.get(&idx);
        }

        // Load line from mmap
        let line = self.load_line(idx)?;

        // Update max width
        let width = line.width();
        if width > self.max_line_width {
            self.max_line_width = width;
        }

        // Store in cache and return reference
        self.line_cache.put(idx, line);
        self.line_cache.get(&idx)
    }

    /// Load a line from the memory-mapped file
    fn load_line(&self, idx: usize) -> Option<Line> {
        if idx >= self.total_lines {
            return None;
        }

        let start = self.line_offsets[idx] as usize;
        let end = self.line_offsets[idx + 1] as usize;

        // Get the bytes for this line
        let bytes = &self.mmap[start..end];

        // Remove trailing newline if present
        let bytes = if bytes.ends_with(b"\n") {
            &bytes[..bytes.len() - 1]
        } else {
            bytes
        };

        // Remove carriage return if present (Windows line endings)
        let bytes = if bytes.ends_with(b"\r") {
            &bytes[..bytes.len() - 1]
        } else {
            bytes
        };

        // Convert to string (lossy for non-UTF8)
        let text = String::from_utf8_lossy(bytes).to_string();

        Some(Line {
            number: idx + 1, // 1-indexed
            spans: vec![StyledSpan::new(text, SpanStyle::default())],
            is_match: false,
            is_context: false,
        })
    }

    /// Get a range of lines (returns a vector of cloned lines)
    pub fn get_lines(&mut self, start: usize, end: usize) -> Vec<Line> {
        let end = end.min(self.total_lines);
        let mut lines = Vec::with_capacity(end.saturating_sub(start));

        for idx in start..end {
            if let Some(line) = self.get_line(idx) {
                lines.push(line.clone());
            }
        }

        lines
    }

    /// Preload lines around a given index (for smooth scrolling)
    pub fn preload(&mut self, center: usize, radius: usize) {
        let start = center.saturating_sub(radius);
        let end = (center + radius).min(self.total_lines);

        for idx in start..end {
            let _ = self.get_line(idx);
        }
    }

    /// Clear the line cache
    pub fn clear_cache(&mut self) {
        self.line_cache.clear();
    }
}

/// Build line offset index by scanning the file
fn build_line_offsets(data: &[u8]) -> Vec<u64> {
    let mut offsets = vec![0];

    for (i, &byte) in data.iter().enumerate() {
        if byte == b'\n' {
            offsets.push((i + 1) as u64);
        }
    }

    // Ensure we have an end marker
    if offsets.last() != Some(&(data.len() as u64)) {
        offsets.push(data.len() as u64);
    }

    offsets
}

/// Alternative: scan file line by line without full mmap (for line count detection)
pub fn count_lines(path: &std::path::Path) -> io::Result<usize> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_lazy_document() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "Line 1").unwrap();
        writeln!(temp, "Line 2").unwrap();
        writeln!(temp, "Line 3").unwrap();
        temp.flush().unwrap();

        let mut doc = LazyDocument::new(temp.path().to_path_buf()).unwrap();

        assert_eq!(doc.line_count(), 3);

        let line = doc.get_line(0).unwrap();
        assert_eq!(line.text(), "Line 1");
        assert_eq!(line.number, 1);

        let line = doc.get_line(2).unwrap();
        assert_eq!(line.text(), "Line 3");
        assert_eq!(line.number, 3);
    }

    #[test]
    fn test_build_line_offsets() {
        let data = b"Hello\nWorld\nTest\n";
        let offsets = build_line_offsets(data);

        // Should have offsets for: start, after "Hello\n", after "World\n", after "Test\n"
        assert_eq!(offsets.len(), 4);
        assert_eq!(offsets[0], 0);
        assert_eq!(offsets[1], 6);  // "Hello\n" = 6 bytes
        assert_eq!(offsets[2], 12); // "World\n" = 6 bytes
        assert_eq!(offsets[3], 17); // "Test\n" = 5 bytes (total length)
    }

    #[test]
    fn test_line_cache() {
        let mut temp = NamedTempFile::new().unwrap();
        for i in 1..=200 {
            writeln!(temp, "Line {}", i).unwrap();
        }
        temp.flush().unwrap();

        let mut doc = LazyDocument::new(temp.path().to_path_buf()).unwrap();

        // Access lines - should populate cache
        for i in 0..100 {
            let _ = doc.get_line(i);
        }

        // Cache should be full (100 items)
        assert_eq!(doc.line_cache.len(), 100);

        // Access more lines - old ones should be evicted
        for i in 100..150 {
            let _ = doc.get_line(i);
        }

        // Cache should still be at capacity
        assert!(doc.line_cache.len() <= 100);
    }

    #[test]
    fn test_should_use_lazy_loading() {
        let mut temp = NamedTempFile::new().unwrap();
        writeln!(temp, "Small file").unwrap();
        temp.flush().unwrap();

        // Small file should not use lazy loading
        assert!(!should_use_lazy_loading(temp.path()).unwrap());
    }
}
