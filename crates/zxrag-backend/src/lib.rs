use zxrag_core::{conf::BackendConf, model::ModelChatSetting, quantized::run_quantized_llm};

pub mod conf;

#[tokio::main]
pub async fn run_backend(config: BackendConf) -> Result<(), anyhow::Error> {
  tracing::info!("{:?}", config);

  let setting = ModelChatSetting {
    temperature: 0.8,
    top_p: None,
    sample_len: 128,
    seed: 299792458,
    repeat_penalty: 1.1,
    repeat_last_n: 64,
    split_prompt: false,
    prompt: None,
  };

  run_quantized_llm(&config, &setting)?;

  Ok(())
}
