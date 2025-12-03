# mat Implementation Plan

## Overview

A Rust CLI tool combining cat, less, grep functionality with markdown rendering and syntax highlighting.

---

## CLI Reference

### Usage

```
mat [OPTIONS] [FILE]
mat [OPTIONS] -        # explicit stdin
cat file | mat         # implicit stdin
```

### Options

| Flag | Long                | Arg      | Description                                     |
| ---- | ------------------- | -------- | ----------------------------------------------- |
| `-n` | `--no-line-numbers` |          | Hide line numbers                               |
| `-N` | `--no-highlight`    |          | Disable syntax highlighting                     |
| `-m` | `--markdown`        |          | Force markdown rendering                        |
| `-M` | `--no-markdown`     |          | Disable markdown auto-detection                 |
| `-f` | `--follow`          |          | Follow mode (tail -f style)                     |
| `-s` | `--search`          | `<PAT>`  | Highlight pattern matches                       |
| `-g` | `--grep`            | `<PAT>`  | Filter to matching lines                        |
| `-i` | `--ignore-case`     |          | Case-insensitive for search/grep                |
| `-A` | `--after`           | `<N>`    | Lines after grep match                          |
| `-B` | `--before`          | `<N>`    | Lines before grep match                         |
| `-C` | `--context`         | `<N>`    | Lines before+after grep match                   |
| `-w` | `--wrap`            | `<MODE>` | Line wrap: wrap, truncate, none (default: wrap) |
| `-W` | `--max-width`       | `<N>`    | Max line width before truncation (default: 200) |
|      | `--force-binary`    |          | Force display of binary files                   |
| `-V` | `--version`         |          | Show version                                    |
| `-h` | `--help`            |          | Show help                                       |

### Pager Keybindings

| Key               | Action                |
| ----------------- | --------------------- |
| `j` / `↓`         | Scroll down one line  |
| `k` / `↑`         | Scroll up one line    |
| `d` / `Page Down` | Scroll down half page |
| `u` / `Page Up`   | Scroll up half page   |
| `g` / `Home`      | Go to top             |
| `G` / `End`       | Go to bottom          |
| `/`               | Open search prompt    |
| `n`               | Next search match     |
| `N`               | Previous search match |
| `f`               | Toggle follow mode    |
| `q` / `Esc`       | Quit                  |

---

## Architecture

### Data Flow

```
Input Source (file/stdin)
    ↓
Content Buffer (String + metadata)
    ↓
Grep Filter (optional, with context)
    ↓
Line Collection (Vec<Line> with line numbers, match info)
    ↓
Syntax Highlighter (syntect)
    ↓
Search Highlighter (overlay on syntax)
    ↓
Markdown Renderer (if applicable)
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
├── input/
│   ├── mod.rs
│   ├── file.rs          # File reading
│   ├── stdin.rs         # Stdin buffering
│   ├── follow.rs        # Follow mode (tail -f)
│   ├── binary.rs        # Binary file detection
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

### Phase 1: Project Setup & CLI

- [ ] **Status: Not Started**

**Goal:** Bootable project with argument parsing

**Tasks:**

1. Initialize Cargo project with workspace
2. Add dependencies to Cargo.toml
3. Create `cli.rs` with clap derive structs
4. Create `main.rs` skeleton

**Deliverable:** `mat --help` works

---

### Phase 2: Input Handling

- [ ] **Status: Not Started**

**Goal:** Read content from file or stdin

**Tasks:**

1. Create `input/mod.rs` with `InputSource` enum
2. Implement `input/file.rs`: Read file, detect extension
3. Implement `input/stdin.rs`: Buffer stdin, detect piped input
4. Create `Content` struct with text, source_name, extension, is_markdown

**Deliverable:** Can load and identify content from file/stdin

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

**Goal:** Scrollable full-terminal view

**Tasks:**

1. Create `pager/app.rs` with App struct
2. Create `pager/ui.rs`: Render lines, line number gutter, status bar
3. Create `pager/input.rs`: j/k, arrows, g/G, d/u, q keybindings
4. Main event loop in `pager/mod.rs`

**Deliverable:** Basic pager viewing plain text files with scrolling

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

### Phase 6: Search Highlighting

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

### Phase 7: Interactive Search

- [ ] **Status: Not Started**

**Goal:** `/` prompt for live search

**Tasks:**

1. Create `pager/search.rs`: Search input mode, incremental highlighting
2. Update `pager/app.rs` with Mode enum (Normal, Search)
3. Render search prompt in status bar area

**Deliverable:** `/` opens search, typing highlights live

---

### Phase 8: Syntax Highlighting

- [ ] **Status: Not Started**

**Goal:** Language-aware code coloring

**Tasks:**

1. Create `highlight/syntax.rs` with apply_syntax_highlight function
2. Initialize syntect with default themes
3. Map syntect styles → ratatui styles
4. Handle unknown languages gracefully (plain text fallback)
5. Integrate theme detection with terminal-light

**Deliverable:** Code files show syntax highlighting

---

### Phase 9: Markdown Rendering

- [ ] **Status: Not Started**

**Goal:** Pretty-print markdown files

**Tasks:**

1. Create `markdown/render.rs` with render_markdown function
2. Use pulldown-cmark to parse
3. Render: Headers, Code blocks, Inline code, Bold/Italic, Lists, Links, Blockquotes, Horizontal rules, Tables
4. Handle line wrapping for prose

**Deliverable:** `.md` files render beautifully

---

### Phase 10: Follow Mode

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

### Phase 11: Theme Detection & Polish

- [ ] **Status: Not Started**

**Goal:** Auto-detect terminal theme, polish UI

**Tasks:**

1. Create `theme/detect.rs` with terminal-light integration
2. Create coherent color schemes for all UI elements
3. Handle terminal resize events
4. Add `--version` flag
5. Error messages for invalid files/patterns

**Deliverable:** Polished, theme-aware UI

---

### Phase 12: Binary Detection & Line Handling

- [ ] **Status: Not Started**

**Goal:** Handle binary files and long lines properly

**Tasks:**

1. Create `input/binary.rs` with is_binary function
2. Exit with warning by default, `--force-binary` to override
3. Implement WrapMode: Wrap, Truncate, None
4. Use unicode-width for correct character width calculation

**Deliverable:** Binary detection works, long lines handled correctly

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

### Phase 14: Integration & Polish

- [ ] **Status: Not Started**

**Goal:** Ensure all features work together seamlessly

**Tasks:**

1. Test combinations: grep+search+syntax, markdown+search, follow+grep, stdin+all, large file+all
2. Handle edge cases: Empty files, mixed line endings, Unicode, terminal resize
3. Add comprehensive error handling
4. Polish status bar and UI elements
5. Write helpful `--help` descriptions

**Deliverable:** Robust, production-ready tool

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
memmap2 = "0.9"          # Memory-mapped files for large file support
unicode-width = "0.2"    # Correct character width calculation
lru = "0.12"             # LRU cache for lazy-loaded lines
```

---

## Key Design Decisions

### 1. Styling Pipeline

Syntax highlighting runs first, then search highlighting overlays on top. This ensures search matches are visible even in syntax-highlighted code.

### 2. Line Number Preservation

When grep filters lines, original line numbers are preserved. Context lines show their real line numbers, not sequential indices.

### 3. Markdown + Search Interaction

Search operates on the **rendered** markdown output, not raw markdown. This means searching for "header" finds rendered header text, not `# header`.

### 4. Memory Model

- Files: Read entirely into memory (simplifies implementation)
- Large files (>10MB): Memory-mapped with lazy loading
- Stdin: Fully buffered (required for paging)

### 5. Long Lines

- Default: Wrap at terminal width
- Hard truncation at 200 chars (configurable via `--max-width`)
- Modes: `wrap` (default), `truncate`, `none`

### 6. Binary Files

- Detect binary content (null bytes, high proportion of non-printable chars)
- Default: Warn and exit
- Override with `--force-binary` flag

### 7. Large Files

- Implement streaming/lazy loading for files >10MB
- Only load visible portion + buffer
- Memory-map for random access

### 8. Markdown Tables

- Full table rendering with box-drawing characters
- Auto-detect column widths
- Handle overflow gracefully

---

## Milestones

| Milestone            | After Phase | Description                          |
| -------------------- | ----------- | ------------------------------------ |
| **MVP**              | 4           | Working pager with scrolling         |
| **Feature Complete** | 11          | All core features implemented        |
| **Production Ready** | 14          | Robust, polished, edge cases handled |
