use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_transformers::models::gemma::{Config, Model as GemmaModel};
use std::path::PathBuf;
use tokenizers::Tokenizer;

use crate::types::{
  conf::LlmConf,
  llm::LlmModel,
  model::{ModelEngine, ModelId},
};
use crate::util::{candle_device, format_size};

#[derive(Debug, Clone)]
pub struct Model {
  id: ModelId,
  engine: ModelEngine,
  device: Device,
  model: GemmaModel,
  tokenizer: Tokenizer,
}
