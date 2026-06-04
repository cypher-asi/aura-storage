use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A note. Blog posts are notes with the extra blog fields populated and a
/// draft/published lifecycle. The note BODY is stored on S3 by another
/// service; aura-storage only keeps metadata plus the S3 reference
/// (`body_url`, `body_s3_key`).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub title: String,
    pub slug: String,
    pub sort_order: i32,
    pub word_count: i32,
    pub body_url: Option<String>,
    pub body_s3_key: Option<String>,
    pub status: String,
    pub blog_type: Option<String>,
    pub excerpt: Option<String>,
    pub hero_image_url: Option<String>,
    pub read_time_minutes: Option<i32>,
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub sections: Option<serde_json::Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NoteFolder {
    pub id: Uuid,
    pub project_id: Uuid,
    pub org_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub sort_order: i32,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct NoteComment {
    pub id: Uuid,
    pub note_id: Uuid,
    pub author_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteRequest {
    pub org_id: Option<Uuid>,
    pub folder_id: Option<Uuid>,
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub sort_order: i32,
    #[serde(default)]
    pub word_count: i32,
    pub body_url: Option<String>,
    pub body_s3_key: Option<String>,
    pub blog_type: Option<String>,
    pub excerpt: Option<String>,
    pub hero_image_url: Option<String>,
    pub read_time_minutes: Option<i32>,
    pub author_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub sections: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteRequest {
    pub folder_id: Option<Uuid>,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub sort_order: Option<i32>,
    pub word_count: Option<i32>,
    pub body_url: Option<String>,
    pub body_s3_key: Option<String>,
    pub blog_type: Option<String>,
    pub excerpt: Option<String>,
    pub hero_image_url: Option<String>,
    pub read_time_minutes: Option<i32>,
    pub author_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub author_avatar_url: Option<String>,
    pub sections: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransitionNoteRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteFolderRequest {
    pub org_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub name: String,
    #[serde(default)]
    pub sort_order: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteFolderRequest {
    pub parent_id: Option<Uuid>,
    pub name: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteCommentRequest {
    pub author_id: Option<Uuid>,
    pub author_name: Option<String>,
    pub body: String,
}
