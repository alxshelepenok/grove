const resolveAppEnv = () => {
  const injected = window.__GROVE_APP_ENV__;
  if (injected === "development" || injected === "production") return injected;
  return window.location.protocol === "tauri:" ||
    window.location.hostname === "tauri.localhost"
      ? "production"
      : "development";
};

window.workspace = {
  target: "grove-desktop",
  appEnv: resolveAppEnv(),
  isTauri: typeof window.__TAURI_INTERNALS__ !== "undefined",
};
