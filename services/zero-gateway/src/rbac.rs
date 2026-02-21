//! RBAC middleware for Zero Gateway.
//!
//! Provides resource-level access control middleware for Axum.

use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zero_common::security::{
    resource_rbac::{Action, ResourceRoleStore, ResourceType},
};

use crate::auth::AuthUser;

/// RBAC state for middleware.
#[derive(Clone)]
pub struct RbacState {
    /// Role store with predefined and custom roles
    pub store: Arc<ResourceRoleStore>,
}

impl RbacState {
    /// Create a new RBAC state.
    pub fn new() -> Self {
        Self {
            store: Arc::new(ResourceRoleStore::new()),
        }
    }

    /// Create with a custom role store.
    pub fn with_store(store: Arc<ResourceRoleStore>) -> Self {
        Self { store }
    }
}

impl Default for RbacState {
    fn default() -> Self {
        Self::new()
    }
}

/// RBAC error response.
#[derive(Debug, Serialize, Deserialize)]
pub struct RbacError {
    pub error: String,
    pub code: String,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub action: Option<String>,
}

impl IntoResponse for RbacError {
    fn into_response(self) -> Response {
        (StatusCode::FORBIDDEN, Json(self)).into_response()
    }
}

/// Resource access request for middleware validation.
#[derive(Debug, Clone)]
pub struct ResourceAccess {
    pub resource_type: ResourceType,
    pub resource_id: String,
    pub action: Action,
}

impl ResourceAccess {
    pub fn new(resource_type: ResourceType, resource_id: &str, action: Action) -> Self {
        Self {
            resource_type,
            resource_id: resource_id.to_string(),
            action,
        }
    }
}

/// Generic RBAC middleware that checks resource access.
///
/// Usage:
/// ```ignore
/// .layer(middleware::from_fn_with_state(
///     rbac_state.clone(),
///     |state, req, next| rbac_middleware(state, req, next, ResourceType::Agent, Action::Execute)
/// ))
/// ```
pub async fn rbac_check_middleware(
    State(state): State<RbacState>,
    resource_type: ResourceType,
    resource_id: String,
    action: Action,
    request: Request,
    next: Next,
) -> Result<Response, RbacError> {
    // Get authenticated user from request extensions
    let auth_user = request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| RbacError {
            error: "Authentication required".into(),
            code: "AUTH_REQUIRED".into(),
            resource_type: None,
            resource_id: None,
            action: None,
        })?
        .clone();

    // Check access
    let allowed = state
        .store
        .can_access(&auth_user.roles, &resource_type, &resource_id, &action);

    if !allowed {
        return Err(RbacError {
            error: format!(
                "Access denied to {}:{} for action {:?}",
                resource_type.as_str(),
                resource_id,
                action
            ),
            code: "FORBIDDEN".into(),
            resource_type: Some(resource_type.as_str().to_string()),
            resource_id: Some(resource_id),
            action: Some(format!("{:?}", action).to_lowercase()),
        });
    }

    Ok(next.run(request).await)
}

/// RBAC middleware for agent routes.
/// Extracts agent name from path and checks Execute permission.
pub async fn agent_rbac_middleware(
    State(state): State<RbacState>,
    Path(agent_name): Path<String>,
    request: Request,
    next: Next,
) -> Result<Response, RbacError> {
    let auth_user = request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| RbacError {
            error: "Authentication required".into(),
            code: "AUTH_REQUIRED".into(),
            resource_type: None,
            resource_id: None,
            action: None,
        })?
        .clone();

    let allowed = state
        .store
        .can_access(&auth_user.roles, &ResourceType::Agent, &agent_name, &Action::Execute);

    if !allowed {
        return Err(RbacError {
            error: format!("Access denied to agent: {}", agent_name),
            code: "FORBIDDEN".into(),
            resource_type: Some("agent".into()),
            resource_id: Some(agent_name),
            action: Some("execute".into()),
        });
    }

    Ok(next.run(request).await)
}

/// RBAC middleware for workflow routes.
pub async fn workflow_rbac_middleware(
    State(state): State<RbacState>,
    Path(workflow_name): Path<String>,
    request: Request,
    next: Next,
) -> Result<Response, RbacError> {
    let auth_user = request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| RbacError {
            error: "Authentication required".into(),
            code: "AUTH_REQUIRED".into(),
            resource_type: None,
            resource_id: None,
            action: None,
        })?
        .clone();

    // Determine action based on HTTP method
    let action = match request.method().as_str() {
        "GET" | "HEAD" => Action::Read,
        "POST" | "PUT" | "PATCH" => Action::Write,
        "DELETE" => Action::Delete,
        _ => Action::Read,
    };

    let allowed = state
        .store
        .can_access(&auth_user.roles, &ResourceType::Workflow, &workflow_name, &action);

    if !allowed {
        return Err(RbacError {
            error: format!("Access denied to workflow: {}", workflow_name),
            code: "FORBIDDEN".into(),
            resource_type: Some("workflow".into()),
            resource_id: Some(workflow_name),
            action: Some(format!("{:?}", action).to_lowercase()),
        });
    }

    Ok(next.run(request).await)
}

/// RBAC middleware for skill routes.
pub async fn skill_rbac_middleware(
    State(state): State<RbacState>,
    Path(skill_name): Path<String>,
    request: Request,
    next: Next,
) -> Result<Response, RbacError> {
    let auth_user = request
        .extensions()
        .get::<AuthUser>()
        .ok_or_else(|| RbacError {
            error: "Authentication required".into(),
            code: "AUTH_REQUIRED".into(),
            resource_type: None,
            resource_id: None,
            action: None,
        })?
        .clone();

    let action = match request.method().as_str() {
        "GET" | "HEAD" => Action::Read,
        "POST" => Action::Execute,
        "PUT" | "PATCH" => Action::Write,
        "DELETE" => Action::Delete,
        _ => Action::Read,
    };

    let allowed = state
        .store
        .can_access(&auth_user.roles, &ResourceType::Skill, &skill_name, &action);

    if !allowed {
        return Err(RbacError {
            error: format!("Access denied to skill: {}", skill_name),
            code: "FORBIDDEN".into(),
            resource_type: Some("skill".into()),
            resource_id: Some(skill_name),
            action: Some(format!("{:?}", action).to_lowercase()),
        });
    }

    Ok(next.run(request).await)
}

/// Helper to create RBAC check for specific resource and action.
pub fn require_access(
    state: &RbacState,
    roles: &[String],
    resource_type: ResourceType,
    resource_id: &str,
    action: Action,
) -> Result<(), RbacError> {
    let allowed = state
        .store
        .can_access(roles, &resource_type, resource_id, &action);

    if allowed {
        Ok(())
    } else {
        Err(RbacError {
            error: format!(
                "Access denied to {}:{} for action {:?}",
                resource_type.as_str(),
                resource_id,
                action
            ),
            code: "FORBIDDEN".into(),
            resource_type: Some(resource_type.as_str().to_string()),
            resource_id: Some(resource_id.to_string()),
            action: Some(format!("{:?}", action).to_lowercase()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_state_creation() {
        let state = RbacState::new();

        // Developer role should have agent execute access
        let roles = vec!["developer".to_string()];
        assert!(state.store.can_access(&roles, &ResourceType::Agent, "any-agent", &Action::Execute));

        // Viewer should not have execute access
        let viewer_roles = vec!["viewer".to_string()];
        assert!(!state.store.can_access(&viewer_roles, &ResourceType::Agent, "any-agent", &Action::Execute));
    }

    #[test]
    fn test_require_access() {
        let state = RbacState::new();
        let roles = vec!["developer".to_string()];

        assert!(require_access(&state, &roles, ResourceType::Agent, "test", Action::Execute).is_ok());
        assert!(require_access(&state, &roles, ResourceType::Agent, "test", Action::Delete).is_err());
    }
}
