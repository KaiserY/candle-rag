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
  #[serde(rename = "gemma-2b-it")]
  #[strum(serialize = "gemma-2b-it")]
  Gemma2bIt,
  #[default]
  None,
}

impl ModelId {
  pub fn is_mistral(&self) -> bool {
    matches!(self, Self::Zephyr7bAlpha | Self::Zephyr7bBeta)
  }

  pub fn is_zephyr(&self) -> bool {
    matches!(self, Self::Zephyr7bAlpha | Self::Zephyr7bBeta)
  }

  pub fn is_open_chat(&self) -> bool {
    false
  }
}

pub struct ChatCompletionSetting {
  pub temperature: f64,
  pub top_p: Option<f64>,
  pub seed: u64,
  pub repeat_penalty: f32,
  pub repeat_last_n: usize,
  pub split_prompt: bool,
  pub sample_len: usize,
  pub prompt: Option<String>,
  pub one_shot: bool,
}
