use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(
  Clone, Default, Debug, Copy, PartialEq, Eq, Deserialize, Serialize, EnumString, Display,
)]
pub enum ModelId {
  #[serde(rename = "zephyr-7b-alpha")]
  #[strum(serialize = "zephyr-7b-alpha")]
  Zephyr7bAlpha,
  #[serde(rename = "zephyr-7b-beta")]
  #[strum(serialize = "zephyr-7b-beta")]
  Zephyr7bBeta,
  #[serde(rename = "Mistral-7B-Instruct-v0.1")]
  #[strum(serialize = "Mistral-7B-Instruct-v0.1")]
  Mistral7bInstructV0_1,
  #[serde(rename = "Mistral-7B-Instruct-v0.2")]
  #[strum(serialize = "Mistral-7B-Instruct-v0.2")]
  Mistral7bInstructV0_2,
  #[serde(rename = "phi-2")]
  #[strum(serialize = "phi-2")]
  PhiV2,
  #[serde(rename = "bge-large-zh-v1.5")]
  #[strum(serialize = "bge-large-zh-v1.5")]
  BgeLargeZhV1_5,
  #[default]
  None,
}

impl ModelId {
  pub fn is_mistral(&self) -> bool {
    matches!(
      self,
      Self::Zephyr7bAlpha
        | Self::Zephyr7bBeta
        | Self::Mistral7bInstructV0_1
        | Self::Mistral7bInstructV0_2
    )
  }

  pub fn is_zephyr(&self) -> bool {
    matches!(self, Self::Zephyr7bAlpha | Self::Zephyr7bBeta)
  }

  pub fn is_open_chat(&self) -> bool {
    false
  }
}

#[derive(
  Clone, Default, Debug, Copy, PartialEq, Eq, Deserialize, Serialize, EnumString, Display,
)]
pub enum ModelEngine {
  #[serde(rename = "huggingface")]
  #[strum(serialize = "huggingface")]
  HuggingFace,
  #[serde(rename = "gguf")]
  #[strum(serialize = "gguf")]
  Gguf,
  #[default]
  None,
}
