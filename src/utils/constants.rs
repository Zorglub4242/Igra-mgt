/// IGRA Orchestra Service Definitions and Constants
///
/// Based on the architecture documentation and docker-compose.yml

use std::collections::HashMap;

/// Service definition
#[derive(Debug, Clone)]
pub struct Service {
    pub name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub container_name: &'static str,
    pub internal_ports: &'static [u16],
    pub external_ports: &'static [(u16, u16)], // (host, container)
    pub dependencies: &'static [&'static str],
    pub healthcheck_type: HealthCheckType,
    pub volume: Option<&'static str>,
    pub critical: bool, // Critical for L2 operation
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthCheckType {
    TcpPort(u16),
    IpcSocket(&'static str),
    HttpEndpoint(&'static str),
    Docker, // Uses Docker's built-in healthcheck
}

/// Docker Compose profiles
pub const PROFILES: &[&str] = &[
    "kaspad",
    "backend",
    "frontend-w1",
    "frontend-w2",
    "frontend-w3",
    "frontend-w4",
    "frontend-w5",
    "kaswallets",
    "rpc-providers",
];

/// Service categories for grouping in UI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceCategory {
    Layer1,      // Kaspad, kaspa-miner
    Backend,     // execution-layer, block-builder, viaduct
    Frontend,    // RPC providers, kaswallets
    Proxy,       // Traefik
    Monitoring,  // node-health-check-client
}

/// All IGRA Orchestra services
pub fn get_services() -> HashMap<&'static str, Service> {
    let mut services = HashMap::new();

    // Layer 1 - Kaspad
    services.insert("kaspad", Service {
        name: "kaspad",
        display_name: "Kaspad (L1 Node)",
        description: "Kaspa blockchain node providing L1 data",
        container_name: "kaspad",
        internal_ports: &[16210, 16211, 17210, 18210],
        external_ports: &[(17210, 17210), (16211, 16211)],
        dependencies: &[],
        healthcheck_type: HealthCheckType::TcpPort(16210),
        volume: Some("kaspad_data"),
        critical: true,
    });

    services.insert("kaspa-miner", Service {
        name: "kaspa-miner",
        display_name: "Kaspa Miner",
        description: "CPU miner for Kaspa (dev/testing only)",
        container_name: "kaspa-miner",
        internal_ports: &[],
        external_ports: &[],
        dependencies: &["kaspad"],
        healthcheck_type: HealthCheckType::Docker,
        volume: None,
        critical: false,
    });

    // Backend Layer (L2)
    services.insert("execution-layer", Service {
        name: "execution-layer",
        display_name: "Execution Layer (Reth)",
        description: "Ethereum-compatible execution environment",
        container_name: "execution-layer",
        internal_ports: &[8545, 8546, 8551, 9001],
        external_ports: &[(9545, 8545), (9546, 8546)],
        dependencies: &[],
        healthcheck_type: HealthCheckType::IpcSocket("/root/reth/socket/auth.ipc"),
        volume: None, // Uses tmpfs
        critical: true,
    });

    services.insert("block-builder", Service {
        name: "block-builder",
        display_name: "Block Builder",
        description: "Constructs L2 blocks from L1 data",
        container_name: "block-builder",
        internal_ports: &[8561],
        external_ports: &[],
        dependencies: &["execution-layer"],
        healthcheck_type: HealthCheckType::TcpPort(8561),
        volume: None,
        critical: true,
    });

    services.insert("viaduct", Service {
        name: "viaduct",
        display_name: "Viaduct (L1-L2 Bridge)",
        description: "Bridges Kaspa to IGRA execution layer",
        container_name: "viaduct",
        internal_ports: &[],
        external_ports: &[],
        dependencies: &["block-builder", "kaspad"],
        healthcheck_type: HealthCheckType::Docker,
        volume: Some("viaduct_data"),
        critical: true,
    });

    // Frontend Layer
    for i in 0..5 {
        let name = format!("rpc-provider-{}", i);
        let container_name = format!("rpc-provider-{}", i);
        let kaswallet = format!("kaswallet-{}", i);

        // Leak strings first to get 'static references
        let name_static: &'static str = Box::leak(name.clone().into_boxed_str());
        let kaswallet_static: &'static str = Box::leak(kaswallet.clone().into_boxed_str());
        let container_name_static: &'static str = Box::leak(container_name.into_boxed_str());
        let display_name_static: &'static str = Box::leak(format!("RPC Provider {}", i).into_boxed_str());
        let kaswallet_container_static: &'static str = Box::leak(format!("kaswallet-{}", i).into_boxed_str());
        let kaswallet_display_static: &'static str = Box::leak(format!("Kaswallet {}", i).into_boxed_str());

        // Create dependencies array with leaked references
        let rpc_deps: &'static [&'static str] = Box::leak(vec![kaswallet_static, "execution-layer"].into_boxed_slice());

        services.insert(name_static, Service {
            name: name_static,
            display_name: display_name_static,
            description: "Ethereum JSON-RPC interface with entry tx support",
            container_name: container_name_static,
            internal_ports: &[8535],
            external_ports: &[],
            dependencies: rpc_deps,
            healthcheck_type: HealthCheckType::Docker,
            volume: None,
            critical: i == 0, // rpc-provider-0 is critical
        });

        services.insert(kaswallet_static, Service {
            name: kaswallet_static,
            display_name: kaswallet_display_static,
            description: "Kaspa wallet daemon for entry transactions",
            container_name: kaswallet_container_static,
            internal_ports: &[8082],
            external_ports: if i == 0 { &[(8082, 8082)] } else { &[] },
            dependencies: &["kaspad"],
            healthcheck_type: HealthCheckType::Docker,
            volume: None,
            critical: i == 0,
        });
    }

    // Proxy
    services.insert("traefik", Service {
        name: "traefik",
        display_name: "Traefik (Reverse Proxy)",
        description: "Load balancer, SSL/TLS termination, routing",
        container_name: "traefik",
        internal_ports: &[80, 443, 8080, 8545, 8001, 8010, 9001],
        external_ports: &[
            (9000, 80),
            (9443, 443),
            (9080, 8080),
            (8545, 8545),
            (8001, 8001),
            (8010, 8010),
            (9001, 9001),
            (17611, 17210),
        ],
        dependencies: &[],
        healthcheck_type: HealthCheckType::HttpEndpoint("http://localhost:8080"),
        volume: Some("traefik_certs"),
        critical: false, // Not critical for L2 operation, but needed for RPC access
    });

    // Monitoring
    services.insert("node-health-check-client", Service {
        name: "node-health-check-client",
        display_name: "Health Check Client",
        description: "Reports node health to central monitoring",
        container_name: "node-health-check-client",
        internal_ports: &[],
        external_ports: &[],
        dependencies: &["execution-layer"],
        healthcheck_type: HealthCheckType::Docker,
        volume: None,
        critical: false,
    });

    services
}

/// Get service category
pub fn get_service_category(service_name: &str) -> ServiceCategory {
    match service_name {
        "kaspad" | "kaspa-miner" => ServiceCategory::Layer1,
        "execution-layer" | "block-builder" | "viaduct" => ServiceCategory::Backend,
        name if name.starts_with("rpc-provider") || name.starts_with("kaswallet") => {
            ServiceCategory::Frontend
        }
        "traefik" => ServiceCategory::Proxy,
        "node-health-check-client" => ServiceCategory::Monitoring,
        _ => ServiceCategory::Monitoring,
    }
}

/// Get services for a specific profile
pub fn get_profile_services(profile: &str) -> Vec<&'static str> {
    match profile {
        "kaspad" => vec!["kaspad"],
        "backend" => vec!["execution-layer", "block-builder", "viaduct"],
        "frontend-w1" => vec!["traefik", "rpc-provider-0", "kaswallet-0", "node-health-check-client"],
        "frontend-w2" => vec!["traefik", "rpc-provider-0", "rpc-provider-1",
                              "kaswallet-0", "kaswallet-1", "node-health-check-client"],
        "frontend-w3" => vec!["traefik", "rpc-provider-0", "rpc-provider-1", "rpc-provider-2",
                              "kaswallet-0", "kaswallet-1", "kaswallet-2", "node-health-check-client"],
        "frontend-w4" => vec!["traefik", "rpc-provider-0", "rpc-provider-1", "rpc-provider-2",
                              "rpc-provider-3", "kaswallet-0", "kaswallet-1", "kaswallet-2",
                              "kaswallet-3", "node-health-check-client"],
        "frontend-w5" => vec!["traefik", "rpc-provider-0", "rpc-provider-1", "rpc-provider-2",
                              "rpc-provider-3", "rpc-provider-4", "kaswallet-0", "kaswallet-1",
                              "kaswallet-2", "kaswallet-3", "kaswallet-4", "node-health-check-client"],
        "kaswallets" => vec!["kaswallet-0", "kaswallet-1"],
        "rpc-providers" => vec!["rpc-provider-1", "rpc-provider-2", "rpc-provider-3", "rpc-provider-4"],
        _ => vec![],
    }
}

/// Volume names
pub const VOLUMES: &[&str] = &[
    "kaspad_data",
    "viaduct_data",
    "traefik_certs",
    "reth_ipc",
];

/// Critical volumes that must be backed up
pub const BACKUP_VOLUMES: &[&str] = &[
    "viaduct_data",  // Critical - L1-L2 state
    "kaspad_data",   // Optional - can resync
    "traefik_certs", // Recommended - avoid rate limits
];

/// Docker networks
pub const NETWORKS: &[&str] = &[
    "igra-network",
    "traefik-network",
];

/// RPC token count
pub const RPC_TOKEN_COUNT: usize = 46;

/// Default paths
pub const DEFAULT_COMPOSE_FILE: &str = "docker-compose.yml";
pub const DEFAULT_ENV_FILE: &str = ".env";
pub const DEFAULT_KEYS_DIR: &str = "./keys";
pub const JWT_SECRET_FILE: &str = "./keys/jwt.hex";

/// Port mappings for display
pub fn get_port_description(port: u16) -> &'static str {
    match port {
        8545 => "JSON-RPC (Execution Layer)",
        8546 => "WebSocket JSON-RPC",
        8551 => "Engine API (JWT auth)",
        9001 => "Metrics/Stats",
        8561 => "Viaduct connection",
        8535 => "RPC Provider interface",
        8082 => "Kaswallet RPC",
        16210 => "Kaspad gRPC",
        17210 => "Kaspad WRPC (Borsh)",
        18210 => "Kaspad WRPC (JSON)",
        16211 => "Kaspad P2P",
        80 => "HTTP",
        443 => "HTTPS",
        8080 => "Traefik Dashboard",
        _ => "Unknown",
    }
}

/// Startup order for services (dependencies first)
pub const STARTUP_ORDER: &[&str] = &[
    // Layer 1
    "kaspad",

    // Backend (L2)
    "execution-layer",  // No dependencies
    "block-builder",    // Depends on execution-layer
    "viaduct",          // Depends on block-builder and kaspad

    // Frontend
    "kaswallet-0",
    "kaswallet-1",
    "kaswallet-2",
    "kaswallet-3",
    "kaswallet-4",
    "rpc-provider-0",
    "rpc-provider-1",
    "rpc-provider-2",
    "rpc-provider-3",
    "rpc-provider-4",

    // Proxy and Monitoring
    "traefik",
    "node-health-check-client",

    // Optional
    "kaspa-miner",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_definitions() {
        let services = get_services();
        assert!(services.contains_key("kaspad"));
        assert!(services.contains_key("execution-layer"));
        assert!(services.contains_key("viaduct"));
    }

    #[test]
    fn test_profile_services() {
        let backend_services = get_profile_services("backend");
        assert_eq!(backend_services.len(), 3);
        assert!(backend_services.contains(&"viaduct"));
    }

    #[test]
    fn test_service_categories() {
        assert_eq!(get_service_category("kaspad"), ServiceCategory::Layer1);
        assert_eq!(get_service_category("viaduct"), ServiceCategory::Backend);
        assert_eq!(get_service_category("rpc-provider-0"), ServiceCategory::Frontend);
    }
}
