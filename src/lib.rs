// Library interface for igra-cli
// Exposes core modules for testing and external use

pub mod core;
pub mod utils;

#[cfg(feature = "server")]
pub mod server;

#[cfg(windows)]
pub mod windows_service;
