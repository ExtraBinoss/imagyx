<p align="center">
  <img src="./src-tauri/icons/imagyx-bigger.avif" alt="Imagyx logo" width="112" height="112" />
</p>

<h1 align="center">Imagyx</h1>

<p align="center">
  <strong>Find any image on your computer with natural language.</strong>
</p>

<p align="center">
  <a href="https://github.com/ExtraBinoss/imagyx/releases"><strong>Download</strong></a>
  ·
  <a href="#how-it-works">How it works</a>
  ·
  <a href="./docs/dev/README.md">Developer setup</a>
</p>

<br />

<p align="center">
  <img src="./docs/assets/imagyx-banner.png" alt="Imagyx banner" width="100%" />
</p>

## Demo

Find a `purple sunset`, a `red texture`, a `person near the sea`, or just part of a filename — without organizing everything first.

[IMAGYX-demo (1).webm](https://github.com/user-attachments/assets/cbd375ac-0d6a-4cf8-ab5b-89958bc7bdcd)

<br />

## Why Imagyx?

- 🔎 **Search how you think** — describe an image naturally, search by filename, or combine both.
- ✨ **Find similar images** — pick any image and discover visually similar shots in your library.
- ⚡ **Spotlight from anywhere** — open the global search overlay, type, and act without opening the full app.
- 🖼️ **Preview instantly** — browse images full-size, inspect dimensions and tags, copy them, or reveal them in Explorer/Finder.
- 🔄 **Convert in a click** — create a new copy as **AVIF, WebP, PNG, JPG, or ICO** directly from Spotlight.
- 📁 **Keep your library current** — Imagyx watches followed folders and updates its index as images change.
- 🚀 **Fast by design** — progressive results, cached thumbnails, background indexing, and a lightweight idle footprint.
- 🎨 **Made for your desktop** — keyboard-first controls, light and dark themes, and 10 interface languages.

## Download

<p>
  <a href="https://github.com/ExtraBinoss/imagyx/releases"><strong>Download Imagyx from GitHub Releases →</strong></a>
</p>

Windows and macOS builds are published manually on the Releases page.

<h2 id="how-it-works">How it works</h2>

<table>
  <thead>
    <tr>
      <th width="33%">1 · Add a folder 📁</th>
      <th width="33%">2 · Let it index ⚙️</th>
      <th width="33%">3 · Search anywhere 🔎</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>Choose a folder containing images. Imagyx discovers its files and prepares the local index.</td>
      <td>The first pass creates metadata and the semantic index. Progress stays visible, and you can keep using the app.</td>
      <td>Search in the app, or open Spotlight with <kbd>Ctrl</kbd> + <kbd>9</kbd> by default and act on a result immediately.</td>
    </tr>
  </tbody>
</table>

The shortcut is configurable in Settings. New and changed images are detected automatically in followed folders.

## Private by default

Your images never leave your computer. Search, indexing, thumbnails, and semantic matching all run locally; Imagyx does not upload your library to a search service. Original files are never moved, renamed, or changed.

The first setup may download the local AI model files it needs. Once cached, normal image search remains local.

## Contributing

Issues, ideas, bug reports, and pull requests are welcome. Development setup is documented separately for each supported platform:

- [Windows development guide](./docs/dev/windows.md)
- [macOS development guide](./docs/dev/mac.md)
- [Developer documentation index](./docs/dev/README.md)

<details>
  <summary><strong>Developer information</strong></summary>

  <br />

  ### Quick start

  Requirements:

  - Node.js 22.12 or newer;
  - Rust 1.88 or newer;
  - the native build dependencies documented in the platform guides.

  ```bash
  npm install
  npm run tauri dev
  ```

  ### Local validation

  ```bash
  npm run typecheck
  npm test
  npm run build
  cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
  cargo check --manifest-path src-tauri/Cargo.toml
  cargo test --manifest-path src-tauri/Cargo.toml
  cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
  ```

  ### Implementation overview

  The desktop shell is built with Tauri and Rust. The interface uses Vue and TypeScript. Metadata, text search, and persisted vectors are stored locally with SQLite. Semantic search uses MobileCLIP-S0 through Transformers.js, with WebGPU acceleration when available and a WASM fallback.

  The Rust backend is divided by responsibility under `src-tauri/src/`, including commands, database access, indexing, fuzzy search, tracing, and the in-memory vector store.

  Release workflows and release publication are intentionally manual. Build artifacts can be produced with:

  ```bash
  npm run tauri build
  ```

  Additional performance and backend notes are available in [docs/RUST_PERFORMANCE_PLAN.md](./docs/RUST_PERFORMANCE_PLAN.md).

</details>
