import { hslToRgb } from "./graph-model.js";

export const CELL_SPACING = 40;
export const STRATUM_GAP = 120;
export const LATTICE_LIFT = 15;
export const FILE_Y = -STRATUM_GAP;
export const WORK_Y = 0;
export const THEME_Y = STRATUM_GAP;
export const GOAL_Y = 2 * STRATUM_GAP;
export const AREA_Y = 3 * STRATUM_GAP;
export const NODE_SIZE = 30;
export const VARIANT_HUES = {
  neutral: [220, 8, 62],
  accent: [262, 70, 66],
  success: [145, 60, 55],
  danger: [0, 70, 62],
  warning: [45, 80, 60],
  info: [200, 45, 58],
};
export const ZONE_HUES = {
  backward: [228, 62, 64],
  seed: [220, 15, 86],
  forward: [38, 92, 62],
};

const avg = (xs) => xs.reduce((a, b) => a + b, 0) / Math.max(1, xs.length);

const memberAdjacency = (edges, byId) => {
  const preds = new Map();
  const succs = new Map();
  for (const e of edges) {
    if (!byId.has(e.from) || !byId.has(e.to)) continue;
    if (!preds.has(e.to)) preds.set(e.to, []);
    preds.get(e.to).push(e.from);
    if (!succs.has(e.from)) succs.set(e.from, []);
    succs.get(e.from).push(e.to);
  }
  return { preds, succs };
};

const walkHops = (seed, byId, next) => {
  const hops = new Map();
  if (!byId.has(seed)) return hops;
  hops.set(seed, 0);
  let frontier = [seed];
  let dist = 0;
  while (frontier.length) {
    dist += 1;
    const level = [];
    for (const id of frontier) {
      for (const n of next(id) ?? []) {
        if (hops.has(n)) continue;
        hops.set(n, dist);
        level.push(n);
      }
    }
    frontier = level;
  }
  return hops;
};

const zoneOf = (col) => (col < 0 ? "backward" : col > 0 ? "forward" : "seed");

const findFreeCell = (taken, col) => {
  for (let d = 0; d <= taken.size; d++) {
    for (const dc of [0, 1, -1, 2, -2, 3, -3, 4, -4]) {
      for (const dr of d === 0 ? [0] : [d, -d]) {
        const key = `${col + dc},${dr}`;
        if (!taken.has(key)) return [col + dc, dr];
      }
    }
  }
  return [col, 0];
};

export const parseConeModel = (blob) => {
  const seed = blob?.seed ?? "";
  const nodes = (blob?.nodes ?? []).map((n) => ({ ...n, kind: "w" }));
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const links = (blob?.edges ?? [])
    .filter((e) => byId.has(e.from) && byId.has(e.to))
    .map((e) => ({ source: e.from, target: e.to }));
  const { preds, succs } = memberAdjacency(blob?.edges ?? [], byId);
  const backward = walkHops(seed, byId, (id) => preds.get(id));
  const forward = walkHops(seed, byId, (id) => succs.get(id));
  const order = new Map((blob?.order ?? []).map((id, i) => [id, i + 1]));
  const critical = (blob?.critical ?? []).filter((id) => byId.has(id));
  const criticalPairs = new Set();
  for (let i = 0; i + 1 < critical.length; i++) {
    criticalPairs.add(`${critical[i]}>${critical[i + 1]}`);
    criticalPairs.add(`${critical[i + 1]}>${critical[i]}`);
  }
  const layers = new Map();
  for (const n of nodes) {
    n.seed = n.id === seed;
    const back = backward.get(n.id) ?? 0;
    const fwd = forward.get(n.id) ?? 0;
    n.hop = back > 0 ? -back : fwd;
    n.order = order.has(n.id) ? order.get(n.id) : null;
    const key = String(n.hop);
    if (!layers.has(key)) layers.set(key, []);
    layers.get(key).push(n);
  }
  let colMin = 0;
  let colMax = 0;
  let rowMin = 0;
  let rowMax = 0;
  const track = (col, row) => {
    colMin = Math.min(colMin, col);
    colMax = Math.max(colMax, col);
    rowMin = Math.min(rowMin, row);
    rowMax = Math.max(rowMax, row);
  };
  for (const layer of layers.values()) {
    layer.sort(
      (a, b) => (a.order ?? Infinity) - (b.order ?? Infinity) || a.id.localeCompare(b.id),
    );
    layer.forEach((n, i) => {
      n.col = n.hop;
      n.row = i - Math.floor(layer.length / 2);
      n.zone = zoneOf(n.col);
      n.position = [n.col * CELL_SPACING, WORK_Y + LATTICE_LIFT, n.row * CELL_SPACING];
      track(n.col, n.row);
    });
  }

  const colOf = new Map(nodes.map((n) => [n.id, n.col]));
  const strata = [];
  const verticalLinks = [];
  const planes = new Map([
    ["g", { y: GOAL_Y, taken: new Set() }],
    ["a", { y: AREA_Y, taken: new Set() }],
    ["t", { y: THEME_Y, taken: new Set() }],
    ["f", { y: FILE_Y, taken: new Set() }],
  ]);
  const place = (entry, kind, cols) => {
    const plane = planes.get(kind);
    const home = Math.max(colMin, Math.min(colMax, Math.round(avg(cols))));
    const [col, row] = findFreeCell(plane.taken, home);
    plane.taken.add(`${col},${row}`);
    const e = {
      ...entry,
      col,
      row,
      zone: zoneOf(col),
      position: [col * CELL_SPACING, plane.y + LATTICE_LIFT, row * CELL_SPACING],
    };
    strata.push(e);
    return e;
  };
  const strataRaw = blob?.strata ?? {};
  for (const g of strataRaw.goals ?? []) {
    const cols = (g.members ?? []).map((m) => colOf.get(m) ?? 0);
    place(g, "g", cols);
    for (const m of g.members ?? []) verticalLinks.push({ from: m, to: g.id, kind: "member" });
  }
  const goalCol = new Map((strataRaw.goals ?? []).map((g) => [g.id, null]));
  for (const g of strata) {
    if (g.kind === "g") goalCol.set(g.id, g.col);
  }
  for (const a of strataRaw.areas ?? []) {
    const cols = (a.goals ?? []).map((g) => goalCol.get(g) ?? 0);
    place(a, "a", cols);
    for (const g of a.goals ?? []) verticalLinks.push({ from: g, to: a.id, kind: "area" });
  }
  for (const t of strataRaw.themes ?? []) {
    const cols = (t.members ?? []).map((m) => colOf.get(m) ?? 0);
    place(t, "t", cols);
    for (const m of t.members ?? []) verticalLinks.push({ from: m, to: t.id, kind: "theme" });
  }
  for (const f of strataRaw.files ?? []) {
    const cols = (f.touchers ?? []).map((m) => colOf.get(m) ?? 0);
    place(f, "f", cols);
    for (const m of f.touchers ?? []) verticalLinks.push({ from: m, to: f.id, kind: "surface" });
  }
  for (const e of strata) track(e.col, e.row);

  return {
    nodes,
    strata,
    links,
    verticalLinks,
    seed,
    truncated: blob?.truncated === true,
    critical,
    criticalPairs,
    extent: { colMin, colMax, rowMin, rowMax },
  };
};

export const variantFill = (variant) => {
  const [h, s, l] = VARIANT_HUES[variant] ?? VARIANT_HUES.neutral;
  return hslToRgb(h, s, l);
};

export const zoneFill = (zone) => {
  const [h, s, l] = ZONE_HUES[zone] ?? ZONE_HUES.seed;
  return hslToRgb(h, s, l);
};

export const screenSpaceSpriteHeight = (px, emFraction, fovDeg, viewportH, dist) =>
  (px / emFraction) * ((2 * Math.tan((fovDeg * Math.PI) / 360)) / viewportH) * dist;
