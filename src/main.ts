import { createApp } from "vue";
import { createPinia } from "pinia";
import { getCurrentWindow } from "@tauri-apps/api/window";
import App from "./App.vue";
import SpotlightSearch from "./components/Spotlight/SpotlightSearch.vue";
import { installPerformanceDiagnostics } from "./utils";
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

const RootComponent =
  currentWindowLabel === "spotlight" ? SpotlightSearch : App;
createApp(RootComponent).use(createPinia()).mount("#app");

