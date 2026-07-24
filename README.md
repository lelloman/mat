# mat

`mat` is a terminal file viewer that combines direct `cat`-style output, an
interactive pager, grep filtering, search, syntax highlighting, and Markdown
rendering.

Version 0.3.0 is currently unreleased and requires Rust 1.88 or newer.

## Installation

```bash
cargo install mat-o-viewer
```

To build the unreleased version:

```bash
git clone https://github.com/lelloman/mat
cd mat
cargo build --release
```

## Output and color

`mat FILE` opens the pager when stdout is a terminal. When stdout is redirected
or piped, it prints directly; `--no-pager` also forces direct output.
`--follow` is pager-only.

`--color auto` is the default: terminal output is styled, while pipes and
`NO_COLOR` output are plain. `--color always` emits ANSI SGR styling even to a
pipe, and `--color never` disables generated styling and styling from `--ansi`.

Input escape sequences are removed by default. With `--ansi`, only SGR text
styling is interpreted. Cursor movement, OSC commands, hyperlinks, and other
terminal controls are discarded.

## Examples

```bash
mat README.md
mat -n src/main.rs
mat -g 'error|warning' -C 2 application.log
mat -s TODO src/main.rs
mat -L 10:30 file.txt
cat config | mat -l yaml
mat --color always README.md > rendered.ansi
```

For Markdown, both line ranges and grep operate on rendered display lines, not
the original Markdown source lines.

## Input and limitations

Input is fully loaded in memory. UTF-8 (with or without BOM), BOM-marked
UTF-16LE/BE, and Windows-1252 fallback are supported. Binary detection can be
overridden with `--force-binary`.

Follow mode reloads the complete file through the same decoding, rendering,
range, highlighting, grep, and search pipeline after coalesced filesystem
events. This favors correctness across truncation and file rotation, but is not
optimized for very large or rapidly changing logs. Lazy/streaming large-file
support is intentionally deferred.

## Main options

```text
-n, --line-numbers
-N, --no-highlight
-m, --markdown
-M, --no-markdown
-f, --follow
-s, --search <PAT>
-g, --grep <PAT>
-i, --ignore-case
-F, --fixed-strings
-w, --word-regexp
-x, --line-regexp
-A, --after <N>
-B, --before <N>
-C, --context <N>
    --wrap <none|wrap|truncate>
-W, --max-width <N>
-l, --language <LANG>
-t, --theme <light|dark>
-L, --lines <RANGE>
-P, --no-pager
    --color <auto|always|never>
    --ansi
    --force-binary
```

Run `mat --help` for the authoritative command-line reference.

## Pager keys

`j`/`k` or arrows scroll, `d`/`u` move half a page, `g`/`G` go to the
top/bottom, `/` and `?` search, `n`/`N` navigate matches, `#` toggles line
numbers, `R` reloads, `f` toggles follow, and `q` quits.

## Dependency security

Syntect is built with the pure-Rust fancy-regex backend and only the features
needed to load precompiled runtime syntax/theme assets. This avoids native
Oniguruma and the unused plist/XML dependency path. Custom syntax compilation
temporarily retains build-time `bincode` and `yaml-rust`; these unmaintained
build-only dependencies are tracked exceptions until Syntect provides a
replacement asset pipeline.

## License

MIT
