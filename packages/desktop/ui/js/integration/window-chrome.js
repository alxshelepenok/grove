import { tauriBridge } from "./tauri-bridge.js";

const updateMaximizeButton = async () => {
  const btn = document.querySelector('[aria-label="Maximize"], [aria-label="Restore"]');
  if (!btn) return;
  const isMax = await tauriBridge.isMaximized();
  btn.setAttribute("aria-label", isMax ? "Restore" : "Maximize");
}

export const initWindowChrome = () => {
  if (!tauriBridge.available) return;

  if (window.workspace?.appEnv === "production") {
    window.addEventListener("contextmenu", (e) => e.preventDefault(), true);
    window.addEventListener("keydown", (e) => {
      if (e.key === "F12" || (e.ctrlKey && e.shiftKey && (e.key === "I" || e.key === "J" || e.key === "C")) || (e.ctrlKey && e.key === "U")) {
        e.preventDefault();
      }
    }, true);
  }

  document.querySelector('[aria-label="Minimize"]')
    ?.addEventListener("click", () => tauriBridge.minimize());

  document.querySelector('[aria-label="Maximize"]')
    ?.addEventListener("click", () => {
      tauriBridge.toggleMaximize().then(() => updateMaximizeButton());
    });

  document.querySelector('[aria-label="Close"]')
    ?.addEventListener("click", () => tauriBridge.close());

  const titleBar = document.querySelector(".title-bar");

  titleBar?.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    if (e.target.closest(".title-bar-controls")) return;
    e.preventDefault();
    tauriBridge.startDragging();
  });

  titleBar?.addEventListener("dblclick", (e) => {
    if (e.target.closest(".title-bar-controls")) return;
    tauriBridge.toggleMaximize().then(() => updateMaximizeButton());
  });

  window.addEventListener("resize", () => updateMaximizeButton());

  updateMaximizeButton();
}
