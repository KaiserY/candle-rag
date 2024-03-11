use std::sync::OnceLock;

use crate::models::llama_cpp_new::Model;

pub static LLM_MODEL_HANDLE: OnceLock<LlmModelHandle> = OnceLock::new();

pub enum LlmModelHandle {
  LlamaCpp(Model),
}
