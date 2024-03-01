use zxrag_core::conf::BackendConf;

pub fn init_backend_conf(cli_conf_path: &str) -> Result<BackendConf, anyhow::Error> {
  let config: BackendConf = config::Config::builder()
    .set_default("log_file_path", "")?
    .set_default("log_file_name", "zxrag.log")?
    .set_default("model_id", "none")?
    .add_source(config::File::with_name("zhixing.json").required(false))
    .add_source(config::File::with_name(cli_conf_path).required(false))
    .add_source(config::Environment::with_prefix("ZX"))
    .build()?
    .try_deserialize()?;

  Ok(config)
}
