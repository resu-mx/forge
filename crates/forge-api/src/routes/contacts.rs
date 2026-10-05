//! Contact CRUD routes.
//!
//! Mirrors the TS contact routes — same URL paths,
//! same JSON shapes, so the webui and MCP server continue working.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use forge_core::{
    Contact, ContactFilter, ContactJDRelationship, ContactOrgRelationship,
    ContactResumeRelationship, ContactWithOrg, CreateContact, ForgeError, UpdateContact,
};
use forge_sdk::db::ContactStore;

use crate::db::with_conn;
use crate::error::ApiError;
use crate::response::{ApiData, ApiList, Created, NoContent};
use crate::state::SharedState;

// ── Query params ────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Default)]
pub struct ContactListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
    pub organization_id: Option<String>,
}

// ── Handlers ────────────────────────────────────────────────────────

async fn create_contact(
    State(state): State<SharedState>,
    Json(input): Json<CreateContact>,
) -> Result<Created<Contact>, ApiError> {
    let result = with_conn(&state, move |conn| ContactStore::create(conn, &input)).await?;
    Ok(Created(result))
}

async fn list_contacts(
    State(state): State<SharedState>,
    Query(q): Query<ContactListQuery>,
) -> Result<Json<ApiList<ContactWithOrg>>, ApiError> {
    let filter = ContactFilter {
        organization_id: q.organization_id,
        search: q.search,
    };
    let offset = q.offset.unwrap_or(0).max(0);
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let (data, pagination) = with_conn(&state, move |conn| {
        ContactStore::list(conn, &filter, offset, limit)
    })
    .await?;

    Ok(Json(ApiList { data, pagination }))
}

async fn get_contact(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<ContactWithOrg>>, ApiError> {
    let result = with_conn(&state, move |conn| {
        ContactStore::get_with_org(conn, &id)?.ok_or_else(|| forge_core::ForgeError::NotFound {
            entity_type: "Contact".into(),
            id: id.clone(),
        })
    })
    .await?;
    Ok(Json(ApiData { data: result }))
}

async fn update_contact(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(input): Json<UpdateContact>,
) -> Result<Json<ApiData<Contact>>, ApiError> {
    let result = with_conn(&state, move |conn| ContactStore::update(conn, &id, &input)).await?;
    Ok(Json(ApiData { data: result }))
}

async fn delete_contact(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<NoContent, ApiError> {
    with_conn(&state, move |conn| ContactStore::delete(conn, &id)).await?;
    Ok(NoContent)
}

// ── Relationship links ──────────────────────────────────────────────
//
// Mirrors packages/core/src/routes/contacts.ts:65-159 and contact-service.ts.

// TS: contact-service.ts:41-62.
const ORG_RELATIONSHIPS: &[&str] = &["recruiter", "hr", "referral", "peer", "manager", "other"];
const JD_RELATIONSHIPS: &[&str] = &[
    "hiring_manager",
    "recruiter",
    "interviewer",
    "referral",
    "other",
];
const RESUME_RELATIONSHIPS: &[&str] = &["reference", "recommender", "other"];

/// Parse a relationship as TS validates it: anything outside `valid` is 400 VALIDATION_ERROR,
/// and a missing value reads as `undefined`, as TS's template string prints it.
fn parse_relationship<R: std::str::FromStr>(
    raw: Option<&str>,
    valid: &[&str],
) -> Result<R, ForgeError> {
    let raw = raw.unwrap_or("undefined");
    raw.parse::<R>().map_err(|_| ForgeError::Validation {
        message: format!(
            "Invalid relationship: {raw}. Must be one of: {}",
            valid.join(", ")
        ),
        field: Some("relationship".into()),
    })
}

/// TS rejects a missing target id through the ELM's required-field check (400).
fn required_id(value: Option<String>, field: &str) -> Result<String, ForgeError> {
    value
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| ForgeError::Validation {
            message: format!("{field} is required"),
            field: Some(field.into()),
        })
}

#[derive(Debug, Deserialize)]
pub struct LinkOrganizationBody {
    pub organization_id: Option<String>,
    pub relationship: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinkJobDescriptionBody {
    pub job_description_id: Option<String>,
    pub relationship: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinkResumeBody {
    pub resume_id: Option<String>,
    pub relationship: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LinkedOrganization {
    pub id: String,
    pub name: String,
    pub relationship: ContactOrgRelationship,
}

#[derive(Debug, Serialize)]
pub struct LinkedJobDescription {
    pub id: String,
    pub title: String,
    pub organization_name: Option<String>,
    pub relationship: ContactJDRelationship,
}

#[derive(Debug, Serialize)]
pub struct LinkedResume {
    pub id: String,
    pub name: String,
    pub relationship: ContactResumeRelationship,
}

async fn list_contact_organizations(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<LinkedOrganization>>>, ApiError> {
    let rows = with_conn(&state, move |conn| {
        ContactStore::list_organizations(conn, &id)
    })
    .await?;
    let data = rows
        .into_iter()
        .map(|(id, name, relationship)| LinkedOrganization {
            id,
            name,
            relationship,
        })
        .collect();
    Ok(Json(ApiData { data }))
}

/// 201 with an empty body, as TS answers (contacts.ts:83).
async fn link_contact_organization(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<LinkOrganizationBody>,
) -> Result<StatusCode, ApiError> {
    let relationship: ContactOrgRelationship =
        parse_relationship(body.relationship.as_deref(), ORG_RELATIONSHIPS)?;
    let org_id = required_id(body.organization_id, "organization_id")?;
    with_conn(&state, move |conn| {
        ContactStore::link_organization(conn, &id, &org_id, relationship)
    })
    .await?;
    Ok(StatusCode::CREATED)
}

async fn unlink_contact_organization(
    State(state): State<SharedState>,
    Path((contact_id, org_id, relationship)): Path<(String, String, String)>,
) -> Result<NoContent, ApiError> {
    let relationship: ContactOrgRelationship =
        parse_relationship(Some(&relationship), ORG_RELATIONSHIPS)?;
    with_conn(&state, move |conn| {
        ContactStore::unlink_organization(conn, &contact_id, &org_id, relationship)
    })
    .await?;
    Ok(NoContent)
}

async fn list_contact_job_descriptions(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<LinkedJobDescription>>>, ApiError> {
    let rows = with_conn(&state, move |conn| {
        ContactStore::list_job_descriptions(conn, &id)
    })
    .await?;
    let data = rows
        .into_iter()
        .map(
            |(id, title, organization_name, relationship)| LinkedJobDescription {
                id,
                title,
                organization_name,
                relationship,
            },
        )
        .collect();
    Ok(Json(ApiData { data }))
}

/// 201 with an empty body, as TS answers (contacts.ts:115).
async fn link_contact_job_description(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<LinkJobDescriptionBody>,
) -> Result<StatusCode, ApiError> {
    let relationship: ContactJDRelationship =
        parse_relationship(body.relationship.as_deref(), JD_RELATIONSHIPS)?;
    let jd_id = required_id(body.job_description_id, "job_description_id")?;
    with_conn(&state, move |conn| {
        ContactStore::link_job_description(conn, &id, &jd_id, relationship)
    })
    .await?;
    Ok(StatusCode::CREATED)
}

async fn unlink_contact_job_description(
    State(state): State<SharedState>,
    Path((contact_id, jd_id, relationship)): Path<(String, String, String)>,
) -> Result<NoContent, ApiError> {
    let relationship: ContactJDRelationship =
        parse_relationship(Some(&relationship), JD_RELATIONSHIPS)?;
    with_conn(&state, move |conn| {
        ContactStore::unlink_job_description(conn, &contact_id, &jd_id, relationship)
    })
    .await?;
    Ok(NoContent)
}

async fn list_contact_resumes(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<ApiData<Vec<LinkedResume>>>, ApiError> {
    let rows = with_conn(&state, move |conn| ContactStore::list_resumes(conn, &id)).await?;
    let data = rows
        .into_iter()
        .map(|(id, name, relationship)| LinkedResume {
            id,
            name,
            relationship,
        })
        .collect();
    Ok(Json(ApiData { data }))
}

/// 201 with an empty body, as TS answers (contacts.ts:147).
async fn link_contact_resume(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<LinkResumeBody>,
) -> Result<StatusCode, ApiError> {
    let relationship: ContactResumeRelationship =
        parse_relationship(body.relationship.as_deref(), RESUME_RELATIONSHIPS)?;
    let resume_id = required_id(body.resume_id, "resume_id")?;
    with_conn(&state, move |conn| {
        ContactStore::link_resume(conn, &id, &resume_id, relationship)
    })
    .await?;
    Ok(StatusCode::CREATED)
}

async fn unlink_contact_resume(
    State(state): State<SharedState>,
    Path((contact_id, resume_id, relationship)): Path<(String, String, String)>,
) -> Result<NoContent, ApiError> {
    let relationship: ContactResumeRelationship =
        parse_relationship(Some(&relationship), RESUME_RELATIONSHIPS)?;
    with_conn(&state, move |conn| {
        ContactStore::unlink_resume(conn, &contact_id, &resume_id, relationship)
    })
    .await?;
    Ok(NoContent)
}

// ── Router ──────────────────────────────────────────────────────────

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/contacts", post(create_contact).get(list_contacts))
        .route(
            "/contacts/{id}",
            get(get_contact)
                .patch(update_contact)
                .delete(delete_contact),
        )
        .route(
            "/contacts/{id}/organizations",
            get(list_contact_organizations).post(link_contact_organization),
        )
        .route(
            "/contacts/{contact_id}/organizations/{org_id}/{relationship}",
            delete(unlink_contact_organization),
        )
        .route(
            "/contacts/{id}/job-descriptions",
            get(list_contact_job_descriptions).post(link_contact_job_description),
        )
        .route(
            "/contacts/{contact_id}/job-descriptions/{jd_id}/{relationship}",
            delete(unlink_contact_job_description),
        )
        .route(
            "/contacts/{id}/resumes",
            get(list_contact_resumes).post(link_contact_resume),
        )
        .route(
            "/contacts/{contact_id}/resumes/{resume_id}/{relationship}",
            delete(unlink_contact_resume),
        )
}
