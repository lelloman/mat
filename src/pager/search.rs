use regex::Regex;

use crate::display::Document;
use crate::filter::build_regex_pattern;
use crate::highlight::apply_search_highlight;

/// Interactive search state for the pager
pub struct InteractiveSearch {
    /// Current search query
    pub query: String,
    /// Whether to use case-insensitive search
    pub ignore_case: bool,
}

impl InteractiveSearch {
    /// Create a new interactive search
    pub fn new(ignore_case: bool) -> Self {
        Self {
            query: String::new(),
            ignore_case,
        }
    }

    /// Add a character to the search query
    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
    }

    /// Remove the last character from the search query
    pub fn pop_char(&mut self) {
        self.query.pop();
    }

    /// Clear the search query
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.query.clear();
    }

    /// Check if the query is empty
    pub fn is_empty(&self) -> bool {
        self.query.is_empty()
    }

    /// Compile the current query into a regex
    pub fn compile_pattern(&self) -> Option<Regex> {
        if self.query.is_empty() {
            return None;
        }

        let pattern = build_regex_pattern(&self.query, self.ignore_case, false, false, false);
        Regex::new(&pattern).ok()
    }

    /// Apply highlighting to the document based on current query
    pub fn apply_highlighting(&self, document: &mut Document) {
        if let Some(pattern) = self.compile_pattern() {
            apply_search_highlight(document, &pattern);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_search_basic() {
        let mut search = InteractiveSearch::new(false);
        assert!(search.is_empty());

        search.push_char('h');
        search.push_char('e');
        search.push_char('l');
        search.push_char('l');
        search.push_char('o');

        assert_eq!(search.query, "hello");
        assert!(!search.is_empty());

        search.pop_char();
        assert_eq!(search.query, "hell");

        search.clear();
        assert!(search.is_empty());
    }

    #[test]
    fn test_compile_pattern() {
        let mut search = InteractiveSearch::new(false);
        search.query = "test".to_string();

        let pattern = search.compile_pattern();
        assert!(pattern.is_some());

        let regex = pattern.unwrap();
        assert!(regex.is_match("this is a test"));
        assert!(!regex.is_match("This is a TEST")); // case sensitive
    }

    #[test]
    fn test_compile_pattern_case_insensitive() {
        let mut search = InteractiveSearch::new(true);
        search.query = "test".to_string();

        let pattern = search.compile_pattern();
        assert!(pattern.is_some());

        let regex = pattern.unwrap();
        assert!(regex.is_match("this is a TEST"));
    }
}
