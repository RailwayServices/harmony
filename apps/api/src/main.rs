use axum::{routing::get, Json, Router};
use harmony_common::config::AppConfig;
use harmony_database::pool::Database;
use serde::Serialize;
use std::sync::Arc;
use sysinfo::System;
use tracing::{error, info};

struct AppState {
    db: Database,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    database: &'static str,
}

#[derive(Serialize)]
struct SystemStats {
    total_memory_kb: u64,
    used_memory_kb: u64,
    cpu_usage_pct: f32,
    os_name: String,
    host_name: String,
}

#[derive(Serialize)]
struct StatsResponse {
    health: HealthResponse,
    system: SystemStats,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_env().map_err(|e| e.to_string())?;

    info!("[API] Connecting to database for health check...");
    let db = Database::connect(&config.database_url, 5).await.map_err(|e| e.to_string())?;
    info!("[API] Database connection initialized.");

    let state = Arc::new(AppState { db });

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/stats", get(stats_handler))
        .with_state(state);

    let port = std::env::var("API_PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("[API] REST Server running on http://0.0.0.0:{}", port);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Json<HealthResponse> {
    let db_status = match sqlx::query("SELECT 1").execute(&state.db.pool).await {
        Ok(_) => "healthy",
        Err(e) => {
            error!("[API] Health check database query failed: {}", e);
            "unhealthy"
        }
    };

    Json(HealthResponse { status: "ok", database: db_status })
}

async fn stats_handler(
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> Json<StatsResponse> {
    let mut sys = System::new_all();
    sys.refresh_all();

    let db_status = match sqlx::query("SELECT 1").execute(&state.db.pool).await {
        Ok(_) => "healthy",
        Err(_) => "unhealthy",
    };

    let total_cpu_usage = sys.global_cpu_info().cpu_usage();

    let system_stats = SystemStats {
        total_memory_kb: sys.total_memory(),
        used_memory_kb: sys.used_memory(),
        cpu_usage_pct: total_cpu_usage,
        os_name: System::name().unwrap_or_default(),
        host_name: System::host_name().unwrap_or_default(),
    };

    Json(StatsResponse {
        health: HealthResponse { status: "ok", database: db_status },
        system: system_stats,
    })
}
