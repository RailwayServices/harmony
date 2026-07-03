use railway_common::error::RailwayError;
use std::sync::Arc;
use tracing::{info, warn};
use twilight_http::{request::Request, routing::Route, Client as DiscordClient};

pub async fn apply_glow(client: Arc<DiscordClient>) -> Result<(), RailwayError> {
    info!("[GLOW] Applying Display Name Styles (Glow) using undocumented API...");

    let payload = serde_json::json!({
        "display_name_font_id": 10,
        "display_name_effect_id": 3,
        "display_name_colors": [16777215]
    });

    let payload_bytes =
        serde_json::to_vec(&payload).map_err(|e| RailwayError::Internal(e.to_string()))?;

    let request = Request::builder(&Route::UpdateCurrentUser)
        .body(payload_bytes)
        .build()
        .map_err(|e| RailwayError::Internal(e.to_string()))?;

    match client.request::<twilight_model::user::CurrentUser>(request).await {
        Ok(_) => {
            info!("[GLOW] Successfully applied Glow styles to the bot profile.");
        }
        Err(e) => {
            warn!("[GLOW] Failed to apply Glow styles: {}", e);
        }
    }

    Ok(())
}
