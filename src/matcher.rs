use regex::Regex;

pub enum SearchMode {
    Regex { pattern: String, case_insensitive: bool, word_regexp: bool, fixed_strings: bool },
    Semantic { query: String, threshold: f64 },
}

pub struct SearchMatch {
    pub line_number: usize,
    pub line: String,
    pub column: usize,
    pub similarity: Option<f64>,
}

pub struct RegexMatcher {
    regex: Regex,
}

impl RegexMatcher {
    pub fn new(pattern: &str, case_insensitive: bool, word_regexp: bool) -> anyhow::Result<Self> {
        let mut pattern = pattern.to_string();
        if word_regexp {
            pattern = format!(r"\b{}\b", pattern);
        }
        let re = if case_insensitive {
            Regex::new(&format!("(?i){}", pattern))
        } else {
            Regex::new(&pattern)
        }?;
        Ok(Self { regex: re })
    }

    pub fn is_match(&self, line: &str) -> bool {
        self.regex.is_match(line)
    }

    pub fn find_iter<'a>(&'a self, line: &'a str) -> impl Iterator<Item = regex::Match<'a>> {
        self.regex.find_iter(line)
    }
}

pub fn match_regex_lines(
    lines: &[String],
    matcher: &RegexMatcher,
    invert: bool,
) -> Vec<SearchMatch> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(i, line)| {
            let matched = matcher.is_match(line);
            if invert { !matched } else { matched }
                .then(|| {
                    let col = matcher.find_iter(line).next().map(|m| m.start()).unwrap_or(0);
                    SearchMatch {
                        line_number: i + 1,
                        line: line.clone(),
                        column: col + 1,
                        similarity: None,
                    }
                })
        })
        .collect()
}

pub fn match_semantic_lines(
    lines: &[String],
    line_embeddings: &[Vec<f32>],
    query_embedding: &[f32],
    threshold: f64,
    invert: bool,
) -> Vec<SearchMatch> {
    let results = crate::embeddings::find_similar_lines(lines, line_embeddings, query_embedding, threshold);
    if invert {
        let matched_indices: std::collections::HashSet<usize> =
            results.iter().map(|(i, _)| *i).collect();
        lines
            .iter()
            .enumerate()
            .filter(|(i, _)| !matched_indices.contains(i))
            .map(|(i, line)| SearchMatch {
                line_number: i + 1,
                line: line.clone(),
                column: 1,
                similarity: Some(0.0),
            })
            .collect()
    } else {
        results
            .into_iter()
            .map(|(i, sim)| SearchMatch {
                line_number: i + 1,
                line: lines[i].clone(),
                column: 1,
                similarity: Some(sim),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_matcher_basic() {
        let m = RegexMatcher::new("hello", false, false).unwrap();
        assert!(m.is_match("hello world"));
        assert!(!m.is_match("world"));
    }

    #[test]
    fn test_regex_matcher_case_insensitive() {
        let m = RegexMatcher::new("hello", true, false).unwrap();
        assert!(m.is_match("Hello World"));
        assert!(m.is_match("HELLO"));
    }

    #[test]
    fn test_regex_matcher_word_regexp() {
        let m = RegexMatcher::new("hello", false, true).unwrap();
        assert!(m.is_match("hello world"));
        assert!(!m.is_match("helloworld"));
    }

    #[test]
    fn test_regex_matcher_case_insensitive_word() {
        let m = RegexMatcher::new("hello", true, true).unwrap();
        assert!(m.is_match("Hello world"));
        assert!(!m.is_match("Helloworld"));
    }

    #[test]
    fn test_regex_matcher_special_chars() {
        let m = RegexMatcher::new(r"\d+", false, false).unwrap();
        assert!(m.is_match("foo 123 bar"));
        assert!(!m.is_match("abc"));
    }

    #[test]
    fn test_regex_matcher_empty_pattern_matches_everything() {
        let m = RegexMatcher::new("", false, false).unwrap();
        assert!(m.is_match("anything"));
        assert!(m.is_match(""));
    }

    #[test]
    fn test_regex_matcher_invalid_regex() {
        let result = RegexMatcher::new(r"\[", false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_matcher_completely_invalid() {
        let result = RegexMatcher::new(r"***", false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_match_regex_lines_basic() {
        let lines: Vec<String> = vec![
            "hello world".into(),
            "goodbye".into(),
            "hello again".into(),
        ];
        let m = RegexMatcher::new("hello", false, false).unwrap();
        let results = match_regex_lines(&lines, &m, false);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line_number, 1);
        assert_eq!(results[1].line_number, 3);
    }

    #[test]
    fn test_match_regex_lines_invert() {
        let lines: Vec<String> = vec![
            "hello world".into(),
            "goodbye".into(),
            "hello again".into(),
        ];
        let m = RegexMatcher::new("hello", false, false).unwrap();
        let results = match_regex_lines(&lines, &m, true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line_number, 2);
        assert_eq!(results[0].line, "goodbye");
    }

    #[test]
    fn test_match_regex_lines_no_matches() {
        let lines: Vec<String> = vec!["foo".into(), "bar".into()];
        let m = RegexMatcher::new("hello", false, false).unwrap();
        let results = match_regex_lines(&lines, &m, false);
        assert!(results.is_empty());
    }

    #[test]
    fn test_match_regex_lines_empty_lines() {
        let lines: Vec<String> = vec!["".into(), "hello".into()];
        let m = RegexMatcher::new("hello", false, false).unwrap();
        let results = match_regex_lines(&lines, &m, false);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_match_regex_lines_invert_no_matches() {
        let lines: Vec<String> = vec!["hello".into()];
        let m = RegexMatcher::new("hello", false, false).unwrap();
        let results = match_regex_lines(&lines, &m, true);
        assert!(results.is_empty());
    }

    #[test]
    fn test_match_semantic_lines_basic() {
        let lines = vec!["apple".into(), "banana".into(), "carrot".into()];
        let query_emb = vec![1.0, 0.0, 0.0];
        let line_embs = vec![
            vec![0.95, 0.1, 0.0],
            vec![0.3, 0.9, 0.1],
            vec![0.2, 0.2, 0.8],
        ];
        let results = match_semantic_lines(&lines, &line_embs, &query_emb, 0.8, false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, "apple");
        assert!(results[0].similarity.unwrap() >= 0.8);
    }

    #[test]
    fn test_match_semantic_lines_invert() {
        let lines = vec!["apple".into(), "banana".into()];
        let query_emb = vec![1.0, 0.0];
        let line_embs = vec![
            vec![0.95, 0.1],
            vec![0.3, 0.9],
        ];
        let results = match_semantic_lines(&lines, &line_embs, &query_emb, 0.8, true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, "banana");
    }

    #[test]
    fn test_match_semantic_lines_empty() {
        let lines: Vec<String> = vec![];
        let query_emb = vec![1.0, 0.0];
        let line_embs: Vec<Vec<f32>> = vec![];
        let results = match_semantic_lines(&lines, &line_embs, &query_emb, 0.5, false);
        assert!(results.is_empty());
    }

    #[test]
    fn test_match_semantic_lines_all_match() {
        let lines = vec!["a".into(), "b".into()];
        let query_emb = vec![1.0, 0.0];
        let line_embs = vec![
            vec![0.99, 0.01],
            vec![0.98, 0.02],
        ];
        let results = match_semantic_lines(&lines, &line_embs, &query_emb, 0.5, false);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_match_semantic_lines_similarity_values() {
        let lines = vec!["a".into(), "b".into()];
        let query_emb = vec![1.0, 0.0];
        let line_embs = vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
        ];
        let results = match_semantic_lines(&lines, &line_embs, &query_emb, 0.0, false);
        assert_eq!(results.len(), 2);
        assert!((results[0].similarity.unwrap() - 1.0).abs() < 1e-6);
        assert!((results[1].similarity.unwrap() - 0.0).abs() < 1e-6);
    }
}
