# Windows development setup

This guide sets up a Windows machine for developing and building Imagyx.

## 1. Install the system requirements

### Windows

Use a supported 64-bit Windows 10 or Windows 11 installation.

### Microsoft C++ Build Tools

Install **Visual Studio 2022 Build Tools** and select the following workload:

- **Desktop development with C++**

Make sure the workload includes:

- MSVC v143 C++ build tools;
- a Windows 10 or Windows 11 SDK;
- C++ CMake tools for Windows.

Tauri's official prerequisite guide is available at [v2.tauri.app/start/prerequisites](https://v2.tauri.app/start/prerequisites/).

### Microsoft Edge WebView2

Imagyx uses the WebView2 runtime supplied by Microsoft. It is normally already installed on current Windows systems. Install the Evergreen Runtime from Microsoft if the app opens with a WebView2-related error.

### Node.js

Install Node.js **22.12.0 or newer**. Node 22 LTS is recommended.

Verify the installation in PowerShell:

```powershell
node --version
npm --version
```

### Rust

Install Rust with [rustup](https://rustup.rs/), then use the MSVC toolchain:

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup default stable-x86_64-pc-windows-msvc
rustup update
rustc --version
cargo --version
```

The project requires Rust **1.88.0 or newer**.

### Git

Install Git for Windows, then verify it:

```powershell
git --version
```

## 2. Clone the repository

```powershell
git clone https://github.com/ExtraBinoss/imagyx.git
cd imagyx
```

To work on the current prototype branch:

```powershell
git switch agent/tauri-vue-prototype
```

## 3. Install project dependencies

```powershell
npm install
```

No global Tauri CLI installation is required. The repository uses the CLI declared in `package.json`.

## 4. Start the desktop application

```powershell
npm run tauri dev
```

The first run may take longer because Rust dependencies are compiled and the local search model is prepared. Required model files are downloaded automatically when they are missing. Later launches reuse the local cache.

The default Spotlight shortcut is:

<kbd>Ctrl</kbd> + <kbd>Numpad 9</kbd>

If this shortcut is already registered by another application, change it from Imagyx settings.

## 5. Validate your changes

Run the frontend checks:

```powershell
npm run typecheck
npm test
npm run build
```

Run the Rust checks:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
```

## 6. Build a release package

```powershell
npm run tauri build
```

Generated Windows bundles are placed below:

```text
src-tauri/target/release/bundle/
```

Release publication is manual. Upload the selected installer artifacts to the repository's GitHub Releases page when preparing a release.

## Local application data

Imagyx stores its generated data under:

```text
%USERPROFILE%\Pictures\imagyx\
├── models\
├── database\imagyx.sqlite3
├── cache\thumbnails\
└── logs\
```

Original images remain in their existing folders. They are not copied into the Imagyx data directory.

## Troubleshooting

### `link.exe` is missing

Reopen PowerShell after installing Visual Studio Build Tools. Confirm that the **Desktop development with C++** workload and a Windows SDK are installed.

### Strange linker errors after a dependency or profile change

Stop the running Tauri process, then clear Rust build artifacts:

```powershell
cargo clean --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml
```

If Windows reports that files are locked, close Imagyx and any active Cargo process before removing the target directory:

```powershell
Remove-Item -Recurse -Force .\src-tauri\target
```

### WebView2 error

Install or repair the Microsoft Edge WebView2 Evergreen Runtime, then restart Windows.

### The Spotlight shortcut does not open

Check whether another application already owns <kbd>Ctrl</kbd> + <kbd>Numpad 9</kbd>. Open Imagyx settings and register another shortcut.

### The first semantic search is slower

The text encoder is warmed after the first UI frames. A completely cold launch may still need to load the local model once; following searches reuse the warm runtime.

### Resetting local development data

Deleting `%USERPROFILE%\Pictures\imagyx` removes the local database, cached thumbnails, settings, and downloaded model files. Only do this when you intentionally want a clean development state. Followed source images are not deleted.