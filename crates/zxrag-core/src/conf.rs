use serde::{Deserialize, Serialize};

use crate::model::ModelId;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BackendConf {
  pub log_file_path: String,
  pub log_file_name: String,
  pub model_id: ModelId,
  pub model_path: String,
  pub tokenizer_path: String,
}
