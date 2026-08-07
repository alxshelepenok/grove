import { tauriBridge } from "../integration/tauri-bridge.js";

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

const invokeOrNull = async (cmd) => {
  try {
    return await tauriBridge.invoke(cmd);
  } catch {
    return null;
  }
};

const row = (label, valueHtml) =>
  `<tr><td>${label}</td><td>${valueHtml}</td></tr>`;

const METRIC_ROWS = [
  ["c", "Content (C)"],
  ["v", "Uncertainty (V)"],
  ["g", "Goals (G)"],
  ["ready", "Work ready (W)"],
  ["done", "Work done (W)"],
  ["updated", "Updated"],
];

const metricRows = (metrics) =>
  METRIC_ROWS.map(([key, label]) =>
    row(label, `<span class="text-mono">${escapeHtml(metrics?.[key] ?? "n/a")}</span>`),
  ).join("");

const projectRows = (current) => {
  if (!current) {
    return row("Project", `<span class="text-muted">none</span>`);
  }
  return (
    row("Name", `<span id="debug-project-name">${escapeHtml(current.name ?? "")}</span>`) +
    row("Path", `<span class="text-mono">${escapeHtml(current.path ?? "")}</span>`)
  );
};

const sessionHtml = (present) => {
  if (present === null) return `<span class="text-muted">n/a</span>`;
  return present
    ? `<span class="badge badge-success">present</span>`
    : `<span class="badge badge-neutral">absent</span>`;
};

export const renderDebug = async (viewRoot) => {
  const appEnv = window.workspace?.appEnv ?? "unknown";
  const [current, list, metrics, session] = await Promise.all([
    invokeOrNull("grove_project_current"),
    invokeOrNull("grove_projects_list"),
    invokeOrNull("grove_status_metrics"),
    invokeOrNull("grove_session_present"),
  ]);
  const recents = list?.recents?.length ?? 0;
  viewRoot.innerHTML = `<section class="view view-debug">
  <div class="page-header">
    <div class="page-header-text">
      <h1>Debug</h1>
      <p class="page-description">Development-only diagnostics. This page never renders in production builds.</p>
    </div>
  </div>
  <div class="card" id="debug-environment">
    <div class="card-title"><span>Environment</span></div>
    <div class="sunken-panel">
      <table class="info-table">
        <tbody>
          ${row("appEnv", `<span class="text-mono" id="debug-appenv">${escapeHtml(appEnv)}</span>`)}
          ${row("Session", sessionHtml(session))}
          ${row("Registry recents", `<span class="text-mono" id="debug-recents">${escapeHtml(recents)}</span>`)}
        </tbody>
      </table>
    </div>
  </div>
  <div class="card" id="debug-project">
    <div class="card-title"><span>Project</span></div>
    <div class="sunken-panel">
      <table class="info-table">
        <tbody>
          ${projectRows(current)}
        </tbody>
      </table>
    </div>
  </div>
  <div class="card" id="debug-metrics">
    <div class="card-title"><span>Status metrics</span></div>
    <div class="sunken-panel">
      <table class="info-table">
        <tbody>
          ${metricRows(metrics)}
        </tbody>
      </table>
    </div>
  </div>
</section>`;
};
