use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use ignore::WalkBuilder;

use crate::embeddings::EmbeddingModel;
use crate::matcher::{
    match_regex_lines, match_semantic_lines, RegexMatcher, SearchMatch, SearchMode,
};
use crate::printer::{ColorWhen, Printer};

pub struct SearchConfig {
    pub color_when: ColorWhen,
    pub show_filename: bool,
    pub show_line_number: bool,
    pub count_mode: bool,
    pub files_with_matches_mode: bool,
    pub invert_match: bool,
    pub after_context: usize,
    pub before_context: usize,
    pub context: usize,
    pub globs: Vec<String>,
    pub file_types: Vec<String>,
    pub type_not: Vec<String>,
    pub max_count: Option<usize>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            color_when: ColorWhen::Auto,
            show_filename: false,
            show_line_number: true,
            count_mode: false,
            files_with_matches_mode: false,
            invert_match: false,
            after_context: 0,
            before_context: 0,
            context: 0,
            globs: Vec::new(),
            file_types: Vec::new(),
            type_not: Vec::new(),
            max_count: None,
        }
    }
}

pub fn search_paths(
    paths: &[PathBuf],
    mode: &SearchMode,
    config: &SearchConfig,
    model: Option<&EmbeddingModel>,
) -> Result<i32> {
    let mut printer = Printer::new(
        config.color_when,
        config.show_filename,
        config.show_line_number,
        config.count_mode,
        config.files_with_matches_mode,
    );

    let context_before = if config.context > 0 {
        config.context
    } else {
        config.before_context
    };
    let context_after = if config.context > 0 {
        config.context
    } else {
        config.after_context
    };

    let query_embedding = if let SearchMode::Semantic { query, .. } = mode {
        let model = model.expect("model required for semantic search");
        let embs = model.encode(&[query.clone()])?;
        Some(Arc::new(embs.into_iter().next().unwrap()))
    } else {
        None
    };

    let show_filename = config.show_filename;

    let mut total_file_groups = 0i32;
    let mut any_matches = false;

    for path in paths {
        if path.is_file() {
            let results = match search_file(path, mode, config, model, &query_embedding) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("warning: skipping {}: {}", path.display(), e);
                    continue;
                }
            };
            let results = apply_max_count(results, mode, config.max_count);
            any_matches |= print_results(
                &mut printer,
                &results,
                path,
                show_filename,
                &mut total_file_groups,
                config,
                context_before,
                context_after,
            )?;
        } else if path.is_dir() {
            let mut walk_builder = WalkBuilder::new(path);
            walk_builder.git_ignore(true).hidden(false);

            if let Ok(types_builder) = build_types(&config.file_types, &config.type_not) {
                if let Ok(types) = types_builder.build() {
                    walk_builder.types(types);
                }
            }

            let walker = walk_builder.build();

            let mut total_candidates = 0usize;
            let mut matched_globs = 0usize;

            for entry in walker.flatten() {
                let file_path = entry.path();
                if !file_path.is_file() {
                    continue;
                }
                total_candidates += 1;

                if !config.globs.is_empty() {
                    let path_str = file_path.to_string_lossy();
                    let matched = config.globs.iter().any(|g| {
                        if let Ok(glob) = globset::Glob::new(g) {
                            glob.compile_matcher().is_match(path_str.as_ref())
                        } else {
                            false
                        }
                    });
                    if !matched {
                        continue;
                    }
                    matched_globs += 1;
                }

                let results = match search_file(file_path, mode, config, model, &query_embedding) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("warning: skipping {}: {}", file_path.display(), e);
                        continue;
                    }
                };
                let results = apply_max_count(results, mode, config.max_count);
                any_matches |= print_results(
                    &mut printer,
                    &results,
                    file_path,
                    show_filename,
                    &mut total_file_groups,
                    config,
                    context_before,
                    context_after,
                )?;
            }

            if !config.globs.is_empty() && total_candidates > 0 && matched_globs == 0 {
                eprintln!(
                    "warning: glob pattern(s) matched 0 of {} candidate files — \
                     check your -g argument is a file-name glob, not a search pattern",
                    total_candidates
                );
            }
        }
    }

    Ok(if any_matches { 0 } else { 1 })
}

fn build_types(
    include: &[String],
    exclude: &[String],
) -> Result<ignore::types::TypesBuilder, ignore::Error> {
    let mut builder = ignore::types::TypesBuilder::new();
    builder.add_defaults();
    for t in include {
        builder.select(t);
    }
    for t in exclude {
        builder.negate(t);
    }
    Ok(builder)
}

fn search_file(
    path: &Path,
    mode: &SearchMode,
    config: &SearchConfig,
    model: Option<&EmbeddingModel>,
    query_embedding: &Option<Arc<Vec<f32>>>,
) -> Result<Vec<SearchMatch>> {
    match mode {
        SearchMode::Regex {
            pattern,
            case_insensitive,
            word_regexp,
            fixed_strings,
        } => {
            let mut actual_pattern = pattern.clone();
            if *fixed_strings {
                actual_pattern = regex::escape(&actual_pattern);
            }
            let matcher = RegexMatcher::new(&actual_pattern, *case_insensitive, *word_regexp)?;
            let lines = read_lines(path)?;
            Ok(match_regex_lines(&lines, &matcher, config.invert_match))
        }
        SearchMode::Semantic { query: _, threshold } => {
            let model = model.expect("model required for semantic search");
            let query_emb = query_embedding
                .as_ref()
                .expect("query embedding required")
                .as_ref();
            let (lines, orig_indices) = read_nonempty_lines(path)?;
            if lines.is_empty() {
                return Ok(Vec::new());
            }
            let line_embs = model.encode(&lines)?;
            let results = match_semantic_lines(
                &lines,
                &line_embs,
                query_emb,
                *threshold,
                false,
            );
            if config.invert_match {
                let matched_original: std::collections::HashSet<usize> = results
                    .iter()
                    .map(|m| orig_indices[m.line_number - 1])
                    .collect();
                let all_lines = read_lines(path)?;
                Ok(all_lines
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !matched_original.contains(i))
                    .map(|(i, line)| SearchMatch {
                        line_number: i + 1,
                        line: line.clone(),
                        column: 1,
                        similarity: None,
                    })
                    .collect())
            } else {
                Ok(results
                    .into_iter()
                    .map(|m| SearchMatch {
                        line_number: orig_indices[m.line_number - 1] + 1,
                        line: m.line,
                        column: m.column,
                        similarity: m.similarity,
                    })
                    .collect())
            }
        }
    }
}

fn apply_max_count(
    mut results: Vec<SearchMatch>,
    mode: &SearchMode,
    max_count: Option<usize>,
) -> Vec<SearchMatch> {
    match (mode, max_count) {
        (SearchMode::Semantic { .. }, Some(k)) => {
            results.sort_by(|a, b| {
                b.similarity
                    .partial_cmp(&a.similarity)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(k);
            results.sort_by_key(|m| m.line_number);
            results
        }
        (_, Some(k)) => {
            results.truncate(k);
            results
        }
        _ => results,
    }
}

fn print_results(
    printer: &mut Printer,
    results: &[SearchMatch],
    path: &Path,
    show_filename: bool,
    total_file_groups: &mut i32,
    config: &SearchConfig,
    context_before: usize,
    context_after: usize,
) -> Result<bool, std::io::Error> {
    if results.is_empty() {
        return Ok(false);
    }

    if config.files_with_matches_mode {
        printer.print_filename_only(&path.display().to_string())?;
        return Ok(true);
    }

    if config.count_mode {
        printer.print_count(results.len(), &path.display().to_string())?;
        return Ok(true);
    }

    if show_filename && *total_file_groups > 0 {
        printer.print_separator()?;
    }

    if show_filename {
        printer.set_filename(path.display().to_string());
    }

    if context_before > 0 || context_after > 0 {
        if let Some(lines) = read_lines_for_context(path, results, context_before, context_after) {
            for m in &lines {
                printer.print_match(m)?;
            }
            *total_file_groups += 1;
            return Ok(true);
        }
    }

    for m in results {
        printer.print_match(m)?;
    }
    *total_file_groups += 1;
    Ok(true)
}

fn read_lines(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read(path)?;
    let text = String::from_utf8(content).map_err(|_| {
        anyhow::anyhow!("file is not valid UTF-8")
    })?;
    Ok(text.lines().map(|s| s.to_string()).collect())
}

fn read_nonempty_lines(path: &Path) -> Result<(Vec<String>, Vec<usize>)> {
    let lines = read_lines(path)?;
    let mut kept = Vec::new();
    let mut indices = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !line.trim().is_empty() {
            kept.push(line.clone());
            indices.push(i);
        }
    }
    Ok((kept, indices))
}

fn read_lines_for_context(
    path: &Path,
    results: &[SearchMatch],
    before: usize,
    after: usize,
) -> Option<Vec<SearchMatch>> {
    let content = std::fs::read_to_string(path).ok()?;
    let all_lines: Vec<&str> = content.lines().collect();
    if all_lines.is_empty() {
        return None;
    }

    let match_lines: std::collections::BTreeSet<usize> =
        results.iter().map(|m| m.line_number).collect();

    let mut context_set: std::collections::BTreeSet<usize> = std::collections::BTreeSet::new();
    for &ln in &match_lines {
        let start = if ln > before { ln - before } else { 1 };
        let end = std::cmp::min(ln + after, all_lines.len());
        for l in start..=end {
            context_set.insert(l);
        }
    }

    let mut output: Vec<SearchMatch> = Vec::new();
    let mut prev: Option<usize> = None;
    for &ln in &context_set {
        if let Some(p) = prev {
            if ln > p + 1 {
                output.push(SearchMatch {
                    line_number: 0,
                    line: "--".to_string(),
                    column: 0,
                    similarity: None,
                });
            }
        }
        let is_match = match_lines.contains(&ln);
        output.push(SearchMatch {
            line_number: ln,
            line: all_lines[ln - 1].to_string(),
            column: 0,
            similarity: if is_match {
                results.iter().find(|m| m.line_number == ln).and_then(|m| m.similarity)
            } else {
                None
            },
        });
        prev = Some(ln);
    }
    Some(output)
}

pub fn display_type_list() {
    let mut builder = ignore::types::TypesBuilder::new();
    builder.add_defaults();
    if let Ok(types) = builder.build() {
        println!("Available file types:");
        for def in types.definitions() {
            println!("  {}: {}", def.name(), def.globs().join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_read_lines_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "line1\nline2\nline3\n").unwrap();
        let lines = read_lines(&file).unwrap();
        assert_eq!(lines, vec!["line1", "line2", "line3"]);
    }

    #[test]
    fn test_read_lines_empty() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        fs::write(&file, "").unwrap();
        let lines = read_lines(&file).unwrap();
        assert!(lines.is_empty());
    }

    #[test]
    fn test_read_lines_no_file() {
        let result = read_lines(Path::new("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_read_lines_for_context_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "a\nb\nc\nd\ne\n").unwrap();

        let results = vec![SearchMatch {
            line_number: 3,
            line: "c".into(),
            column: 1,
            similarity: None,
        }];

        let context = read_lines_for_context(&file, &results, 1, 1).unwrap();
        assert_eq!(context.len(), 3);
        // Lines 2, 3, 4 (b, c, d) without separator
        assert_eq!(context[0].line_number, 2);
        assert_eq!(context[0].line, "b");
        assert_eq!(context[1].line_number, 3);
        assert_eq!(context[1].line, "c");
        assert_eq!(context[2].line_number, 4);
        assert_eq!(context[2].line, "d");
    }

    #[test]
    fn test_read_lines_for_context_with_separator() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "a\nb\nc\nd\ne\nf\ng\n").unwrap();

        let results = vec![
            SearchMatch { line_number: 2, line: "b".into(), column: 1, similarity: None },
            SearchMatch { line_number: 6, line: "f".into(), column: 1, similarity: None },
        ];

        let context = read_lines_for_context(&file, &results, 1, 1).unwrap();
        // Lines 1-3 and 5-7, with a separator between
        assert!(context.len() > 3);
        let sep_count = context.iter().filter(|m| m.line_number == 0).count();
        assert_eq!(sep_count, 1);
    }

    #[test]
    fn test_read_lines_for_context_no_file() {
        let results = vec![SearchMatch {
            line_number: 1,
            line: "x".into(),
            column: 1,
            similarity: None,
        }];
        let context = read_lines_for_context(Path::new("/nonexistent"), &results, 1, 1);
        assert!(context.is_none());
    }

    #[test]
    fn test_search_file_regex_basic() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world\ngoodbye\nhello again\n").unwrap();

        let mode = SearchMode::Regex {
            pattern: "hello".into(),
            case_insensitive: false,
            word_regexp: false,
            fixed_strings: false,
        };
        let config = SearchConfig::default();

        let results = search_file(&file, &mode, &config, None, &None).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line, "hello world");
        assert_eq!(results[1].line, "hello again");
    }

    #[test]
    fn test_search_file_regex_invert() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello world\ngoodbye\nhello again\n").unwrap();

        let mode = SearchMode::Regex {
            pattern: "hello".into(),
            case_insensitive: false,
            word_regexp: false,
            fixed_strings: false,
        };
        let config = SearchConfig {
            invert_match: true,
            ..Default::default()
        };

        let results = search_file(&file, &mode, &config, None, &None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, "goodbye");
    }

    #[test]
    fn test_search_file_regex_no_matches() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "foo\nbar\nbaz\n").unwrap();

        let mode = SearchMode::Regex {
            pattern: "hello".into(),
            case_insensitive: false,
            word_regexp: false,
            fixed_strings: false,
        };
        let config = SearchConfig::default();

        let results = search_file(&file, &mode, &config, None, &None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_file_regex_fixed_strings() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "foo.bar\nfoobar\n").unwrap();

        let mode = SearchMode::Regex {
            pattern: "foo.bar".into(),
            case_insensitive: false,
            word_regexp: false,
            fixed_strings: true,
        };
        let config = SearchConfig::default();

        let results = search_file(&file, &mode, &config, None, &None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, "foo.bar");
    }

    #[test]
    fn test_search_file_regex_case_insensitive() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "Hello\nHELLO\nworld\n").unwrap();

        let mode = SearchMode::Regex {
            pattern: "hello".into(),
            case_insensitive: true,
            word_regexp: false,
            fixed_strings: false,
        };
        let config = SearchConfig::default();

        let results = search_file(&file, &mode, &config, None, &None).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_print_results_files_with_matches() {
        let mut printer = Printer::new(ColorWhen::Never, false, true, false, true);
        let path = Path::new("test.txt");
        let results = vec![SearchMatch {
            line_number: 1,
            line: "hello".into(),
            column: 1,
            similarity: None,
        }];
        let ok = print_results(
            &mut printer,
            &results,
            path,
            false,
            &mut 0,
            &SearchConfig::default(),
            0,
            0,
        )
        .unwrap();
        assert!(ok);
    }

    #[test]
    fn test_print_results_count_mode() {
        let mut printer = Printer::new(ColorWhen::Never, true, true, true, false);
        let path = Path::new("test.txt");
        let results = vec![
            SearchMatch {
                line_number: 1,
                line: "a".into(),
                column: 1,
                similarity: None,
            },
            SearchMatch {
                line_number: 2,
                line: "b".into(),
                column: 1,
                similarity: None,
            },
        ];
        let ok = print_results(
            &mut printer,
            &results,
            path,
            true,
            &mut 0,
            &SearchConfig::default(),
            0,
            0,
        )
        .unwrap();
        assert!(ok);
    }

    #[test]
    fn test_build_types() {
        let result = build_types(&[], &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_types_with_select() {
        let result = build_types(&["rust".into()], &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_types_with_negate() {
        let result = build_types(&[], &["markdown".into()]);
        assert!(result.is_ok());
    }
}
