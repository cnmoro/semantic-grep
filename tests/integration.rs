use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::prelude::*;
use tempfile::TempDir;

struct TestEnv {
    dir: TempDir,
}

impl TestEnv {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        Self { dir }
    }

    fn create_file(&self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::cargo_bin("semanticgrep").unwrap();
        cmd.current_dir(self.dir.path());
        cmd
    }

    fn sg(&self, args: &[&str]) -> (std::process::Output, String, String) {
        let mut c = self.cmd();
        c.args(args);
        let output = c.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        (output, stdout, stderr)
    }
}

#[test]
fn test_basic_regex_search() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello world\ngoodbye\nhello again\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "hello", "test.txt"]);
    assert!(output.status.success());
    assert!(!stdout.is_empty());
}

#[test]
fn test_basic_regex_search_no_match() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello world\ngoodbye\n");
    let (output, _, _) = env.sg(&["--color", "never", "zzz", "test.txt"]);
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_case_insensitive_search() {
    let env = TestEnv::new();
    env.create_file("test.txt", "Hello World\ngoodbye\nHELLO\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-i", "hello", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.lines().count(), 2);
}

#[test]
fn test_count_matches() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\nworld\nhello\nhello\nfoo\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-c", "hello", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.trim(), "3");
}

#[test]
fn test_count_with_filename() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\nworld\nhello\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-c", "-H", "hello", "test.txt"]);
    assert!(output.status.success());
    assert!(stdout.trim().contains("test.txt:"));
    assert!(stdout.trim().contains("2"));
}

#[test]
fn test_files_with_matches() {
    let env = TestEnv::new();
    env.create_file("a.txt", "hello\nworld\n");
    env.create_file("b.txt", "foo\nbar\n");
    env.create_file("c.txt", "hello again\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-l", "hello"]);
    assert!(output.status.success());
    assert!(stdout.contains("a.txt"));
    assert!(stdout.contains("c.txt"));
    assert!(!stdout.contains("b.txt"));
}

#[test]
fn test_invert_match() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\nworld\nfoo\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-v", "hello", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.lines().count(), 2);
    assert!(stdout.contains("world"));
    assert!(stdout.contains("foo"));
    assert!(!stdout.contains("hello"));
}

#[test]
fn test_fixed_strings_mode() {
    let env = TestEnv::new();
    env.create_file("test.txt", "foo.bar\nfooxbar\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-F", "foo.bar", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.lines().count(), 1);
    assert!(stdout.contains("foo.bar"));
}

#[test]
fn test_word_regexp() {
    let env = TestEnv::new();
    env.create_file("test.txt", "the cat\ntheater\ncat\nthe cat and dog\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-w", "cat", "test.txt"]);
    assert!(output.status.success());
    // \bcat\b matches "the cat", "cat", and "the cat and dog" = 3 lines
    assert_eq!(stdout.lines().count(), 3);
}

#[test]
fn test_line_numbers_default() {
    let env = TestEnv::new();
    env.create_file("test.txt", "first\nsecond\nthird\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "second", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.trim(), "2:second");
}

#[test]
fn test_no_line_number() {
    let env = TestEnv::new();
    env.create_file("test.txt", "first\nsecond\nthird\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-N", "second", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.trim(), "second");
}

#[test]
fn test_search_multiple_files() {
    let env = TestEnv::new();
    env.create_file("a.txt", "hello from a\n");
    env.create_file("b.txt", "hello from b\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "hello", "a.txt", "b.txt"]);
    assert!(output.status.success());
    assert!(stdout.contains("a.txt:"));
    assert!(stdout.contains("b.txt:"));
    // With 2 files, we get "a.txt:hello from a\n--\nb.txt:hello from b\n" = 3 lines
    assert_eq!(stdout.lines().count(), 3);
}

#[test]
fn test_recursive_search() {
    let env = TestEnv::new();
    env.create_file("src/main.rs", "fn main() {}\n");
    env.create_file("src/lib.rs", "pub fn hello() {}\n");
    env.create_file("README.md", "Hello world\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "hello"]);
    assert!(output.status.success());
    assert!(stdout.contains("hello"));
}

#[test]
fn test_with_filename_flag() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-H", "hello", "test.txt"]);
    assert!(output.status.success());
    assert!(stdout.contains("test.txt:"));
}

#[test]
fn test_no_filename_flag() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-h", "hello", "test.txt"]);
    assert!(output.status.success());
    assert!(!stdout.contains("test.txt:"));
}

#[test]
fn test_color_never_no_ansi() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\n");
    let (_output, stdout, _) = env.sg(&["--color", "never", "hello", "test.txt"]);
    assert!(_output.status.success());
    assert!(!stdout.is_empty());
}

#[test]
fn test_semantic_threshold_zero_finds_all() {
    let env = TestEnv::new();
    env.create_file("test.txt", "apple\nbanana\ncarrot\n");
    let (_output, stdout, _) = env.sg(&["--color", "never", "-s", "0.0", "fruit", "test.txt"]);
    assert!(_output.status.success());
    assert_eq!(stdout.lines().count(), 3);
}

#[test]
fn test_semantic_threshold_one_finds_exact() {
    let env = TestEnv::new();
    env.create_file("test.txt", "exact match\nsomething else\n");
    let (_output, stdout, _) = env.sg(&["--color", "never", "-s", "1.0", "exact match", "test.txt"]);
    assert!(_output.status.success());
    assert!(!stdout.is_empty());
}

#[test]
fn test_semantic_with_custom_model() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello world\n");
    let (_output, _stdout, _) = env.sg(&[
        "--color", "never",
        "-m", "minishlab/potion-base-8M",
        "-s", "0.5",
        "hello",
        "test.txt",
    ]);
    assert!(_output.status.success());
}

#[test]
fn test_semantic_shows_score() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello world\n");
    let (_output, stdout, _) = env.sg(&["--color", "never", "-s", "0.0", "greeting", "test.txt"]);
    assert!(_output.status.success());
    assert!(stdout.contains(":"));
}

#[test]
fn test_long_lines() {
    let env = TestEnv::new();
    let long_line = "x".repeat(10_000) + "hello" + &"x".repeat(10_000);
    env.create_file("test.txt", &format!("{}\n{}\n", long_line, "short line"));
    let (output, stdout, _) = env.sg(&["--color", "never", "hello", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.lines().count(), 1);
}

#[test]
fn test_unicode_search() {
    let env = TestEnv::new();
    env.create_file("test.txt", "café\nhello\n世界\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "hello", "test.txt"]);
    assert!(output.status.success());
    assert!(stdout.contains("hello"));
}

#[test]
fn test_ignore_case_with_special_chars() {
    let env = TestEnv::new();
    env.create_file("test.txt", "HELLO.WORLD\nhello.world\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-i", r"hello\.world", "test.txt"]);
    assert!(output.status.success());
    assert_eq!(stdout.lines().count(), 2);
}

#[test]
fn test_semantic_threshold_too_high() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\n");
    let (output, _, _) = env.sg(&["--color", "never", "-s", "1.5", "hello", "test.txt"]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_semantic_threshold_negative() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\n");
    let (output, _, _) = env.sg(&["--color", "never", "-s", "-0.1", "hello", "test.txt"]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_output_does_not_contain_ansi_default() {
    let env = TestEnv::new();
    env.create_file("test.txt", "hello\n");
    let (_, stdout, _) = env.sg(&["--color", "never", "hello", "test.txt"]);
    assert!(!stdout.contains("\x1b["));
}

#[test]
fn test_version_flag() {
    let env = TestEnv::new();
    let (output, stdout, _) = env.sg(&["--version", "hello"]);
    assert!(output.status.success());
    assert!(stdout.contains("semanticgrep"));
}

#[test]
fn test_single_file_in_subdir() {
    let env = TestEnv::new();
    env.create_file("subdir/test.txt", "hello from subdir\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "hello", "subdir"]);
    assert!(output.status.success());
    assert!(stdout.contains("hello from subdir"));
}

#[test]
fn test_after_context() {
    let env = TestEnv::new();
    env.create_file("test.txt", "a\nb\nc\nd\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-A", "1", "b", "test.txt"]);
    assert!(output.status.success());
    assert!(stdout.contains("c"));
}

#[test]
fn test_before_context() {
    let env = TestEnv::new();
    env.create_file("test.txt", "a\nb\nc\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-B", "1", "c", "test.txt"]);
    assert!(output.status.success());
    assert!(stdout.contains("b"));
}

#[test]
fn test_context_lines() {
    let env = TestEnv::new();
    env.create_file("test.txt", "a\nb\nc\nd\ne\nf\ng\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-C", "1", "d", "test.txt"]);
    assert!(output.status.success());
    assert!(stdout.contains("c"));
    assert!(stdout.contains("d"));
    assert!(stdout.contains("e"));
}

#[test]
fn test_glob_filter() {
    let env = TestEnv::new();
    env.create_file("data.csv", "name,value\nhello,1\n");
    env.create_file("data.txt", "hello world\n");
    let (output, stdout, _) = env.sg(&["--color", "never", "-g", "*.txt", "hello"]);
    assert!(output.status.success());
    assert!(stdout.contains("data.txt"));
}
