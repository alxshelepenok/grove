export const CLUSTER_HUES = [
  [262, 70, 60],
  [184, 96, 53],
  [166, 50, 44],
  [43, 68, 62],
  [342, 50, 62],
  [280, 65, 68],
  [191, 55, 35],
  [262, 45, 66],
  [166, 95, 40],
  [43, 42, 52],
  [342, 63, 55],
];
export const ROOT_CLUSTER = "root";
export const ROOT_CLUSTER_RGB = [0.45, 0.45, 0.48];
export const FALLBACK_COLOR = "#5a5a5a";
export const NODE_RADIUS = 9;
export const ROOT_RADIUS = 15;
export const RAYTRACE_MAX_NODES = 400;

export const hexToRgb = (hex) => {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
};

export const hslToRgb = (h, s, l) => {
  s /= 100;
  l /= 100;
  const k = (n) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [f(0), f(8), f(4)];
};

export const parseGraphModel = (model) => ({
  nodes: (model?.nodes ?? []).map((n) => ({ ...n })),
  links: (model?.edges ?? []).map((e) => ({
    source: e.from,
    target: e.to,
    label: e.label,
    virtual: e.virtual === true,
  })),
});

export const createClusterFills = (nodes) => {
  const clusterIds = [...new Set(nodes.map((n) => n.cluster ?? ROOT_CLUSTER))].sort();
  const fills = new Map([[ROOT_CLUSTER, ROOT_CLUSTER_RGB]]);
  clusterIds
    .filter((id) => id !== ROOT_CLUSTER)
    .forEach((id, i) => {
      const [h, s, l] = CLUSTER_HUES[i % CLUSTER_HUES.length];
      fills.set(id, hslToRgb(h, s, l));
    });
  return fills;
};

export const nodeFillFor = (fills) => (n) => fills.get(n.cluster) ?? hexToRgb(FALLBACK_COLOR);

export const nodeAlpha = (n) => {
  if (n.archived) return 0.3;
  if (n.kind === "w" && (n.status === "done" || n.status === "rejected")) return 0.45;
  return 1.0;
};

export const nodeRadius = (n) => (n.kind === "root" ? ROOT_RADIUS : NODE_RADIUS);

export const kindStatus = (n) => (n.status ? `${n.kind}/${n.status}` : n.kind);
