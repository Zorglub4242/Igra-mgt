pub mod dashboard;

// All screens are implemented in dashboard.rs as a unified TUI interface:
// - Screen 1: Services (container management)
// - Screen 2: Profiles (docker-compose profile management)
// - Screen 3: Wallets (kaspa wallet management)
// - Screen 4: RPC Tokens (token generation and testing)
// - Screen 5: Config (environment variable editing)
// - Screen 6: SSL (certificate status and renewal)
// - Screen 7: Logs (interactive log viewer with filtering)
//
// Additional features:
// - System resource monitoring (CPU, Memory, Disk, Network I/O)
// - Search/filter functionality
// - Help overlay (press '?')
// - Real-time auto-refresh

pub use dashboard::Dashboard;
