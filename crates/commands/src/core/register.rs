use harmony_common::error::HarmonyError;
use std::sync::Arc;
use tokio::sync::OnceCell;
use twilight_http::Client as HttpClient;
use twilight_model::id::Id;
use twilight_model::id::marker::ApplicationMarker;

static APP_ID: OnceCell<Id<ApplicationMarker>> = OnceCell::const_new();

pub async fn register_global_commands(http: Arc<HttpClient>) -> Result<(), HarmonyError> {
    let app_info = http.current_user_application().await?.model().await?;
    let app_id = app_info.id;
    APP_ID.set(app_id).ok();

    let interaction_client = http.interaction(app_id);

    let router = crate::core::router::CommandRouter::new("!".to_string());
    let commands = router.get_commands();

    interaction_client.set_global_commands(&commands).await?;

    tracing::info!("Registered global application commands successfully");
    Ok(())
}
