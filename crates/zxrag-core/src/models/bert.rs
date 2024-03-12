use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use std::path::PathBuf;
use tokenizers::Tokenizer;

use crate::types::{
  conf::EmbeddingConf,
  model::{ModelEngine, ModelId},
};
use crate::util::candle_device;

pub struct Model {
  id: ModelId,
  engine: ModelEngine,
  device: Device,
  bert_model: BertModel,
  tokenizer: Tokenizer,
}

impl Model {
  pub fn new(conf: &EmbeddingConf) -> anyhow::Result<Self> {
    tracing::info!(
      "avx: {}, neon: {}, simd128: {}, f16c: {}",
      candle_core::utils::with_avx(),
      candle_core::utils::with_neon(),
      candle_core::utils::with_simd128(),
      candle_core::utils::with_f16c()
    );

    let device = candle_device(&conf.device);

    let model_path = PathBuf::from(&conf.model_path);

    let config_filename = model_path.join("config.json");
    let tokenizer_filename = model_path.join("tokenizer.json");
    let weights_filename = model_path.join("model.safetensors");

    let config = std::fs::read_to_string(config_filename)?;
    let config: Config = serde_json::from_str(&config)?;

    let tokenizer = Tokenizer::from_file(tokenizer_filename).map_err(anyhow::Error::msg)?;

    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&[weights_filename], DTYPE, &device)? };

    let model = BertModel::load(vb, &config)?;

    Ok(Self {
      id: conf.model_id,
      engine: conf.model_engine,
      device,
      bert_model: model,
      tokenizer,
    })
  }

  pub fn embedding(&mut self, prompt: &str) -> anyhow::Result<Tensor> {
    tracing::info!("id={}", self.id);
    tracing::info!("engine={}", self.engine);

    let start = std::time::Instant::now();

    let tokenizer = self
      .tokenizer
      .with_padding(None)
      .with_truncation(None)
      .map_err(anyhow::Error::msg)?;

    let tokens = tokenizer
      .encode(prompt, true)
      .map_err(anyhow::Error::msg)?
      .get_ids()
      .to_vec();

    let token_ids = Tensor::new(&tokens[..], &self.device)?.unsqueeze(0)?;
    let token_type_ids = token_ids.zeros_like()?;

    tracing::info!("Loaded and encoded {:?}", start.elapsed());

    let start = std::time::Instant::now();

    let ys = self.bert_model.forward(&token_ids, &token_type_ids)?;

    tracing::info!("Took {:?}", start.elapsed());

    Ok(ys)
  }
}
