use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "semanticgrep",
    version,
    about = "A semantic-aware grep tool",
    long_about = "semanticgrep (sg) is a line-oriented search tool that recursively \
searches your current directory for lines matching a pattern. \
It supports both traditional regex search (like ripgrep) and \
semantic similarity search using Model2Vec embeddings.",
    disable_help_flag = true,
    disable_version_flag = true,
)]
pub struct Cli {
    /// The search pattern (regex or semantic query)
    pub pattern: String,

    /// Path(s) to search (defaults to current directory)
    #[arg(default_value = ".")]
    pub paths: Vec<String>,

    /// Enable semantic similarity search with threshold 0.0-1.0
    #[arg(short = 's', long = "semantic-threshold", value_name = "THRESHOLD")]
    pub semantic_threshold: Option<f64>,

    /// Model ID or local path for semantic search
    #[arg(short = 'm', long = "model", default_value = "minishlab/potion-code-16M")]
    pub model: String,

    /// Treat pattern as a literal string (not regex)
    #[arg(short = 'F', long = "fixed-strings")]
    pub fixed_strings: bool,

    /// Perform case-insensitive search
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Only match whole words
    #[arg(short = 'w', long = "word-regexp")]
    pub word_regexp: bool,

    /// Invert match (select non-matching lines)
    #[arg(short = 'v', long = "invert-match")]
    pub invert_match: bool,

    /// Show line numbers (default for stdout)
    #[arg(short = 'n', long = "line-number", overrides_with = "no_line_number")]
    pub line_number: bool,

    /// Suppress line numbers
    #[arg(short = 'N', long = "no-line-number")]
    pub no_line_number: bool,

    /// Show count of matching lines per file
    #[arg(short = 'c', long = "count")]
    pub count: bool,

    /// Show only filenames with matches
    #[arg(short = 'l', long = "files-with-matches")]
    pub files_with_matches: bool,

    /// Show N lines after each match
    #[arg(short = 'A', long = "after-context", default_value = "0")]
    pub after_context: usize,

    /// Show N lines before each match
    #[arg(short = 'B', long = "before-context", default_value = "0")]
    pub before_context: usize,

    /// Show N lines around each match
    #[arg(short = 'C', long = "context", default_value = "0")]
    pub context: usize,

    /// Include only files matching glob
    #[arg(short = 'g', long = "glob")]
    pub glob: Vec<String>,

    /// Include files matching file type
    #[arg(short = 't', long = "type")]
    pub file_type: Vec<String>,

    /// Exclude files matching file type
    #[arg(short = 'T', long = "type-not")]
    pub type_not: Vec<String>,

    /// Show all available file types
    #[arg(long = "type-list")]
    pub type_list: bool,

    /// Print filenames with results
    #[arg(short = 'H', long = "with-filename")]
    pub with_filename: bool,

    /// Suppress filenames
    #[arg(short = 'h', long = "no-filename")]
    pub no_filename: bool,

    /// Control when color is used (auto, always, never)
    #[arg(long = "color", default_value = "auto")]
    pub color: String,

    /// Number of threads to use (not yet implemented)
    #[arg(short = 'j', long = "threads")]
    pub threads: Option<usize>,

    /// Show only matching parts of a line
    #[arg(short = 'o', long = "only-matching")]
    pub only_matching: bool,

    /// Print help
    #[arg(long = "help")]
    pub help: bool,

    /// Print version
    #[arg(long = "version")]
    pub version: bool,
}

impl Cli {
    pub fn show_line_numbers(&self) -> bool {
        !self.no_line_number && (self.line_number || !self.count)
    }

    pub fn show_filenames(&self) -> bool {
        let default_path = self.paths.first().map_or(true, |p| p == ".");
        if self.no_filename {
            return false;
        }
        self.with_filename || self.paths.len() > 1 || default_path
    }

    pub fn should_use_color(&self) -> crate::printer::ColorWhen {
        match self.color.as_str() {
            "always" => crate::printer::ColorWhen::Always,
            "never" => crate::printer::ColorWhen::Never,
            _ => crate::printer::ColorWhen::Auto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_path() {
        let cli = Cli::try_parse_from(&["semanticgrep", "pattern"]).unwrap();
        assert_eq!(cli.pattern, "pattern");
        assert_eq!(cli.paths, vec!["."]);
        assert!(cli.semantic_threshold.is_none());
    }

    #[test]
    fn test_cli_custom_path() {
        let cli = Cli::try_parse_from(&["semanticgrep", "pattern", "/tmp"]).unwrap();
        assert_eq!(cli.paths, vec!["/tmp"]);
    }

    #[test]
    fn test_cli_semantic_threshold() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-s", "0.8", "query"]).unwrap();
        assert_eq!(cli.semantic_threshold, Some(0.8));
    }

    #[test]
    fn test_cli_model() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-m", "custom/model", "query"]).unwrap();
        assert_eq!(cli.model, "custom/model");
    }

    #[test]
    fn test_cli_ignore_case() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-i", "pattern"]).unwrap();
        assert!(cli.ignore_case);
    }

    #[test]
    fn test_cli_fixed_strings() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-F", "pattern"]).unwrap();
        assert!(cli.fixed_strings);
    }

    #[test]
    fn test_cli_word_regexp() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-w", "pattern"]).unwrap();
        assert!(cli.word_regexp);
    }

    #[test]
    fn test_cli_invert_match() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-v", "pattern"]).unwrap();
        assert!(cli.invert_match);
    }

    #[test]
    fn test_cli_count() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-c", "pattern"]).unwrap();
        assert!(cli.count);
    }

    #[test]
    fn test_cli_files_with_matches() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-l", "pattern"]).unwrap();
        assert!(cli.files_with_matches);
    }

    #[test]
    fn test_cli_context() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-C", "3", "pattern"]).unwrap();
        assert_eq!(cli.context, 3);
    }

    #[test]
    fn test_cli_after_context() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-A", "5", "pattern"]).unwrap();
        assert_eq!(cli.after_context, 5);
    }

    #[test]
    fn test_cli_before_context() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-B", "2", "pattern"]).unwrap();
        assert_eq!(cli.before_context, 2);
    }

    #[test]
    fn test_cli_glob() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-g", "*.rs", "pattern"]).unwrap();
        assert_eq!(cli.glob, vec!["*.rs"]);
    }

    #[test]
    fn test_cli_multiple_glob() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-g", "*.rs", "-g", "*.toml", "pattern"]).unwrap();
        assert_eq!(cli.glob, vec!["*.rs", "*.toml"]);
    }

    #[test]
    fn test_cli_type_filter() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-t", "rust", "pattern"]).unwrap();
        assert_eq!(cli.file_type, vec!["rust"]);
    }

    #[test]
    fn test_cli_type_not() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-T", "markdown", "pattern"]).unwrap();
        assert_eq!(cli.type_not, vec!["markdown"]);
    }

    #[test]
    fn test_cli_color() {
        let cli = Cli::try_parse_from(&["semanticgrep", "--color", "never", "pattern"]).unwrap();
        assert_eq!(cli.color, "never");
    }

    #[test]
    fn test_cli_with_filename() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-H", "pattern"]).unwrap();
        assert!(cli.with_filename);
    }

    #[test]
    fn test_cli_no_filename() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-h", "pattern"]).unwrap();
        assert!(cli.no_filename);
    }

    #[test]
    fn test_cli_threads() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-j", "4", "pattern"]).unwrap();
        assert_eq!(cli.threads, Some(4));
    }

    #[test]
    fn test_cli_only_matching() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-o", "pattern"]).unwrap();
        assert!(cli.only_matching);
    }

    #[test]
    fn test_cli_type_list() {
        let cli = Cli::try_parse_from(&["semanticgrep", "--type-list", "pattern"]).unwrap();
        assert!(cli.type_list);
    }

    #[test]
    fn test_show_line_numbers_default() {
        let cli = Cli::try_parse_from(&["semanticgrep", "pattern"]).unwrap();
        assert!(cli.show_line_numbers());
    }

    #[test]
    fn test_show_line_numbers_suppressed() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-N", "pattern"]).unwrap();
        assert!(!cli.show_line_numbers());
    }

    #[test]
    fn test_show_line_numbers_count() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-c", "pattern"]).unwrap();
        assert!(!cli.show_line_numbers());
    }

    #[test]
    fn test_show_line_numbers_explicit_n() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-n", "pattern"]).unwrap();
        assert!(cli.show_line_numbers());
    }

    #[test]
    fn test_show_filenames_single_path() {
        let cli = Cli::try_parse_from(&["semanticgrep", "pattern", "/tmp"]).unwrap();
        assert!(!cli.show_filenames());
    }

    #[test]
    fn test_show_filenames_multiple_paths() {
        let cli = Cli::try_parse_from(&["semanticgrep", "pattern", "/tmp", "/var"]).unwrap();
        assert!(cli.show_filenames());
    }

    #[test]
    fn test_show_filenames_with_filename_flag() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-H", "pattern"]).unwrap();
        assert!(cli.show_filenames());
    }

    #[test]
    fn test_show_filenames_default_path() {
        let cli = Cli::try_parse_from(&["semanticgrep", "pattern"]).unwrap();
        assert!(cli.show_filenames());
    }

    #[test]
    fn test_show_filenames_no_filename_flag() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-h", "pattern"]).unwrap();
        assert!(!cli.show_filenames());
    }

    #[test]
    fn test_color_auto() {
        let cli = Cli::try_parse_from(&["semanticgrep", "pattern"]).unwrap();
        assert!(matches!(cli.should_use_color(), crate::printer::ColorWhen::Auto));
    }

    #[test]
    fn test_color_always() {
        let cli = Cli::try_parse_from(&["semanticgrep", "--color", "always", "pattern"]).unwrap();
        assert!(matches!(cli.should_use_color(), crate::printer::ColorWhen::Always));
    }

    #[test]
    fn test_color_never() {
        let cli = Cli::try_parse_from(&["semanticgrep", "--color", "never", "pattern"]).unwrap();
        assert!(matches!(cli.should_use_color(), crate::printer::ColorWhen::Never));
    }

    #[test]
    fn test_cli_version() {
        let cli = Cli::try_parse_from(&["semanticgrep", "--version", "pattern"]).unwrap();
        assert!(cli.version);
    }

    #[test]
    fn test_cli_help() {
        let cli = Cli::try_parse_from(&["semanticgrep", "--help", "pattern"]).unwrap();
        assert!(cli.help);
    }

    #[test]
    fn test_cli_multiple_flags() {
        let cli = Cli::try_parse_from(&[
            "semanticgrep",
            "-i", "-w", "-v", "-n",
            "--color", "never",
            "search_pattern",
            "/some/path",
        ])
        .unwrap();
        assert!(cli.ignore_case);
        assert!(cli.word_regexp);
        assert!(cli.invert_match);
        assert!(cli.line_number);
        assert_eq!(cli.color, "never");
    }

    #[test]
    fn test_cli_combined_flags() {
        let cli = Cli::try_parse_from(&["semanticgrep", "-c", "-H", "-s", "0.75", "query", "src/"])
            .unwrap();
        assert!(cli.count);
        assert!(cli.with_filename);
        assert_eq!(cli.semantic_threshold, Some(0.75));
        assert_eq!(cli.paths, vec!["src/"]);
    }
}
