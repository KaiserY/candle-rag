use serde::{Deserialize, Serialize};

use crate::types::model::{ModelEngine, ModelId};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BackendConf {
  pub log_file_path: String,
  pub log_file_name: String,
  pub bind_addr: String,
  pub llm_conf: LlmConf,
  pub embedding_conf: EmbeddingConf,
  pub lancedb_path: String,
  pub redb_path: String,
  pub opendal_path: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct LlmConf {
  pub enabled: bool,
  pub model_id: ModelId,
  pub model_engine: ModelEngine,
  pub model_path: String,
  pub tokenizer_path: String,
  pub device: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct EmbeddingConf {
  pub enabled: bool,
  pub model_id: ModelId,
  pub model_engine: ModelEngine,
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

pub fn init_backend_conf(cli_conf_path: &str) -> Result<BackendConf, anyhow::Error> {
  let config: BackendConf = config::Config::builder()
    .set_default("log_file_path", "")?
    .set_default("log_file_name", "zxrag.log")?
    .set_default("bind_addr", "0.0.0.0:3000")?
    .set_default("llm_conf.model_id", "none")?
    .set_default("llm_conf.model_engine", "llama.cpp")?
    .set_default("llm_conf.model_path", "")?
    .set_default("llm_conf.tokenizer_path", "")?
    .set_default("embedding_conf.model_id", "none")?
    .set_default("embedding_conf.model_engine", "huggingface")?
    .set_default("embedding_conf.model_path", "")?
    .set_default("embedding_conf.tokenizer_path", "")?
    .set_default("lancedb_path", "lancedb")?
    .set_default("redb_path", "kb.redb")?
    .set_default("opendal_path", "opendal")?
    .add_source(config::File::with_name("zhixing.json").required(false))
    .add_source(config::File::with_name(cli_conf_path).required(false))
    .add_source(config::Environment::with_prefix("ZX"))
    .build()?
    .try_deserialize()?;

  Ok(config)
}
