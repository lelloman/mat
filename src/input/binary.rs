/// Size of buffer to check for binary detection
const CHECK_SIZE: usize = 8192;

/// Threshold for non-printable character proportion (30%)
const NON_PRINTABLE_THRESHOLD: f64 = 0.30;

/// Check if the given bytes represent binary content
///
/// Binary detection is based on:
/// 1. Presence of null bytes in the first 8KB
/// 2. Proportion of non-printable characters exceeding 30%
pub fn is_binary(bytes: &[u8]) -> bool {
    let check_len = bytes.len().min(CHECK_SIZE);
    let sample = &bytes[..check_len];

    // Check for null bytes (strong indicator of binary)
    if sample.contains(&0) {
        return true;
    }

    // Count non-printable characters
    let non_printable_count = sample
        .iter()
        .filter(|&&b| !is_printable_byte(b))
        .count();

    let proportion = non_printable_count as f64 / sample.len() as f64;
    proportion > NON_PRINTABLE_THRESHOLD
}

/// Check if a byte is considered printable text
///
/// Printable includes:
/// - ASCII printable characters (0x20-0x7E)
/// - Tab (0x09), newline (0x0A), carriage return (0x0D)
/// - High bytes (0x80+) which could be UTF-8 continuation bytes
fn is_printable_byte(b: u8) -> bool {
    matches!(b, 0x09 | 0x0A | 0x0D | 0x20..=0x7E | 0x80..=0xFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_file() {
        let text = b"Hello, world!\nThis is a text file.\n";
        assert!(!is_binary(text));
    }

    #[test]
    fn test_binary_with_null() {
        let binary = b"Hello\x00World";
        assert!(is_binary(binary));
    }

    #[test]
    fn test_empty_file() {
        let empty: &[u8] = &[];
        assert!(!is_binary(empty));
    }

    #[test]
    fn test_utf8_text() {
        let utf8 = "Hello, 世界! Привет мир! 🌍".as_bytes();
        assert!(!is_binary(utf8));
    }

    #[test]
    fn test_mostly_binary() {
        // Create content that's >30% non-printable (excluding high bytes)
        let mut binary = vec![0x01u8; 40]; // non-printable control chars
        binary.extend_from_slice(b"Hello"); // some printable text
        // 40/45 = ~89% non-printable, should be detected as binary
        assert!(is_binary(&binary));
    }

    #[test]
    fn test_is_printable_byte() {
        assert!(is_printable_byte(b' '));
        assert!(is_printable_byte(b'A'));
        assert!(is_printable_byte(b'\n'));
        assert!(is_printable_byte(b'\t'));
        assert!(is_printable_byte(0x80)); // UTF-8 continuation
        assert!(!is_printable_byte(0x00)); // null
        assert!(!is_printable_byte(0x01)); // control char
        assert!(!is_printable_byte(0x1F)); // control char
    }
}
