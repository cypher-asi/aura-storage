use sqlx::PgPool;
use uuid::Uuid;

use aura_storage_core::AppError;

use crate::models::{
    CreateNoteCommentRequest, CreateNoteFolderRequest, CreateNoteRequest, Note, NoteComment,
    NoteFolder, TransitionNoteRequest, UpdateNoteFolderRequest, UpdateNoteRequest,
};

// ============================================================================
// Notes
// ============================================================================

pub async fn create(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    input: &CreateNoteRequest,
) -> Result<Note, AppError> {
    if input.title.trim().is_empty() {
        return Err(AppError::BadRequest("Note title must not be empty".into()));
    }
    if input.slug.trim().is_empty() {
        return Err(AppError::BadRequest("Note slug must not be empty".into()));
    }

    let note = sqlx::query_as::<_, Note>(
        r#"
        INSERT INTO notes (
            project_id, org_id, folder_id, title, slug, sort_order, word_count,
            body_url, body_s3_key, blog_type, excerpt, hero_image_url,
            read_time_minutes, author_id, author_name, author_avatar_url,
            sections, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(input.org_id)
    .bind(input.folder_id)
    .bind(input.title.trim())
    .bind(input.slug.trim())
    .bind(input.sort_order)
    .bind(input.word_count)
    .bind(&input.body_url)
    .bind(&input.body_s3_key)
    .bind(&input.blog_type)
    .bind(&input.excerpt)
    .bind(&input.hero_image_url)
    .bind(input.read_time_minutes)
    .bind(input.author_id)
    .bind(&input.author_name)
    .bind(&input.author_avatar_url)
    .bind(&input.sections)
    .bind(created_by)
    .fetch_one(pool)
    .await?;

    Ok(note)
}

pub async fn list_by_project(pool: &PgPool, project_id: Uuid) -> Result<Vec<Note>, AppError> {
    let notes = sqlx::query_as::<_, Note>(
        "SELECT * FROM notes WHERE project_id = $1 ORDER BY sort_order, created_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(notes)
}

/// List only published notes for a project, newest published first.
/// Used by public blog reads (internal endpoint).
pub async fn list_published_by_project(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Vec<Note>, AppError> {
    let notes = sqlx::query_as::<_, Note>(
        "SELECT * FROM notes WHERE project_id = $1 AND status = 'published' ORDER BY published_at DESC",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(notes)
}

pub async fn get(pool: &PgPool, id: Uuid) -> Result<Note, AppError> {
    sqlx::query_as::<_, Note>("SELECT * FROM notes WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Note not found".into()))
}

pub async fn update(pool: &PgPool, id: Uuid, input: &UpdateNoteRequest) -> Result<Note, AppError> {
    sqlx::query_as::<_, Note>(
        r#"
        UPDATE notes SET
            folder_id = COALESCE($2, folder_id),
            title = COALESCE($3, title),
            slug = COALESCE($4, slug),
            sort_order = COALESCE($5, sort_order),
            word_count = COALESCE($6, word_count),
            body_url = COALESCE($7, body_url),
            body_s3_key = COALESCE($8, body_s3_key),
            blog_type = COALESCE($9, blog_type),
            excerpt = COALESCE($10, excerpt),
            hero_image_url = COALESCE($11, hero_image_url),
            read_time_minutes = COALESCE($12, read_time_minutes),
            author_id = COALESCE($13, author_id),
            author_name = COALESCE($14, author_name),
            author_avatar_url = COALESCE($15, author_avatar_url),
            sections = COALESCE($16, sections),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(input.folder_id)
    .bind(&input.title)
    .bind(&input.slug)
    .bind(input.sort_order)
    .bind(input.word_count)
    .bind(&input.body_url)
    .bind(&input.body_s3_key)
    .bind(&input.blog_type)
    .bind(&input.excerpt)
    .bind(&input.hero_image_url)
    .bind(input.read_time_minutes)
    .bind(input.author_id)
    .bind(&input.author_name)
    .bind(&input.author_avatar_url)
    .bind(&input.sections)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Note not found".into()))
}

/// Transition a note's lifecycle status. Moving to `published` stamps
/// `published_at = NOW()`; moving back to `draft` clears it.
pub async fn transition_status(
    pool: &PgPool,
    id: Uuid,
    input: &TransitionNoteRequest,
) -> Result<Note, AppError> {
    if input.status != "draft" && input.status != "published" {
        return Err(AppError::BadRequest(format!(
            "Invalid note status: '{}'. Must be draft or published",
            input.status
        )));
    }

    sqlx::query_as::<_, Note>(
        r#"
        UPDATE notes SET
            status = $2,
            published_at = CASE
                WHEN $2 = 'published' THEN NOW()
                ELSE NULL
            END,
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&input.status)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Note not found".into()))
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM notes WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Note not found".into()));
    }

    Ok(())
}

// ============================================================================
// Note folders
// ============================================================================

pub async fn create_folder(
    pool: &PgPool,
    project_id: Uuid,
    created_by: Uuid,
    input: &CreateNoteFolderRequest,
) -> Result<NoteFolder, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Note folder name must not be empty".into(),
        ));
    }

    let folder = sqlx::query_as::<_, NoteFolder>(
        r#"
        INSERT INTO notes_folders (project_id, org_id, parent_id, name, sort_order, created_by)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(project_id)
    .bind(input.org_id)
    .bind(input.parent_id)
    .bind(input.name.trim())
    .bind(input.sort_order)
    .bind(created_by)
    .fetch_one(pool)
    .await?;

    Ok(folder)
}

pub async fn list_folders_by_project(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Vec<NoteFolder>, AppError> {
    let folders = sqlx::query_as::<_, NoteFolder>(
        "SELECT * FROM notes_folders WHERE project_id = $1 ORDER BY sort_order, name",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    Ok(folders)
}

pub async fn update_folder(
    pool: &PgPool,
    id: Uuid,
    input: &UpdateNoteFolderRequest,
) -> Result<NoteFolder, AppError> {
    sqlx::query_as::<_, NoteFolder>(
        r#"
        UPDATE notes_folders SET
            parent_id = COALESCE($2, parent_id),
            name = COALESCE($3, name),
            sort_order = COALESCE($4, sort_order),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(input.parent_id)
    .bind(&input.name)
    .bind(input.sort_order)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Note folder not found".into()))
}

pub async fn delete_folder(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM notes_folders WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Note folder not found".into()));
    }

    Ok(())
}

// ============================================================================
// Note comments
// ============================================================================

pub async fn create_comment(
    pool: &PgPool,
    note_id: Uuid,
    input: &CreateNoteCommentRequest,
) -> Result<NoteComment, AppError> {
    if input.body.trim().is_empty() {
        return Err(AppError::BadRequest(
            "Note comment body must not be empty".into(),
        ));
    }

    let comment = sqlx::query_as::<_, NoteComment>(
        r#"
        INSERT INTO note_comments (note_id, author_id, author_name, body)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(note_id)
    .bind(input.author_id)
    .bind(&input.author_name)
    .bind(&input.body)
    .fetch_one(pool)
    .await?;

    Ok(comment)
}

pub async fn list_comments_by_note(
    pool: &PgPool,
    note_id: Uuid,
) -> Result<Vec<NoteComment>, AppError> {
    let comments = sqlx::query_as::<_, NoteComment>(
        "SELECT * FROM note_comments WHERE note_id = $1 ORDER BY created_at ASC",
    )
    .bind(note_id)
    .fetch_all(pool)
    .await?;

    Ok(comments)
}

pub async fn delete_comment(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM note_comments WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Note comment not found".into()));
    }

    Ok(())
}
