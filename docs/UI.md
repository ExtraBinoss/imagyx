# UI and Product Interface Guidelines

This document defines the UI rules for Imagyx. It applies to Vue components, shared styles, native-window surfaces, onboarding previews, Spotlight, dialogs, popovers, and all interactive product states.

## 1. Core principles

- Imagyx should feel like one coherent desktop product, not a collection of separately styled pages.
- Prefer clarity, responsiveness, and predictable interaction over decorative complexity.
- Reuse existing components and design tokens before adding new markup or CSS.
- Keep UI work local-first: no remote assets or network-dependent visuals for core product behavior.
- Every interaction must work in light and dark themes.
- Every animation must remain usable with `prefers-reduced-motion`.

## 2. Component hierarchy

### Shared primitives

Generic reusable controls belong in `src/components/ui/`.

Current primitives include, among others:

- `Button/Button.vue`
- `ButtonGroup/ButtonGroup.vue`
- `Input/Input.vue`
- `Badge/Badge.vue`
- `Popover/Popover.vue`
- `Accordion/Accordion.vue`
- `MovingBorder/MovingBorder.vue`
- `Skeleton/Skeleton.vue`
- toast and dialog primitives

Before creating a component:

1. Search `src/components/ui/` for an existing primitive.
2. Search feature folders for a reusable implementation.
3. Extend the existing public API with a prop, slot, variant, size, or state.
4. Create a new primitive only when the behavior is genuinely generic.

Do not copy the markup or CSS of an existing component into another file.

### Feature components

Feature-specific components belong together:

- `src/components/Spotlight/`
- `src/components/Onboarding/`
- other feature folders following the same pattern

A feature parent coordinates data and state. Child components own focused presentation or interaction responsibilities.

### Keyboard components

Reusable keybinding and keyboard visualizations belong in `src/components/shortcuts/`.

Do not recreate key badges or shortcut recording behavior in a feature component.

## 3. Buttons and clickable actions

All application actions must use `src/components/ui/Button/Button.vue` unless a native element is technically required by the platform or accessibility semantics.

This includes:

- icon buttons
- menu actions
- dialog actions
- pagination or step controls
- toolbar actions
- result actions
- settings choices
- onboarding navigation

Do not style raw `<button>` elements locally.

Use the existing Button API:

- `variant`
- `size`
- `block`
- `pressed`
- `depth`
- leading and trailing slots

When a new visual state is needed, add a reusable variant or prop to `Button.vue` rather than overriding its internals.

## 4. No private component overrides

Do not use `:deep()` to reach into a shared UI component.

Do not depend on private class names such as `.ui-button__content` from outside the component.

Instead:

- add a public prop
- add a named slot
- add a public variant or size
- expose a documented root class contract
- compose the component inside a wrapper

Shared components must remain independently refactorable.

## 5. Design tokens

Use the CSS variables defined in `src/style.css`.

Prefer tokens such as:

- `--background`
- `--surface`
- `--surface-elevated`
- `--surface-hover`
- `--border`
- `--border-strong`
- `--text`
- `--text-muted`
- `--primary`
- `--primary-soft`
- `--focus-ring`
- radius, spacing, typography, shadow, and transition tokens

Do not introduce a second token system inside a component.

Hardcoded colors are allowed only for illustrative content where the color is the content itself, such as onboarding demo artwork. Product chrome must use theme tokens.

When adding a new token:

1. Define it for both light and dark themes.
2. Give it a semantic name, not a page-specific name.
3. Reuse it in more than one place or justify why it belongs globally.

## 6. Depth language

Imagyx uses restrained depth:

- a subtle top highlight
- a small, diffuse shadow
- a visible but not exaggerated hover lift
- a pressed state that feels physically inset
- stronger elevation only for dialogs, popovers, Spotlight, and floating windows

Avoid:

- large white glows
- heavy drop shadows clipped by native window bounds
- excessive blur
- multiple competing elevation styles
- glossy effects that reduce text contrast

Use the shared button depth variables and popover/dialog shadows instead of recreating depth per component.

## 7. Layout and sizing

- Avoid fixed dimensions when content can grow, except for native-window contracts and virtualized rows.
- Use `min-width: 0` on flexible grid and flex children that contain ellipsized text.
- Use `box-sizing: border-box` consistently.
- Avoid horizontal overflow in sidebars, menus, and settings panels.
- Keep native transparent windows tightly sized to their interactive surface so transparent space does not block desktop clicks.
- For Spotlight, resize the native window before revealing expanded content.

A component must not rely on content being hidden outside a native window to create an animation.

## 8. Interaction states

Every interactive control must define:

- default state
- hover state
- focus-visible state
- active or pressed state
- disabled state when applicable
- loading state when an operation is not immediate
- error state when failure is recoverable

Hover-only actions must also become available through keyboard focus.

Do not move an action target while the pointer is interacting with it. Reserve layout space and animate opacity or transform rather than toggling layout-affecting display values.

Context menus and `...` menus should expose equivalent actions when they represent the same object.

## 9. Selection and focus

- Selected image styling covers the complete card, not only the image thumbnail.
- Focus rings must be visible in both themes.
- Do not remove outlines without providing an equivalent focus-visible treatment.
- Keyboard navigation must keep the selected result in view.
- Typing in the main application starts a search only when the user is not editing an input, textarea, select, or contenteditable element.
- Space must not be intercepted while editing text.

## 10. Inputs and search

Use the shared input component or an existing feature input such as `SpotlightInput.vue`.

Search behavior should:

- provide a fast lexical result path first
- allow semantic ranking to complete afterward
- debounce expensive work
- ignore stale async results
- preserve input responsiveness while indexing
- avoid changing placeholder content in a way that flashes unrelated text

Animated placeholders must not interfere with typed text or accessibility labels.

## 11. Dialogs and popovers

Dialogs must support the appropriate subset of:

- Escape to close
- click outside to close
- explicit close button when product requirements call for it
- focus containment or a clear focus return path
- entry and exit animation
- `aria-modal`, label, and description relationships

Popovers must:

- remain stable under the pointer
- fit within the window bounds
- not be wrapped in conflicting hover tooltips
- close predictably on outside interaction

Do not use native `title` attributes for paths or duplicated information.

## 12. Animation

Animations must be:

- fast
- interruptible
- symmetric on entry and exit
- based primarily on opacity and transform
- synchronized with native window resizing when applicable
- disabled or reduced under `prefers-reduced-motion`

Recommended timings:

- micro-interactions: 120–180 ms
- menus and panels: 180–260 ms
- larger onboarding/dialog transitions: 240–380 ms

Use easing curves already present in the codebase, especially `cubic-bezier(0.16, 1, 0.3, 1)` for responsive spring-like motion.

Avoid:

- infinite decorative animation that competes with content
- layout thrashing in JavaScript
- corner speed changes in border animations
- showing content before the native window has resized
- entry animation without a matching exit animation

## 13. Loading, progress, and background work

Long-running work must expose a clear state:

- discovering
- reading metadata
- queued
- embedding
- saving
- complete
- error

Progress should be determinate when real totals are known. Use an indeterminate state only when the total cannot yet be calculated.

Indexing UI must not block searching, navigation, or Spotlight.

Completed indexing batches must already be persisted; UI progress is never a substitute for durable progress.

## 14. Image grids and large result sets

- Use virtualization for large image collections.
- Do not mount thousands of cards at once.
- Do not eagerly calculate tags or explanations for every result.
- Calculate hover details lazily and cache them.
- Keep thumbnail rendering independent from full-resolution preview loading.
- Bound Spotlight result counts even when the library contains many more images.

Avoid expensive filters, sorting, or allocation in template render paths.

## 15. Spotlight-specific rules

Spotlight is a native secondary window, not a modal inside the main application.

Preserve these behaviors:

- compact search-only state
- one continuous surface when results or settings expand
- native window resize before panel reveal
- launch near 25% of the usable screen height
- transparent space outside the panel should not intercept desktop interaction
- result actions appear on hover and focus
- Escape returns from Settings before closing Spotlight
- Settings can search only settings
- adding a folder can continue in the background while Spotlight remains searchable

Do not add labels that explain a shortcut while the user is already inside Spotlight unless they are part of Settings.

## 16. Onboarding-specific rules

Onboarding should render lightweight live versions of real components rather than screenshots whenever practical.

It must:

- appear on first use
- persist completion locally
- offer “do not ask again” behavior
- be closable
- support previous and next navigation
- show component name, title, and description
- demonstrate Sidebar, Search, ImageGrid, Preview, Spotlight, Settings, and indexing
- allow adding a folder directly
- be relaunchable from Settings

Onboarding previews must use shared primitives and must not fork production component styles.

## 17. Accessibility

- Icon-only buttons require an `aria-label`.
- Dialogs require an accessible title.
- Lists with keyboard selection should expose appropriate roles and selected state.
- Informative progress should use meaningful text in addition to visual bars.
- Do not rely on color alone for completion, error, or selection.
- Preserve readable contrast in light and dark themes.
- Respect reduced motion.

## 18. UI review checklist

Before finishing a UI change, verify:

- Existing primitives were reused.
- No shared component markup or CSS was copied.
- No raw styled button was introduced.
- No `:deep()` override was added.
- Light and dark themes are coherent.
- Hover behavior also works through focus.
- Entry and exit animations are both present.
- Reduced motion is respected.
- Long work has progress or loading feedback.
- Large collections remain virtualized or bounded.
- Native window bounds do not clip content or shadows.
- The component file remains below 500 lines.
- Relevant local checks were run and reported honestly.
