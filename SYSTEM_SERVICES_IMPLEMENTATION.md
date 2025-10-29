# System Services Implementation Summary

## Overview

This document summarizes the implementation of system services support in igra-cli, allowing management of native systemd services alongside Docker containers.

## Completed Backend Implementation

### 1. Core System Service Module (`src/core/system_service.rs`)

**Created complete system service management functionality:**

- `SystemServiceInfo` structure with all necessary fields:
  - Service identification (name, display_name, type)
  - Status tracking (status, enabled, loaded, active, sub_state)
  - Resource metrics (pid, memory, cpu, uptime)
  - Configuration (auto_restart, config_files, log_paths, dependencies)
  - Project grouping (project_name: "System Services")

- `SystemServiceManager` with full CRUD operations:
  - `list_services()` - List all systemd services
  - `get_service_details()` - Get detailed service information
  - `start_service()`, `stop_service()`, `restart_service()` - Service control
  - `enable_service()`, `disable_service()` - Startup configuration
  - `get_logs()` - Retrieve journalctl logs
  - `get_metrics()` - Collect service metrics
  - `filter_relevant_services()` - Exclude system/core services

### 2. Service Categories Module (`src/core/service_categories.rs`)

**Flexible category management system:**

- `ServiceCategory` structure:
  - Category metadata (id, name, icon, color, order)
  - Service associations
  - Active state for filtering
  - Default vs custom categories

- `TrackedService` for service tracking:
  - Category assignment
  - Display name customization
  - Metrics enablement
  - Plugin association

- `CategoryManager` with full CRUD:
  - Default categories: Node Services, Web Services, Databases, Game Servers
  - Add/update/delete custom categories
  - Service-to-category mapping
  - Tracked services management
  - Configuration persistence to `~/.config/igra-cli/service_categories.json`
  - Category reordering
  - Import/export functionality

### 3. API Handlers (`src/server/system_service_handlers.rs`)

**Complete REST API implementation:**

**Service Operations:**
- `GET /api/system-services` - List all tracked services
- `GET /api/system-services/:name/details` - Get service details
- `GET /api/system-services/:name/logs` - Get service logs
- `POST /api/system-services/:name/start` - Start service
- `POST /api/system-services/:name/stop` - Stop service
- `POST /api/system-services/:name/restart` - Restart service
- `POST /api/system-services/:name/enable` - Enable on boot
- `POST /api/system-services/:name/disable` - Disable on boot

**Category Management:**
- `GET /api/categories` - List all categories
- `GET /api/categories/:id` - Get specific category
- `POST /api/categories` - Create new category
- `PUT /api/categories/:id` - Update category
- `DELETE /api/categories/:id` - Delete category
- `POST /api/categories/:id/services` - Add service to category

**Tracked Services:**
- `GET /api/tracked-services` - Get all tracked services
- `PUT /api/tracked-services/:name` - Update tracked service
- `DELETE /api/tracked-services/:name` - Remove tracked service

### 4. Routing Integration (`src/server/routes.rs`)

**Integrated system service routes:**
- Initialized shared state managers (SystemServiceManager, CategoryManager)
- Added routes to protected and public route groups
- Proper state management with Arc<RwLock<>>
- Authentication middleware applied where needed

### 5. Metrics Plugin System Extension (`src/core/metrics/plugin.rs`, `registry.rs`)

**Extended plugin system for system services:**
- Added `MatchType::ServiceNameEquals` and `MatchType::ServiceNameContains`
- Added `FetcherType::Systemd` and `FetcherType::SystemLogs`
- Structure in place for future TOML plugin configurations

### 6. Build System

**Successfully compiles:**
- ✅ Backend builds successfully with `--features server`
- ✅ No compilation errors
- ⚠️  Only warnings (unused imports, style issues)

## Remaining Frontend Work

### 1. Update ServicesPanel Component

**Required changes to `igra-web-ui/src/components/ServicesPanel.jsx`:**

- Add system services data fetching:
  ```javascript
  const [systemServices, setSystemServices] = useState([])
  const [categories, setCategories] = useState([])
  ```

- Fetch from new endpoints:
  ```javascript
  const systemServicesData = await api.get('/system-services')
  const categoriesData = await api.get('/categories')
  ```

- Merge Docker and system services into unified display
- Group by project name ("System Services" vs Docker project names)
- Apply existing filtering logic to both service types
- Extend status filters to include systemd states (activating, deactivating, etc.)

### 2. Enhanced Filter Modal

**Create new filter types:**
- Service type filter: Docker / System / Both
- Category filter (similar to profile filter)
- Combined status filter supporting both Docker and systemd states
- Maintain existing include/exclude modes

### 3. Service Details Modal

**Create `SystemServiceDetailsModal.jsx`:**
- Similar structure to existing Docker container details
- Show systemd-specific information:
  - Service unit file path
  - Dependencies and dependents
  - Journal logs integration
  - Enable/disable controls
  - Auto-restart configuration
- Metrics display (if plugin available)
- Start/stop/restart controls

### 4. Category Manager Component

**Create `CategoryManager.jsx`:**
- List all categories with drag-to-reorder
- Add/edit/delete custom categories
- Icon and color picker
- Service assignment interface
- Mark services as tracked/untracked
- Import/export category configuration

### 5. Settings Page Integration

**Add to Settings:**
- System services configuration section
- Category management link
- Tracked services overview
- Metrics plugin configuration for system services

## API Endpoint Summary

### System Services
```
GET    /api/system-services              - List all tracked services
GET    /api/system-services/:name/details - Service details
GET    /api/system-services/:name/logs   - Service logs (?lines=N)
POST   /api/system-services/:name/start  - Start service (auth required)
POST   /api/system-services/:name/stop   - Stop service (auth required)
POST   /api/system-services/:name/restart - Restart service (auth required)
POST   /api/system-services/:name/enable - Enable on boot (auth required)
POST   /api/system-services/:name/disable - Disable on boot (auth required)
```

### Categories
```
GET    /api/categories                   - List all categories
GET    /api/categories/:id               - Get specific category
POST   /api/categories                   - Create category (auth required)
PUT    /api/categories/:id               - Update category (auth required)
DELETE /api/categories/:id               - Delete category (auth required)
POST   /api/categories/:id/services      - Add service to category (auth required)
```

### Tracked Services
```
GET    /api/tracked-services             - Get all tracked services
PUT    /api/tracked-services/:name       - Update tracked service (auth required)
DELETE /api/tracked-services/:name       - Remove tracked service (auth required)
```

## Data Structures

### SystemServiceInfo
```json
{
  "name": "nginx.service",
  "display_name": "nginx",
  "service_type": "systemd",
  "status": "running",
  "category": "web-services",
  "description": "A high performance web server",
  "pid": 1234,
  "memory": 52428800,
  "cpu": 2.5,
  "uptime": {"secs": 86400, "nanos": 0},
  "auto_restart": true,
  "enabled": true,
  "config_files": ["/etc/nginx/nginx.conf"],
  "log_paths": [],
  "dependencies": ["network.target"],
  "project_name": "System Services",
  "loaded": true,
  "active": true,
  "sub_state": "running"
}
```

### ServiceCategory
```json
{
  "id": "web-services",
  "name": "Web Services",
  "icon": "🌐",
  "color": "#10b981",
  "services": ["nginx.service", "apache2.service"],
  "order": 2,
  "is_default": true,
  "is_active": true
}
```

### TrackedService
```json
{
  "category": "web-services",
  "display_name": "Nginx Web Server",
  "metrics_enabled": true,
  "plugin": "nginx-metrics"
}
```

## Configuration Files

### Category Configuration
Location: `~/.config/igra-cli/service_categories.json`

```json
{
  "categories": [
    {
      "id": "node-services",
      "name": "Node Services",
      "icon": "🔗",
      "color": "#6366f1",
      "services": ["kaspa-mainnet.service"],
      "order": 1,
      "is_default": true,
      "is_active": true
    }
  ],
  "tracked_services": {
    "kaspa-mainnet.service": {
      "category": "node-services",
      "display_name": "Kaspa Mainnet Node",
      "metrics_enabled": true,
      "plugin": null
    }
  }
}
```

### Metrics Plugin (Future)
Example plugin for nginx in `plugins/nginx-system.toml`:

```toml
[plugin]
name = "nginx-system"
description = "Nginx system service metrics"

[[match]]
type = "service_name_equals"
value = "nginx.service"

[fetcher]
type = "systemd"

[[metrics]]
name = "Active Connections"
prometheus_metric = "nginx_active_connections"
display_format = "{} connections"
display_priority = "primary"
refresh_interval_secs = 5
```

## Testing Checklist

### Backend
- [x] Backend compiles successfully
- [ ] API endpoints respond correctly
- [ ] Service listing filters out system services
- [ ] Service operations (start/stop/restart) work with sudo
- [ ] Category CRUD operations persist to config file
- [ ] Tracked services management works correctly

### Frontend
- [ ] System services display in ServicesPanel
- [ ] Filtering works for both Docker and system services
- [ ] Service type filter (Docker/System/Both) functions
- [ ] Category filter functions
- [ ] Service details modal shows systemd information
- [ ] Category manager UI works
- [ ] Settings page integration complete
- [ ] Real-time updates work for system services

## Future Enhancements

1. **Metrics Implementation:**
   - Implement Systemd and SystemLogs fetchers
   - Create default plugins for common services (nginx, postgresql, etc.)
   - Add CPU/memory graphing from systemd data

2. **Advanced Features:**
   - Service dependency visualization
   - Bulk operations (start/stop multiple services)
   - Service templates and instances support
   - Timer units support
   - Socket activation monitoring

3. **Configuration:**
   - Service-specific environment variables
   - Resource limits configuration
   - Restart policy management
   - Drop-in file editor

4. **Monitoring:**
   - Service failure notifications
   - Historical uptime tracking
   - Performance trends
   - Alert rules for service states

## Architecture Notes

- System services managed through SystemServiceManager (no Docker dependency)
- Categories provide flexible organization separate from systemd structure
- Tracked services allow selective monitoring (avoid overwhelming UI with all system services)
- Plugin system extensible for custom metrics without code changes
- Frontend and backend completely independent (REST API communication)
- State management through Arc<RwLock<>> for thread-safe async access
