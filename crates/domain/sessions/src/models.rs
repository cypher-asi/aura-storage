use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: Uuid,
    pub project_agent_id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub created_by: Uuid,
    pub model: Option<String>,
    pub status: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub context_usage: f32,
    pub summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    /// Maintained by the `session_events_after_insert` trigger on
    /// `session_events`. Lets the chat-app session list filter empty
    /// orphan rows in a single indexed `WHERE event_count > 0` instead
    /// of fanning out one `list_events?limit=1` probe per session from
    /// aura-os-server (see migration 0015).
    pub event_count: i32,
    /// Timestamp of the most recent `session_events` row for this
    /// session. Used as the primary sort key on the chat-app session
    /// list so the most recently-active sessions float to the top
    /// regardless of when the row was first created.
    pub last_event_at: Option<DateTime<Utc>>,
    /// Whether this session is publicly shareable. Flipped to `true`
    /// when an owner creates a share link and back to `false` on
    /// unshare. The public read path gates on this flag so an
    /// `public_share_id` alone never exposes a private session.
    /// `#[serde(default)]` keeps older payloads (pre-migration 0017)
    /// deserializable.
    #[serde(default)]
    pub is_public: bool,
    /// Opaque capability token for the public share link, formatted as
    /// `t_` + 32 lowercase hex chars (a v4 UUID with dashes stripped),
    /// e.g. `t_6a1e3d8f6e548191948c1f0a9c68cbda`. `None` until the
    /// session is first shared. Treated as a secret: never log it in
    /// full. Backed by a partial unique index (migration 0017).
    #[serde(default)]
    pub public_share_id: Option<String>,
}

/// `Session` joined with the agent metadata the chat-app left
/// panel needs to render a row (agent avatar resolution + stream
/// lane keying) without a follow-up `listProjectBindings` fan-out
/// per agent. Returned by the user-scoped session list endpoint
/// (`/api/me/sessions`, see migration 0016) which collapses what
/// used to be `A x (1 + B)` HTTP calls from the chat-app left
/// panel down to one.
///
/// Notes on absent fields:
/// - There is no `agent_name` on `project_agents` (see
///   `crates/db/migrations/0001_create_project_agents.sql`); the
///   FE resolves agent names from its existing per-agent caches.
/// - There is no `projects` table in aura-storage; project
///   metadata lives in aura-os and is resolved client-side from
///   `useProjectsListStore`. We deliberately omit a stub
///   `project_name` field rather than wire a column that would
///   always be `NULL`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichedSession {
    #[serde(flatten)]
    pub session: Session,
    /// `project_agents.agent_id` -- the agent identifier the FE
    /// keys avatars and stream lanes by. Distinct from
    /// `Session.project_agent_id` (which is the per-project
    /// instance binding row id, not the agent definition). May be
    /// `None` if the binding row was deleted or migrated away from
    /// underneath the session.
    pub agent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub model: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSessionRequest {
    pub status: Option<String>,
    pub total_input_tokens: Option<i64>,
    pub total_output_tokens: Option<i64>,
    pub context_usage: Option<f32>,
    pub summary: Option<String>,
    pub ended_at: Option<DateTime<Utc>>,
    /// Set the public-share flag. `None` leaves it unchanged (the repo
    /// `update` uses `COALESCE`), `Some(true)`/`Some(false)` share or
    /// unshare the session.
    pub is_public: Option<bool>,
    /// Set the public share token (`t_` + 32 lowercase hex chars).
    /// `None` leaves the existing value unchanged. Capability token:
    /// callers must not log it in full.
    pub public_share_id: Option<String>,
}

/// Validate a public share token's shape (`^t_[0-9a-f]{32}$`).
///
/// Share ids are capability tokens, so validation is centralized here
/// and callers should avoid logging the raw value.
pub fn is_valid_public_share_id(token: &str) -> bool {
    let Some(hex) = token.strip_prefix("t_") else {
        return false;
    };
    hex.len() == 32
        && hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_session() -> Session {
        Session {
            id: Uuid::nil(),
            project_agent_id: Uuid::nil(),
            project_id: Uuid::nil(),
            org_id: None,
            created_by: Uuid::nil(),
            model: Some("gpt-test".to_string()),
            status: "active".to_string(),
            total_input_tokens: 10,
            total_output_tokens: 20,
            context_usage: 0.5,
            summary: None,
            started_at: DateTime::<Utc>::from_timestamp(0, 0).expect("valid epoch timestamp"),
            ended_at: None,
            event_count: 3,
            last_event_at: None,
            is_public: true,
            public_share_id: Some("t_6a1e3d8f6e548191948c1f0a9c68cbda".to_string()),
        }
    }

    #[test]
    fn session_serde_round_trip_preserves_share_fields() {
        let session = sample_session();

        let json = serde_json::to_value(&session).expect("session serializes to JSON");
        // Fields are camelCased on the wire.
        assert_eq!(json["isPublic"], serde_json::json!(true));
        assert_eq!(
            json["publicShareId"],
            serde_json::json!("t_6a1e3d8f6e548191948c1f0a9c68cbda")
        );

        let decoded: Session =
            serde_json::from_value(json).expect("session deserializes back from JSON");
        assert_eq!(decoded.is_public, session.is_public);
        assert_eq!(decoded.public_share_id, session.public_share_id);
        assert_eq!(decoded.id, session.id);
        assert_eq!(decoded.status, session.status);
    }

    #[test]
    fn session_defaults_share_fields_when_absent() {
        // A payload produced before migration 0017 omits the share
        // fields entirely; `#[serde(default)]` must fill them in.
        let legacy = serde_json::json!({
            "id": Uuid::nil(),
            "projectAgentId": Uuid::nil(),
            "projectId": Uuid::nil(),
            "orgId": null,
            "createdBy": Uuid::nil(),
            "model": null,
            "status": "active",
            "totalInputTokens": 0,
            "totalOutputTokens": 0,
            "contextUsage": 0.0,
            "summary": null,
            "startedAt": "1970-01-01T00:00:00Z",
            "endedAt": null,
            "eventCount": 0,
            "lastEventAt": null
        });

        let decoded: Session =
            serde_json::from_value(legacy).expect("legacy session payload deserializes");
        assert!(!decoded.is_public);
        assert_eq!(decoded.public_share_id, None);
    }

    #[test]
    fn validates_public_share_id_shape() {
        assert!(is_valid_public_share_id(
            "t_6a1e3d8f6e548191948c1f0a9c68cbda"
        ));
        assert!(!is_valid_public_share_id(
            "6a1e3d8f-6e54-4191-948c-1f0a9c68cbda"
        ));
        assert!(!is_valid_public_share_id(
            "t_6A1e3d8f6e548191948c1f0a9c68cbda"
        ));
        assert!(!is_valid_public_share_id("t_6a1e3d8f"));
        assert!(!is_valid_public_share_id(""));
    }
}
