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
    match self {
      Self::Mistral7bInstructV0_1 | Self::Mistral7bInstructV0_2 | Self::None => false,
      Self::Zephyr7bAlpha | Self::Zephyr7bBeta => true,
    }
  }

  pub fn is_open_chat(&self) -> bool {
    false
  }
}
