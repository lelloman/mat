use crate::error::MatError;

/// UTF-8 BOM bytes
const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

/// UTF-16 LE BOM
const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];

/// UTF-16 BE BOM
const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];

/// Detect the encoding of the given bytes
///
/// Returns one of: "UTF-8", "UTF-8-BOM", "UTF-16LE", "UTF-16BE", "Windows-1252"
pub fn detect_encoding(bytes: &[u8]) -> &'static str {
    // Check for BOMs first
    if bytes.starts_with(UTF8_BOM) {
        return "UTF-8-BOM";
    }
    if bytes.starts_with(UTF16_LE_BOM) {
        return "UTF-16LE";
    }
    if bytes.starts_with(UTF16_BE_BOM) {
        return "UTF-16BE";
    }

    // Try to validate as UTF-8
    if std::str::from_utf8(bytes).is_ok() {
        return "UTF-8";
    }

    // Fallback to Latin-1 (ISO-8859-1)
    "Windows-1252"
}

/// Decode bytes to a String using the detected encoding
pub fn decode_bytes(bytes: Vec<u8>, encoding: &str) -> Result<String, MatError> {
    match encoding {
        "UTF-8" => {
            // Already validated as UTF-8
            Ok(String::from_utf8(bytes)
                .unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
        }
        "UTF-8-BOM" => {
            // Skip the BOM and decode as UTF-8
            let without_bom = if bytes.len() >= 3 {
                &bytes[3..]
            } else {
                &bytes
            };
            Ok(String::from_utf8_lossy(without_bom).into_owned())
        }
        "UTF-16LE" => {
            let payload = bytes.strip_prefix(UTF16_LE_BOM).unwrap_or(&bytes);
            let (cow, _, _) = encoding_rs::UTF_16LE.decode(payload);
            Ok(cow.into_owned())
        }
        "UTF-16BE" => {
            let payload = bytes.strip_prefix(UTF16_BE_BOM).unwrap_or(&bytes);
            let (cow, _, _) = encoding_rs::UTF_16BE.decode(payload);
            Ok(cow.into_owned())
        }
        _ => {
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
            Ok(cow.into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8() {
        let text = "Hello, world!".as_bytes();
        assert_eq!(detect_encoding(text), "UTF-8");
    }

    #[test]
    fn test_detect_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"Hello");
        assert_eq!(detect_encoding(&bytes), "UTF-8-BOM");
    }

    #[test]
    fn test_detect_utf16_le_bom() {
        let bytes = vec![0xFF, 0xFE, 0x48, 0x00]; // "H" in UTF-16LE
        assert_eq!(detect_encoding(&bytes), "UTF-16LE");
    }

    #[test]
    fn test_detect_utf16_be_bom() {
        let bytes = vec![0xFE, 0xFF, 0x00, 0x48]; // "H" in UTF-16BE
        assert_eq!(detect_encoding(&bytes), "UTF-16BE");
    }

    #[test]
    fn test_detect_latin1() {
        // Invalid UTF-8 sequence that's valid Latin-1
        let bytes = vec![0xE4, 0xF6, 0xFC]; // äöü in Latin-1
        assert_eq!(detect_encoding(&bytes), "Windows-1252");
    }

    #[test]
    fn test_decode_utf8() {
        let text = "Hello, 世界!".as_bytes().to_vec();
        let result = decode_bytes(text, "UTF-8").unwrap();
        assert_eq!(result, "Hello, 世界!");
    }

    #[test]
    fn test_decode_utf8_bom() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice("Hello".as_bytes());
        let result = decode_bytes(bytes, "UTF-8-BOM").unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_decode_latin1() {
        let bytes = vec![0xE4, 0xF6, 0xFC]; // äöü in Windows-1252/Latin-1
        let result = decode_bytes(bytes, "Windows-1252").unwrap();
        assert!(result.contains('ä') || result.contains('ö') || result.contains('ü'));
    }
}
