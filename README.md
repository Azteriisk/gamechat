# Native Discord Alternative

A high-performance, native desktop chat application built with Rust, Slint, and the Matrix protocol.

## Features
- **Native UI**: Built with Slint for a responsive, memory-efficient interface (no Electron).
- **Matrix Backend**: Decentralized communication using `matrix-sdk`.
- **Discord-like UX**: Familiar server rail, channel list, and chat area layout.

## Prerequisites
- **Rust**: Latest stable toolchain (`rustup update`).
- **Windows**: Visual C++ Build Tools (for MSVC linker).

## Build & Run

### 1. Clone the Repository
```bash
git clone <repository-url>
cd gamechat
```

### 2. Run the Application
```bash
cargo run -p ui
```

### 3. Run Tests
```bash
cargo test --workspace
```

## Troubleshooting
**Start-up Crash (Stack Overflow)**:
If the app crashes silently on launch:
1. Ensure you are not running on a restrictive VM or container.
2. Check `.cargo/config.toml` ensures a large stack size (16MB).
3. The `network` crate is currently configured with minimal features (`default-features = false`) to prevent stack overflows.

## License
[License Name]
