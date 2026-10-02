//! Derivation routes — prepare/commit protocol for AI-driven bullet
//! and perspective derivation.
//!
//! Mirrors `packages/core/src/routes/derivations.ts`.
//!
//!   POST /derivations/prepare       — acquire lock, render prompt, return context
//!   POST /derivations/:id/commit    — validate LLM response, write entities, release lock

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use forge_core::{
    BulletStatus, CreatePendingDerivationInput, CreatePerspectiveInput,
    DerivePerspectiveInput, ForgeError, Framing, PerspectiveStatus, SourceStatus, now_iso,
};
use forge_sdk::db::{
    ArchetypeStore, BulletStore, DerivationStore, DomainStore, PerspectiveStore,
    SourceStore,
};

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::Created;
use crate::state::SharedState;

const LOCK_TIMEOUT_SECS: i64 = 120;

const BULLET_INSTRUCTIONS: &str = r#"Respond with a JSON object:
{
  "bullets": [
    {
      "content": "factual bullet text",
      "technologies": ["tech1", "tech2"],
      "metrics": "quantitative metric if present, null otherwise"
    }
  ]
}"#;

const PERSPECTIVE_INSTRUCTIONS: &str = r#"Respond with a JSON object:
{
  "content": "reframed perspective text",
  "reasoning": "brief explanation of framing choices"
}"#;

// ── Request/Response types ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct PrepareRequest {
    pub entity_type: String,
    pub entity_id: String,
    pub client_id: String,
    pub params: Option<DerivePerspectiveInput>,
}

#[derive(Debug, serde::Serialize)]
pub struct PrepareResponse {
    pub derivation_id: String,
    pub prompt: String,
    pub snapshot: String,
    pub instructions: String,
    pub expires_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum CommitRequest {
    Bullets { bullets: Vec<BulletCommitItem> },
    Perspective { content: String, reasoning: String },
}

#[derive(Debug, Deserialize, serde::Serialize)]
pub struct BulletCommitItem {
    pub content: String,
    pub technologies: Vec<String>,
    pub metrics: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn prepare(
    State(state): State<SharedState>,
    Json(req): Json<PrepareRequest>,
) -> Result<Created<PrepareResponse>, ApiError> {
    // Validate entity_type
    if req.entity_type != "source" && req.entity_type != "bullet" {
        return Err(ApiError(ForgeError::Validation {
            message: "entity_type must be \"source\" or \"bullet\"".into(),
            field: Some("entity_type".into()),
        }));
    }
    if req.entity_id.trim().is_empty() {
        return Err(ApiError(ForgeError::Validation {
            message: "entity_id is required".into(),
            field: Some("entity_id".into()),
        }));
    }
    if req.client_id.trim().is_empty() {
        return Err(ApiError(ForgeError::Validation {
            message: "client_id is required".into(),
            field: Some("client_id".into()),
        }));
    }

    let result = with_conn(&state, move |conn| {
        if req.entity_type == "source" {
            prepare_bullet_derivation(conn, &req.entity_id, &req.client_id)
        } else {
            let params = req.params.ok_or_else(|| ForgeError::Validation {
                message: "params (archetype, domain, framing) are required for entity_type \"bullet\"".into(),
                field: Some("params".into()),
            })?;
            prepare_perspective_derivation(conn, &req.entity_id, &params, &req.client_id)
        }
    })
    .await?;

    Ok(Created(result))
}

async fn commit(
    State(state): State<SharedState>,
    Path(derivation_id): Path<String>,
    Json(req): Json<CommitRequest>,
) -> Result<Created<serde_json::Value>, ApiError> {
    let result = with_conn(&state, move |conn| {
        let pending = DerivationStore::get(conn, &derivation_id)?
            .ok_or_else(|| ForgeError::NotFound {
                entity_type: "pending_derivation".into(),
                id: derivation_id.clone(),
            })?;

        // Check expiry
        if DerivationStore::check_and_cleanup_if_expired(conn, &pending)? {
            return Err(ForgeError::Gone {
                message: "Derivation has expired".into(),
            });
        }

        match req {
            CommitRequest::Bullets { bullets } => {
                if pending.entity_type != "source" {
                    return Err(ForgeError::Validation {
                        message: "Bullet commit requires entity_type \"source\"".into(),
                        field: None,
                    });
                }
                let created = commit_bullet_derivation(conn, &pending.id, &pending.entity_id, &pending.snapshot, &bullets)?;
                Ok(json!({ "bullets": created }))
            }
            CommitRequest::Perspective { content, reasoning } => {
                if pending.entity_type != "bullet" {
                    return Err(ForgeError::Validation {
                        message: "Perspective commit requires entity_type \"bullet\"".into(),
                        field: None,
                    });
                }
                let perspective = commit_perspective_derivation(
                    conn, &pending.id, &pending.entity_id, &pending.snapshot,
                    pending.derivation_params.as_deref(), &content, &reasoning,
                )?;
                Ok(json!({ "perspective": perspective }))
            }
        }
    })
    .await?;

    Ok(Created(result))
}

// ── Orchestration functions (run inside spawn_blocking) ─────────────

fn prepare_bullet_derivation(
    conn: &rusqlite::Connection,
    source_id: &str,
    client_id: &str,
) -> Result<PrepareResponse, ForgeError> {
    let source = SourceStore::get(conn, source_id)?
        .ok_or_else(|| ForgeError::NotFound {
            entity_type: "source".into(),
            id: source_id.into(),
        })?;

    if source.status == SourceStatus::Archived {
        return Err(ForgeError::Validation {
            message: "Cannot derive from an archived source".into(),
            field: None,
        });
    }

    if DerivationStore::has_active_lock(conn, "source", source_id)? {
        return Err(ForgeError::Conflict {
            message: format!("A derivation is already in progress for source {source_id}"),
        });
    }

    let snapshot = &source.description;
    let rendered = forge_ai::prompts::source_to_bullet::render(snapshot);
    let expires_at = compute_expires_at();

    let pending = DerivationStore::create(conn, &CreatePendingDerivationInput {
        entity_type: "source".into(),
        entity_id: source_id.into(),
        client_id: client_id.into(),
        prompt: format!("{}\n\n{}", rendered.system, rendered.user),
        snapshot: snapshot.clone(),
        derivation_params: None,
        expires_at: expires_at.clone(),
    })?;

    Ok(PrepareResponse {
        derivation_id: pending.id,
        prompt: format!("{}\n\n{}", rendered.system, rendered.user),
        snapshot: snapshot.clone(),
        instructions: BULLET_INSTRUCTIONS.into(),
        expires_at,
    })
}

fn prepare_perspective_derivation(
    conn: &rusqlite::Connection,
    bullet_id: &str,
    params: &DerivePerspectiveInput,
    client_id: &str,
) -> Result<PrepareResponse, ForgeError> {
    let bullet = BulletStore::get_hydrated(conn, bullet_id)?
        .ok_or_else(|| ForgeError::NotFound {
            entity_type: "bullet".into(),
            id: bullet_id.into(),
        })?;

    if bullet.status == BulletStatus::Archived {
        return Err(ForgeError::Validation {
            message: "Cannot derive from an archived bullet".into(),
            field: None,
        });
    }

    if bullet.status != BulletStatus::Approved {
        return Err(ForgeError::Validation {
            message: "Bullet must be approved before deriving perspectives".into(),
            field: None,
        });
    }

    // Validate archetype exists
    let (archetypes, _) = ArchetypeStore::list(conn, 0, 1000)?;
    let archetype_exists = archetypes.iter().any(|a| a.name.eq_ignore_ascii_case(&params.archetype));
    if !archetype_exists {
        return Err(ForgeError::NotFound {
            entity_type: "archetype".into(),
            id: params.archetype.clone(),
        });
    }

    // Validate domain exists
    let domains = DomainStore::list(conn)?;
    let domain_exists = domains.iter().any(|d| d.name.eq_ignore_ascii_case(&params.domain));
    if !domain_exists {
        return Err(ForgeError::NotFound {
            entity_type: "domain".into(),
            id: params.domain.clone(),
        });
    }

    if DerivationStore::has_active_lock(conn, "bullet", bullet_id)? {
        return Err(ForgeError::Conflict {
            message: format!("A derivation is already in progress for bullet {bullet_id}"),
        });
    }

    let snapshot = &bullet.content;
    let rendered = forge_ai::prompts::bullet_to_perspective::render(
        snapshot,
        &bullet.technologies,
        bullet.metrics.as_deref(),
        &params.archetype,
        &params.domain,
        params.framing.as_ref(),
    );
    let expires_at = compute_expires_at();
    let derivation_params = serde_json::to_string(params).ok();

    let pending = DerivationStore::create(conn, &CreatePendingDerivationInput {
        entity_type: "bullet".into(),
        entity_id: bullet_id.into(),
        client_id: client_id.into(),
        prompt: format!("{}\n\n{}", rendered.system, rendered.user),
        snapshot: snapshot.clone(),
        derivation_params,
        expires_at: expires_at.clone(),
    })?;

    Ok(PrepareResponse {
        derivation_id: pending.id,
        prompt: format!("{}\n\n{}", rendered.system, rendered.user),
        snapshot: snapshot.clone(),
        instructions: PERSPECTIVE_INSTRUCTIONS.into(),
        expires_at,
    })
}

fn commit_bullet_derivation(
    conn: &rusqlite::Connection,
    derivation_id: &str,
    source_id: &str,
    snapshot: &str,
    bullets: &[BulletCommitItem],
) -> Result<Vec<serde_json::Value>, ForgeError> {
    let mut created = Vec::new();

    for item in bullets {
        let bullet = BulletStore::create(
            conn,
            &item.content,
            Some(snapshot),
            item.metrics.as_deref(),
            None, // domain
            &[(source_id.into(), true)], // primary source
            &item.technologies,
        )?;

        // Set status to in_review
        BulletStore::transition_status(conn, &bullet.id, BulletStatus::InReview, None)?;

        // Create prompt log
        let log_id = DerivationStore::create_prompt_log(
            conn,
            "bullet",
            &bullet.id,
            forge_ai::prompts::source_to_bullet::TEMPLATE_VERSION,
            snapshot,
            &serde_json::to_string(&item).unwrap_or_default(),
        )?;

        // Update bullet's prompt_log_id
        conn.execute(
            "UPDATE bullets SET prompt_log_id = ?1 WHERE id = ?2",
            rusqlite::params![log_id, bullet.id],
        )?;

        // Re-fetch hydrated
        let hydrated = BulletStore::get_hydrated(conn, &bullet.id)?
            .ok_or_else(|| ForgeError::Internal("Bullet committed but not found".into()))?;

        created.push(serde_json::to_value(&hydrated).unwrap_or_default());
    }

    // Release lock
    DerivationStore::delete(conn, derivation_id)?;

    // Update source.last_derived_at
    let now = now_iso();
    conn.execute(
        "UPDATE sources SET last_derived_at = ?1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, source_id],
    )?;

    Ok(created)
}

fn commit_perspective_derivation(
    conn: &rusqlite::Connection,
    derivation_id: &str,
    bullet_id: &str,
    snapshot: &str,
    derivation_params: Option<&str>,
    content: &str,
    reasoning: &str,
) -> Result<serde_json::Value, ForgeError> {
    // Parse derivation params
    let params: DerivePerspectiveInput = derivation_params
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(DerivePerspectiveInput {
            archetype: String::new(),
            domain: String::new(),
            framing: Framing::Accomplishment,
        });

    let perspective = PerspectiveStore::create(conn, &CreatePerspectiveInput {
        bullet_id: bullet_id.into(),
        content: content.into(),
        bullet_content_snapshot: snapshot.into(),
        target_archetype: params.archetype,
        domain: params.domain,
        framing: params.framing,
        status: Some(PerspectiveStatus::InReview),
        prompt_log_id: None,
    })?;

    // Create prompt log
    let log_id = DerivationStore::create_prompt_log(
        conn,
        "perspective",
        &perspective.id,
        forge_ai::prompts::bullet_to_perspective::TEMPLATE_VERSION,
        snapshot,
        &json!({ "content": content, "reasoning": reasoning }).to_string(),
    )?;

    // Update perspective's prompt_log_id
    conn.execute(
        "UPDATE perspectives SET prompt_log_id = ?1 WHERE id = ?2",
        rusqlite::params![log_id, perspective.id],
    )?;

    // Release lock
    DerivationStore::delete(conn, derivation_id)?;

    // Re-fetch
    let result = PerspectiveStore::get(conn, &perspective.id)?
        .ok_or_else(|| ForgeError::Internal("Perspective committed but not found".into()))?;

    Ok(serde_json::to_value(&result).unwrap_or_default())
}

fn compute_expires_at() -> String {
    let expires = chrono::Utc::now() + chrono::Duration::seconds(LOCK_TIMEOUT_SECS);
    expires.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

// -- Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/derivations/prepare", post(prepare))
        .route("/derivations/{id}/commit", post(commit))
}
