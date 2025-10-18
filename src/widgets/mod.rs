// Widgets are implemented inline in dashboard.rs using ratatui primitives
//
// The TUI uses ratatui built-in widgets:
// - Table for services, profiles, wallets, RPC tokens, config
// - Paragraph for logs, SSL info, status messages
// - Block for borders and titles
// - Layout for screen organization
//
// Custom rendering includes:
// - Help overlay dialog
// - Send transaction dialog
// - Service detail view with logs
// - Color-coded resource alerts
