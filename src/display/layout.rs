use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use super::StyledSpan;

/// Split a line at display-cell boundaries without splitting grapheme clusters.
pub fn wrap_ranges(text: &str, width: usize) -> Vec<Range<usize>> {
    let width = width.max(1);
    let mut rows = Vec::new();
    let mut start = 0;
    let mut columns = 0;
    for (offset, grapheme) in text.grapheme_indices(true) {
        let size = UnicodeWidthStr::width(grapheme);
        if columns + size > width && columns > 0 {
            rows.push(start..offset);
            start = offset;
            columns = 0;
        }
        columns += size;
    }
    rows.push(start..text.len());
    rows
}

/// Retain styles while selecting a byte range with UTF-8 boundaries.
pub fn slice_spans(spans: &[StyledSpan], range: Range<usize>) -> Vec<StyledSpan> {
    let mut offset = 0;
    let mut result = Vec::new();
    for span in spans {
        let end = offset + span.text.len();
        if range.start < end && range.end > offset {
            let start = range.start.saturating_sub(offset);
            let end = range.end.min(end) - offset;
            result.push(StyledSpan::new(&span.text[start..end], span.style.clone()));
        }
        offset = end;
    }
    result
}
