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

impl Model {
  pub fn new(conf: &LlmConf) -> anyhow::Result<Self> {
    tracing::info!(
      "avx: {}, neon: {}, simd128: {}, f16c: {}",
      candle_core::utils::with_avx(),
      candle_core::utils::with_neon(),
      candle_core::utils::with_simd128(),
      candle_core::utils::with_f16c()
    );

    let device = candle_device(&conf.device);

    let model_path = PathBuf::from(&conf.model_path);
    let mut model_file = std::fs::File::open(&conf.model_path)?;
    let start = std::time::Instant::now();

    let model_weights = match model_path.extension().and_then(|v| v.to_str()) {
      Some("gguf") => {
        let model =
          gguf_file::Content::read(&mut model_file).map_err(|e| e.with_path(model_path))?;
        let mut total_size_in_bytes = 0;
        for (_, tensor) in model.tensor_infos.iter() {
          let elem_count = tensor.shape.elem_count();
          total_size_in_bytes +=
            elem_count * tensor.ggml_dtype.type_size() / tensor.ggml_dtype.block_size();
        }

        tracing::info!(
          "loaded {:?} tensors ({}) in {:.2}s",
          model.tensor_infos.len(),
          &format_size(total_size_in_bytes),
          start.elapsed().as_secs_f32(),
        );

        ModelWeights::from_gguf(model, &mut model_file, &device)?
      }
      Some("ggml" | "bin") | Some(_) | None => {
        let model = ggml_file::Content::read(&mut model_file, &device)
          .map_err(|e| e.with_path(model_path))?;
        let mut total_size_in_bytes = 0;
        for (_, tensor) in model.tensors.iter() {
          let elem_count = tensor.shape().elem_count();
          total_size_in_bytes +=
            elem_count * tensor.dtype().type_size() / tensor.dtype().block_size();
        }

        tracing::info!(
          "loaded {:?} tensors ({}) in {:.2}s",
          model.tensors.len(),
          &format_size(total_size_in_bytes),
          start.elapsed().as_secs_f32(),
        );

        tracing::info!("params: {:?}", model.hparams);

        let default_gqa = match &conf.model_id {
          ModelId::Zephyr7bAlpha | ModelId::Zephyr7bBeta => 8,
          _ => 1,
        };
        ModelWeights::from_ggml(model, default_gqa)?
      }
    };

    tracing::info!("model built");

    let tokenizer =
      Tokenizer::from_file(PathBuf::from(&conf.tokenizer_path)).map_err(anyhow::Error::msg)?;

    Ok(Self {
      id: conf.model_id,
      engine: conf.model_engine,
      device,
      model_weights,
      tokenizer,
    })
  }
}
