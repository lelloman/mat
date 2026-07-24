use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

/// Get the path to the mat binary
fn mat_binary() -> std::path::PathBuf {
    assert_cmd::cargo::cargo_bin!("mat").to_path_buf()
}

/// Run mat with given args and return (stdout, stderr, exit_code)
fn run_mat(args: &[&str]) -> (String, String, i32) {
    use std::process::Stdio;

    let output = Command::new(mat_binary())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TERM", "dumb")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute mat");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

/// Run mat with stdin input
fn run_mat_with_stdin(args: &[&str], stdin: &str) -> (String, String, i32) {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new(mat_binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TERM", "dumb")
        .env_remove("NO_COLOR")
        .spawn()
        .expect("Failed to execute mat");

    if let Some(mut stdin_handle) = child.stdin.take() {
        stdin_handle.write_all(stdin.as_bytes()).unwrap();
    }

    let output = child.wait_with_output().expect("Failed to wait on mat");

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}

// ============ CLI Tests ============

#[test]
fn test_help_flag() {
    let (stdout, _, code) = run_mat(&["--help"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Usage:"));
    assert!(stdout.contains("--line-numbers"));
    assert!(stdout.contains("--grep"));
    assert!(stdout.contains("--search"));
}

#[test]
fn test_version_flag() {
    let (stdout, _, code) = run_mat(&["--version"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("mat"));
}

#[test]
fn test_file_not_found() {
    let (_, stderr, code) = run_mat(&["-P", "nonexistent_file_12345.txt"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("nonexistent") || stderr.contains("No such file"));
}

#[test]
fn test_missing_input_is_usage_error() {
    let (_, stderr, code) = run_mat(&[]);
    assert_eq!(code, 2);
    assert!(stderr.contains("No input file"));
}

// ============ Basic File Reading Tests ============

#[test]
fn test_read_simple_file() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, World!").unwrap();
    writeln!(temp, "This is a test.").unwrap();

    let (stdout, _, code) = run_mat(&["-P", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Hello, World!"));
    assert!(stdout.contains("This is a test."));
}

#[test]
fn test_read_with_line_numbers() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Line one").unwrap();
    writeln!(temp, "Line two").unwrap();
    writeln!(temp, "Line three").unwrap();

    let (stdout, _, code) = run_mat(&["-P", "-n", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("1"));
    assert!(stdout.contains("2"));
    assert!(stdout.contains("3"));
}

#[test]
fn test_stdin_input() {
    let (stdout, _, code) = run_mat_with_stdin(&["-P"], "Hello from stdin\nLine 2\n");
    assert_eq!(code, 0);
    assert!(stdout.contains("Hello from stdin"));
    assert!(stdout.contains("Line 2"));
}

// ============ Line Range Tests ============

#[test]
fn test_line_range_full() {
    let mut temp = NamedTempFile::new().unwrap();
    for i in 1..=10 {
        writeln!(temp, "Line {}", i).unwrap();
    }

    let (stdout, _, code) = run_mat(&["-P", "-L", "3:5", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Line 3"));
    assert!(stdout.contains("Line 4"));
    assert!(stdout.contains("Line 5"));
    assert!(!stdout.contains("Line 1"));
    assert!(!stdout.contains("Line 6"));
}

#[test]
fn test_line_range_from_start() {
    let mut temp = NamedTempFile::new().unwrap();
    for i in 1..=10 {
        writeln!(temp, "Line {}", i).unwrap();
    }

    let (stdout, _, code) = run_mat(&["-P", "-L", ":3", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Line 1"));
    assert!(stdout.contains("Line 2"));
    assert!(stdout.contains("Line 3"));
    assert!(!stdout.contains("Line 4"));
}

#[test]
fn test_line_range_to_end() {
    let mut temp = NamedTempFile::new().unwrap();
    for i in 1..=5 {
        writeln!(temp, "Line {}", i).unwrap();
    }

    let (stdout, _, code) = run_mat(&["-P", "-L", "4:", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(!stdout.contains("Line 3"));
    assert!(stdout.contains("Line 4"));
    assert!(stdout.contains("Line 5"));
}

#[test]
fn test_line_range_invalid() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "test").unwrap();

    let (_, stderr, code) = run_mat(&["-P", "-L", "abc", temp.path().to_str().unwrap()]);
    assert_eq!(code, 2); // Invalid args exit code
    assert!(stderr.contains("Invalid") || stderr.contains("invalid"));
}

// ============ Grep Tests ============

#[test]
fn test_grep_basic() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "apple").unwrap();
    writeln!(temp, "banana").unwrap();
    writeln!(temp, "apricot").unwrap();
    writeln!(temp, "cherry").unwrap();

    let (stdout, _, code) = run_mat(&["-P", "-g", "^a", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("apple"));
    assert!(stdout.contains("apricot"));
    assert!(!stdout.contains("banana"));
    assert!(!stdout.contains("cherry"));
}

#[test]
fn test_grep_case_insensitive() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello").unwrap();
    writeln!(temp, "HELLO").unwrap();
    writeln!(temp, "hello").unwrap();
    writeln!(temp, "world").unwrap();

    let (stdout, _, code) = run_mat(&["-P", "-g", "hello", "-i", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Hello"));
    assert!(stdout.contains("HELLO"));
    assert!(stdout.contains("hello"));
    assert!(!stdout.contains("world"));
}

#[test]
fn test_grep_fixed_strings() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "test[0]").unwrap();
    writeln!(temp, "test1").unwrap();

    let (stdout, _, code) = run_mat(&["-P", "-g", "[0]", "-F", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("test[0]"));
    assert!(!stdout.contains("test1"));
}

#[test]
fn test_grep_word_boundary() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "test").unwrap();
    writeln!(temp, "testing").unwrap();
    writeln!(temp, "a test here").unwrap();

    let (stdout, _, code) = run_mat(&["-P", "-g", "test", "-w", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("test"));
    assert!(stdout.contains("a test here"));
    // "testing" should not match with -w
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(!lines
        .iter()
        .any(|l| l.contains("testing") && !l.contains("a test")));
}

#[test]
fn test_grep_invalid_regex() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "test").unwrap();

    let (_, stderr, code) = run_mat(&["-P", "-g", "[invalid", temp.path().to_str().unwrap()]);
    assert_eq!(code, 2); // Invalid args
    assert!(stderr.contains("regex") || stderr.contains("pattern") || stderr.contains("Invalid"));
}

// ============ Binary Detection Tests ============

#[test]
fn test_binary_file_detection() {
    let mut temp = NamedTempFile::new().unwrap();
    // Write some binary content with null bytes
    temp.write_all(b"Hello\x00World\x00Binary").unwrap();

    let (_, stderr, code) = run_mat(&["-P", temp.path().to_str().unwrap()]);
    assert_eq!(code, 1);
    assert!(stderr.contains("Binary") || stderr.contains("binary"));
}

#[test]
fn test_force_binary() {
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(b"Hello\x00World").unwrap();

    let (stdout, _, code) = run_mat(&["-P", "--force-binary", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Hello"));
}

// ============ Empty File Tests ============

#[test]
fn test_empty_file() {
    let temp = NamedTempFile::new().unwrap();

    let (stdout, _, code) = run_mat(&["-P", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.is_empty() || stdout.trim().is_empty());
}

// ============ Markdown Tests ============

#[test]
fn test_markdown_disabled() {
    let mut temp = NamedTempFile::with_suffix(".md").unwrap();
    writeln!(temp, "# Heading").unwrap();
    writeln!(temp, "Normal text").unwrap();

    let (stdout, _, code) = run_mat(&["-P", "-M", "-N", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    // With -M (no markdown), we should see the raw # character
    assert!(stdout.contains("# Heading"));
}

// ============ Encoding Tests ============

#[test]
fn test_utf8_content() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "Hello, 世界!").unwrap();
    writeln!(temp, "Émojis: 🎉🚀").unwrap();

    let (stdout, _, code) = run_mat(&["-P", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("世界"));
    assert!(stdout.contains("🎉"));
}

// ============ Tab Expansion Tests ============

#[test]
fn test_tab_expansion() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "a\tb").unwrap();

    let (stdout, _, code) = run_mat(&["-P", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    // Tabs should be expanded to spaces
    assert!(stdout.contains("a") && stdout.contains("b"));
    assert!(!stdout.contains('\t'));
}

#[test]
fn test_cli_rejects_conflicting_and_meaningless_options() {
    for args in [
        vec!["--markdown", "--no-markdown", "-"],
        vec!["--grep", "x", "--context", "1", "--before", "1", "-"],
        vec!["--context", "1", "-"],
        vec!["--no-highlight", "--language", "rust", "-"],
        vec!["--ignore-case", "-"],
        vec!["--max-width", "0", "-"],
        vec!["--theme", "sepia", "-"],
    ] {
        let (_, _, code) = run_mat_with_stdin(&args, "text\n");
        assert_eq!(code, 2, "arguments were unexpectedly accepted: {args:?}");
    }
}

#[test]
fn test_out_of_bounds_range_is_invalid_but_end_is_clamped() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "one\ntwo\nthree").unwrap();
    let path = temp.path().to_str().unwrap();

    let (_, _, code) = run_mat(&["--lines", "4:99", path]);
    assert_eq!(code, 2);

    let (stdout, _, code) = run_mat(&["--lines", "2:99", path]);
    assert_eq!(code, 0);
    assert_eq!(stdout, "two\nthree\n");
}

#[test]
fn test_color_modes_for_redirected_output() {
    let mut temp = NamedTempFile::with_suffix(".md").unwrap();
    writeln!(temp, "# Heading").unwrap();
    let path = temp.path().to_str().unwrap();

    let (auto, _, _) = run_mat(&["--color", "auto", path]);
    let (always, _, _) = run_mat(&["--color", "always", path]);
    let (never, _, _) = run_mat(&["--color", "never", path]);
    assert!(!auto.contains("\x1b["));
    assert!(always.contains("\x1b["));
    assert!(!never.contains("\x1b["));
}

#[test]
fn test_bom_encodings_are_text_and_windows_1252_falls_back() {
    for bytes in [
        vec![0xff, 0xfe, b'H', 0, b'i', 0, b'\n', 0],
        vec![0xfe, 0xff, 0, b'H', 0, b'i', 0, b'\n'],
        vec![0xef, 0xbb, 0xbf, b'H', b'i', b'\n'],
    ] {
        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(&bytes).unwrap();
        let (stdout, stderr, code) = run_mat(&[temp.path().to_str().unwrap()]);
        assert_eq!(code, 0, "{stderr}");
        assert!(stdout.contains("Hi"));
        assert!(!stdout.contains('\u{feff}'));
    }

    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(&[0x93, b'h', b'i', 0x94, b'\n']).unwrap();
    let (stdout, _, code) = run_mat(&[temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("“hi”"));
}

#[test]
fn test_ansi_accepts_sgr_but_discards_terminal_controls() {
    let mut temp = NamedTempFile::new().unwrap();
    temp.write_all(b"\x1b[31mred\x1b[0m\x1b[2J\x1b[10;10H\x1b]0;owned\x07safe\n")
        .unwrap();
    let (stdout, _, code) =
        run_mat(&["--ansi", "--color", "always", temp.path().to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(stdout.contains("\x1b[31mred"));
    assert!(stdout.contains("safe"));
    assert!(!stdout.contains("[2J"));
    assert!(!stdout.contains("[10;10H"));
    assert!(!stdout.contains("owned"));
}

#[test]
fn test_follow_is_rejected_for_direct_output() {
    let mut temp = NamedTempFile::new().unwrap();
    writeln!(temp, "line").unwrap();
    let (_, stderr, code) = run_mat(&["--follow", temp.path().to_str().unwrap()]);
    assert_eq!(code, 2);
    assert!(stderr.contains("interactive pager"));
}

#[test]
fn test_broken_stdout_pipe_is_a_normal_exit() {
    use std::process::Stdio;

    let mut temp = NamedTempFile::new().unwrap();
    for _ in 0..50_000 {
        writeln!(temp, "a sufficiently long output line to fill the pipe").unwrap();
    }
    let mut child = Command::new(mat_binary())
        .arg(temp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    assert!(status.success());
}
