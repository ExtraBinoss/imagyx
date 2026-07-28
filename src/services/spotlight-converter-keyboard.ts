const CONVERTER_SELECTOR = ".spotlight-converter";
const BACK_BUTTON_SELECTOR = ".spotlight-conversion-input__back";
const CONVERT_BUTTON_SELECTOR = ".spotlight-converter__command";
const VISUAL_SEARCH_SELECTOR = ".unified-search-input--visual";
const VISUAL_BACK_BUTTON_SELECTOR = ".unified-search-input--visual .unified-search-input__nav";
const ACTION_POPOVER_SELECTOR = ".spotlight-action-popover";
const MORE_ACTIONS_BUTTON_SELECTOR = ".spotlight-dock-button--more";

let installed = false;

function converterIsOpen(): boolean {
  return document.querySelector(CONVERTER_SELECTOR) !== null;
}

function visualSearchIsOpen(): boolean {
  return document.querySelector(VISUAL_SEARCH_SELECTOR) !== null;
}

function actionPopoverIsOpen(): boolean {
  return document.querySelector(ACTION_POPOVER_SELECTOR) !== null;
}

function enabledButton(selector: string): HTMLButtonElement | null {
  const button = document.querySelector<HTMLButtonElement>(selector);
  return button && !button.disabled ? button : null;
}

function clickEnabledButton(selector: string): boolean {
  const button = enabledButton(selector);
  if (!button) return false;
  button.click();
  return true;
}

function consume(event: KeyboardEvent): void {
  event.preventDefault();
  event.stopImmediatePropagation();
}

function handleConverterKeydown(event: KeyboardEvent): void {
  if (event.isComposing) return;

  if (converterIsOpen()) {
    if (event.key !== "Escape" && event.key !== "Enter") return;

    // Installed before Vue mounts so converter keys never reach Spotlight's
    // global Escape/Enter handlers and accidentally hide/open Imagyx.
    consume(event);

    if (event.key === "Escape") {
      const wentBack = clickEnabledButton(BACK_BUTTON_SELECTOR);
      if (import.meta.env.DEV) {
        console.debug("[Imagyx][SpotlightConverterKeyboard] Escape", {
          action: wentBack ? "back-to-search" : "ignored-while-converting",
        });
      }
      return;
    }

    const hasModifier = event.ctrlKey || event.metaKey || event.altKey || event.shiftKey;
    const started = !event.repeat && !hasModifier
      ? clickEnabledButton(CONVERT_BUTTON_SELECTOR)
      : false;

    if (import.meta.env.DEV) {
      console.debug("[Imagyx][SpotlightConverterKeyboard] Enter", {
        action: started ? "convert" : "consumed",
        repeat: event.repeat,
        hasModifier,
      });
    }
    return;
  }

  if (event.key === "Escape" && visualSearchIsOpen()) {
    consume(event);
    const wentBack = clickEnabledButton(VISUAL_BACK_BUTTON_SELECTOR);
    if (import.meta.env.DEV) {
      console.debug("[Imagyx][SpotlightVisualSearchKeyboard] Escape", {
        action: wentBack ? "back-to-previous-search" : "consumed",
      });
    }
    return;
  }

  const key = event.key.toLocaleLowerCase();
  const primaryModifier = event.ctrlKey || event.metaKey;
  if (primaryModifier && !event.shiftKey && !event.altKey && key === "k") {
    if (!enabledButton(MORE_ACTIONS_BUTTON_SELECTOR)) return;
    consume(event);
    const toggled = !event.repeat && clickEnabledButton(MORE_ACTIONS_BUTTON_SELECTOR);
    if (import.meta.env.DEV) {
      console.debug("[Imagyx][SpotlightActionDockKeyboard] Ctrl+K", {
        action: toggled ? "toggle-actions" : "consumed-repeat",
        repeat: event.repeat,
      });
    }
    return;
  }

  if (event.key === "Escape" && actionPopoverIsOpen()) {
    consume(event);
    const closed = clickEnabledButton(MORE_ACTIONS_BUTTON_SELECTOR);
    if (import.meta.env.DEV) {
      console.debug("[Imagyx][SpotlightActionDockKeyboard] Escape", {
        action: closed ? "close-actions" : "consumed",
      });
    }
  }
}

export function installSpotlightConverterKeyboardGuard(): void {
  if (installed || typeof window === "undefined") return;
  installed = true;
  window.addEventListener("keydown", handleConverterKeydown, { capture: true });
}
