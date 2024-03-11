use std::sync::OnceLock;

use crate::models::llama_cpp_new::Model as LlamaCppModel;
use crate::models::phi::Model as PhiModel;

pub static LLM_MODEL_HANDLE: OnceLock<LlmModelHandle> = OnceLock::new();

pub enum LlmModelHandle {
  LlamaCpp(LlamaCppModel),
  // Phi(PhiModel),
}
