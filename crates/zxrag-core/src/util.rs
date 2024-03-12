use candle_core::utils::{cuda_is_available, metal_is_available};
use candle_core::Device;
use std::path::Path;

use crate::types::model::ModelId;

pub fn format_size(size_in_bytes: usize) -> String {
  if size_in_bytes < 1_000 {
    format!("{}B", size_in_bytes)
  } else if size_in_bytes < 1_000_000 {
    format!("{:.2}KB", size_in_bytes as f64 / 1e3)
  } else if size_in_bytes < 1_000_000_000 {
    format!("{:.2}MB", size_in_bytes as f64 / 1e6)
  } else {
    format!("{:.2}GB", size_in_bytes as f64 / 1e9)
  }
}

pub fn candle_device(device: &str) -> Device {
  match device {
    "cpu" => Device::Cpu,
    "cuda" => {
      if cuda_is_available() {
        Device::new_cuda(0).unwrap_or_else(|err| {
          tracing::error!("cuda error: {}", err);
          Device::Cpu
        })
      } else {
        Device::Cpu
      }
    }
    "metal" => {
      if metal_is_available() {
        Device::new_metal(0).unwrap_or_else(|err| {
          tracing::error!("metal error: {}", err);
          Device::Cpu
        })
      } else {
        Device::Cpu
      }
    }
    _ => Device::Cpu,
  }
}

pub fn local_load_safetensors(
  repo: &Path,
  json_file: &str,
) -> candle_core::Result<Vec<std::path::PathBuf>> {
  let json_file = repo.join(json_file);
  let json_file = std::fs::File::open(json_file)?;
  let json: serde_json::Value =
    serde_json::from_reader(&json_file).map_err(candle_core::Error::wrap)?;
  let weight_map = match json.get("weight_map") {
    None => candle_core::bail!("no weight map in {json_file:?}"),
    Some(serde_json::Value::Object(map)) => map,
    Some(_) => candle_core::bail!("weight map in {json_file:?} is not a map"),
  };
  let mut safetensors_files = std::collections::HashSet::new();
  for value in weight_map.values() {
    if let Some(file) = value.as_str() {
      safetensors_files.insert(file.to_string());
    }
  }
  let safetensors_files = safetensors_files
    .iter()
    .map(|v| Ok(repo.join(v)))
    .collect::<candle_core::Result<Vec<_>>>()?;
  Ok(safetensors_files)
}

pub fn eos_token(model_id: ModelId) -> &'static str {
  match model_id {
    ModelId::PhiV2 => "<|endoftext|>",
    _ => "</s>",
  }
}
