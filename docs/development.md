# Development Guide

This guide covers how to build `igra-cli` from source, develop new features, and contribute to the project.

## Prerequisites

- **Rust** 1.70+ with Cargo
- **Docker** 23.0+ (for testing Docker integration)
- **Node.js** 18+ and npm (for Web UI development)
- **Git**

## Building from Source

### Clone the Repository

```bash
git clone https://github.com/Zorglub4242/Igra-mgt.git
cd Igra-mgt
```

### Build Rust Binary

```bash
# Debug build (faster compilation, slower runtime)
cargo build

# Release build (optimized)
cargo build --release

# Binary location
./target/release/igra-cli
```

### Build with Web UI

To build with the embedded Web UI, you must first build the React frontend:

```bash
# Build React frontend
cd igra-web-ui
npm install
npm run build

# Build Rust with server feature
cd ..
cargo build --release --features server
```

The Web UI assets from `igra-web-ui/dist/` are embedded into the binary via `rust-embed`.

## Development Workflow

### TUI Development

```bash
# Run in debug mode
cargo run

# Run with backtrace for debugging
RUST_BACKTRACE=1 cargo run

# Run with specific log level
RUST_LOG=debug cargo run
```

### Web UI Development

For faster iteration during Web UI development:

```bash
# Terminal 1: Run Rust backend with CORS enabled
cargo run --features server -- serve --cors

# Terminal 2: Run React dev server
cd igra-web-ui
npm run dev
```

Open http://localhost:5173 - Vite will proxy API calls to the Rust backend on port 3000.

Changes to React code will hot-reload automatically. Changes to Rust code require restarting the backend.

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

## Project Structure

```
tools/igra-cli/
├── src/
│   ├── main.rs           # Entry point, CLI parsing
│   ├── tui/              # TUI implementation
│   │   ├── mod.rs        # App state and event loop
│   │   └── screens/      # Individual TUI screens
│   ├── server/           # Web server (feature-gated)
│   │   ├── mod.rs        # Axum setup
│   │   └── handlers.rs   # API endpoints
│   ├── docker/           # Docker API client
│   ├── config/           # Configuration loading
│   └── wallets/          # Wallet management
├── igra-web-ui/          # React frontend
│   ├── src/
│   │   ├── components/   # React components
│   │   └── services/     # API client
│   ├── dist/             # Built assets
│   └── package.json
└── Cargo.toml            # Rust dependencies
```

## Adding New Features

### Adding a New TUI Screen

1. Create new screen module in `src/tui/screens/`:
   ```rust
   // src/tui/screens/my_screen.rs
   use ratatui::prelude::*;

   pub fn render(frame: &mut Frame, area: Rect) {
       // Rendering logic
   }
   ```

2. Add to `src/tui/screens/mod.rs`:
   ```rust
   pub mod my_screen;
   ```

3. Update screen enum and rendering in `src/tui/mod.rs`

4. Add keyboard shortcut for navigation

### Adding a New API Endpoint

1. Define handler in `src/server/handlers.rs`:
   ```rust
   pub async fn my_handler() -> Json<MyResponse> {
       // Handler logic
       Json(response)
   }
   ```

2. Add route in `src/server/mod.rs`:
   ```rust
   let app = Router::new()
       .route("/api/my-endpoint", get(my_handler));
   ```

3. Update frontend API client in `igra-web-ui/src/services/api.js`:
   ```javascript
   async myEndpoint() {
       return this.get('/api/my-endpoint')
   }
   ```

### Adding a New React Component

1. Create component in `igra-web-ui/src/components/`:
   ```javascript
   // MyPanel.jsx
   export default function MyPanel() {
       return <div className="card">...</div>
   }
   ```

2. Import and use in `App.jsx`:
   ```javascript
   import MyPanel from './components/MyPanel'
   ```

3. Rebuild frontend:
   ```bash
   cd igra-web-ui && npm run build
   ```

## Code Style

### Rust
- Follow Rust standard style (enforced by `rustfmt`)
- Run `cargo fmt` before committing
- Run `cargo clippy` to catch common issues

### JavaScript/React
- Use functional components with hooks
- Follow ESLint configuration
- Run `npm run lint` before committing

## Building Release Artifacts

### Linux

```bash
cargo build --release --features server
strip target/release/igra-cli
tar -czf igra-cli-linux-x86_64.tar.gz -C target/release igra-cli
```

### Windows (cross-compile from Linux)

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu --features server
```

### macOS (cross-compile)

Requires macOS SDK and osxcross toolchain.

## Contributing

1. Fork the repository
2. Create a feature branch:
   ```bash
   git checkout -b feature/my-feature
   ```

3. Make your changes
4. Run tests and linters:
   ```bash
   cargo test
   cargo fmt
   cargo clippy
   cd igra-web-ui && npm run lint
   ```

5. Commit your changes:
   ```bash
   git commit -m "Add my feature"
   ```

6. Push to your fork:
   ```bash
   git push origin feature/my-feature
   ```

7. Open a Pull Request on GitHub

## Common Development Tasks

### Update Dependencies

```bash
# Update Cargo dependencies
cargo update

# Update npm dependencies
cd igra-web-ui
npm update
```

### Clean Build Artifacts

```bash
# Clean Rust build
cargo clean

# Clean npm build
cd igra-web-ui
rm -rf node_modules dist
npm install
npm run build
```

### Debug Docker API Issues

```bash
# Enable debug logging
RUST_LOG=bollard=debug cargo run

# Test Docker connectivity
docker ps
```

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Ratatui Documentation](https://ratatui.rs/)
- [Axum Documentation](https://docs.rs/axum/)
- [React Documentation](https://react.dev/)
- [Vite Documentation](https://vitejs.dev/)
- [Docker API Reference](https://docs.docker.com/engine/api/)

## Getting Help

- Open an issue on [GitHub](https://github.com/Zorglub4242/Igra-mgt/issues)
- Check existing issues for similar problems
- Provide detailed reproduction steps and error messages
