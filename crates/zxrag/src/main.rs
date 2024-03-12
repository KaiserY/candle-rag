use clap::Args;
use clap::{Parser, Subcommand};
use time::format_description::well_known;
use time::UtcOffset;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zxrag_backend::conf::init_backend_conf;
use zxrag_backend::run_backend;
use zxrag_core::types::conf::{BackendConf, LlmConf};
use zxrag_core::types::handle::{get_text_gen, set_llm_model_handle};
use zxrag_core::types::llm::TextGenerationSetting;

#[derive(Debug, Default, Args)]
pub struct CliConfig {
  #[clap(long, default_value_t = String::from("zxrag.toml"))]
  pub config: String,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  #[clap(about = "Run the backend")]
  Backend(CliConfig),
  #[clap(about = "Run the Cli")]
  Cli(CliConfig),
}

fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Backend(cli_config) => {
      let config: BackendConf = init_backend_conf(&cli_config.config)?;

      let offset = UtcOffset::from_hms(8, 0, 0).expect("should get CST offset");

      let timer = tracing_subscriber::fmt::time::OffsetTime::new(offset, well_known::Rfc3339);

      let file_appender =
        tracing_appender::rolling::daily(&config.log_file_path, &config.log_file_name);
      let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

      tracing_subscriber::registry()
        .with(
          tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(
          tracing_subscriber::fmt::layer()
            .with_writer(non_blocking)
            .with_timer(timer.clone())
            .with_ansi(false),
        )
        .with(
          tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_timer(timer),
        )
        .init();

      let model_config = LlmConf {
        model_id: config.llm_conf.model_id,
        model_engine: config.llm_conf.model_engine,
        model_path: config.llm_conf.model_path.clone(),
        tokenizer_path: config.llm_conf.tokenizer_path.clone(),
        device: config.llm_conf.device.clone(),
      };

      tracing::info!("model_config={:?}", model_config);

      set_llm_model_handle(config.llm_conf.model_id, &model_config)?;

      run_backend(config)?;
    }
    Commands::Cli(cli_config) => {
      let config: BackendConf = init_backend_conf(&cli_config.config)?;

      tracing_subscriber::fmt()
        .with_level(false)
        .with_ansi(false)
        .with_target(false)
        .without_time()
        .init();

      let model_config = LlmConf {
        model_id: config.llm_conf.model_id,
        model_engine: config.llm_conf.model_engine,
        model_path: config.llm_conf.model_path.clone(),
        tokenizer_path: config.llm_conf.tokenizer_path.clone(),
        device: config.llm_conf.device.clone(),
      };

      tracing::info!("model_config={:?}", model_config);

      set_llm_model_handle(config.llm_conf.model_id, &model_config)?;

      let text_gen_setting = TextGenerationSetting {
        temperature: 0.8,
        top_p: None,
        seed: 299792458,
        repeat_penalty: 1.1,
        repeat_last_n: 64,
        sample_len: 128,
        prompt: "<s>[INST] Hello! [/INST]".to_string(),
      };

      let mut text_gen = get_text_gen(text_gen_setting)?;

      let output = text_gen.generate()?;

      tracing::info!("{output}");
    }
  }

  Ok(())
}
