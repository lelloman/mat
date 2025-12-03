# mat

A modern terminal file viewer combining the best of `cat`, `less`, and `grep` with syntax highlighting and markdown rendering.

## Features

- **Syntax Highlighting** - Automatic language detection with 50+ languages supported
- **Markdown Rendering** - Beautiful formatted output for `.md` files with styled headings, code blocks, lists, and more
- **Grep Filtering** - Filter content with regex patterns, with context lines support
- **Search & Navigate** - Highlight matches and jump between them with `n`/`N`
- **Follow Mode** - Tail files in real-time like `tail -f`
- **Large File Support** - Efficient memory-mapped loading for files >10MB
- **Theme Detection** - Automatically adapts to light/dark terminal themes

## Installation

```bash
cargo install mat
```

Or build from source:

```bash
git clone https://github.com/yourusername/mat
cd mat
cargo build --release
```

## Usage

```bash
# View a file
mat file.txt

# View with line numbers
mat -n file.txt

# Pipe from stdin
cat file.txt | mat
echo "hello world" | mat

# Force syntax highlighting language
cat config | mat -l yaml
```

### Grep Mode

Filter to matching lines (like `grep`, but with context and paging):

```bash
# Show only matching lines
mat -g "pattern" file.txt

# With context lines
mat -g "error" -C 3 logfile.txt    # 3 lines before and after
mat -g "error" -B 2 -A 5 log.txt   # 2 before, 5 after

# Case insensitive
mat -g -i "warning" file.txt

# Fixed string (not regex)
mat -g -F "literal[string" file.txt
```

### Search Mode

Highlight all matches of a pattern:

```bash
# Highlight pattern
mat -s "TODO" file.txt

# Then use n/N to jump between matches
```

### Markdown Rendering

```bash
# Auto-detected for .md files
mat README.md

# Force markdown rendering
mat -m somefile.txt

# Disable markdown rendering
mat -M README.md
```

### Follow Mode

Watch a file for changes (like `tail -f`):

```bash
mat -f /var/log/syslog
```

### Line Selection

View specific line ranges:

```bash
mat -L 10:20 file.txt   # Lines 10-20
mat -L 50: file.txt     # Line 50 to end
mat -L :100 file.txt    # First 100 lines
mat -L 42 file.txt      # Just line 42
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Scroll down one line |
| `k` / `↑` | Scroll up one line |
| `h` / `←` | Scroll left |
| `l` / `→` | Scroll right |
| `d` / `Page Down` | Scroll down half page |
| `u` / `Page Up` | Scroll up half page |
| `g` / `Home` | Go to top |
| `G` / `End` | Go to bottom |
| `0` | Scroll to line start |
| `$` | Scroll to line end |
| `/` | Open search prompt |
| `n` | Next search match |
| `N` | Previous search match |
| `f` | Toggle follow mode |
| `q` / `Esc` | Quit |

## Options

```
Usage: mat [OPTIONS] [FILE]

Arguments:
  [FILE]  File to view (use '-' for stdin)

Options:
  -n, --line-numbers      Show line numbers
  -N, --no-highlight      Disable syntax highlighting
  -m, --markdown          Force markdown rendering
  -M, --no-markdown       Disable markdown rendering
  -f, --follow            Follow mode (like tail -f)
  -s, --search <PATTERN>  Highlight pattern matches
  -g, --grep <PATTERN>    Filter to matching lines
  -i, --ignore-case       Case-insensitive search/grep
  -F, --fixed-strings     Treat pattern as literal string
  -w, --word-regexp       Match whole words only
  -x, --line-regexp       Match whole lines only
  -A, --after <N>         Lines to show after grep match
  -B, --before <N>        Lines to show before grep match
  -C, --context <N>       Lines to show before and after match
      --wrap <MODE>       Line wrap mode: none, wrap, truncate
  -W, --max-width <N>     Max line width for truncation
  -l, --language <LANG>   Force syntax highlighting language
  -t, --theme <THEME>     Color theme (light/dark)
  -L, --lines <RANGE>     Show line range (e.g., 10:20, :50, 100:)
  -P, --no-pager          Print directly without pager
      --ansi              Preserve ANSI escape codes in input
      --force-binary      Force display of binary files
  -h, --help              Print help
  -V, --version           Print version
```

## Highlighting

### Grep vs Search

- **Grep** (`-g`): Filters the file to show only matching lines (with optional context). Matches highlighted in **cyan**.
- **Search** (`-s`): Shows the entire file with matches highlighted in **yellow**. Use `n`/`N` to navigate.

You can use both together:
```bash
mat -g "error" -s "critical" logfile.txt
```

### Supported Languages

mat uses [syntect](https://github.com/trishume/syntect) for syntax highlighting and supports 50+ languages including:

Rust, Python, JavaScript, TypeScript, Go, C, C++, Java, Ruby, PHP, Swift, Kotlin, Scala, Haskell, Lua, Perl, R, SQL, HTML, CSS, JSON, YAML, TOML, Markdown, Bash, and many more.

## License

MIT
