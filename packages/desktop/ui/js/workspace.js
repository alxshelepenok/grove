const resolveAppEnv = () => {
  const injected = window.__GROVE_APP_ENV__;
  if (injected === "development" || injected === "production") return injected;
  return window.location.protocol === "tauri:" ||
    window.location.hostname === "tauri.localhost"
      ? "production"
      : "development";
};

const resolvePlatform = () => {
  const injected = window.__GROVE_PLATFORM__;
  if (injected === "windows" || injected === "macos" || injected === "linux") {
    return injected;
  }

  return navigator.userAgent.includes("Windows") ? "windows" : "linux";
};

window.workspace = {
  target: "grove-desktop",
  appEnv: resolveAppEnv(),
  platform: resolvePlatform(),
  isTauri: typeof window.__TAURI_INTERNALS__ !== "undefined",
};

document.documentElement.dataset.platform = window.workspace.platform;
