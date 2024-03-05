use clap::Args;
use clap::{Parser, Subcommand};
use time::format_description::well_known;
use time::UtcOffset;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zxrag_backend::conf::init_backend_conf;
use zxrag_backend::run_backend;
use zxrag_core::conf::BackendConf;
use zxrag_core::llama_cpp::{
  LlamaCppModel, LlamaCppModelConf, LlamaCppModelPipeline, LLAMA_CPP_MODEL,
};
use zxrag_core::model::ChatCompletionSetting;

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

      let model_config = LlamaCppModelConf {
        model_id: config.model_id,
        model_path: config.model_path,
        tokenizer_path: config.tokenizer_path,
      };

      let model = (*LLAMA_CPP_MODEL
        .get_or_init(|| LlamaCppModel::load_model(model_config).expect("")))
      .clone();

      let setting = ChatCompletionSetting {
        temperature: 0.8,
        top_p: None,
        seed: 299792458,
        repeat_penalty: 1.1,
        repeat_last_n: 64,
        sample_len: 128,
        prompt: None,
        one_shot: false,
      };

      let mut pipeline = LlamaCppModelPipeline::init_pipeline(model, setting)?;

      pipeline.run_cli()?;
    }
  }

  Ok(())
}
