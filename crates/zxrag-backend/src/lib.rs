use axum::{
  body::Body,
  http::{header, Method, StatusCode, Uri},
  response::{IntoResponse, Response},
  routing::{delete, get, post},
  Router,
};
use rust_embed::RustEmbed;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool};
use sqlx::ConnectOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use std::{net::SocketAddr, str::FromStr};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use zxrag_core::types::conf::BackendConf;

use crate::controller::knowledge_base_controller;
use crate::controller::openai_controller;

pub mod controller;
pub mod error;

#[derive(RustEmbed)]
#[folder = "../../zxrag-ui/dist/"]
struct Dist;

#[derive(Clone)]
pub struct BackendState {
  config: Arc<BackendConf>,
  pool: Pool<Sqlite>,
}

#[tokio::main]
pub async fn run_backend(config: BackendConf) -> anyhow::Result<()> {
  let addr: SocketAddr = config.bind_addr.parse()?;

  SqliteConnectOptions::from_str(&config.database_url)?
    .journal_mode(SqliteJournalMode::Wal)
    .create_if_missing(true)
    .connect()
    .await?;

  let pool = SqlitePool::connect(&config.database_url).await?;

  let migration_sql = include_str!("../migrations/sqlite/zxrag.sql");

  sqlx::query(migration_sql).execute(&pool).await?;

  let shared_state = BackendState {
    config: Arc::new(config),
    pool,
  };

  let cors = CorsLayer::new()
    .allow_methods([Method::GET, Method::POST])
    .allow_headers(Any)
    .allow_origin(Any);

  let knowledge_base_routes = Router::new()
    .route(
      "/tables",
      get(knowledge_base_controller::list_tables).post(knowledge_base_controller::create_tables),
    )
    .route(
      "/:table_id",
      delete(knowledge_base_controller::delete_table),
    )
    .route(
      "/:table_id/chat/completions",
      post(knowledge_base_controller::create_chat_completion),
    );

  let v1_routes = Router::new()
    .route(
      "/chat/completions",
      post(openai_controller::create_chat_completion),
    )
    .route("/embeddings", post(openai_controller::embeddings))
    .route("/models", post(openai_controller::models))
    .route(
      "/files",
      post(openai_controller::upload_file).get(openai_controller::list_files),
    )
    .route("/files/:file_id", delete(openai_controller::delete_file))
    .nest("/knowledgebase", knowledge_base_routes);

  let app = Router::new()
    .nest("/v1", v1_routes)
    .route("/*file", get(static_handler))
    .layer(CatchPanicLayer::new())
    .layer(TraceLayer::new_for_http())
    .layer(cors)
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
