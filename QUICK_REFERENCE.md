# Quick Reference - Service Details & Metrics Plugin System

## Using Service Details

### TUI (Terminal UI)

**Opening Service Details:**
1. Navigate to Services screen (press `1` from any screen)
2. Use `↑`/`↓` arrows to select a service
3. Press `Enter` to open service details

**Navigation in Service Details:**
- `Tab` - Next tab
- `Shift+Tab` - Previous tab
- `↑`/`↓` - Scroll content in current tab
- `r` - Refresh service details
- `Esc` or `q` - Return to services list

**Tabs Available:**
1. **Overview** - Status, resource usage, timestamps, description
2. **Metrics** - All plugin metrics with categories
3. **Configuration** - Environment variables, command, entrypoint
4. **Storage** - Volume mounts and bind mounts
5. **Network** - Networks, IP addresses, port mappings
6. **Logs** - Placeholder (use main Logs screen)

### Web UI

**Opening Service Details:**
1. Navigate to Services tab in web interface
2. Click on any service name (blue, underlined on hover)
3. Service details page opens with full information

**Navigation in Service Details:**
- Click tab names to switch between tabs
- Use "← Back to Services" button to return
- Click "🔄 Refresh" button to manually refresh
- Auto-refreshes every 5 seconds

**Tabs Available:**
1. **Overview** - Status badge, resource usage, description, basic info
2. **Metrics** - Grid view of all plugin metrics with categories
3. **Configuration** - Scrollable table of environment variables, command, entrypoint
4. **Storage** - Table of volume mounts (source → destination → mode)
5. **Network** - Networks section + port mappings table

---

## Metrics Plugin System

### Creating a Custom Plugin

Create a TOML file in `plugins/` directory:

```toml
[plugin]
name = "my-service"
description = "Description of your service"

# Match containers (at least one matcher required)
[[match]]
type = "image_contains"  # or "name_equals", "name_contains"
value = "my-service"

# Fetcher configuration
[fetcher]
type = "prometheus"      # or "http", "logs"
method = "docker_exec"   # optional, defaults to docker_exec
port = 9090
path = "/metrics"

# Define metrics
[[metrics]]
name = "requests_total"
prometheus_metric = "http_requests_total"
display_format = "{value} requests"
display_priority = "primary"    # "primary", "secondary", or "detail"
category = "performance"
refresh_interval_secs = 5       # How often to fetch (default: 5)
cache_duration_secs = 5         # How long to cache (default: same as refresh)
```

### Plugin is Loaded Automatically

The plugin system loads built-in plugins at startup. Custom plugins can be added to a `plugins/` directory.

---

## Service Details API

### Endpoints

**Get Full Service Details:**
```bash
curl http://localhost:3000/api/services/execution-layer/details
```

**Get Service Note:**
```bash
curl http://localhost:3000/api/services/execution-layer/note
```

**Update Service Note (requires auth):**
```bash
curl -X PUT http://localhost:3000/api/services/execution-layer/note \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"note": "Custom note for this service"}'
```

### Response Format

**Service Details Response:**
```json
{
  "name": "execution-layer",
  "image": "reth:latest",
  "status": "Up 2 hours",
  "state": "Running",
  "created": "2025-10-23T10:00:00Z",
  "started": "2025-10-23T10:01:00Z",
  "note": "Reth Ethereum execution client...",
  "metrics": [
    {
      "name": "block_height",
      "value": 123456.0,
      "formatted": "Block #123456",
      "category": "blockchain"
    }
  ],
  "env_vars": {
    "NETWORK": "testnet",
    "PASSWORD": "***HIDDEN***"
  },
  "labels": {...},
  "command": "/app/reth node",
  "entrypoint": null,
  "volumes": [...],
  "mounts": [...],
  "networks": [...],
  "ports": [...],
  "cpu_stats": {...},
  "memory_stats": {...},
  "block_io_stats": {...},
  "network_stats": {...}
}
```

---

## Service Notes

### Default Notes (Based on Image)

The system provides intelligent default notes for common services:

- **reth** → "Reth Ethereum execution client..."
- **geth** → "Geth (Go Ethereum) execution client..."
- **kaspad** → "Kaspa L1 node..."
- **viaduct** → "Viaduct L1→L2 bridge..."
- **block-builder** → "Block builder service..."
- **traefik** → "Traefik reverse proxy..."
- **rpc-provider** → "RPC provider worker..."
- **kaswallet** → "Kaspa wallet daemon..."

### Custom Notes

Custom notes are stored in `~/.config/igra-cli/service_notes.json`

To set a custom note via API:
```bash
curl -X PUT http://localhost:3000/api/services/my-service/note \
  -H "Authorization: Bearer TOKEN" \
  -d '{"note": "My custom description"}'
```

To reset to default, set an empty note:
```bash
curl -X PUT http://localhost:3000/api/services/my-service/note \
  -H "Authorization: Bearer TOKEN" \
  -d '{"note": ""}'
```

---

## Refresh Intervals

### Recommended Intervals by Metric Type

**Fast-Changing (2-5 seconds):**
- Block height
- Transaction count
- Active connections

**Medium-Changing (10-15 seconds):**
- Peer count
- Queue sizes
- Request rates

**Slow-Changing (30+ seconds):**
- Memory usage
- Disk usage
- Configuration values

### Example Configuration

```toml
# Block height changes every 1 second (Kaspa)
[[metrics]]
name = "block_height"
refresh_interval_secs = 2

# Peer count changes occasionally
[[metrics]]
name = "peer_count"
refresh_interval_secs = 10

# Memory usage is relatively stable
[[metrics]]
name = "memory_usage"
refresh_interval_secs = 30
```

---

## Display Priorities

### Primary Metric
Shown prominently in the services list (first metric displayed):
```toml
display_priority = "primary"
```

### Secondary Metric
Shown as additional info in services list:
```toml
display_priority = "secondary"
```

### Detail Only
Only shown in detail views, not in main services list:
```toml
display_priority = "detail"
```

---

## Built-in Plugins

### Reth Plugin (`plugins/reth.toml`)

**Metrics:**
- Block height (2s refresh)
- Peer count (10s)
- Sync status (10s)
- Transactions (5s)
- Memory usage (30s)
- And 30+ more...

### Geth Plugin (`plugins/geth.toml`)

**Metrics:**
- Block height (2s refresh)
- Peer count (10s)
- Gas processed (10s)
- Memory usage (30s)

### Kaspad Plugin (`plugins/kaspad.toml`)

**Metrics:**
- DAA score (2s refresh)
- Block count (5s)
- Virtual DAA score (5s)
- Connections (10s)

---

## File Locations

```
igra-cli/
├── plugins/                    # Plugin TOML files
│   ├── reth.toml              # Built-in reth plugin
│   ├── geth.toml              # Built-in geth plugin
│   └── kaspad.toml            # Built-in kaspad plugin
├── src/core/
│   ├── metrics/               # Metrics plugin system
│   │   ├── plugin.rs          # Plugin configuration
│   │   ├── registry.rs        # Plugin registry with caching
│   │   └── fetchers/          # Metric fetchers
│   ├── service_notes.rs       # Service notes storage
│   └── docker.rs              # ServiceDetails struct
├── src/server/
│   ├── handlers.rs            # API endpoints (3 new)
│   └── routes.rs              # Route registration
└── ~/.config/igra-cli/        # User config directory
    └── service_notes.json     # Custom service notes
```

---

## Troubleshooting

### Plugin Not Loading

Check plugin syntax:
```bash
cargo run -- # Will show plugin load errors at startup
```

### Metrics Not Updating

Check refresh intervals in plugin TOML - may be cached.

### Sensitive Data Showing

The following env var patterns are automatically hidden:
- `*PASSWORD*`
- `*SECRET*`
- `*KEY*`
- `*TOKEN*`
- `*API_KEY*`

### Service Notes Not Persisting

Check file permissions on `~/.config/igra-cli/service_notes.json`

---

## Performance Tips

1. **Use appropriate refresh intervals** - Don't poll slow-changing metrics every second
2. **Cache durations** - Set `cache_duration_secs` equal to or greater than `refresh_interval_secs`
3. **Limit detail metrics** - Use `display_priority = "detail"` for metrics not needed in main view
4. **Group by category** - Use consistent `category` values for organized display

---

## Migration from Old System

**No migration needed!** The new system is 100% backward compatible:

- Existing services continue to work
- Services list display is unchanged
- No breaking API changes

The new features are additive only.

---

## Screenshots & Usage Examples

### TUI Service Details Example

```
┌─ Service Details: execution-layer ─────────────────────────────┐
│                                                                 │
│ ┌─ Tabs ───────────────────────────────────────────────────┐  │
│ │  Overview  │  Metrics  │  Configuration  │  Storage  │...│  │
│ └────────────────────────────────────────────────────────────┘  │
│                                                                 │
│ Status: Up 2 hours (healthy)                                   │
│ State: Running                                                  │
│ Image: reth:latest                                             │
│ Created: 2025-10-25T10:00:00Z                                  │
│ Started: 2025-10-25T10:01:00Z                                  │
│                                                                 │
│ Description:                                                    │
│ Reth Ethereum execution client. Provides EVM compatibility...  │
│                                                                 │
│ Resource Usage:                                                 │
│ CPU: 45.23%                                                     │
│ Memory: 78.50% (6.28 GB / 8.00 GB)                            │
└─────────────────────────────────────────────────────────────────┘
```

### Web UI Service Details Example

Click "execution-layer" → Opens page with:
- Clean tabbed interface
- Color-coded status badges
- Auto-refreshing metrics
- Shareable URLs (`/service/execution-layer`)

---

## Next Steps

### Optional Future Enhancements

1. **Add more plugins** - Create TOML files for your custom services (PostgreSQL, Redis, etc.)
2. **Export metrics** - Add Prometheus export endpoint for external monitoring
3. **Alerting** - Define alert thresholds in plugin TOML files
4. **Historical metrics** - Store and display metric trends over time

---

*For detailed implementation information, see `IMPLEMENTATION_SUMMARY.md`*
*For full architecture details, see `METRICS_PLUGIN_PLAN.md`*
