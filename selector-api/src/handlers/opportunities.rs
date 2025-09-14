use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::models::{
    CreateOpportunityRequest, Opportunity, OpportunityResponse, OpportunityStatus,
    PaginatedResponse, PaginationQuery,
};
use crate::services::opportunity_service;
use crate::state::AppState;

pub async fn list_opportunities(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<PaginatedResponse<Opportunity>>, StatusCode> {
    let page = pagination.page.unwrap_or(1);
    let per_page = pagination.per_page.unwrap_or(20).min(state.config.max_opportunities_per_page);

    match opportunity_service::list_opportunities(&state, page, per_page).await {
        Ok(response) => Ok(Json(response)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_opportunity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<OpportunityResponse>, StatusCode> {
    match opportunity_service::get_opportunity(&state, id).await {
        Ok(Some(response)) => Ok(Json(response)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_opportunity(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateOpportunityRequest>,
) -> Result<(StatusCode, Json<Opportunity>), StatusCode> {
    // Validate request
    if let Err(_) = request.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match opportunity_service::create_opportunity(&state, request).await {
        Ok(opportunity) => Ok((StatusCode::CREATED, Json(opportunity))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_opportunity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(status): Json<OpportunityStatus>,
) -> Result<Json<Opportunity>, StatusCode> {
    match opportunity_service::update_opportunity_status(&state, id, status).await {
        Ok(Some(opportunity)) => Ok(Json(opportunity)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}



