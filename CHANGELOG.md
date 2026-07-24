# Changelog

## 0.3.0 (unreleased)

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
