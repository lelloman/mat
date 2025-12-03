# mat Implementation Plan

## Overview

A Rust CLI tool combining cat, less, grep functionality with markdown rendering and syntax highlighting.

---

## CLI Reference

### Usage

```
mat [OPTIONS] <FILE>   # single file only
mat [OPTIONS] -        # explicit stdin
cat file | mat         # implicit stdin
```

### Options

| Flag | Long                | Arg        | Description                                     |
| ---- | ------------------- | ---------- | ----------------------------------------------- |
| `-n` | `--line-numbers`    |            | Show line numbers (off by default)              |
| `-N` | `--no-highlight`    |            | Disable syntax highlighting                     |
| `-m` | `--markdown`        |            | Force markdown rendering                        |
| `-M` | `--no-markdown`     |            | Disable markdown auto-detection                 |
| `-f` | `--follow`          |            | Follow mode (tail -f style)                     |
| `-s` | `--search`          | `<PAT>`    | Highlight pattern matches                       |
| `-g` | `--grep`            | `<PAT>`    | Filter to matching lines                        |
| `-i` | `--ignore-case`     |            | Case-insensitive for search/grep                |
| `-F` | `--fixed-strings`   |            | Treat pattern as literal string, not regex      |
| `-w` | `--word-regexp`     |            | Match whole words only                          |
| `-x` | `--line-regexp`     |            | Match whole lines only                          |
| `-A` | `--after`           | `<N>`      | Lines after grep match                          |
| `-B` | `--before`          | `<N>`      | Lines before grep match                         |
| `-C` | `--context`         | `<N>`      | Lines before+after grep match                   |
|      | `--wrap`            | `<MODE>`   | Line wrap: none, wrap, truncate (default: none) |
| `-W` | `--max-width`       | `<N>`      | Max line width before truncation (default: 200) |
| `-l` | `--language`        | `<LANG>`   | Force syntax highlighting language              |
| `-t` | `--theme`           | `<NAME>`   | Select color theme                              |
| `-L` | `--lines`           | `<RANGE>`  | Show line range: 50:100, :100, 50:, or 50       |
| `-P` | `--no-pager`        |            | Direct output, skip TUI                         |
|      | `--ansi`            |            | Preserve ANSI escape codes in input             |
|      | `--force-binary`    |            | Force display of binary files                   |
| `-V` | `--version`         |            | Show version                                    |
| `-h` | `--help`            |            | Show help                                       |

### Pager Keybindings

| Key               | Action                 |
| ----------------- | ---------------------- |
| `j` / `↓`         | Scroll down one line   |
| `k` / `↑`         | Scroll up one line     |
| `h` / `←`         | Scroll left            |
| `l` / `→`         | Scroll right           |
| `d` / `Page Down` | Scroll down half page  |
| `u` / `Page Up`   | Scroll up half page    |
| `0`               | Scroll to line start   |
| `$`               | Scroll to line end     |
| `g` / `Home`      | Go to top              |
| `G` / `End`       | Go to bottom           |
| `/`               | Open search prompt     |
| `n`               | Next search match      |
| `N`               | Previous search match  |
| `f`               | Toggle follow mode     |
| `q` / `Esc`       | Quit                   |

---

## Architecture

### Data Flow

```
Input Source (file/stdin)
    ↓
Binary Detection (warn/exit or continue)
    ↓
Encoding Detection (UTF-8, Latin-1, etc.)
    ↓
Content Buffer (String + metadata)
    ↓
Markdown Renderer (if applicable)
    ↓
Grep Filter (optional, with context) [operates on rendered output]
    ↓
Line Collection (Vec<Line> with line numbers, match info)
    ↓
Syntax Highlighter (syntect) [visible lines + buffer only]
    ↓
Search Highlighter (overlay on syntax) [operates on rendered output]
    ↓
Display Renderer (line numbers, final styling)
    ↓
Pager TUI (ratatui)
```

### Module Structure

```
src/
├── main.rs              # Entry point, CLI setup
├── cli.rs               # Clap argument definitions
├── error.rs             # Error types (defined early)
├── input/
│   ├── mod.rs
│   ├── file.rs          # File reading
│   ├── stdin.rs         # Stdin buffering
│   ├── follow.rs        # Follow mode (tail -f)
│   ├── binary.rs        # Binary file detection
│   ├── encoding.rs      # Encoding detection
│   └── large.rs         # Large file lazy loading (mmap)
├── filter/
│   ├── mod.rs
│   └── grep.rs          # Grep filtering with context
├── highlight/
│   ├── mod.rs
│   ├── syntax.rs        # Syntax highlighting (syntect)
│   └── search.rs        # Search pattern highlighting
├── markdown/
│   ├── mod.rs
│   └── render.rs        # Markdown → styled text
├── display/
│   ├── mod.rs
│   ├── line.rs          # Line abstraction with styling
│   └── renderer.rs      # Final display preparation
├── pager/
│   ├── mod.rs
│   ├── app.rs           # Main TUI app state
│   ├── ui.rs            # UI rendering
│   ├── input.rs         # Keybinding handling
│   └── search.rs        # Interactive search mode
└── theme/
    ├── mod.rs
    └── detect.rs        # Terminal theme detection
```

---

## Implementation Phases

### Phase 1: Project Setup, CLI & Error Types

- [ ] **Status: Not Started**

**Goal:** Bootable project with argument parsing and proper error handling foundation

**Tasks:**

1. Initialize Cargo project with workspace
2. Add dependencies to Cargo.toml
3. Create `error.rs` with custom error types using thiserror
4. Create `cli.rs` with clap derive structs
5. Create `main.rs` skeleton with error propagation

**Deliverable:** `mat --help` works, error types defined

---

### Phase 2: Input Handling & Binary Detection

- [ ] **Status: Not Started**

**Goal:** Read content from file or stdin, detect binary files and encoding

**Tasks:**

1. Create `input/mod.rs` with `InputSource` enum
2. Implement `input/file.rs`: Read file, detect extension
3. Implement `input/stdin.rs`: Buffer stdin, detect piped input
4. Implement `input/binary.rs`: Detect binary content (null bytes, non-printable chars)
5. Implement `input/encoding.rs`: Detect and handle UTF-8, Latin-1, UTF-8 BOM
6. Create `Content` struct with text, source_name, extension, is_markdown
7. Handle `--force-binary` and `--ansi` flags

**Deliverable:** Can load and identify content, binary detection works, encoding handled

---

### Phase 3: Core Line Abstraction

- [ ] **Status: Not Started**

**Goal:** Unified line representation with styling

**Tasks:**

1. Create `display/line.rs` with StyledSpan, Line, Document structs
2. Implement conversion from raw text to `Vec<Line>`

**Deliverable:** Text → Line abstraction working

---

### Phase 4: Basic Pager (MVP)

- [ ] **Status: Not Started**

**Goal:** Scrollable full-terminal view with horizontal scrolling

**Tasks:**

1. Create `pager/app.rs` with App struct (scroll position: line + column)
2. Create `pager/ui.rs`: Render lines, line number gutter (with line position), status bar (file, mode, col info)
3. Create `pager/input.rs`: j/k/h/l, arrows, g/G, 0/$, d/u, q keybindings
4. Main event loop in `pager/mod.rs`
5. Implement horizontal scrolling (default: no wrap)
6. Implement `--lines` flag for line range selection (formats: X:Y, :Y, X:, X)
7. Implement `--no-pager` flag for direct output
8. Use alternate screen buffer, handle Ctrl+C for clean exit

**Deliverable:** Basic pager viewing plain text files with vertical + horizontal scrolling

---

### Phase 5: Grep Filtering

- [ ] **Status: Not Started**

**Goal:** Filter lines with context support

**Tasks:**

1. Create `filter/grep.rs` with GrepOptions and grep_filter function
2. Implement context line tracking (is_match, is_context)
3. Handle overlapping contexts, preserve original line numbers
4. Add visual separator between non-contiguous matches

**Deliverable:** `-g pattern -C 2` works correctly

---

### Phase 6: Search Highlighting & Navigation

- [ ] **Status: Not Started**

**Goal:** Highlight pattern matches in content

**Tasks:**

1. Create `highlight/search.rs` with apply_search_highlight function
2. Split spans at match boundaries
3. Apply highlight style to matched portions
4. Track match positions for navigation
5. Implement n/N navigation in pager

**Deliverable:** `-s pattern` highlights all matches, n/N navigates

---

### Phase 7: Theme Detection

- [ ] **Status: Not Started**

**Goal:** Auto-detect terminal theme for proper color schemes

**Tasks:**

1. Create `theme/detect.rs` with terminal-light integration
2. Define light/dark theme variants
3. Create coherent color schemes for all UI elements
4. Implement `--theme` flag for manual override
5. Use once_cell for lazy theme initialization

**Deliverable:** Theme auto-detection works, `--theme` override available

---

### Phase 8: Syntax Highlighting

- [ ] **Status: Not Started**

**Goal:** Language-aware code coloring

**Tasks:**

1. Create `highlight/syntax.rs` with apply_syntax_highlight function
2. Initialize syntect with default themes (lazy via once_cell)
3. Map syntect styles → ratatui styles
4. Handle unknown languages gracefully (plain text fallback)
5. Implement `--language` flag for manual override
6. Only highlight visible lines + buffer for performance
7. Cache highlighted results

**Deliverable:** Code files show syntax highlighting

---

### Phase 9: Interactive Search

- [ ] **Status: Not Started**

**Goal:** `/` prompt for live search

**Tasks:**

1. Create `pager/search.rs`: Search input mode, incremental highlighting
2. Update `pager/app.rs` with Mode enum (Normal, Search)
3. Render search prompt in status bar area

**Deliverable:** `/` opens search, typing highlights live

---

### Phase 10: Markdown Rendering

- [ ] **Status: Not Started**

**Goal:** Pretty-print markdown files

**Tasks:**

1. Create `markdown/render.rs` with render_markdown function
2. Use pulldown-cmark to parse
3. Render: Headers, Code blocks, Inline code, Bold/Italic, Lists, Links, Blockquotes, Horizontal rules, Tables
4. Handle line wrapping for prose

**Deliverable:** `.md` files render beautifully

---

### Phase 11: Follow Mode

- [ ] **Status: Not Started**

**Goal:** tail -f style live updates

**Tasks:**

1. Create `input/follow.rs` with FollowReader struct
2. Non-blocking file polling (100ms intervals)
3. Update pager with new lines
4. Auto-scroll to bottom on new content
5. `f` key to toggle follow mode
6. Visual indicator in status bar

**Deliverable:** `-f` follows file updates, `f` toggles

---

### Phase 12: Line Handling & Polish

- [ ] **Status: Not Started**

**Goal:** Handle wrap modes, polish UI

**Tasks:**

1. Implement WrapMode enum: None (default), Wrap, Truncate
2. Implement `--wrap` flag to switch modes
3. Use unicode-width for correct character width calculation
4. Handle terminal resize events
5. Add `--version` flag
6. Error messages for invalid files/patterns

**Deliverable:** Wrap modes working, polished UI

---

### Phase 13: Large File Support

- [ ] **Status: Not Started**

**Goal:** Handle files >10MB efficiently

**Tasks:**

1. Create `input/large.rs` with LazyDocument struct
2. Scan file once to build line offset index
3. Memory-map file, load lines on demand
4. LRU cache for recently viewed lines
5. Threshold: files >10MB use lazy loading

**Deliverable:** Large files load instantly, low memory usage

---

### Phase 14: Integration, Edge Cases & Testing

- [ ] **Status: Not Started**

**Goal:** Ensure all features work together seamlessly, comprehensive tests

**Tasks:**

1. Test combinations: grep+search+syntax, markdown+search, follow+grep, stdin+all, large file+all
2. Address all edge cases (see Edge Cases section below)
3. Polish status bar and UI elements
4. Write helpful `--help` descriptions
5. Implement full test suite (see Testing Strategy)

**Deliverable:** Robust, production-ready tool

---

## Testing Strategy

### Directory Structure

```
tests/
├── integration/
│   ├── cli_test.rs          # Full CLI argument tests
│   ├── pager_test.rs        # Pager behavior tests
│   ├── grep_test.rs         # Grep filtering tests
│   ├── markdown_test.rs     # Markdown rendering tests
│   └── large_file_test.rs   # Large file handling tests
├── fixtures/
│   ├── code/                # Sample source files (various languages)
│   ├── markdown/            # Sample markdown files
│   ├── binary/              # Binary file samples
│   ├── large/               # Large file samples (generated)
│   ├── edge_cases/          # Edge case files
│   └── encodings/           # Various encoding samples
└── unit/                    # Unit tests (inline in modules)
```

### Testing Approach

1. **Unit tests:** Inline in each module using `#[cfg(test)]`
2. **Integration tests:** Full CLI invocation tests in `tests/integration/`
3. **Fixtures:** Pre-made test files for consistent testing
4. **Property-based testing:** Consider proptest for edge cases
5. **Snapshot testing:** For markdown rendering output

### Key Test Scenarios

- All CLI flag combinations
- Empty files, single-line files, huge files
- All supported encodings
- Binary detection accuracy
- Grep with context edge cases
- Search highlighting correctness
- Markdown rendering fidelity
- Terminal resize handling
- Follow mode updates

---

## Edge Cases & Known Concerns

### Input Handling
- [ ] Stdin + follow mode: Should error with clear message
- [ ] Very long single line (e.g., 10MB single line): Handle gracefully
- [ ] Mixed line endings (CR, LF, CRLF in same file): Normalize
- [ ] Empty files: Display empty pager or message
- [ ] Files with only whitespace: Handle correctly

### Character Handling
- [ ] CJK characters: Test unicode-width thoroughly
- [ ] Emoji (multi-codepoint): Correct width calculation
- [ ] RTL text: Best-effort display (no full RTL support)
- [ ] Zero-width characters: Handle/strip appropriately
- [ ] Combining characters: Correct width calculation

### ANSI & Special Content
- [ ] ANSI escape codes: Strip by default, preserve with `--ansi`
- [ ] Tab characters: Expand to spaces (configurable width?)
- [ ] Control characters: Display placeholder or strip

### Markdown Rendering
- [ ] Footnotes: Render or skip gracefully
- [ ] HTML in markdown: Strip tags, render text content
- [ ] Images: Show `[alt text]` or `(image: filename)`
- [ ] Nested code blocks (fenced inside lists): Handle correctly
- [ ] Task lists (`- [ ]` checkboxes): Render as checkboxes
- [ ] Very deep nesting: Limit depth, handle gracefully
- [ ] Tables with wide content: Truncate or wrap cells

### Large Files
- [ ] Large file + grep: Stream/filter before loading all
- [ ] Large file + syntax highlighting: Only visible lines
- [ ] Line count threshold: Consider files with few very long lines

### Pattern/Regex Handling
- [ ] Invalid regex: Show clear error with syntax position
- [ ] Empty pattern: Error with helpful message ("did you mean to omit -s/-g?")
- [ ] Very complex regex: May be slow, but regex crate guarantees linear time

### Error Conditions
- [ ] File not found: Clear error message
- [ ] Permission denied: Clear error message
- [ ] Invalid regex pattern: Clear error message with position
- [ ] Invalid encoding: Fallback or error
- [ ] Disk full during follow: Handle gracefully

---

## Dependencies (Cargo.toml)

```toml
[package]
name = "mat"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
ratatui = "0.28"
crossterm = "0.28"
syntect = "5"
pulldown-cmark = "0.12"
terminal-light = "1"
regex = "1"
anyhow = "1"
thiserror = "1"              # Custom error types
once_cell = "1"              # Lazy initialization for syntect
memmap2 = "0.9"              # Memory-mapped files for large file support
unicode-width = "0.2"        # Correct character width calculation
lru = "0.12"                 # LRU cache for lazy-loaded lines
encoding_rs = "0.8"          # Encoding detection

[dev-dependencies]
tempfile = "3"               # Temporary files for tests
assert_cmd = "2"             # CLI testing
predicates = "3"             # Assertion helpers
proptest = "1"               # Property-based testing
```

---

## Key Design Decisions

### 1. Styling Pipeline

Markdown rendering runs first (if applicable), then syntax highlighting, then search highlighting overlays on top. This ensures search matches are visible and search/grep operate on rendered content.

### 2. Line Number Preservation

When grep filters lines, original line numbers are preserved. Context lines show their real line numbers, not sequential indices.

### 3. Markdown + Search/Grep Interaction

Both search and grep operate on the **rendered** markdown output, not raw markdown. This means searching for "header" finds rendered header text, not `# header`.

### 4. Memory Model

- Files <10MB: Read entirely into memory
- Files >10MB: Memory-mapped with lazy loading
- Stdin: Fully buffered (required for paging)

### 5. Syntax Highlighting Performance

- Only highlight visible lines + small buffer
- Cache highlighted results in LRU cache
- Lazy initialization of syntect via once_cell

### 6. Long Lines & Horizontal Scrolling

- **Default:** No wrap, horizontal scrolling enabled
- Status bar shows total columns (max line width) so user knows if content extends beyond view
- Horizontal scroll keys: `h`/`l`, `←`/`→`, `0` (start), `$` (end)
- **Wrap modes:**
  - `none` (default): Horizontal scrolling
  - `wrap`: Wrap at terminal width
  - `truncate`: Cut lines at `--max-width` (default: 200 chars)

### 7. Binary Files

- Detect binary content (null bytes, high proportion of non-printable chars)
- Default: Warn and exit
- Override with `--force-binary` flag

### 8. ANSI Escape Codes

- Default: Strip ANSI escape codes from input
- Preserve with `--ansi` flag

### 9. Encoding

- Detect encoding using encoding_rs
- Support UTF-8 (with/without BOM), Latin-1
- Fallback to lossy UTF-8 conversion

### 10. Line Numbers

- Off by default (unlike some pagers)
- Enable with `-n` / `--line-numbers` (matches grep convention)

### 11. Pattern Matching (Regex)

- **Default:** Patterns are regex (matches grep/ripgrep convention)
- **Flavor:** Rust `regex` crate (RE2-like syntax)
  - Fast and safe (guaranteed linear time)
  - No backreferences
  - No lookahead/lookbehind
  - Unicode-aware by default
- **Literal mode:** Use `-F` / `--fixed-strings` for literal matching
- **Modifiers:**
  - `-i` for case-insensitive
  - `-w` for whole-word matching (wraps pattern in `\b...\b`)
  - `-x` for whole-line matching (wraps pattern in `^...$`)
- **Errors:** Invalid regex shows clear error with syntax position

### 12. Status Bar

The status bar (bottom of screen) displays:
- **Left:** File path or `stdin`
- **Center:** Mode indicators: `[FOLLOW]`, `[SEARCH: pattern]`
- **Right:** `Col X/Y` (current column / max columns), encoding (if non-UTF-8)

Line position (current line / total lines) is shown in the left gutter alongside line numbers.

### 13. Exit Codes

| Code | Meaning |
|------|---------|
| `0`  | Success |
| `1`  | General error (file not found, permission denied, I/O error) |
| `2`  | Invalid arguments (bad regex, invalid flags, invalid line range) |

### 14. Terminal Behavior

- **Alternate screen:** Uses alternate screen buffer (like `less`), terminal restored on exit
- **Ctrl+C:** Clean exit, restores terminal state
- **SIGPIPE:** When using `--no-pager`, gracefully handle downstream pipe closure (e.g., `mat file | head`)

---

## Milestones

| Milestone            | After Phase | Description                          |
| -------------------- | ----------- | ------------------------------------ |
| **MVP**              | 4           | Working pager with scrolling         |
| **Searchable**       | 6           | Search highlighting working          |
| **Pretty**           | 10          | Syntax + markdown rendering          |
| **Feature Complete** | 13          | All core features implemented        |
| **Production Ready** | 14          | Robust, polished, edge cases handled |
