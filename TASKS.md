# mat Implementation Tasks

Detailed breakdown of each phase from PLAN.md into actionable implementation tasks.

---

## Phase 1: Project Setup, CLI & Error Types

### 1.1 Initialize Project
- [ ] Run `cargo init` in project directory
- [ ] Set up Cargo.toml with package metadata (name, version, edition, description, license)
- [ ] Add all dependencies from PLAN.md
- [ ] Add dev-dependencies (tempfile, assert_cmd, predicates, proptest)
- [ ] Verify `cargo build` succeeds

### 1.2 Create Error Types (`src/error.rs`)
- [ ] Define `MatError` enum with thiserror
- [ ] Add variants:
  - `Io { source: std::io::Error, path: PathBuf }` - file I/O errors
  - `InvalidRegex { source: regex::Error, pattern: String }` - bad regex
  - `EmptyPattern` - empty search/grep pattern
  - `BinaryFile { path: PathBuf }` - binary file detected
  - `InvalidLineRange { range: String }` - bad --lines format
  - `EncodingError { path: PathBuf }` - encoding detection failed
- [ ] Implement `From` conversions where appropriate
- [ ] Define exit code constants: `EXIT_SUCCESS = 0`, `EXIT_ERROR = 1`, `EXIT_INVALID_ARGS = 2`

### 1.3 Create CLI Definitions (`src/cli.rs`)
- [ ] Define `Args` struct with clap derive
- [ ] Add all flags from PLAN.md CLI reference:
  - `file: Option<PathBuf>` - input file (positional)
  - `line_numbers: bool` (-n)
  - `no_highlight: bool` (-N)
  - `markdown: bool` (-m)
  - `no_markdown: bool` (-M)
  - `follow: bool` (-f)
  - `search: Option<String>` (-s)
  - `grep: Option<String>` (-g)
  - `ignore_case: bool` (-i)
  - `fixed_strings: bool` (-F)
  - `word_regexp: bool` (-w)
  - `line_regexp: bool` (-x)
  - `after: Option<usize>` (-A)
  - `before: Option<usize>` (-B)
  - `context: Option<usize>` (-C)
  - `wrap: Option<WrapMode>` (--wrap)
  - `max_width: Option<usize>` (-W)
  - `language: Option<String>` (-l)
  - `theme: Option<String>` (-t)
  - `lines: Option<String>` (-L)
  - `no_pager: bool` (-P)
  - `ansi: bool` (--ansi)
  - `force_binary: bool` (--force-binary)
- [ ] Define `WrapMode` enum: None, Wrap, Truncate
- [ ] Add help descriptions for all flags
- [ ] Add version flag

### 1.4 Create Main Entry Point (`src/main.rs`)
- [ ] Parse CLI args with clap
- [ ] Set up basic error handling with anyhow
- [ ] Return appropriate exit codes
- [ ] Stub out main logic (just print args for now)

### 1.5 Verify Phase 1 Complete
- [ ] `mat --help` displays all flags with descriptions
- [ ] `mat --version` displays version
- [ ] `mat nonexistent.txt` returns exit code 1 (placeholder)
- [ ] `cargo test` passes (no tests yet, but compiles)

---

## Phase 2: Input Handling & Binary Detection

### 2.1 Create Input Module Structure
- [ ] Create `src/input/mod.rs` with module declarations
- [ ] Define `InputSource` enum: `File(PathBuf)`, `Stdin`
- [ ] Define `Content` struct:
  ```rust
  struct Content {
      text: String,
      source_name: String,      // filename or "stdin"
      extension: Option<String>,
      is_markdown: bool,
      encoding: String,         // "UTF-8", "Latin-1", etc.
  }
  ```
- [ ] Define `load_content(source: InputSource, args: &Args) -> Result<Content>`

### 2.2 Implement File Reading (`src/input/file.rs`)
- [ ] `read_file(path: &Path) -> Result<Vec<u8>>` - read raw bytes
- [ ] `detect_extension(path: &Path) -> Option<String>`
- [ ] `is_markdown_extension(ext: &str) -> bool` - check .md, .markdown, .mdown
- [ ] Handle file not found, permission denied errors with proper MatError

### 2.3 Implement Stdin Reading (`src/input/stdin.rs`)
- [ ] `read_stdin() -> Result<Vec<u8>>` - buffer entire stdin
- [ ] `is_stdin_piped() -> bool` - detect if stdin is a pipe vs TTY
- [ ] Handle stdin + no file argument case

### 2.4 Implement Binary Detection (`src/input/binary.rs`)
- [ ] `is_binary(bytes: &[u8]) -> bool`
  - Check for null bytes in first 8KB
  - Check proportion of non-printable chars (threshold: >30%)
- [ ] Return `Err(MatError::BinaryFile)` if binary and not `--force-binary`

### 2.5 Implement Encoding Detection (`src/input/encoding.rs`)
- [ ] `detect_encoding(bytes: &[u8]) -> &'static str`
  - Check for UTF-8 BOM (EF BB BF)
  - Check for UTF-16 BOM (FF FE or FE FF) - convert or error
  - Try UTF-8 validation
  - Fallback to Latin-1
- [ ] `decode_bytes(bytes: Vec<u8>, encoding: &str) -> Result<String>`
  - Use encoding_rs for conversion
  - Handle lossy conversion for invalid sequences

### 2.6 Implement ANSI Stripping
- [ ] `strip_ansi(text: &str) -> String` - remove ANSI escape sequences
- [ ] Only strip if `--ansi` flag is NOT set

### 2.7 Wire Up Input Loading
- [ ] Update `main.rs` to detect input source (file arg, `-`, or piped stdin)
- [ ] Call `load_content()` and print content info for testing

### 2.8 Verify Phase 2 Complete
- [ ] `mat file.txt` reads and displays file info
- [ ] `cat file.txt | mat` reads from stdin
- [ ] `mat -` reads from explicit stdin
- [ ] `mat binary.png` errors with "binary file" message
- [ ] `mat --force-binary binary.png` proceeds
- [ ] `mat latin1.txt` detects and converts encoding

---

## Phase 3: Core Line Abstraction

### 3.1 Create Display Module Structure
- [ ] Create `src/display/mod.rs`
- [ ] Create `src/display/line.rs`

### 3.2 Define Core Types (`src/display/line.rs`)
- [ ] Define `Style` struct (or use ratatui's Style directly):
  ```rust
  struct SpanStyle {
      fg: Option<Color>,
      bg: Option<Color>,
      bold: bool,
      italic: bool,
      underline: bool,
  }
  ```
- [ ] Define `StyledSpan`:
  ```rust
  struct StyledSpan {
      text: String,
      style: SpanStyle,
  }
  ```
- [ ] Define `Line`:
  ```rust
  struct Line {
      number: usize,           // original line number (1-indexed)
      spans: Vec<StyledSpan>,
      is_match: bool,          // for grep highlighting
      is_context: bool,        // for grep context lines
  }
  ```
- [ ] Define `Document`:
  ```rust
  struct Document {
      lines: Vec<Line>,
      max_line_width: usize,   // for horizontal scroll info
      source_name: String,
      encoding: String,
  }
  ```

### 3.3 Implement Conversions
- [ ] `Line::plain(number: usize, text: &str) -> Line` - unstyled line
- [ ] `Document::from_text(content: &Content) -> Document`
- [ ] `Line::width(&self) -> usize` - calculate display width using unicode-width
- [ ] `Document::max_width(&self) -> usize` - max line width in document

### 3.4 Verify Phase 3 Complete
- [ ] Can create Document from Content
- [ ] Line widths calculated correctly for ASCII
- [ ] Line widths calculated correctly for Unicode/CJK

---

## Phase 4: Basic Pager (MVP)

### 4.1 Create Pager Module Structure
- [ ] Create `src/pager/mod.rs`
- [ ] Create `src/pager/app.rs`
- [ ] Create `src/pager/ui.rs`
- [ ] Create `src/pager/input.rs`

### 4.2 Implement App State (`src/pager/app.rs`)
- [ ] Define `App` struct:
  ```rust
  struct App {
      document: Document,
      scroll_line: usize,      // top visible line
      scroll_col: usize,       // horizontal scroll offset
      mode: Mode,
      should_quit: bool,
      terminal_size: (u16, u16),
      show_line_numbers: bool,
  }
  ```
- [ ] Define `Mode` enum: `Normal`, `Search { query: String }`
- [ ] `App::new(document: Document, args: &Args) -> App`
- [ ] `App::visible_lines(&self) -> &[Line]` - slice of currently visible lines
- [ ] `App::scroll_down(&mut self, n: usize)`
- [ ] `App::scroll_up(&mut self, n: usize)`
- [ ] `App::scroll_left(&mut self, n: usize)`
- [ ] `App::scroll_right(&mut self, n: usize)`
- [ ] `App::scroll_to_line_start(&mut self)`
- [ ] `App::scroll_to_line_end(&mut self)`
- [ ] `App::go_to_top(&mut self)`
- [ ] `App::go_to_bottom(&mut self)`

### 4.3 Implement UI Rendering (`src/pager/ui.rs`)
- [ ] `render(frame: &mut Frame, app: &App)`
- [ ] Calculate layout: gutter (line numbers) + content + status bar
- [ ] Render line number gutter:
  - Show line numbers if enabled
  - Show current line / total lines indicator
- [ ] Render content area:
  - Apply horizontal scroll offset
  - Render styled spans
- [ ] Render status bar:
  - Left: file path or "stdin"
  - Center: mode indicator (if applicable)
  - Right: `Col X/Y`, encoding (if non-UTF-8)

### 4.4 Implement Input Handling (`src/pager/input.rs`)
- [ ] `handle_key(key: KeyEvent, app: &mut App) -> bool` (returns should_quit)
- [ ] Implement keybindings:
  - `j` / `↓` - scroll down 1
  - `k` / `↑` - scroll up 1
  - `h` / `←` - scroll left
  - `l` / `→` - scroll right
  - `d` / `PageDown` - scroll down half page
  - `u` / `PageUp` - scroll up half page
  - `0` - scroll to line start
  - `$` - scroll to line end
  - `g` / `Home` - go to top
  - `G` / `End` - go to bottom
  - `q` / `Esc` - quit

### 4.5 Implement Main Event Loop (`src/pager/mod.rs`)
- [ ] `run_pager(document: Document, args: &Args) -> Result<()>`
- [ ] Initialize terminal with crossterm (alternate screen, raw mode)
- [ ] Create ratatui Terminal
- [ ] Main loop: render → poll events → handle input
- [ ] Clean up terminal on exit (including panic handler)
- [ ] Handle Ctrl+C for clean exit
- [ ] Handle terminal resize events

### 4.6 Implement Line Range Selection
- [ ] Parse `--lines` argument:
  - `X:Y` → lines X to Y (inclusive)
  - `:Y` → lines 1 to Y
  - `X:` → lines X to end
  - `X` → just line X
- [ ] Filter document lines before passing to pager
- [ ] Validate range (return InvalidLineRange error if bad)

### 4.7 Implement No-Pager Mode
- [ ] If `--no-pager`, print document directly to stdout
- [ ] Handle SIGPIPE (ignore broken pipe errors)
- [ ] Apply styling if terminal supports it

### 4.8 Wire Up Main
- [ ] Update `main.rs` to:
  - Load content
  - Create document
  - Apply line range filter
  - Run pager or print directly

### 4.9 Verify Phase 4 Complete
- [ ] `mat file.txt` opens pager with scrolling
- [ ] j/k/h/l navigation works
- [ ] g/G jumps to top/bottom
- [ ] 0/$ jumps to line start/end
- [ ] Status bar shows filename, column info
- [ ] `mat -L 10:20 file.txt` shows only lines 10-20
- [ ] `mat -P file.txt` prints without pager
- [ ] Ctrl+C exits cleanly
- [ ] Terminal restored properly on exit

---

## Phase 5: Grep Filtering

### 5.1 Create Filter Module
- [ ] Create `src/filter/mod.rs`
- [ ] Create `src/filter/grep.rs`

### 5.2 Implement Grep Options
- [ ] Define `GrepOptions` struct:
  ```rust
  struct GrepOptions {
      pattern: Regex,
      before: usize,          // -B
      after: usize,           // -C
      context: usize,         // -C (overrides before/after)
  }
  ```
- [ ] `GrepOptions::from_args(args: &Args) -> Result<Option<GrepOptions>>`
- [ ] Build regex with case-insensitivity, word boundaries, line anchors as needed

### 5.3 Implement Grep Filtering
- [ ] `grep_filter(document: &Document, options: &GrepOptions) -> Document`
- [ ] For each line, check if pattern matches
- [ ] Track which lines are matches vs context
- [ ] Handle overlapping context ranges
- [ ] Preserve original line numbers

### 5.4 Implement Visual Separators
- [ ] Insert separator line (`--`) between non-contiguous match groups
- [ ] Style separator differently (dim/gray)

### 5.5 Wire Up Grep
- [ ] Apply grep filter after loading document, before pager
- [ ] Update Document with filtered lines

### 5.6 Verify Phase 5 Complete
- [ ] `mat -g pattern file.txt` shows only matching lines
- [ ] `mat -g pattern -C 2 file.txt` shows context
- [ ] `mat -g pattern -B 1 -A 3 file.txt` shows asymmetric context
- [ ] Line numbers are preserved from original file
- [ ] Separators appear between non-contiguous groups
- [ ] `mat -g -i PATTERN file.txt` is case-insensitive
- [ ] `mat -g -F 'literal[string'` treats pattern as literal

---

## Phase 6: Search Highlighting & Navigation

### 6.1 Create Search Highlight Module
- [ ] Create `src/highlight/mod.rs`
- [ ] Create `src/highlight/search.rs`

### 6.2 Implement Search Highlighting
- [ ] `apply_search_highlight(document: &mut Document, pattern: &Regex, style: SpanStyle)`
- [ ] For each line, find all matches
- [ ] Split spans at match boundaries
- [ ] Apply highlight style to matched portions
- [ ] Track match count and positions

### 6.3 Implement Match Tracking
- [ ] Define `MatchPosition { line: usize, start_col: usize, end_col: usize }`
- [ ] Store all match positions in App state
- [ ] Track current match index

### 6.4 Implement Match Navigation
- [ ] `App::next_match(&mut self)` - jump to next match (n key)
- [ ] `App::prev_match(&mut self)` - jump to previous match (N key)
- [ ] Scroll to show match in view
- [ ] Wrap around at document boundaries
- [ ] Update status bar with "Match X/Y"

### 6.5 Wire Up Search Highlighting
- [ ] Apply search highlight after document creation
- [ ] Update keybindings for n/N

### 6.6 Verify Phase 6 Complete
- [ ] `mat -s pattern file.txt` highlights all matches
- [ ] n jumps to next match, scrolling if needed
- [ ] N jumps to previous match
- [ ] Navigation wraps around
- [ ] Status shows current match number

---

## Phase 7: Theme Detection

### 7.1 Create Theme Module
- [ ] Create `src/theme/mod.rs`
- [ ] Create `src/theme/detect.rs`

### 7.2 Implement Theme Detection
- [ ] Use terminal-light to detect light/dark terminal
- [ ] Define `Theme` enum: `Light`, `Dark`
- [ ] `detect_theme() -> Theme`
- [ ] Use once_cell for lazy initialization

### 7.3 Define Color Schemes
- [ ] Define colors for each theme:
  - Line numbers (dim)
  - Status bar (inverted)
  - Search highlight (yellow/bright)
  - Match line highlight (subtle)
  - Context line (dim)
  - Separator (dim)
  - Error messages (red)
- [ ] Create `ThemeColors` struct with all colors

### 7.4 Implement Theme Override
- [ ] Parse `--theme` flag
- [ ] List available themes in help text
- [ ] Override detected theme if specified

### 7.5 Wire Up Theme
- [ ] Pass theme colors to UI renderer
- [ ] Apply theme to all UI elements

### 7.6 Verify Phase 7 Complete
- [ ] Theme auto-detected correctly
- [ ] Colors look good on light terminal
- [ ] Colors look good on dark terminal
- [ ] `--theme dark` forces dark theme
- [ ] `--theme light` forces light theme

---

## Phase 8: Syntax Highlighting

### 8.1 Create Syntax Highlight Module
- [ ] Create `src/highlight/syntax.rs`

### 8.2 Initialize Syntect
- [ ] Load default syntax set (lazy via once_cell)
- [ ] Load theme set
- [ ] Map terminal theme (light/dark) to syntect theme

### 8.3 Implement Language Detection
- [ ] `detect_language(extension: Option<&str>, first_line: &str) -> Option<SyntaxReference>`
- [ ] Check file extension first
- [ ] Check shebang/modeline as fallback
- [ ] Handle `--language` override

### 8.4 Implement Syntax Highlighting
- [ ] `apply_syntax_highlight(lines: &mut [Line], syntax: &SyntaxReference, theme: &Theme)`
- [ ] Only highlight visible lines + buffer (e.g., +/- 50 lines)
- [ ] Map syntect styles to SpanStyle
- [ ] Cache highlighted line ranges

### 8.5 Implement Highlight Caching
- [ ] Use LRU cache for highlighted lines
- [ ] Invalidate cache on scroll beyond buffer
- [ ] Re-highlight on theme change

### 8.6 Wire Up Syntax Highlighting
- [ ] Apply after document creation, before search highlight
- [ ] Skip if `--no-highlight` flag

### 8.7 Verify Phase 8 Complete
- [ ] `.rs` files show Rust highlighting
- [ ] `.py` files show Python highlighting
- [ ] Unknown extensions show plain text
- [ ] `--language rust` forces Rust highlighting
- [ ] `--no-highlight` disables highlighting
- [ ] Performance acceptable for large files

---

## Phase 9: Interactive Search

### 9.1 Create Search Mode Module
- [ ] Create `src/pager/search.rs`

### 9.2 Implement Search Mode State
- [ ] Define `SearchState`:
  ```rust
  struct SearchState {
      query: String,
      cursor_pos: usize,
      error: Option<String>,  // regex parse error
  }
  ```
- [ ] Add to App mode enum

### 9.3 Implement Search Input
- [ ] Handle `/` to enter search mode
- [ ] Handle character input (append to query)
- [ ] Handle Backspace (delete char)
- [ ] Handle Enter (confirm search)
- [ ] Handle Esc (cancel search)
- [ ] Handle Ctrl+U (clear query)

### 9.4 Implement Incremental Search
- [ ] On each keystroke, try to compile regex
- [ ] If valid, apply highlighting incrementally
- [ ] If invalid, show error in status bar
- [ ] Jump to first match as user types

### 9.5 Implement Search UI
- [ ] Render search prompt in status bar: `/query_here`
- [ ] Show cursor position
- [ ] Show error message if regex invalid
- [ ] Show match count

### 9.6 Wire Up Search Mode
- [ ] Update input handler for search mode
- [ ] Update UI renderer for search mode
- [ ] Preserve search after exiting search mode

### 9.7 Verify Phase 9 Complete
- [ ] `/` opens search prompt
- [ ] Typing shows incremental matches
- [ ] Enter confirms and exits prompt
- [ ] Esc cancels search
- [ ] n/N navigate matches after search
- [ ] Invalid regex shows error

---

## Phase 10: Markdown Rendering

### 10.1 Create Markdown Module
- [ ] Create `src/markdown/mod.rs`
- [ ] Create `src/markdown/render.rs`

### 10.2 Implement Markdown Parser
- [ ] Use pulldown-cmark to parse markdown
- [ ] `render_markdown(text: &str, width: usize) -> Vec<Line>`
- [ ] Handle document structure events

### 10.3 Implement Element Rendering
- [ ] **Headers:** Bold, color by level (H1 > H2 > H3), blank lines around
- [ ] **Paragraphs:** Word wrap at width, blank line after
- [ ] **Code blocks:** Syntax highlight content, border/indent
- [ ] **Inline code:** Background color, monospace
- [ ] **Bold:** Bold style
- [ ] **Italic:** Italic style (if terminal supports) or color
- [ ] **Links:** Show `text (url)` or just `text` with underline
- [ ] **Lists:** Bullet points (•, ◦, ▪), numbered lists, proper indentation
- [ ] **Blockquotes:** Vertical bar prefix, indented, different color
- [ ] **Horizontal rules:** Full-width line (─────)
- [ ] **Tables:** Box-drawing borders, column alignment

### 10.4 Implement Markdown Detection
- [ ] Auto-detect if content is markdown:
  - Check file extension (.md, .markdown, .mdown)
  - Check `--markdown` flag
  - Respect `--no-markdown` flag

### 10.5 Wire Up Markdown Rendering
- [ ] If markdown, render before creating Document
- [ ] Rendered output becomes the new document text

### 10.6 Verify Phase 10 Complete
- [ ] `.md` files render with formatting
- [ ] Headers are styled and sized
- [ ] Code blocks are highlighted
- [ ] Lists render correctly
- [ ] Tables render with borders
- [ ] `--no-markdown` shows raw markdown
- [ ] `--markdown` forces rendering on any file

---

## Phase 11: Follow Mode

### 11.1 Create Follow Module
- [ ] Create `src/input/follow.rs`

### 11.2 Implement Follow Reader
- [ ] `FollowReader` struct:
  ```rust
  struct FollowReader {
      file: File,
      path: PathBuf,
      last_size: u64,
      last_modified: SystemTime,
  }
  ```
- [ ] `FollowReader::new(path: PathBuf) -> Result<Self>`
- [ ] `FollowReader::poll(&mut self) -> Option<String>` - check for new content

### 11.3 Implement Follow Logic
- [ ] Poll file every 100ms for changes
- [ ] Read only new bytes since last check
- [ ] Handle file truncation (log rotation)
- [ ] Handle file deletion/recreation

### 11.4 Implement Follow Mode UI
- [ ] Add `follow: bool` to App state
- [ ] Show `[FOLLOW]` indicator in status bar
- [ ] Auto-scroll to bottom on new content
- [ ] `f` key toggles follow mode

### 11.5 Implement Follow Event Loop
- [ ] Modify main loop to poll follow reader
- [ ] Append new lines to document
- [ ] Update display

### 11.6 Wire Up Follow Mode
- [ ] Enable with `-f` flag
- [ ] Error if stdin + follow (cannot follow stdin)

### 11.7 Verify Phase 11 Complete
- [ ] `mat -f log.txt` follows file
- [ ] New lines appear automatically
- [ ] View auto-scrolls to bottom
- [ ] `f` toggles follow on/off
- [ ] `[FOLLOW]` indicator shows in status
- [ ] `mat -f -` errors appropriately

---

## Phase 12: Line Handling & Polish

### 12.1 Implement Wrap Modes
- [ ] `WrapMode::None` - horizontal scroll (already done)
- [ ] `WrapMode::Wrap` - soft wrap at terminal width
- [ ] `WrapMode::Truncate` - hard cut at `--max-width`
- [ ] Apply wrap mode in renderer

### 12.2 Implement Soft Wrapping
- [ ] Break lines at terminal width
- [ ] Preserve original line numbers (wrapped continuations have no number)
- [ ] Handle wrap + horizontal scroll (should disable h-scroll when wrapping)

### 12.3 Implement Hard Truncation
- [ ] Cut lines at max_width
- [ ] Show truncation indicator (`…` or `>`)

### 12.4 Improve Unicode Width Handling
- [ ] Use unicode-width for all width calculations
- [ ] Handle zero-width characters
- [ ] Handle combining characters
- [ ] Test with CJK characters
- [ ] Test with emoji

### 12.5 Implement Terminal Resize
- [ ] Handle SIGWINCH / resize events
- [ ] Recalculate layout
- [ ] Adjust scroll position if needed
- [ ] Re-wrap if in wrap mode

### 12.6 Polish UI
- [ ] Ensure colors work on 16-color terminals
- [ ] Test on various terminal emulators
- [ ] Improve status bar layout

### 12.7 Verify Phase 12 Complete
- [ ] `--wrap wrap` soft wraps lines
- [ ] `--wrap truncate` hard truncates
- [ ] Wrap mode disables horizontal scroll
- [ ] CJK characters display correctly
- [ ] Terminal resize works smoothly

---

## Phase 13: Large File Support

### 13.1 Create Large File Module
- [ ] Create `src/input/large.rs`

### 13.2 Implement File Size Detection
- [ ] Check file size before reading
- [ ] Threshold: 10MB for lazy loading
- [ ] Also consider line count (many short lines vs few long lines)

### 13.3 Implement Lazy Document
- [ ] `LazyDocument` struct:
  ```rust
  struct LazyDocument {
      mmap: Mmap,
      line_offsets: Vec<u64>,  // byte offset of each line start
      line_cache: LruCache<usize, Line>,
      total_lines: usize,
  }
  ```
- [ ] Scan file once to build line offset index
- [ ] Memory-map file for random access

### 13.4 Implement On-Demand Loading
- [ ] `LazyDocument::get_line(n: usize) -> &Line`
- [ ] Load line from mmap if not cached
- [ ] Apply syntax highlighting on load
- [ ] LRU cache for recently viewed lines

### 13.5 Implement Lazy Document Pager Integration
- [ ] Modify pager to work with LazyDocument
- [ ] Only request visible lines + buffer
- [ ] Handle grep with streaming (don't load all lines)

### 13.6 Verify Phase 13 Complete
- [ ] 100MB file opens instantly
- [ ] Memory usage stays low
- [ ] Scrolling is smooth
- [ ] Grep works on large files
- [ ] Syntax highlighting works on visible portion

---

## Phase 14: Integration, Edge Cases & Testing

### 14.1 Test Feature Combinations
- [ ] grep + search + syntax highlighting
- [ ] markdown + search
- [ ] follow + grep
- [ ] stdin + all features
- [ ] large file + all features

### 14.2 Address Input Edge Cases
- [ ] Stdin + follow mode → error
- [ ] Very long single line (10MB)
- [ ] Mixed line endings (normalize to LF)
- [ ] Empty files
- [ ] Files with only whitespace

### 14.3 Address Character Edge Cases
- [ ] CJK characters width
- [ ] Emoji (multi-codepoint)
- [ ] Zero-width characters
- [ ] Tab expansion (default 4 spaces)
- [ ] Control characters (display placeholder)

### 14.4 Address ANSI Edge Cases
- [ ] Strip ANSI by default
- [ ] Preserve with --ansi
- [ ] Handle malformed ANSI sequences

### 14.5 Address Markdown Edge Cases
- [ ] Footnotes (render or skip)
- [ ] HTML tags (strip)
- [ ] Images (show alt text)
- [ ] Task lists
- [ ] Deep nesting

### 14.6 Address Error Conditions
- [ ] File not found → clear message, exit 1
- [ ] Permission denied → clear message, exit 1
- [ ] Invalid regex → message with position, exit 2
- [ ] Invalid line range → message, exit 2
- [ ] Invalid encoding → fallback or error

### 14.7 Write Integration Tests
- [ ] CLI argument parsing tests
- [ ] File reading tests
- [ ] Grep filtering tests
- [ ] Search highlighting tests
- [ ] Markdown rendering tests (snapshot)
- [ ] Large file tests

### 14.8 Write Unit Tests
- [ ] Binary detection
- [ ] Encoding detection
- [ ] Line range parsing
- [ ] Regex building (word/line anchors)
- [ ] Width calculation

### 14.9 Final Polish
- [ ] Review all help text
- [ ] Ensure consistent error messages
- [ ] Test on Linux, macOS (if available)
- [ ] Performance profiling
- [ ] Documentation

### 14.10 Verify Phase 14 Complete
- [ ] All feature combinations work
- [ ] All edge cases handled
- [ ] All tests pass
- [ ] No panics in normal usage
- [ ] Clean exit in all scenarios
