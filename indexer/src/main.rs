mod api;
mod db;
mod graphql;
#[cfg(feature = "ingest")]
mod ingest;
mod openapi;
mod poller;
mod ws;

use crate::api::{health, list_events, ApiState};
use crate::openapi::ApiDoc;
use anyhow::Context;
use axum::{routing::get, Router};
use axum_prometheus::PrometheusMetricLayer;
use clap::Parser;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Parser, Debug)]
#[command(name = "propchain-indexer")]
#[command(about = "PropChain event indexer and query API", long_about = None)]
struct Config {
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    #[arg(long, env = "SUBSTRATE_WS", default_value = "ws://127.0.0.1:9944")]
    substrate_ws: String,

    #[arg(long, env = "BIND_ADDR", default_value = "0.0.0.0:8088")]
    bind_addr: String,

    #[arg(long, env = "DB_MAX_CONNS", default_value_t = 10)]
    db_max_conns: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse()?))
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    let cfg = Config::parse();

    let db = db::Db::connect(&cfg.database_url, cfg.db_max_conns)
        .await
        .context("connect database")?;
    db.migrate().await.context("run migrations")?;

    let db = Arc::new(db);

    // WebSocket broadcast channel for streaming events
    let ws_state = ws::WsState::new();

    // Spawn DB poller — publishes new events to WebSocket subscribers
    {
        let db_clone = db.clone();
        let ws_clone = ws_state.clone();
        tokio::spawn(async move {
            poller::run_poller(db_clone, ws_clone).await;
        });
    }

    // Start ingestor in background
    #[cfg(feature = "ingest")]
    {
        let db_clone = db.clone();
        let ws = cfg.substrate_ws.clone();
        tokio::spawn(async move {
            if let Err(e) = ingest::run_ingestor(db_clone, ws).await {
                tracing::error!("ingestor exited: {e}");
            }
        });
    }

    // Rate limiting: 100 requests per second per IP, burst of 20
    let governor_conf = GovernorConfigBuilder::default()
        .per_second(100)
        .burst_size(20)
        .finish()
        .expect("valid governor config");
    let governor_layer = GovernorLayer {
        config: std::sync::Arc::new(governor_conf),
    };

    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_state = ApiState { db: db.clone() };
    let schema = graphql::build_schema(db.clone());

    // #174: API versioning — all REST endpoints live under /api/v1/
    // The unversioned /health and /metrics paths are preserved for infrastructure tooling.
    let v1_router = Router::new()
        .route("/events", get(list_events))
        .route("/contracts", get(crate::api::list_contracts))
        .route("/version", get(crate::api::api_version))
        .with_state(api_state.clone())
        .layer(axum::middleware::from_fn(crate::api::set_api_version_header));

    let rest_router = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(|| async move { metric_handle.render() }))
        .nest("/api/v1", v1_router)
        .with_state(api_state);

    let graphql_router = Router::new()
        .route(
            "/graphql",
            get(graphql::graphql_playground).post(graphql::graphql_handler),
        )
        .with_state(schema);

    let ws_router = Router::new()
        .route("/ws/events", get(ws::ws_handler))
        .with_state(ws_state);

    let app = Router::new()
        .merge(rest_router)
        .merge(graphql_router)
        .merge(ws_router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(prometheus_layer)
        .layer(cors)
        .layer(governor_layer);

    let addr: SocketAddr = cfg.bind_addr.parse().context("parse bind addr")?;
    tracing::info!("Indexer API listening on http://{}", addr);
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serve")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
