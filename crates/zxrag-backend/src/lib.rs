pub mod tracing;

use zxrag_core::conf::BackendConf;

#[tokio::main]
pub async fn run_backend(_config: BackendConf) -> Result<(), anyhow::Error> {
  Ok(())
}
