use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BackendConf {
  pub log_file_path: String,
  pub log_file_name: String,
}

pub fn init_backend_conf(cli_conf_path: &str) -> Result<BackendConf, anyhow::Error> {
  let config: BackendConf = config::Config::builder()
    .set_default("log_file_path", "")?
    .set_default("log_file_name", "zxrag.log")?
    .add_source(config::File::with_name("zhixing.toml").required(false))
    .add_source(config::File::with_name(cli_conf_path).required(false))
    .add_source(config::Environment::with_prefix("ZX"))
    .build()?
    .try_deserialize()?;

  Ok(config)
}
