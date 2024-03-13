use std::sync::Arc;
use std::sync::OnceLock;

use crate::models::bert::Model as BertModel;
use crate::models::llama_cpp::Model as LlamaCppModel;
use crate::models::phi::Model as PhiModel;
use crate::types::conf::{EmbeddingConf, LlmConf};
use crate::types::llm::{TextGeneration, TextGenerationSetting};
use crate::types::model::ModelId;

pub enum LlmModelHandle {
  LlamaCpp(LlamaCppModel),
  Phi(PhiModel),
}

pub static LLM_MODEL_HANDLE: OnceLock<LlmModelHandle> = OnceLock::new();

pub static EMBEDDING_MODEL_HANDLE: OnceLock<Arc<BertModel>> = OnceLock::new();

pub fn set_llm_model_handle(model_id: ModelId, conf: &LlmConf) -> anyhow::Result<()> {
  match model_id {
    ModelId::Mistral7bInstructV0_2 | ModelId::Mistral7bInstructV0_1 | ModelId::Zephyr7bBeta => {
      let model = LlamaCppModel::new(conf)?;

      LLM_MODEL_HANDLE
        .set(LlmModelHandle::LlamaCpp(model))
        .map_err(|_| anyhow::anyhow!("set_llm_model_handle failed"))?;

      Ok(())
    }
    ModelId::PhiV2 => {
      let model = PhiModel::new(conf)?;

      LLM_MODEL_HANDLE
        .set(LlmModelHandle::Phi(model))
        .map_err(|_| anyhow::anyhow!("set_llm_model_handle failed"))?;

      Ok(())
    }
    _ => Err(anyhow::anyhow!("{} not unimplemented", model_id)),
  }
}

pub fn get_text_gen(setting: TextGenerationSetting) -> anyhow::Result<TextGeneration> {
  let text_gen = match LLM_MODEL_HANDLE
    .get()
    .ok_or(anyhow::anyhow!("Get LLM_MODEL_HANDLE failed"))?
  {
    LlmModelHandle::LlamaCpp(model) => TextGeneration::new(Box::new(model.clone()), setting)?,
    LlmModelHandle::Phi(model) => TextGeneration::new(Box::new(model.clone()), setting)?,
  };

  Ok(text_gen)
}

pub fn set_embedding_model_handle(model_id: ModelId, conf: &EmbeddingConf) -> anyhow::Result<()> {
  tracing::info!("{}", model_id);

  EMBEDDING_MODEL_HANDLE
    .set(Arc::new(BertModel::new(conf)?))
    .map_err(|_| anyhow::anyhow!("set_embedding_model_handle failed"))?;

  Ok(())
}

pub fn get_embedding_model(model_id: ModelId) -> anyhow::Result<Arc<BertModel>> {
  tracing::info!("{}", model_id);

  let model = EMBEDDING_MODEL_HANDLE
    .get()
    .ok_or(anyhow::anyhow!("set_embedding_model_handle failed"))?;

  Ok(model.clone())
}
