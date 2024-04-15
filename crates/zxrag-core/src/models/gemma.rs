use candle_core::DType;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::gemma::{Config, Model as GemmaModel};
use std::path::PathBuf;
use tokenizers::Tokenizer;

use crate::types::{
  conf::LlmConf,
  llm::LlmModel,
  model::{ModelEngine, ModelId},
};
use crate::util::{candle_device, local_load_safetensors};

#[derive(Debug, Clone)]
pub struct Model {
  id: ModelId,
  engine: ModelEngine,
  device: Device,
  model: GemmaModel,
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
    let logits = self.model.forward(x, index_pos)?;

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

    let start = std::time::Instant::now();

    let device = candle_device(&conf.device);

    let model_path = PathBuf::from(&conf.model_path);

    let tokenizer_filename = model_path.join("tokenizer.json");

    let filenames = local_load_safetensors(&model_path, "model.safetensors.index.json")?;

    tracing::info!("retrieved the files in {:?}", start.elapsed());

    let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(anyhow::Error::msg)?;

    let start = std::time::Instant::now();

    let dtype = if device.is_cuda() {
      DType::BF16
    } else {
      DType::F32
    };

    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&filenames, dtype, &device)? };

    let config_filename = model_path.join("config.json");
    let config: Config = serde_json::from_reader(std::fs::File::open(config_filename)?)?;
    let model = GemmaModel::new(&config, vb)?;

    tracing::info!("loaded the model in {:?}", start.elapsed());

    Ok(Self {
      id: conf.model_id,
      engine: conf.model_engine,
      device,
      model,
      tokenizer,
    })
  }
}
