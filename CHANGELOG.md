# Changelog

## 0.3.3 (2026-09-26)

- Rendered Markdown tables with borders, bold headers, and aligned columns
- Honored left, center, and right cell alignment with Unicode-aware widths
- Preserved inline formatting and handled empty cells and escaped pipes in tables

## 0.3.2 (2026-09-26)

- Made terminal-width wrapping the default, including direct output
- Added `w` to switch between wrapping and horizontal scrolling in the pager
- Displayed the active wrapping mode and toggle key in the status bar
- Preserved reading position during reflow and kept the bottom visible on resize
  and line-number changes

## 0.3.1 (2026-09-16)

- Fixed pager crashes after wrapping and gutter changes, including panic cleanup
- Fixed whole-word and whole-line matching of regex alternatives
- Preserved newline and multiline context in syntax highlighting and line ranges
- Fixed follow-mode startup and preserved wrapped positions during reloads
- Applied color policies consistently to search, reloads, and terminal output
- Preserved blank lines in Markdown code blocks and corrected list prefixes
- Added Unicode-aware wrapping and truncation to direct output
- Prevented grep context overflow and removed quadratic match lookups
- Surfaced watcher and reload errors while retaining the last good document
- Added real pager regression tests to Linux and macOS CI

## 0.3.0

- Added automatic direct output and `--color auto|always|never`
- Added safe SGR-only `--ansi` parsing and terminal-state restoration
- Unified initial load, reload, and follow processing
- Fixed BOM-aware UTF-16 detection and documented Windows-1252 fallback
- Preserved base styling under grep/search overlays and dimmed context lines
- Improved wrapping, display-width calculations, rotation handling, and CLI validation
- Removed the unused memory-mapped large-file prototype and modernized dependencies
- Raised the minimum supported Rust version to 1.88

## 0.2.1

- Fixed reloads so Markdown rendering and configured filters/highlighting are preserved

## 0.2.0

- File change detection with automatic notification when viewed file is modified externally
- Press `R` to reload file, preserving scroll position and search state
- Visual scroll progress bar in status bar
- Interactive line number toggle with `#` key showing total lines count
- Case-insensitive (`/`) and case-sensitive (`?`) search modes
- Bash syntax highlighting support
- TOML syntax highlighting support
- Kotlin syntax highlighting support

## 0.1.0

- Initial release
- Terminal file viewer combining cat, less, and grep functionality
- Syntax highlighting with syntect
- Markdown rendering with pulldown-cmark
- Interactive search with incremental highlighting
- Follow mode for tailing files (`-f` flag)
- Line wrapping and truncation modes
- Grep filtering with context lines
- Support for multiple encodings (UTF-8, UTF-16, Latin-1)
- Theme detection and adaptive colors
- Large file support with lazy loading
