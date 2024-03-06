use axum::{
  body::Body,
  http::{header, StatusCode, Uri},
  response::{IntoResponse, Response},
  routing::{get, post},
  Router,
};
use rust_embed::RustEmbed;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;
use zxrag_core::types::conf::BackendConf;

use crate::controller::openai_controller;

pub mod conf;
pub mod controller;
pub mod error;

#[derive(RustEmbed)]
#[folder = "../../zxrag-ui/dist/"]
struct Dist;

#[derive(Clone)]
pub struct BackendState {
  config: Arc<BackendConf>,
}

#[tokio::main]
pub async fn run_backend(config: BackendConf) -> anyhow::Result<()> {
  let addr: SocketAddr = config.bind_addr.parse()?;

  let shared_state = BackendState {
    config: Arc::new(config),
  };

  let v1_routes = Router::new().route(
    "chat/completions",
    post(openai_controller::chat_completions),
  );

  let app = Router::new()
    .nest("/v1", v1_routes)
    .route("/*file", get(static_handler))
    .layer(CatchPanicLayer::new())
    .layer(TraceLayer::new_for_http())
    .with_state(shared_state);

  tracing::info!("listening on {}", addr);

  let listener = tokio::net::TcpListener::bind(addr).await?;

  axum::serve(listener, app).await?;

  Ok(())
}

async fn static_handler(uri: Uri) -> impl IntoResponse {
  let path = uri.path().trim_start_matches(['.', '/']).to_string();

  StaticFile(path)
}

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
  T: Into<String>,
{
  fn into_response(self) -> Response {
    let path = self.0.into();

    match Dist::get(path.as_str()) {
      Some(content) => {
        let body = Body::from(content.data);
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Response::builder()
          .header(header::CONTENT_TYPE, mime.as_ref())
          .header(header::ETAG, hex::encode(content.metadata.sha256_hash()))
          .header(header::CACHE_CONTROL, "public, max-age=31536000")
          .body(body)
          .unwrap()
      }
      None => Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("404"))
        .unwrap(),
    }
  }
}
