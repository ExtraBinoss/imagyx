# Imagyx developer setup

Choose the guide for your platform:

- [Windows development setup](./windows.md)
- [macOS development setup](./mac.md)

## Project requirements

| Requirement | Minimum |
|---|---:|
| Node.js | 22.12.0 |
| Rust | 1.88.0 |
| Git | Recent version |

The platform guides cover the native system dependencies, first launch, validation commands, release builds, local data paths, and common troubleshooting steps.

## Daily development command

```bash
npm run tauri dev
```

This starts the frontend development server and the native desktop application together. Running `npm run dev` alone is useful for frontend-only work, but native commands and desktop-window behavior will not be available.

## Full local validation

```bash
npm run typecheck
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

Release workflows are intentionally manual. Build and publish release artifacts only when a release is being prepared.