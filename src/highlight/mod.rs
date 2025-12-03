mod search;
mod syntax;

#[allow(unused_imports)]
pub use search::{apply_search_highlight, MatchPosition, SearchState};
#[allow(unused_imports)]
pub use syntax::{apply_syntax_highlight, detect_language};
