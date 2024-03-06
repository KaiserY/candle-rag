use serde::{Deserialize, Serialize};

use crate::types::model::ModelId;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BackendConf {
  pub log_file_path: String,
  pub log_file_name: String,
  pub bind_addr: String,
  pub model_id: ModelId,
  pub model_path: String,
  pub tokenizer_path: String,
  pub device: String,
}

#[derive(Debug, Clone)]
pub struct ChatCompletionSetting {
  pub temperature: f64,
  pub top_p: Option<f64>,
  pub seed: u64,
  pub repeat_penalty: f32,
  pub repeat_last_n: usize,
  pub sample_len: usize,
  pub prompt: Option<String>,
}
