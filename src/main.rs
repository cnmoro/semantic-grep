use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use semanticgrep::cli::Cli;
use semanticgrep::embeddings::EmbeddingModel;
use semanticgrep::matcher::SearchMode;
use semanticgrep::search::{search_paths, SearchConfig};

fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    if cli.help {
        clap::Command::new("semanticgrep")
            .about("A semantic-aware grep tool")
            .long_about("semanticgrep (sg) is a line-oriented search tool that recursively \
searches your current directory for lines matching a pattern. \
It supports both traditional regex search (like ripgrep) and \
semantic similarity search using Model2Vec embeddings.")
            .arg(clap::Arg::new("pattern").help("The search pattern").required(true))
            .arg(clap::Arg::new("paths").help("Path(s) to search").default_value("."))
            .print_help()?;
        println!();
        return Ok(());
    }

    if cli.version {
        println!("semanticgrep {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if cli.type_list {
        semanticgrep::search::display_type_list();
        return Ok(());
    }

    let paths: Vec<PathBuf> = cli.paths.iter().map(PathBuf::from).collect();

    let mode = if let Some(threshold) = cli.semantic_threshold {
        if !(0.0..=1.0).contains(&threshold) {
            eprintln!("error: semantic threshold must be between 0.0 and 1.0");
            std::process::exit(2);
        }
        SearchMode::Semantic {
            query: cli.pattern.clone(),
            threshold,
        }
    } else {
        SearchMode::Regex {
            pattern: cli.pattern.clone(),
            case_insensitive: cli.ignore_case,
            word_regexp: cli.word_regexp,
            fixed_strings: cli.fixed_strings,
        }
    };

    let model = if matches!(mode, SearchMode::Semantic { .. }) {
        eprintln!("Loading model: {} ...", cli.model);
        let m = EmbeddingModel::new(&cli.model)?;
        eprintln!("Model loaded.");
        Some(m)
    } else {
        None
    };

    let context = cli.context;
    let after_context = if context > 0 { 0 } else { cli.after_context };
    let before_context = if context > 0 { 0 } else { cli.before_context };

    let config = SearchConfig {
        color_when: cli.should_use_color(),
        show_filename: cli.show_filenames(),
        show_line_number: cli.show_line_numbers(),
        count_mode: cli.count,
        files_with_matches_mode: cli.files_with_matches,
        invert_match: cli.invert_match,
        after_context,
        before_context,
        context,
        globs: cli.glob.clone(),
        file_types: cli.file_type.clone(),
        type_not: cli.type_not.clone(),
        max_count: cli.max_count,
    };

    let exit_code = search_paths(&paths, &mode, &config, model.as_ref())?;
    std::process::exit(exit_code);
}
