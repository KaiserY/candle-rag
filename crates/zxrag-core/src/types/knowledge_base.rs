use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Serialize, Deserialize)]
pub struct EmbeddingResponse<'a> {
  pub object: Cow<'a, str>,
  pub embeddings: Vec<Embedding<'a>>,
  pub model: Cow<'a, str>,
  pub usage: EmbeddingsUsage,
}

#[derive(Serialize, Deserialize)]
pub struct Embedding<'a> {
  pub id: Cow<'a, str>,
  pub kb_id: i64,
  pub file_id: i64,
  pub filename: Cow<'a, str>,
  pub object: Cow<'a, str>,
  pub text: Cow<'a, str>,
  pub embedding: Vec<f32>,
  pub index: usize,
}

#[derive(Serialize, Deserialize)]
pub struct EmbeddingsUsage {
  pub prompt_tokens: usize,
  pub total_tokens: usize,
}
