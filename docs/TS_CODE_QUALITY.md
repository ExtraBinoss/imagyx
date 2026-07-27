# TypeScript and Vue Code Quality

This document defines the TypeScript and Vue conventions for Imagyx.

## 1. Scope

These rules apply to:

- Vue single-file components
- Pinia stores
- frontend services
- Tauri API wrappers
- composables
- utilities
- frontend tests

The project uses Vue 3, the Composition API, `<script setup lang="ts">`, Pinia, strict TypeScript, and Tauri 2.

## 2. File size and responsibility

- Keep every source file below **500 lines**.
- Split earlier when a file owns more than one meaningful responsibility.
- A Vue page or feature parent coordinates state; child components render focused sections.
- A store owns shared application state, not presentation details.
- A service owns reusable asynchronous or runtime behavior.
- A utility should be pure whenever practical.

Good reasons to split a file:

- multiple independent panels or views
- unrelated event lifecycles
- a large template plus large business logic
- reusable keyboard, animation, or persistence logic
- multiple data domains in one store

Do not split a file into arbitrary fragments that only forward props without improving ownership.

## 3. TypeScript strictness

- Keep `strict` enabled.
- Do not add `@ts-ignore` or disable compiler rules to avoid fixing a type problem.
- Prefer explicit domain types over broad records.
- Use `unknown` at untrusted boundaries and narrow it.
- Avoid `any`.
- When an external library forces `any`, isolate it at the boundary and document why.
- Do not use non-null assertions unless the invariant is local and obvious.
- Prefer discriminated unions for state machines and variants.

Example:

```ts
type IndexStage =
  | { type: 'idle' }
  | { type: 'running'; current: number; total: number }
  | { type: 'error'; message: string }
```

Do not encode the same state as unrelated booleans that can become contradictory.

## 4. Naming

Use names that describe product meaning, not implementation accidents.

- Components: `PascalCase.vue`
- Stores: `useXStore`
- Composables: `useX`
- Async actions: verbs such as `load`, `refresh`, `open`, `save`, `prepare`
- Boolean values: `is`, `has`, `can`, `should`
- Event handlers: `handleX`
- Event emitters: action names such as `close`, `select`, `addFolder`
- Constants: `UPPER_SNAKE_CASE` only for real constants

Avoid vague names such as `data`, `thing`, `item2`, `manager`, or `handleStuff`.

## 5. Vue component conventions

Use:

```vue
<script setup lang="ts">
```

Prefer this order inside a component:

1. imports
2. props and emits
3. stores and injected dependencies
4. refs and reactive state
5. computed values
6. watchers
7. actions and handlers
8. lifecycle hooks

This order is a guideline, not a reason to keep unrelated logic in one file.

### Props

- Type props explicitly.
- Use `withDefaults` for optional values with defaults.
- Do not mutate props.
- Do not pass a large store object as one prop when a child only needs a few values.
- Prefer stable primitive or typed object props.

### Emits

Type emits explicitly:

```ts
const emit = defineEmits<{
  close: []
  select: [imageId: string]
  updateQuery: [value: string]
}>()
```

Use event names that describe user intent.

### Exposed methods

Use `defineExpose` sparingly for imperative behaviors such as:

- focus
- select
- scroll to an item

Do not expose internal state that should be modeled through props and emits.

## 6. State ownership

Each state value must have one clear owner.

Use local component state for:

- transient visual state
- temporary hover or focus state
- local animation state
- local draft values

Use Pinia for:

- shared state used by multiple components or windows
- persisted user preferences
- library state
- indexing state
- platform information

Use services for:

- model/runtime initialization
- reusable async pipelines
- caching independent from Vue rendering
- concurrency control

Do not mirror store state into local refs unless editing a draft or buffering user input. When mirroring is necessary, define the synchronization direction clearly.

## 7. Persistence

User-facing preferences must persist locally and apply immediately when possible.

- Centralize each preference in one store or service.
- Use stable, namespaced keys such as `imagyx.theme`.
- Version cached structures when their schema may change.
- Parse persisted values defensively.
- Provide a safe default when local data is invalid.
- Do not scatter direct `localStorage` calls across components.

Security-sensitive or native settings should be persisted through Rust rather than directly from the UI.

## 8. Tauri API boundary

All frontend `invoke` calls belong in `src/api/tauri.ts` or another dedicated typed API module.

Components and stores should call typed wrapper methods rather than raw command names.

The wrapper must:

- type command inputs and outputs
- normalize optional values
- use frontend naming conventions
- hide transport details

Do not call `invoke()` directly from presentation components.

## 9. Async behavior and race prevention

Search, indexing, thumbnails, and model loading can overlap. Code must explicitly handle stale work.

Use one or more of:

- sequence numbers
- request IDs
- `AbortController` where supported
- shared in-flight promises
- deduplication sets
- result caches

Before applying an async result, verify that it still belongs to the active request.

Example:

```ts
const sequence = ++searchSequence
const result = await runSearch(query)
if (sequence !== searchSequence) return
results.value = result
```

Do not let an older request overwrite a newer search.

## 10. Watchers

- Watch the smallest relevant source.
- Avoid deep watchers on large objects.
- Do not perform unbounded expensive work directly in a watcher.
- Debounce search-like work.
- Clean up timers and subscriptions.
- Use `flush` options only when the render timing requirement is understood.

A watcher should coordinate a side effect, not replace a computed property.

## 11. Computed values and render paths

Computed properties must remain cheap and deterministic.

Do not perform in a computed or template expression:

- filesystem or Tauri calls
- model inference
- sorting thousands of items repeatedly
- large allocations
- mutation
- timer creation

Precompute or cache expensive derived data in a service or store.

Avoid creating new large arrays or objects in templates on every render.

## 12. Event listeners and lifecycle

Every listener must have a clear cleanup path.

This includes:

- `window` listeners
- Tauri event listeners
- media query listeners
- timers
- animation frames
- observers

Store returned unlisten functions and call them in `onBeforeUnmount`.

Do not register duplicate global listeners when a store or service can own one shared subscription.

## 13. Keyboard behavior

Global keyboard handlers must ignore events originating from:

- `input`
- `textarea`
- `select`
- `[contenteditable="true"]`

Do not intercept shortcuts when a native or text-editing behavior should win.

Keyboard behavior must match visible focus and selection state.

Do not rely on hover state as the only keyboard target.

## 14. Errors

- Catch errors at the layer that can add useful context or recover.
- Do not silently swallow unexpected failures.
- Convert unknown errors with a helper or `String(error)` only at a UI boundary.
- Preserve the original error when adding context.
- Use stores or toast services for user-visible application errors.
- Avoid displaying raw implementation details unless in a debug surface.

Empty `catch` blocks are allowed only for explicitly optional fallback behavior and must include a comment explaining why failure is safe.

## 15. Loading state

Avoid one generic `loading` boolean for unrelated operations.

Use focused states such as:

- `searching`
- `copyingImageId`
- `shortcutUpdating`
- `modelLoading`
- `indexingStage`

Loading state must be reset in `finally` when the active request still owns it.

## 16. Caching

Cache expensive stable work such as:

- loaded runtime models
- text concept embeddings
- image explanations
- thumbnails
- repeated Spotlight searches
- top tags

Rules:

- bound caches where entries can grow indefinitely
- define invalidation behavior
- do not cache user-visible stale state without a refresh path
- share in-flight promises to prevent duplicate loading

## 17. Performance

- Keep image grids virtualized.
- Bound result sets returned to Spotlight.
- Keep the lexical search path available while semantic ranking completes.
- Lazy-load per-image details.
- Reuse stored image embeddings.
- Avoid serial processing when bounded parallel work is safe.
- Avoid loading the vision model in a text-only Spotlight process.
- Prewarm expensive runtime paths invisibly when it does not delay useful UI.

Measure before introducing complex abstractions.

## 18. CSS in Vue files

Follow `docs/UI.md`.

Additionally:

- Keep scoped styles focused on the component's own DOM.
- Do not use `:deep()` to modify shared UI internals.
- Prefer shared tokens over hardcoded values.
- Avoid global selectors inside feature components.
- Put truly global tokens and application layout rules in `src/style.css`.
- Avoid JavaScript-driven styling when CSS states and transitions are sufficient.

## 19. Imports

- Remove unused imports.
- Prefer relative imports consistent with the existing project structure.
- Keep import groups readable.
- Do not create circular dependencies between stores and services.
- Avoid importing a feature parent into a shared primitive.

Shared layers must not depend on feature layers.

## 20. Comments

Comments should explain:

- why a non-obvious workaround exists
- why a race guard is needed
- why a fallback is safe
- a platform-specific constraint

Do not comment obvious syntax.

Remove stale comments when behavior changes.

## 21. Tests

Add tests for logic that is easy to isolate and likely to regress:

- parsing persisted preferences
- normalization helpers
- search tokenization
- cache invalidation
- state transitions
- keyboard normalization

Prefer pure functions for testable logic.

UI behavior that depends on Tauri or native windows should also be validated manually in `tauri dev`.

## 22. Validation commands

Run the relevant checks before considering a change complete:

```bash
npm run typecheck
npm run test
```

When frontend behavior interacts with Tauri commands or native windows, also run:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

Do not claim a check passed unless it was actually run.

## 23. Review checklist

- File is below 500 lines.
- Component or module has one clear responsibility.
- Strict typing is preserved.
- No unnecessary `any`, assertion, or ignored compiler error was added.
- Props and emits are typed.
- Shared state has one owner.
- Raw `invoke()` is not used outside the API wrapper.
- Async results cannot overwrite newer state.
- Timers and listeners are cleaned up.
- Expensive work is not in render paths.
- Cache growth is bounded or justified.
- Input fields are protected from global keyboard handlers.
- Errors are contextual and visible when appropriate.
- Relevant checks were run and reported honestly.
