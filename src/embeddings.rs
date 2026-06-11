use anyhow::Result;
use model2vec_rs::model::StaticModel;

pub struct EmbeddingModel {
    model: StaticModel,
}

impl EmbeddingModel {
    pub fn new(model_id: &str) -> Result<Self> {
        let model = StaticModel::from_pretrained(model_id, None, Some(true), None)?;
        Ok(Self { model })
    }

    pub fn encode(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(self.model.encode(texts))
    }
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    (dot / (norm_a * norm_b)) as f64
}

pub fn find_similar_lines(
    lines: &[String],
    line_embeddings: &[Vec<f32>],
    query_embedding: &[f32],
    threshold: f64,
) -> Vec<(usize, f64)> {
    lines
        .iter()
        .enumerate()
        .zip(line_embeddings.iter())
        .filter_map(|((i, _line), emb)| {
            let sim = cosine_similarity(emb, query_embedding);
            if sim >= threshold {
                Some((i, sim))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0];
        let b = vec![-1.0, -2.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_partial() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        let expected = 1.0 / 2.0_f64.sqrt();
        assert!((sim - expected).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.is_nan());
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.is_nan() || sim.is_infinite());
    }

    #[test]
    fn test_find_similar_lines_all_above_threshold() {
        let lines = vec!["hello".into(), "world".into(), "foo".into()];
        let query = vec![1.0, 0.0];
        let line_embs = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![0.8, 0.2],
        ];
        let results = find_similar_lines(&lines, &line_embs, &query, 0.7);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_find_similar_lines_some_below_threshold() {
        let lines = vec!["hello".into(), "world".into(), "foo".into()];
        let query = vec![1.0, 0.0];
        let line_embs = vec![
            vec![1.0, 0.0],
            vec![0.6, 0.8],
            vec![0.9, 0.1],
        ];
        let results = find_similar_lines(&lines, &line_embs, &query, 0.8);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0);
        assert_eq!(results[1].0, 2);
    }

    #[test]
    fn test_find_similar_lines_empty_input() {
        let lines: Vec<String> = vec![];
        let line_embs: Vec<Vec<f32>> = vec![];
        let query = vec![1.0, 0.0];
        let results = find_similar_lines(&lines, &line_embs, &query, 0.5);
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_similar_lines_threshold_one() {
        let lines = vec!["hello".into(), "world".into()];
        let query = vec![1.0, 0.0];
        let line_embs = vec![
            vec![1.0, 0.0],
            vec![0.5, 0.5],
        ];
        let results = find_similar_lines(&lines, &line_embs, &query, 1.0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, 0);
    }

    #[test]
    fn test_find_similar_lines_returns_sorted() {
        let lines = vec!["a".into(), "b".into(), "c".into()];
        let query = vec![1.0, 0.0];
        let line_embs = vec![
            vec![0.3, 0.3],
            vec![0.9, 0.1],
            vec![0.95, 0.05],
        ];
        let results = find_similar_lines(&lines, &line_embs, &query, 0.5);
        assert_eq!(results.len(), 3);
        for i in 1..results.len() {
            assert!(results[i - 1].0 < results[i].0);
        }
    }
}
