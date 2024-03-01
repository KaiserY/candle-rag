use clap::Args;
use clap::{Parser, Subcommand};
use zxrag_backend::conf::init_backend_conf;
use zxrag_backend::run_backend;
use zxrag_backend::trace::init_backend_tracing;
use zxrag_core::conf::BackendConf;

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
}

fn main() -> Result<(), anyhow::Error> {
  let cli = Cli::parse();

  match cli.command {
    Commands::Backend(cli_config) => {
      let config: BackendConf = init_backend_conf(&cli_config.config)?;

      init_backend_tracing(&config)?;

      run_backend(config)?;
    }
  }

  Ok(())
}
