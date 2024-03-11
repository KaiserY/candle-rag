use candle_core::quantized::{ggml_file, gguf_file};
use candle_core::{Device, Tensor};
use candle_transformers::models::quantized_llama::ModelWeights;
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
  model_weights: ModelWeights,
  tokenizer: Tokenizer,
}

impl LlmModel for Model {
  fn id(&self) -> ModelId {
    self.id
  }

  fn engine(&self) -> ModelEngine {
    self.engine
  }

  fn tokenizer(&self) -> &Tokenizer {
    &self.tokenizer
  }

  fn device(&self) -> &Device {
    &self.device
  }

  fn forward(&mut self, x: &Tensor, index_pos: usize) -> anyhow::Result<Tensor> {
    let logits = self.model_weights.forward(x, index_pos)?;

    Ok(logits)
  }
}
