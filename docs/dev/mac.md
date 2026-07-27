# macOS development setup

This guide sets up a Mac for developing and building Imagyx.

## 1. Install the system requirements

### macOS

Use macOS 10.15 or newer. A current supported macOS release is recommended.

### Xcode Command Line Tools

Desktop development only requires Apple's Command Line Tools:

```bash
xcode-select --install
```

Verify the active developer directory:

```bash
xcode-select -p
xcrun --show-sdk-path
```

Full Xcode is only necessary when you need its additional tooling. If you install it, launch Xcode once so it can finish installing components and accept the license.

Tauri's official prerequisite guide is available at [v2.tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/).

### Node.js

Install Node.js **22.12.0 or newer** using the official installer or your preferred version manager. Node 22 LTS is recommended.

Verify the installation:

```bash
node --version
npm --version
```

### Rust

Install Rust with rustup:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup default stable
rustup update
rustc --version
cargo --version
```

The project requires Rust **1.88.0 or newer**. Rustup selects the native Apple Silicon or Intel target automatically.

### Git

Git is normally installed with the Xcode Command Line Tools. Verify it:

```bash
git --version
```

## 2. Clone the repository

```bash
git clone https://github.com/ExtraBinoss/imagyx.git
cd imagyx
```

To work on the current prototype branch:

```bash
git switch agent/tauri-vue-prototype
```

## 3. Install project dependencies

```bash
npm install
```

No global Tauri CLI installation is required. The repository uses the CLI declared in `package.json`.

## 4. Start the desktop application

```bash
npm run tauri dev
```

The first run may take longer because Rust dependencies are compiled and the local search model is prepared. Required model files are downloaded automatically when they are missing. Later launches reuse the local cache.

The default Spotlight shortcut is:

<kbd>Control</kbd> + <kbd>Numpad 9</kbd>

Compact keyboards may not have a numeric keypad. Change the shortcut from Imagyx settings when necessary.

## 5. Validate your changes

Run the frontend checks:

```bash
npm run typecheck
npm test
npm run build
```

Run the Rust checks:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

## 6. Build a release package

```bash
npm run tauri build
```

Generated macOS bundles are placed below:

```text
src-tauri/target/release/bundle/
```

Local unsigned builds are suitable for development. Public distribution requires the appropriate signing and notarization setup. Release publication is manual; upload the selected artifacts to GitHub Releases when preparing a release.

## Local application data

Imagyx stores its generated data under:

```text
~/Pictures/imagyx/
├── models/
├── database/imagyx.sqlite3
├── cache/thumbnails/
└── logs/
```

Original images remain in their existing folders. They are not copied into the Imagyx data directory.

## Troubleshooting

### `xcrun` cannot find the SDK

Install or repair the Command Line Tools:

```bash
xcode-select --install
```

When full Xcode is installed, select it explicitly:

```bash
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

### Rust linker or SDK errors after an Xcode update

Confirm the active SDK and rebuild:

```bash
xcrun --show-sdk-path
cargo clean --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

### The Spotlight shortcut does not open

A compact Mac keyboard may not expose <kbd>Numpad 9</kbd>, or another application may already own the shortcut. Open Imagyx settings and register another combination.

### The first semantic search is slower

The text encoder is warmed after the first UI frames. A completely cold launch may still need to load the local model once; following searches reuse the warm runtime.

### The app is blocked after downloading a release build

Development builds launched through `npm run tauri dev` are not affected in the same way as downloaded unsigned applications. Public macOS releases should be signed and notarized before distribution.

### Resetting local development data

Deleting `~/Pictures/imagyx` removes the local database, cached thumbnails, settings, and downloaded model files. Only do this when you intentionally want a clean development state. Followed source images are not deleted.