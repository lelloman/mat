use std::process::Command;
use tempfile::NamedTempFile;
use std::io::Write;

/// Get the path to the mat binary
fn mat_binary() -> std::path::PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // Remove test binary name
    path.pop(); // Remove deps
    path.push("mat");
    path
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
    use std::process::Stdio;
    use std::io::Write;

    let mut child = Command::new(mat_binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("TERM", "dumb")
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
    assert!(!lines.iter().any(|l| l.contains("testing") && !l.contains("a test")));
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
