import { tauriBridge } from "./integration/tauri-bridge.js";
import { initWindowChrome } from "./integration/window-chrome.js";
import { openProjectPicker } from "./integration/project-picker.js";
import { SearchableSelect } from "./utils/searchable-select.js";
import { initFloatingTooltips } from "./utils/tooltip-float.js";
import { openCommandMenu } from "./utils/command-menu.js";

const escapeHtml = (value) =>
  String(value).replace(
    /[&<>"']/g,
    (c) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[c],
  );

const setActiveNav = (level) => {
  for (const el of document.querySelectorAll(".side-rail-item[data-level]")) {
    if (el.dataset.level === level) {
      el.setAttribute("aria-selected", "true");
    } else {
      el.removeAttribute("aria-selected");
    }
  }
};

const wirePacketSelect = () => {
  const container = document.getElementById("packet-select");
  if (!container) return;
  new SearchableSelect({
    container,
    placeholder: "Select a work item...",
    emptyText: "No work items",
    renderOption: (item) => item.label,
    onSelect: (item) => loadView("packet", { id: item.id }),
  });
};

let viewCleanup = null;
let currentLevel = "overview";
let currentParams = {};

const ROUTES = new Set([
  "overview",
  "areas",
  "discovery",
  "goals",
  "work",
  "themes",
  "graph",
  "packet",
]);

const DEV_ROUTES = new Set(["debug"]);
const isDevEnv = () => window.workspace?.appEnv === "development";

const reloadCurrentView = () => loadView(currentLevel, currentParams);

const STATUS_METRIC_IDS = ["c", "v", "g", "ready", "done", "lock", "updated"];

const refreshStatusMetrics = async () => {
  if (!tauriBridge.available) return;
  let metrics;
  try {
    metrics = await tauriBridge.invoke("grove_status_metrics");
  } catch (e) {
    if (String(e?.message ?? e).startsWith("no_project:")) {
      for (const key of STATUS_METRIC_IDS) {
        const el = document.getElementById(`status-${key}`);
        if (el) el.textContent = "n/a";
      }
      const badge = document.getElementById("goals-badge");
      if (badge) badge.hidden = true;
    }
    return;
  }
  for (const key of STATUS_METRIC_IDS) {
    const el = document.getElementById(`status-${key}`);
    if (el) el.textContent = String(metrics?.[key] ?? "");
  }
  const badge = document.getElementById("goals-badge");
  if (badge) {
    const n = Number(metrics?.g) || 0;
    badge.textContent = String(n);
    badge.hidden = !n;
    badge.classList.toggle("badge-count-wide", n > 99);
  }
};

const wireView = async (level) => {
  const viewRoot = document.getElementById("view-root");
  const cleanups = [];
  if (level === "packet") {
    wirePacketSelect();
    const mod = await import("./views/packet.js");
    cleanups.push(mod.wirePacket(viewRoot, { reload: reloadCurrentView }));
  }
  if (level === "overview") {
    const mod = await import("./views/overview.js");
    cleanups.push(mod.wireOverview(viewRoot, { navigate: loadView }) ?? null);
  }
  if (level === "areas") {
    const mod = await import("./views/areas.js");
    cleanups.push(mod.wireAreas(viewRoot, { navigate: loadView }) ?? null);
  }
  if (level === "work") {
    const mod = await import("./views/work.js");
    cleanups.push(mod.wireWork(viewRoot, { navigate: loadView }) ?? null);
  }
  if (level === "themes") {
    const mod = await import("./views/themes.js");
    cleanups.push(mod.wireThemes(viewRoot, { navigate: loadView }) ?? null);
  }
  if (level === "graph") {
    const mod = await import("./views/graph.js");
    cleanups.push(mod.initGraph(viewRoot, { navigate: loadView }) ?? null);
  }
  const addMod = await import("./views/add-node.js");
  cleanups.push(addMod.wireAddModal(viewRoot, { reload: reloadCurrentView }));
  viewCleanup = () => {
    for (const c of cleanups) c?.();
  };
};

const loadView = async (level, params = {}) => {
  const devRoute = isDevEnv() && DEV_ROUTES.has(level);
  if (!devRoute && !ROUTES.has(level)) {
    level = "overview";
    params = {};
  }
  const scrollArea = document.getElementById("app-scrollable-area");
  const keepScroll =
    level === currentLevel &&
    JSON.stringify(params) === JSON.stringify(currentParams);
  const prevTop = scrollArea ? scrollArea.scrollTop : 0;
  currentLevel = level;
  currentParams = params;
  setActiveNav(level);
  const viewRoot = document.getElementById("view-root");
  if (!viewRoot) return;
  const prevCleanup = viewCleanup;
  viewCleanup = null;
  try {
    if (devRoute) {
      const mod = await import("./views/debug.js");
      await mod.renderDebug(viewRoot);
    } else {
      const html = await tauriBridge.invoke("grove_view", { level, params });
      viewRoot.innerHTML = html;
    }
    prevCleanup?.();
    if (scrollArea) scrollArea.scrollTop = keepScroll ? prevTop : 0;
    await wireView(level);
    await refreshStatusMetrics();
  } catch (e) {
    prevCleanup?.();
    viewRoot.innerHTML = `<div class="alert alert-danger" role="alert"><div class="alert-content"><p class="alert-title">View failed</p><p class="alert-description">${escapeHtml(e?.message ?? e)}</p></div></div>`;
  }
};

const initNav = () => {
  for (const el of document.querySelectorAll(".side-rail-item[data-level]")) {
    el.querySelector("button")?.addEventListener("click", () =>
      loadView(el.dataset.level),
    );
  }
  document.getElementById("add-node-open")?.addEventListener("click", () => {
    document.getElementById("add-node-modal")?.classList.add("active");
    document.getElementById("add-node-title")?.focus();
  });
};

const setRailEnabled = (enabled) => {
  for (const el of document.querySelectorAll(
    ".side-rail-item[data-level], .side-rail-action:not(.side-rail-project)",
  )) {
    el.classList.toggle("disabled", !enabled);
    const btn = el.querySelector("button");
    if (!btn) continue;
    if (enabled) {
      btn.removeAttribute("aria-disabled");
    } else {
      btn.setAttribute("aria-disabled", "true");
    }
  }
};

const initProjectState = () => {
  document.addEventListener(
    "click",
    (e) => {
      if (e.target.closest(".side-rail-item.disabled")) {
        e.preventDefault();
        e.stopPropagation();
      }
    },
    true,
  );
  document.addEventListener("click", (e) => {
    const btn = e.target.closest('[data-action="project-picker"]');
    if (!btn) return;
    window.dispatchEvent(
      new CustomEvent("grove:project-picker", {
        detail: { mode: btn.dataset.mode ?? "default" },
      }),
    );
  });
  window.addEventListener("grove:project-picker", (e) => {
    openProjectPicker(e?.detail?.mode ?? "default", { navigate: loadView });
  });
  window.addEventListener("grove:project-changed", () =>
    refreshProjectState(),
  );
};

const restoreTreeTile = (tile) => {
  const icon = document.createElement("span");
  icon.className = "nav-icon nav-icon-tree";
  tile.replaceChildren(icon);
};

let avatarToken = 0;

const renderProjectAvatar = async (current) => {
  const tile = document.getElementById("project-avatar");
  if (!tile) return;
  const token = ++avatarToken;
  const name = String(current?.name ?? "").trim();
  let svg = null;
  if (name && tauriBridge.available) {
    try {
      svg = await tauriBridge.invoke("grove_project_avatar", { name });
    } catch {
      svg = null;
    }
  }
  if (token !== avatarToken) return;
  if (svg) {
    tile.innerHTML = svg;
    return;
  }
  restoreTreeTile(tile);
};

const refreshProjectState = async () => {
  let current = null;
  if (tauriBridge.available) {
    try {
      current = await tauriBridge.invoke("grove_project_current");
    } catch {
      current = null;
    }
  }
  renderProjectAvatar(current);
  setRailEnabled(!!current);
  const statusBar = document.querySelector(".status-bar");
  if (statusBar) statusBar.hidden = !current;
  const addAction = document.querySelector(
    ".side-rail-action:not(.side-rail-project)",
  );
  if (addAction) addAction.classList.toggle("side-rail-action-hidden", !current);
  await loadView(currentLevel, currentParams);
};

const initCommandPalette = () => {
  document.addEventListener("keydown", (e) => {
    if (!(e.ctrlKey && e.shiftKey && e.key === "P")) return;
    if (document.querySelector(".modal-overlay.active")) return;
    e.preventDefault();
    openProjectPicker("default", { navigate: loadView });
  });
};

const reveal = () => document.documentElement.classList.add("fonts-ready");

initWindowChrome();
initFloatingTooltips();
initNav();
initProjectState();
initCommandPalette();
refreshProjectState();
if (document.fonts?.ready) {
  document.fonts.ready.then(reveal, reveal);
  setTimeout(reveal, 800);
} else {
  reveal();
}
