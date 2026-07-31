import { createApp } from "vue";
import { createPinia } from "pinia";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.vue";
import SpotlightSearch from "./components/Spotlight/SpotlightSearch.vue";
import { installSpotlightConverterKeyboardGuard } from "./services/spotlight-converter-keyboard";
import { semanticRuntime } from "./services/semantic";
import { installUnifiedSearchEngine } from "./services/unified-search-engine";
import { visualSearchSession } from "./services/visual-search-session";
import { installPerformanceDiagnostics } from "./utils";
import { initI18n } from "./i18n";
import "./style.css";
import "./virtual-grid.css";
import "./spotlight-window.css";

installPerformanceDiagnostics();

let currentWindowLabel = "main";
try {
  const currentWindow = getCurrentWindow();
  currentWindowLabel = currentWindow.label;
} catch {
  // Running in browser context without Tauri APIs
}

const isDemoServer =
  window.location.port === "8500" ||
  Boolean(
    (import.meta as { env?: { DEV?: boolean } }).env?.DEV &&
    window.location.port === "8500",
  );

const savedTheme = localStorage.getItem("imagyx-theme");
const resolvedTheme =
  savedTheme === "light" || savedTheme === "dark"
    ? savedTheme
    : isDemoServer
      ? "light"
      : window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light";

document.documentElement.dataset.theme = resolvedTheme;
document.documentElement.dataset.window = currentWindowLabel;
document.documentElement.style.colorScheme = resolvedTheme;

installUnifiedSearchEngine();
if (currentWindowLabel === "main") {
  // Start loading before Vue mounts the main UI so an immediately opened
  // Spotlight has the shortest possible path to a text embedding.
  void semanticRuntime.prewarmText().catch(() => undefined);
}
if (currentWindowLabel === "spotlight") {
  installSpotlightConverterKeyboardGuard();
  void listen("spotlight-opened", () => {
    // Spotlight intentionally resets its local input while opening. Re-emit the
    // retained visual session after that reset so the query and its results are
    // restored together instead of showing an orphaned reference thumbnail.
    window.setTimeout(() => {
      const state = visualSearchSession.state;
      if (state.status !== "ready" || !state.token) return;
      window.dispatchEvent(new CustomEvent(visualSearchSession.eventName, {
        detail: { status: state.status, token: state.token },
      }));
    }, 0);
  });
}

const i18n = initI18n();
const RootComponent =
  currentWindowLabel === "spotlight" ? SpotlightSearch : App;
createApp(RootComponent).use(createPinia()).use(i18n).mount("#app");
