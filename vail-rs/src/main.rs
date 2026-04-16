use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use vail_rs::api;
use vail_rs::db;

#[tokio::main]
async fn main() {
    let config = vail_rs::config::load_config();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vail_rs=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(
        "Starting Vail server on {}:{}",
        config.server.host,
        config.server.port
    );

    let db_pool = db::init_pool(&config.database).await;
    db::run_migrations(&db_pool).await;
    db::migrate::ensure_partitions(&db_pool).await;
    db::migrate::ensure_default_admin_menu(&db_pool).await;

    let state = api::AppState {
        db: db_pool,
        config: config.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_router = Router::new()
        .merge(api::auth::router())
        .merge(api::host::router())
        .merge(api::iam::router())
        .merge(api::orion::router())
        .merge(api::ssh::router())
        .merge(api::ssh_key::router())
        .merge(api::sftp::router());

    let app = Router::new()
        .nest("/api", api_router.clone())
        .merge(api_router)
        .route("/", get(api::web::index))
        .route("/*path", get(api::web::assets))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::new(config.server.host.parse().unwrap(), config.server.port);
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
