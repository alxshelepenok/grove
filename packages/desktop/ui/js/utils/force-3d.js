export const clusterCenters3d = (clusterIds, nodeCount) => {
  const centers = new Map([["root", [0, 0, 0]]]);
  const others = clusterIds.filter((id) => id !== "root");
  const radius = Math.max(240, 45 * Math.sqrt(Math.max(1, nodeCount)));
  const golden = Math.PI * (3 - Math.sqrt(5));
  others.forEach((id, i) => {
    const y = 1 - (i / Math.max(1, others.length)) * 2;
    const r = Math.sqrt(Math.max(0, 1 - y * y));
    const a = golden * i;
    centers.set(id, [Math.cos(a) * r * radius, y * radius, Math.sin(a) * r * radius]);
  });
  return centers;
};

export const createLayout = (nodes, links, options = {}) => {
  const {
    repulsion = 2600,
    springLength = 90,
    springStrength = 0.08,
    anchorStrength = 0.06,
    minSeparation = 26,
    damping = 0.82,
    alphaDecay = 0.02,
    minAlpha = 0.001,
  } = options;

  const n = nodes.length;
  const pos = new Float64Array(n * 3);
  const vel = new Float64Array(n * 3);
  const anchor = new Float64Array(n * 3);
  const index = new Map(nodes.map((node, i) => [node.id, i]));

  const centers = clusterCenters3d(
    [...new Set(nodes.map((node) => node.cluster ?? "root"))],
    n,
  );
  const memberCount = new Map();
  nodes.forEach((node, i) => {
    const c = centers.get(node.cluster) ?? [0, 0, 0];
    anchor[i * 3] = c[0];
    anchor[i * 3 + 1] = c[1];
    anchor[i * 3 + 2] = c[2];
    const k = memberCount.get(node.cluster) ?? 0;
    memberCount.set(node.cluster, k + 1);
    const phi = Math.acos(1 - (2 * (k + 0.5)) / Math.max(1, n));
    const r = 14 * Math.sqrt(k + 0.5);
    const theta = k * 2.399963;
    pos[i * 3] = c[0] + r * Math.sin(phi) * Math.cos(theta);
    pos[i * 3 + 1] = c[1] + r * Math.cos(phi);
    pos[i * 3 + 2] = c[2] + r * Math.sin(phi) * Math.sin(theta);
  });

  const pairs = [];
  for (const l of links) {
    const a = index.get(typeof l.source === "object" ? l.source.id : l.source);
    const b = index.get(typeof l.target === "object" ? l.target.id : l.target);
    if (a !== undefined && b !== undefined && a !== b) pairs.push([a, b]);
  }

  let alpha = 1;
  const step = () => {
    for (let i = 0; i < n; i++) {
      const px = pos[i * 3];
      const py = pos[i * 3 + 1];
      const pz = pos[i * 3 + 2];
      for (let j = i + 1; j < n; j++) {
        const dx = pos[j * 3] - px;
        const dy = pos[j * 3 + 1] - py;
        const dz = pos[j * 3 + 2] - pz;
        const d2 = dx * dx + dy * dy + dz * dz;
        if (d2 > 360000) continue;
        const dist = Math.sqrt(d2) || 0.001;
        const f =
          (repulsion * alpha) / Math.max(d2, 36) +
          (dist < minSeparation ? (minSeparation - dist) * 0.5 * alpha : 0);
        const ux = (dx / dist) * f;
        const uy = (dy / dist) * f;
        const uz = (dz / dist) * f;
        vel[i * 3] -= ux;
        vel[i * 3 + 1] -= uy;
        vel[i * 3 + 2] -= uz;
        vel[j * 3] += ux;
        vel[j * 3 + 1] += uy;
        vel[j * 3 + 2] += uz;
      }
    }
    for (const [a, b] of pairs) {
      const dx = pos[b * 3] - pos[a * 3];
      const dy = pos[b * 3 + 1] - pos[a * 3 + 1];
      const dz = pos[b * 3 + 2] - pos[a * 3 + 2];
      const dist = Math.sqrt(dx * dx + dy * dy + dz * dz) || 0.001;
      const f = (dist - springLength) * springStrength * alpha;
      const ux = (dx / dist) * f;
      const uy = (dy / dist) * f;
      const uz = (dz / dist) * f;
      vel[a * 3] += ux;
      vel[a * 3 + 1] += uy;
      vel[a * 3 + 2] += uz;
      vel[b * 3] -= ux;
      vel[b * 3 + 1] -= uy;
      vel[b * 3 + 2] -= uz;
    }
    let speedSum = 0;
    for (let i = 0; i < n; i++) {
      vel[i * 3] = (vel[i * 3] + (anchor[i * 3] - pos[i * 3]) * anchorStrength * alpha) * damping;
      vel[i * 3 + 1] =
        (vel[i * 3 + 1] + (anchor[i * 3 + 1] - pos[i * 3 + 1]) * anchorStrength * alpha) * damping;
      vel[i * 3 + 2] =
        (vel[i * 3 + 2] + (anchor[i * 3 + 2] - pos[i * 3 + 2]) * anchorStrength * alpha) * damping;
      pos[i * 3] += vel[i * 3];
      pos[i * 3 + 1] += vel[i * 3 + 1];
      pos[i * 3 + 2] += vel[i * 3 + 2];
      speedSum += Math.hypot(vel[i * 3], vel[i * 3 + 1], vel[i * 3 + 2]);
    }
    alpha = Math.max(minAlpha, alpha * (1 - alphaDecay));
    return speedSum / Math.max(1, n);
  };

  return {
    step,
    reheat: (value = 0.6) => {
      alpha = Math.max(alpha, value);
    },
    get alpha() {
      return alpha;
    },
    positions: pos,
    anchors: anchor,
  };
};
