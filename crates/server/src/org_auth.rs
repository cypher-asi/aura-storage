use axum::http::StatusCode;
use uuid::Uuid;

use aura_storage_auth::AuthUser;
use aura_storage_core::AppError;

use crate::state::AppState;

pub async fn require_org_access(
    state: &AppState,
    auth: &AuthUser,
    org_id: Uuid,
) -> Result<(), AppError> {
    let Some(network_url) = state.aura_network_url.as_deref() else {
        return Err(AppError::ServiceUnavailable(
            "Aura Network URL is not configured".into(),
        ));
    };

    let url = format!("{network_url}/api/orgs/{org_id}");
    let response = state
        .http_client
        .get(url)
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", auth.bearer_token),
        )
        .send()
        .await
        .map_err(|e| AppError::ServiceUnavailable(format!("Aura Network request failed: {e}")))?;

    match response.status() {
        status if status.is_success() => Ok(()),
        StatusCode::UNAUTHORIZED => Err(AppError::Unauthorized(
            "JWT is not authorized for this organization".into(),
        )),
        StatusCode::FORBIDDEN | StatusCode::NOT_FOUND => Err(AppError::Forbidden(
            "JWT does not grant access to this organization".into(),
        )),
        status => Err(AppError::ServiceUnavailable(format!(
            "Aura Network membership check failed with status {status}"
        ))),
    }
}
