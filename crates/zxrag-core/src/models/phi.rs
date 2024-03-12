use candle_core::DType;
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::mixformer::MixFormerSequentialForCausalLM as MixFormer;
use candle_transformers::models::phi::{Config as PhiConfig, Model as Phi};
use candle_transformers::models::quantized_mixformer::MixFormerSequentialForCausalLM as QMixFormer;
use std::path::PathBuf;
use tokenizers::Tokenizer;

use crate::types::{
  conf::LlmConf,
  llm::LlmModel,
  model::{ModelEngine, ModelId},
};
use crate::util::{candle_device, local_load_safetensors};

#[derive(Clone)]
enum PhiModel {
  _MixFormer(MixFormer),
  Phi(Phi),
  _Quantized(QMixFormer),
}

#[derive(Clone)]
pub struct Model {
  id: ModelId,
  engine: ModelEngine,
  device: Device,
  phi_model: PhiModel,
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

  fn forward(&mut self, x: &Tensor, _index_pos: usize) -> anyhow::Result<Tensor> {
    let logits = match &mut self.phi_model {
      PhiModel::_MixFormer(m) => m.forward(x)?,
      PhiModel::Phi(m) => m.forward(x)?,
      PhiModel::_Quantized(m) => m.forward(x)?,
    };

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

    let vb = unsafe { VarBuilder::from_mmaped_safetensors(&filenames, DType::F32, &device)? };

    let config_filename = model_path.join("config.json");
    let config = std::fs::read_to_string(config_filename)?;
    let config: PhiConfig = serde_json::from_str(&config)?;
    let phi = Phi::new(&config, vb)?;
    let model = PhiModel::Phi(phi);

    tracing::info!("loaded the model in {:?}", start.elapsed());

    Ok(Self {
      id: conf.model_id,
      engine: conf.model_engine,
      device,
      phi_model: model,
      tokenizer,
    })
  }
}
