const CONVERTER_SELECTOR = ".spotlight-converter";
const BACK_BUTTON_SELECTOR = ".spotlight-conversion-input__back";
const CONVERT_BUTTON_SELECTOR = ".spotlight-converter__command";

let installed = false;

function converterIsOpen(): boolean {
  return document.querySelector(CONVERTER_SELECTOR) !== null;
}

function clickEnabledButton(selector: string): boolean {
  const button = document.querySelector<HTMLButtonElement>(selector);
  if (!button || button.disabled) return false;
  button.click();
  return true;
}

function handleConverterKeydown(event: KeyboardEvent): void {
  if (event.isComposing || !converterIsOpen()) return;
  if (event.key !== "Escape" && event.key !== "Enter") return;

  // This listener is installed before Vue mounts so converter keys never reach
  // Spotlight's global Escape/Enter handlers and accidentally hide/open Imagyx.
  event.preventDefault();
  event.stopImmediatePropagation();

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
}

export function installSpotlightConverterKeyboardGuard(): void {
  if (installed || typeof window === "undefined") return;
  installed = true;
  window.addEventListener("keydown", handleConverterKeydown, { capture: true });
}
