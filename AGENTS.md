# AGENTS.md

This file defines the repository-wide rules for coding agents and contributors.
Keep it short. Detailed rules belong in the documents referenced below.

## Read first

Before changing code, read the relevant guide:

- UI and product behavior: [`docs/UI.md`](docs/UI.md)
- Rust conventions and backend quality: [`docs/RUST_CODE_QUALITY.md`](docs/RUST_CODE_QUALITY.md)
- TypeScript and Vue conventions: [`docs/TS_CODE_QUALITY.md`](docs/TS_CODE_QUALITY.md)

Read every guide touched by a cross-layer change. For example, a native Spotlight change normally requires all three documents.
When a detailed guide conflicts with this file, the detailed guide wins for its area.

## Repository principles

- Imagyx is a local-first Vue 3 + TypeScript + Tauri application.
- User images, embeddings, settings, shortcuts, and indexes stay local unless a feature explicitly requires otherwise.
- Keep the application responsive while indexing, searching, opening previews, and using Spotlight.
- Prefer lazy work, caching, batching, virtualization, and resumable background jobs over eager computation.
- Do not silently replace native desktop behavior with a browser-only approximation when Tauri can implement it correctly.

## Code organization

- Keep source files below **500 lines**. Split earlier when a file owns multiple responsibilities.
- A component should have one clear purpose. Extract reusable behavior instead of growing page-level components.
- Group feature-specific components by feature, for example `components/Spotlight/` and `components/Onboarding/`.
- Put generic reusable controls in `components/ui/`.
- Put reusable keyboard controls in `components/shortcuts/`.
- Keep platform-specific and privileged behavior in Rust or a dedicated Tauri API wrapper.
- Avoid duplicated state. Shared persistent behavior belongs in a store or a dedicated service.

## UI rules

- Reuse the existing UI components. Do not copy their markup or recreate their styles locally.
- All clickable actions must use `components/ui/Button/Button.vue` unless a native control is technically required.
- Use `ButtonGroup.vue`, `MovingBorder.vue`, `ShortcutView.vue`, popovers, dialogs, badges, inputs, accordions, and other existing primitives before creating a new one.
- Extend a shared component when the design system needs a new capability; do not style native `<button>` elements independently.
- Do not use `:deep()` to reach into a UI component. Add a prop, slot, variant, size, or public class contract instead.
- Do not add native `title` tooltips for paths or repeated information. Use an intentional UI only when the information is useful.
- Preserve the current depth language: subtle top highlight, restrained shadow, clear pressed state, and theme-aware contrast.
- Animations must be fast, interruptible, symmetric on entry and exit, and respect `prefers-reduced-motion`.
- Avoid screenshots in product education when a lightweight live rendering of the real component can demonstrate the feature.
- Keep light and dark themes visually equivalent and test both when changing shared UI.

## Vue and TypeScript

- Use Vue 3 Composition API with `<script setup lang="ts">`.
- Keep TypeScript strict and avoid `any` unless an external API makes it unavoidable and the boundary is documented.
- Do not leave unused imports, props, emits, or state.
- Use stores for shared application state and services for reusable async/runtime behavior.
- Avoid expensive work in render paths, computed properties, and hover handlers without caching.
- Search and preview keyboard handlers must not intercept input, textarea, select, or contenteditable elements.
- Persist user-facing preferences locally and apply changes immediately when possible.

## Rust and Tauri

- Validate every path received from the frontend before filesystem access.
- Keep Tauri commands small; move substantial logic into focused modules.
- Do blocking filesystem, image, database, and model work outside the async UI thread.
- Emit explicit progress events for long-running work.
- Background indexing must be resumable and must save completed batches incrementally.
- A failure to register an optional global shortcut must not prevent the application from starting.
- Prefer platform-native Explorer, Finder, clipboard, window, and shortcut behavior.

## Performance

- Never render thousands of image cards without virtualization.
- Do not calculate per-image tags or explanations for every search result eagerly.
- Reuse stored image embeddings and cache derived concepts.
- Return bounded result sets to the frontend and measure before introducing more complex indexes.
- Prewarm expensive runtime paths when it can be done invisibly without blocking startup.
- Keep the first useful result available through a fast lexical path while semantic ranking finishes.

## Product behavior to preserve

- Typing while the main app is focused starts a search unless the user is already editing a field.
- Image selection is visible around the complete card.
- Space opens the selected or hovered image preview.
- Spotlight remains available while Imagyx runs in the background.
- Spotlight actions appear on result hover and use shared buttons.
- Folder indexing can continue while the user searches.
- Onboarding is shown on first use, can add a folder, and can be reopened from Settings.

## Validation

Before considering a change complete, run the relevant local checks:

```bash
npm run typecheck
cargo check --manifest-path src-tauri/Cargo.toml
```

Run the application when behavior or native windowing changed:

```bash
npm run tauri dev
```

Do not claim a check passed unless it was actually run. Do not trigger GitHub Actions unless explicitly requested.
