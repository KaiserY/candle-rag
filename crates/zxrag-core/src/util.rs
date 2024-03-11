use candle_core::utils::{cuda_is_available, metal_is_available};
use candle_core::Device;

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
