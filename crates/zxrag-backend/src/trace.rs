use time::format_description::well_known;
use time::UtcOffset;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use zxrag_core::conf::BackendConf;

pub fn init_backend_tracing(config: &BackendConf) -> Result<(), anyhow::Error> {
  let offset = UtcOffset::from_hms(8, 0, 0).expect("should get CST offset");

  let timer = tracing_subscriber::fmt::time::OffsetTime::new(offset, well_known::Rfc3339);

  let file_appender =
    tracing_appender::rolling::daily(&config.log_file_path, &config.log_file_name);
  let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

  let layer = tracing_subscriber::fmt::layer()
    .with_writer(non_blocking)
    .with_timer(timer.clone())
    .with_ansi(false);

  tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
    .with(layer)
    .with(tracing_subscriber::fmt::layer().with_timer(timer))
    .init();

  Ok(())
}
