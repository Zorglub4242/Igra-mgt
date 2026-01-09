/// System Service API Handlers
///
/// API endpoints for managing system services and categories
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::service_categories::TrackedService;
use crate::core::{
    log_parser, CategoryManager, ServiceCategory, SystemServiceInfo, SystemServiceManager,
};

pub type SharedSystemServiceManager = Arc<RwLock<SystemServiceManager>>;
pub type SharedCategoryManager = Arc<RwLock<CategoryManager>>;
pub type SystemServiceState = (SharedSystemServiceManager, SharedCategoryManager);

#[derive(Serialize)]
pub struct SystemServiceResponse {
    pub success: bool,
    pub data: Option<Vec<SystemServiceInfo>>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct AvailableServiceInfo {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub status: String,
    pub is_tracked: bool,
    pub category: Option<String>,
}

#[derive(Serialize)]
pub struct AvailableServicesResponse {
    pub success: bool,
    pub data: Option<Vec<AvailableServiceInfo>>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct ServiceDetailResponse {
    pub success: bool,
    pub data: Option<SystemServiceInfo>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct LogsResponse {
    pub success: bool,
    pub data: Option<String>,
    pub error: Option<String>,
    pub status_text: Option<String>,
    pub primary_metric: Option<String>,
    pub secondary_metric: Option<String>,
    pub is_healthy_metric: Option<bool>,
}

#[derive(Serialize)]
pub struct CategoryResponse {
    pub success: bool,
    pub data: Option<Vec<ServiceCategory>>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct SingleCategoryResponse {
    pub success: bool,
    pub data: Option<ServiceCategory>,
    pub error: Option<String>,
}

#[derive(Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_log_lines")]
    pub lines: usize,
}

fn default_log_lines() -> usize {
    100
}

#[derive(Deserialize)]
pub struct ServiceActionRequest {
    pub action: String, // "start", "stop", "restart", "enable", "disable"
}

#[derive(Deserialize)]
pub struct CategoryRequest {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub color: String,
    #[serde(default)]
    pub services: Vec<String>,
    #[serde(default)]
    pub order: i32,
    #[serde(default)]
    pub is_active: bool,
}

#[derive(Deserialize)]
pub struct AddServiceToCategoryRequest {
    pub service_name: String,
}

#[derive(Deserialize)]
pub struct TrackedServiceRequest {
    pub category: String,
    pub display_name: String,
    #[serde(default = "default_metrics_enabled")]
    pub metrics_enabled: bool,
    pub plugin: Option<String>,
}

fn default_metrics_enabled() -> bool {
    true
}

/// List all system services
pub async fn list_system_services(
    State((manager, cat_manager)): State<SystemServiceState>,
) -> Result<Json<SystemServiceResponse>, StatusCode> {
    let mgr = manager.read().await;
    let cat_mgr = cat_manager.read().await;

    match mgr.list_services().await {
        Ok(mut services) => {
            // Filter to relevant services only
            services = mgr.filter_relevant_services(services);

            // Load plugin registry to check for metric availability
            let registry =
                crate::core::metrics::registry::PluginRegistry::load_from_standard_locations()
                    .unwrap_or_else(|_| crate::core::metrics::registry::PluginRegistry::new());

            // Apply categories and check for metrics availability
            for service in &mut services {
                service.category = cat_mgr.get_service_category(&service.name);

                // Check if this service has a plugin (indicating metrics are available)
                let service_base_name = service.name.trim_end_matches(".service");
                if registry.find_service_plugin(service_base_name).is_some() {
                    service.has_metrics = true;
                }
            }

            // Filter to tracked services only
            services.retain(|s| cat_mgr.is_tracked(&s.name));

            Ok(Json(SystemServiceResponse {
                success: true,
                data: Some(services),
                error: None,
            }))
        }
        Err(e) => Ok(Json(SystemServiceResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// List all available system services (for management UI)
pub async fn list_available_services(
    State((manager, cat_manager)): State<SystemServiceState>,
) -> Result<Json<AvailableServicesResponse>, StatusCode> {
    let mgr = manager.read().await;
    let cat_mgr = cat_manager.read().await;

    match mgr.list_services().await {
        Ok(mut services) => {
            // Filter to relevant services only (exclude systemd-*, getty@, etc.)
            services = mgr.filter_relevant_services(services);

            // Transform to AvailableServiceInfo with tracking status
            let available: Vec<AvailableServiceInfo> = services
                .into_iter()
                .map(|s| {
                    let category = cat_mgr.get_service_category(&s.name);
                    AvailableServiceInfo {
                        name: s.name.clone(),
                        display_name: s.display_name,
                        description: Some(s.description),
                        status: format!("{:?}", s.status).to_lowercase(),
                        is_tracked: cat_mgr.is_tracked(&s.name),
                        category: if category.is_empty() {
                            None
                        } else {
                            Some(category)
                        },
                    }
                })
                .collect();

            Ok(Json(AvailableServicesResponse {
                success: true,
                data: Some(available),
                error: None,
            }))
        }
        Err(e) => Ok(Json(AvailableServicesResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Get service details
pub async fn get_service_details(
    Path(name): Path<String>,
    State((manager, cat_manager)): State<SystemServiceState>,
) -> Result<Json<ServiceDetailResponse>, StatusCode> {
    let mgr = manager.read().await;
    let cat_mgr = cat_manager.read().await;

    match mgr.get_service_details(&name).await {
        Ok(mut service) => {
            service.category = cat_mgr.get_service_category(&service.name);

            // Fetch metrics from plugin registry (similar to Docker services)
            let registry =
                crate::core::metrics::registry::PluginRegistry::load_from_standard_locations()
                    .unwrap_or_else(|_| crate::core::metrics::registry::PluginRegistry::new());
            let service_base_name = service.name.trim_end_matches(".service");

            // Find plugin that matches this system service
            if let Some(plugin) = registry.find_service_plugin(service_base_name) {
                // Fetch metrics using the new fetch_service_metrics method
                service.metrics = registry
                    .fetch_service_metrics(service_base_name, plugin)
                    .await
                    .unwrap_or_else(|e| {
                        eprintln!(
                            "[WARN] Failed to fetch metrics for {}: {}",
                            service_base_name, e
                        );
                        Vec::new()
                    });
            }

            Ok(Json(ServiceDetailResponse {
                success: true,
                data: Some(service),
                error: None,
            }))
        }
        Err(e) => Ok(Json(ServiceDetailResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Start a service
pub async fn start_service(
    Path(name): Path<String>,
    State((manager, _cat_manager)): State<SystemServiceState>,
) -> Result<Json<ServiceDetailResponse>, StatusCode> {
    let mgr = manager.read().await;

    match mgr.start_service(&name).await {
        Ok(_) => {
            // Get updated service details
            match mgr.get_service_details(&name).await {
                Ok(service) => Ok(Json(ServiceDetailResponse {
                    success: true,
                    data: Some(service),
                    error: None,
                })),
                Err(e) => Ok(Json(ServiceDetailResponse {
                    success: false,
                    data: None,
                    error: Some(e.to_string()),
                })),
            }
        }
        Err(e) => Ok(Json(ServiceDetailResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Stop a service
pub async fn stop_service(
    Path(name): Path<String>,
    State((manager, _cat_manager)): State<SystemServiceState>,
) -> Result<Json<ServiceDetailResponse>, StatusCode> {
    let mgr = manager.read().await;

    match mgr.stop_service(&name).await {
        Ok(_) => match mgr.get_service_details(&name).await {
            Ok(service) => Ok(Json(ServiceDetailResponse {
                success: true,
                data: Some(service),
                error: None,
            })),
            Err(e) => Ok(Json(ServiceDetailResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })),
        },
        Err(e) => Ok(Json(ServiceDetailResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Restart a service
pub async fn restart_service(
    Path(name): Path<String>,
    State((manager, _cat_manager)): State<SystemServiceState>,
) -> Result<Json<ServiceDetailResponse>, StatusCode> {
    let mgr = manager.read().await;

    match mgr.restart_service(&name).await {
        Ok(_) => match mgr.get_service_details(&name).await {
            Ok(service) => Ok(Json(ServiceDetailResponse {
                success: true,
                data: Some(service),
                error: None,
            })),
            Err(e) => Ok(Json(ServiceDetailResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })),
        },
        Err(e) => Ok(Json(ServiceDetailResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Enable a service
pub async fn enable_service(
    Path(name): Path<String>,
    State((manager, _cat_manager)): State<SystemServiceState>,
) -> Result<Json<ServiceDetailResponse>, StatusCode> {
    let mgr = manager.read().await;

    match mgr.enable_service(&name).await {
        Ok(_) => match mgr.get_service_details(&name).await {
            Ok(service) => Ok(Json(ServiceDetailResponse {
                success: true,
                data: Some(service),
                error: None,
            })),
            Err(e) => Ok(Json(ServiceDetailResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })),
        },
        Err(e) => Ok(Json(ServiceDetailResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Disable a service
pub async fn disable_service(
    Path(name): Path<String>,
    State((manager, _cat_manager)): State<SystemServiceState>,
) -> Result<Json<ServiceDetailResponse>, StatusCode> {
    let mgr = manager.read().await;

    match mgr.disable_service(&name).await {
        Ok(_) => match mgr.get_service_details(&name).await {
            Ok(service) => Ok(Json(ServiceDetailResponse {
                success: true,
                data: Some(service),
                error: None,
            })),
            Err(e) => Ok(Json(ServiceDetailResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
            })),
        },
        Err(e) => Ok(Json(ServiceDetailResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Get service logs
pub async fn get_service_logs(
    Path(name): Path<String>,
    Query(query): Query<LogsQuery>,
    State((manager, _cat_manager)): State<SystemServiceState>,
) -> Result<Json<LogsResponse>, StatusCode> {
    let mgr = manager.read().await;

    match mgr.get_logs(&name, query.lines).await {
        Ok(logs) => {
            // Parse logs using the appropriate parser based on service name
            // For kaspa-mainnet.service, kaspa-testnet-11.service, etc., use kaspad parser
            let service_base_name = if name.ends_with(".service") {
                &name[..name.len() - 8] // Remove ".service" suffix
            } else {
                &name
            };

            let parsed_metrics = log_parser::parse_service_logs(service_base_name, &logs);

            // Return logs with parsed metrics embedded (similar to Docker services)
            // For now, just return raw logs - metrics are shown separately in the UI
            Ok(Json(LogsResponse {
                success: true,
                data: Some(logs),
                error: None,
                status_text: parsed_metrics.status_text,
                primary_metric: parsed_metrics.primary_metric,
                secondary_metric: parsed_metrics.secondary_metric,
                is_healthy_metric: Some(parsed_metrics.is_healthy),
            }))
        }
        Err(e) => Ok(Json(LogsResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
            status_text: None,
            primary_metric: None,
            secondary_metric: None,
            is_healthy_metric: None,
        })),
    }
}

// Category Management Endpoints

/// List all categories
pub async fn list_categories(
    State((_manager, manager)): State<SystemServiceState>,
) -> Result<Json<CategoryResponse>, StatusCode> {
    let mgr = manager.read().await;
    let categories = mgr.get_categories();

    Ok(Json(CategoryResponse {
        success: true,
        data: Some(categories),
        error: None,
    }))
}

/// Get a specific category
pub async fn get_category(
    Path(id): Path<String>,
    State((_manager, manager)): State<SystemServiceState>,
) -> Result<Json<SingleCategoryResponse>, StatusCode> {
    let mgr = manager.read().await;

    match mgr.get_category(&id) {
        Some(category) => Ok(Json(SingleCategoryResponse {
            success: true,
            data: Some(category),
            error: None,
        })),
        None => Ok(Json(SingleCategoryResponse {
            success: false,
            data: None,
            error: Some(format!("Category '{}' not found", id)),
        })),
    }
}

/// Create a new category
pub async fn create_category(
    State((_manager, manager)): State<SystemServiceState>,
    Json(req): Json<CategoryRequest>,
) -> Result<Json<SingleCategoryResponse>, StatusCode> {
    let mut mgr = manager.write().await;

    let category = ServiceCategory {
        id: req.id,
        name: req.name,
        icon: req.icon,
        color: req.color,
        services: req.services,
        order: req.order,
        is_default: false,
        is_active: req.is_active,
    };

    match mgr.add_category(category.clone()) {
        Ok(_) => Ok(Json(SingleCategoryResponse {
            success: true,
            data: Some(category),
            error: None,
        })),
        Err(e) => Ok(Json(SingleCategoryResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Update a category
pub async fn update_category(
    Path(id): Path<String>,
    State((_manager, manager)): State<SystemServiceState>,
    Json(req): Json<CategoryRequest>,
) -> Result<Json<SingleCategoryResponse>, StatusCode> {
    let mut mgr = manager.write().await;

    let category = ServiceCategory {
        id: req.id,
        name: req.name,
        icon: req.icon,
        color: req.color,
        services: req.services,
        order: req.order,
        is_default: false,
        is_active: req.is_active,
    };

    match mgr.update_category(&id, category.clone()) {
        Ok(_) => Ok(Json(SingleCategoryResponse {
            success: true,
            data: Some(category),
            error: None,
        })),
        Err(e) => Ok(Json(SingleCategoryResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Delete a category
pub async fn delete_category(
    Path(id): Path<String>,
    State((_manager, manager)): State<SystemServiceState>,
) -> Result<Json<SingleCategoryResponse>, StatusCode> {
    let mut mgr = manager.write().await;

    match mgr.delete_category(&id) {
        Ok(_) => Ok(Json(SingleCategoryResponse {
            success: true,
            data: None,
            error: None,
        })),
        Err(e) => Ok(Json(SingleCategoryResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Add service to category
pub async fn add_service_to_category(
    Path(id): Path<String>,
    State((_manager, manager)): State<SystemServiceState>,
    Json(req): Json<AddServiceToCategoryRequest>,
) -> Result<Json<SingleCategoryResponse>, StatusCode> {
    let mut mgr = manager.write().await;

    match mgr.add_service_to_category(&id, req.service_name) {
        Ok(_) => {
            if let Some(category) = mgr.get_category(&id) {
                Ok(Json(SingleCategoryResponse {
                    success: true,
                    data: Some(category),
                    error: None,
                }))
            } else {
                Ok(Json(SingleCategoryResponse {
                    success: false,
                    data: None,
                    error: Some("Category not found after update".to_string()),
                }))
            }
        }
        Err(e) => Ok(Json(SingleCategoryResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Get tracked services
#[derive(Serialize)]
pub struct TrackedServicesResponse {
    pub success: bool,
    pub data: Option<HashMap<String, TrackedService>>,
    pub error: Option<String>,
}

pub async fn get_tracked_services(
    State((_manager, manager)): State<SystemServiceState>,
) -> Result<Json<TrackedServicesResponse>, StatusCode> {
    let mgr = manager.read().await;
    let tracked = mgr.get_tracked_services().clone();

    Ok(Json(TrackedServicesResponse {
        success: true,
        data: Some(tracked),
        error: None,
    }))
}

/// Update tracked service
pub async fn update_tracked_service(
    Path(name): Path<String>,
    State((_manager, manager)): State<SystemServiceState>,
    Json(req): Json<TrackedServiceRequest>,
) -> Result<Json<TrackedServicesResponse>, StatusCode> {
    let mut mgr = manager.write().await;

    let tracked = TrackedService {
        category: req.category,
        display_name: req.display_name,
        metrics_enabled: req.metrics_enabled,
        plugin: req.plugin,
    };

    match mgr.update_tracked_service(name, tracked) {
        Ok(_) => {
            let all_tracked = mgr.get_tracked_services().clone();
            Ok(Json(TrackedServicesResponse {
                success: true,
                data: Some(all_tracked),
                error: None,
            }))
        }
        Err(e) => Ok(Json(TrackedServicesResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}

/// Remove tracked service
pub async fn remove_tracked_service(
    Path(name): Path<String>,
    State((_manager, manager)): State<SystemServiceState>,
) -> Result<Json<TrackedServicesResponse>, StatusCode> {
    let mut mgr = manager.write().await;

    match mgr.remove_tracked_service(&name) {
        Ok(_) => {
            let all_tracked = mgr.get_tracked_services().clone();
            Ok(Json(TrackedServicesResponse {
                success: true,
                data: Some(all_tracked),
                error: None,
            }))
        }
        Err(e) => Ok(Json(TrackedServicesResponse {
            success: false,
            data: None,
            error: Some(e.to_string()),
        })),
    }
}
