# Rust and Tauri Code Quality

This document defines the Rust, Tauri, SQLite, filesystem, indexing, and native desktop conventions for Imagyx.

## 1. Scope

These rules apply to:

- `src-tauri/src/`
- Tauri commands
- native window behavior
- global shortcuts
- filesystem access
- SQLite access
- indexing and thumbnail generation
- model download and preparation
- native clipboard and Explorer/Finder integration
- Rust tests

## 2. File size and module ownership

- Keep every Rust source file below **500 lines**.
- Split a module before it becomes a mixed collection of unrelated commands and helpers.
- Tauri commands should be thin adapters.
- Business logic belongs in focused modules such as indexing, preferences, thumbnails, model preparation, or onboarding.
- Platform-specific behavior should be isolated behind focused functions or modules.

A module should own one domain. Avoid a generic `utils.rs` that becomes a dumping ground.

## 3. Tauri command boundaries

A Tauri command should usually:

1. deserialize and validate input
2. obtain managed state
3. delegate substantial work to a domain module
4. map the domain result to a frontend-safe response

Commands must not contain large indexing loops, image pipelines, or platform orchestration when that logic can be tested separately.

Use `#[tauri::command(rename_all = "camelCase")]` when frontend argument names use camelCase.

Keep command names stable because they are part of the frontend contract.

## 4. Error handling

- Use typed domain errors internally.
- Extend `AppError` or create a focused error type for a domain when necessary.
- Convert errors to `String` only at the Tauri command boundary when required by the transport.
- Preserve context when mapping errors.
- Avoid generic messages such as “operation failed” when a useful cause is available.
- Do not expose sensitive internal data unnecessarily.

Prefer:

```rust
operation().map_err(|error| format!("Unable to index folder: {error}"))?
```

Avoid:

```rust
operation().map_err(|_| "error".to_owned())?
```

### Panics

- Do not use `unwrap()` or `expect()` for user-controlled data, filesystem state, database data, window availability, or optional platform features.
- `expect()` is acceptable only for a startup invariant that is statically controlled by the application and truly cannot be recovered from.
- Optional integrations, such as a global shortcut registration failure, must not prevent the application from starting.

## 5. Filesystem security

Every path received from the frontend is untrusted.

Before reading, writing, opening, copying, or revealing a path:

1. convert it to `PathBuf`
2. canonicalize it when the path must already exist
3. verify its type when required (`is_file`, `is_dir`)
4. verify it belongs to an allowed followed folder or application-owned directory
5. reject traversal or unmanaged paths

Do not rely only on the frontend to provide safe paths.

Prefer central helpers such as `managed_path` rather than duplicating validation.

Application-owned paths should come from `AppPaths`, not from hardcoded strings.

## 6. Platform-specific behavior

Use compile-time platform branches:

```rust
#[cfg(target_os = "windows")]
#[cfg(target_os = "macos")]
#[cfg(all(unix, not(target_os = "macos")))]
```

Keep each branch small and consistent in behavior.

Use platform-native behavior for:

- Explorer/Finder/file manager reveal
- clipboard image copy
- global shortcuts
- native windows
- focus and visibility

Do not emulate native desktop behavior in the browser layer when Rust can implement it correctly.

## 7. Async and blocking work

Tauri async commands must not block the async runtime or UI thread with:

- directory walking
- image decoding
- thumbnail generation
- SQLite-heavy work
- model file checks or downloads
- CPU-intensive embedding preparation

Use `tauri::async_runtime::spawn_blocking` for blocking work.

Example:

```rust
let state = Arc::clone(state.inner());
tauri::async_runtime::spawn_blocking(move || run_blocking_work(&state))
    .await
    .map_err(|error| error.to_string())??;
```

Do not hold a lock across `.await`.

## 8. Shared state and locking

- Keep managed state focused and explicit.
- Use `Arc` for shared ownership across background work.
- Use `parking_lot` locks where already established.
- Keep lock scopes short.
- Do not perform filesystem, image, network, or database work while holding a broad shared lock unless the lock specifically protects that operation.
- Avoid nested lock acquisition when possible.
- Document lock ordering when multiple locks are unavoidable.

Use shared in-flight state or a dedicated lock to prevent duplicate model downloads and duplicate indexing work.

## 9. Database access

- SQLite is the durable source of truth for followed folders, images, and embeddings.
- Persist completed batches incrementally.
- Use transactions for multi-row writes that must succeed together.
- Avoid opening many short-lived connections inside tight loops.
- Keep schema and query logic in the database module.
- Avoid SQL strings scattered across unrelated command modules.
- Refresh in-memory vector state only after durable database writes succeed.

Database migrations must be explicit and backward-compatible with existing local libraries.

## 10. Indexing

Indexing must be resumable.

Required behavior:

- discover supported images
- compare path, modified time, and size to identify changes
- save metadata in bounded batches
- emit progress between batches
- remove missing files
- queue only images without current embeddings
- persist embedding batches immediately
- resume from pending images after pause, restart, or failure

Do not erase all progress because one image fails.

Individual corrupt or unsupported files should be skipped or reported without crashing the entire folder job when safe.

The current batch may be retried after a hard shutdown; completed batches must not be recalculated.

## 11. Progress events

Long-running work must emit explicit typed progress.

Progress should include when relevant:

- stable object or folder identifier
- display name
- current count
- total count
- stage
- human-readable message

Use determinate progress when totals are known.

Stage names should be stable because the frontend maps them to product UI.

Do not emit excessively on every file if it creates IPC overhead. Emit per bounded batch or meaningful phase.

## 12. Search and vector work

- Reuse in-memory normalized embeddings when available.
- Keep result limits bounded.
- Use parallel iteration only when the operation is CPU-bound, independent, and large enough to benefit.
- Avoid cloning complete image or vector collections unnecessarily.
- Do not hold the vector read lock longer than needed.
- Keep lexical search available when semantic vectors are absent.
- Measure before introducing ANN or more complex indexing structures.

For large result sets, prefer top-K selection or bounded sorting over returning every match to the frontend.

## 13. Rayon and parallelism

Rayon is appropriate for independent CPU-heavy operations such as:

- image metadata preparation
- similarity calculations
- concept ranking

Rules:

- do not nest uncontrolled parallel loops
- do not perform blocking IPC or UI event emission inside a hot parallel iterator
- collect bounded results
- preserve deterministic ordering when product behavior depends on it
- confirm that synchronization overhead does not exceed the work

Parallelism is not a substitute for batching or result limits.

## 14. Model files and downloads

- Store model files only in application-managed model directories.
- Validate expected files before reporting a model as ready.
- Use a single shared download/preparation lock.
- Write downloads to temporary files and rename atomically when practical.
- Preserve previous valid files if a new download fails.
- Emit file and byte progress.
- Do not perform the same download concurrently from the main window and Spotlight.
- Keep model selection and file layout explicit.

Never assume a remote model repository layout without verifying the required runtime paths.

## 15. Native windows

Native window operations must be ordered deliberately.

For Spotlight-like windows:

1. prepare hidden frontend state
2. calculate monitor work area and scale factor
3. resize and position the native window
4. show and focus the window
5. emit the event that reveals animated content

When collapsing:

1. animate content out
2. resize the native window
3. hide it when appropriate

Do not depend on rendering outside current native bounds.

Window lookup can fail and must return a recoverable error rather than panic.

## 16. Global shortcuts

- Persist the configured shortcut locally.
- Parse and normalize shortcut strings in one module.
- Unregister the previous shortcut before registering a new one.
- Restore the previous shortcut if the new registration fails.
- Persist only after successful registration.
- A startup registration failure must be logged and must not abort the app.
- The shortcut handler must respond only to the intended press state.

Shortcut changes should take effect immediately without restart.

## 17. Native clipboard and image operations

- Validate the path before decoding.
- Decode image data in blocking work.
- Bound memory use when possible.
- Return useful errors for unsupported or corrupt images.
- Do not keep clipboard resources locked longer than necessary.

Full-resolution image decoding must not run on the async UI thread.

## 18. Serialization and frontend contracts

- Use `serde` derives for command payloads.
- Use `#[serde(rename_all = "camelCase")]` when the frontend expects camelCase.
- Keep Rust and TypeScript models aligned.
- Prefer explicit structs over loosely typed JSON values.
- Avoid changing field meaning without a coordinated frontend update.
- Use stable string stages or enums with clear migration expectations.

When a payload is reused across multiple commands or events, define it in `models.rs` or a focused model module.

## 19. Logging and diagnostics

- Log actionable failures that cannot be surfaced to the user.
- Avoid noisy logs in hot paths.
- Do not log embedding vectors, image bytes, or clipboard contents.
- Avoid logging full private paths unless required for local debugging and already expected by the diagnostic surface.
- Include operation context.

Use user-visible errors for recoverable product actions and logs for developer diagnostics.

## 20. Resource cleanup

Ensure cleanup for:

- filesystem watchers
- temporary download files
- window event handlers
- background jobs
- database connections
- registered shortcuts during replacement

Do not leave a partially persisted preference or model file after an error.

## 21. Testing

Add unit tests for pure or filesystem-isolated logic such as:

- path layout creation
- shortcut parsing and preference persistence
- search scoring
- supported image extension detection
- metadata change detection
- migration behavior
- managed-path validation

Use temporary directories and temporary databases.

Do not write tests that depend on the developer's Pictures directory.

Platform-specific native integration may require manual testing on the target OS.

## 22. Formatting and linting

Follow the repository Rust lint configuration.

Run:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Use the subset relevant to the change, but at minimum run `cargo check` for Rust modifications.

Do not claim a command passed unless it was actually run.

## 23. Review checklist

- File is below 500 lines.
- Tauri command is a thin boundary.
- Frontend paths are canonicalized and authorized.
- Blocking work is outside the async UI thread.
- No lock is held across `.await`.
- Errors preserve useful context.
- No user-controlled `unwrap()` or `expect()` exists.
- Long work emits bounded progress updates.
- Indexing persists batches and remains resumable.
- Database writes are durable before in-memory refresh.
- Result sets and allocations are bounded.
- Platform behavior uses clear `cfg` branches.
- Optional shortcut failure cannot abort startup.
- Rust and TypeScript payloads remain aligned.
- Relevant checks were run and reported honestly.
