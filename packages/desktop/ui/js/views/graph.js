import { SearchableSelect } from "../utils/searchable-select.js";
import { wireFilterTabsFades } from "../utils/filter-tabs.js";
import { pickSearchRadius, screenHitRadius } from "../utils/graph-math.js";
import { KIND_INFO, statusVariant } from "../utils/graph-status.js";
import { parseCssColor } from "../utils/css-color.js";

const CLUSTER_HUES = [
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
const ROOT_CLUSTER = "root";
const ROOT_CLUSTER_RGB = [0.45, 0.45, 0.48];

const FALLBACK_COLOR = "#5a5a5a";
const EDGE_RGB = [0.55, 0.57, 0.62];
const EDGE_ALPHA = 0.55;
const CONTAINS_RGB = [0.42, 0.72, 0.62];
const CONTAINS_ALPHA = 0.7;
const BORDER_LIVE = [1, 1, 1];
const BORDER_SOFT = [0.85, 0.87, 0.92];
const NODE_RADIUS = 9;
const ROOT_RADIUS = 15;
const LABEL_MIN_SCALE = 0.4;
const LABEL_MAX_NODES = 200;
const MIN_ZOOM = 0.15;
const MAX_ZOOM = 4;

const metaBadge = (text, variant = "neutral") => {
  const chip = document.createElement("span");
  chip.className = `badge badge-${variant} capitalize`;
  chip.textContent = text;
  return chip;
};

const NODE_VS = `
attribute vec2 a_center;
attribute vec2 a_corner;
attribute float a_radius;
attribute vec3 a_fill;
attribute vec3 a_border;
attribute float a_alpha;
uniform vec2 u_resolution;
uniform float u_scale;
uniform vec2 u_offset;
varying vec2 v_corner;
varying vec3 v_fill;
varying vec3 v_border;
varying float v_alpha;
void main() {
  vec2 screen = a_center * u_scale + u_offset + a_corner * a_radius;
  vec2 clip = screen / u_resolution * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
  v_corner = a_corner;
  v_fill = a_fill;
  v_border = a_border;
  v_alpha = a_alpha;
}`;

const NODE_FS = `
precision mediump float;
varying vec2 v_corner;
varying vec3 v_fill;
varying vec3 v_border;
varying float v_alpha;
void main() {
  float d = length(v_corner);
  if (d > 1.0) discard;
  float ring = smoothstep(0.60, 0.80, d);
  gl_FragColor = vec4(mix(v_fill, v_border, ring), v_alpha);
}`;

const EDGE_VS = `
attribute vec2 a_pos;
uniform vec2 u_resolution;
uniform float u_scale;
uniform vec2 u_offset;
void main() {
  vec2 screen = a_pos * u_scale + u_offset;
  vec2 clip = screen / u_resolution * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
}`;

const EDGE_FS = `
precision mediump float;
uniform vec3 u_color;
uniform float u_alpha;
void main() {
  gl_FragColor = vec4(u_color, u_alpha);
}`;

const hexToRgb = (hex) => {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
};

const hslToRgb = (h, s, l) => {
  s /= 100;
  l /= 100;
  const k = (n) => (n + h / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const f = (n) => l - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)));
  return [f(0), f(8), f(4)];
};

const nodeAlpha = (n) => {
  if (n.archived) return 0.3;
  if (n.kind === "w" && (n.status === "done" || n.status === "rejected")) return 0.45;
  return 1.0;
};

const nodeBorder = (n) => (n.status === "progress" ? BORDER_LIVE : BORDER_SOFT);

const nodeRadius = (n) => (n.kind === "root" ? ROOT_RADIUS : NODE_RADIUS);

const kindStatus = (n) => (n.status ? `${n.kind}/${n.status}` : n.kind);

const compile = (gl, type, src) => {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, src);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(shader) || "shader compile failed");
  }
  return shader;
};

const link = (gl, vs, fs) => {
  const prog = gl.createProgram();
  gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, vs));
  gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(prog) || "program link failed");
  }
  return prog;
};

const CORNERS = [-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1];
const FLOATS_PER_VERTEX = 12;

export function initGraph(root, { navigate } = {}) {
  const stage = root.querySelector("#graph-stage");
  const canvas = root.querySelector("#graph-canvas");
  const labelsEl = root.querySelector("#graph-labels");
  const tooltip = root.querySelector("#graph-tooltip");
  const info = root.querySelector("#graph-info");
  const dataEl = root.querySelector("#graph-data");
  const reheatBtn = root.querySelector("#graph-reheat");
  const archivedBox = root.querySelector("#graph-archived");
  if (!stage || !canvas || !labelsEl || !tooltip || !info || !dataEl) return null;

  let model;
  try {
    model = JSON.parse(dataEl.textContent);
  } catch (e) {
    stage.insertAdjacentHTML(
      "beforeend",
      '<div class="alert alert-danger" role="alert"><div class="alert-content"><p class="alert-title">Graph data unreadable</p></div></div>',
    );
    return null;
  }

  const gl = canvas.getContext("webgl", { antialias: true, alpha: false });
  if (!gl) {
    stage.insertAdjacentHTML(
      "beforeend",
      '<div class="alert alert-danger" role="alert"><div class="alert-content"><p class="alert-title">WebGL unavailable</p><p class="alert-description">The graph needs a WebGL1 context.</p></div></div>',
    );
    return null;
  }

  const nodes = (model.nodes ?? []).map((n) => ({ ...n }));
  const links = (model.edges ?? []).map((e) => ({
    source: e.from,
    target: e.to,
    label: e.label,
    virtual: e.virtual === true,
  }));
  canvas.graphNodes = nodes;

  const clusterIds = [...new Set(nodes.map((n) => n.cluster ?? ROOT_CLUSTER))].sort();
  const clusterFills = new Map([[ROOT_CLUSTER, ROOT_CLUSTER_RGB]]);
  clusterIds
    .filter((id) => id !== ROOT_CLUSTER)
    .forEach((id, i) => {
      const [h, s, l] = CLUSTER_HUES[i % CLUSTER_HUES.length];
      clusterFills.set(id, hslToRgb(h, s, l));
    });
  const nodeFill = (n) => clusterFills.get(n.cluster) ?? hexToRgb(FALLBACK_COLOR);

  const clusterCenters = new Map([[ROOT_CLUSTER, [0, 0]]]);
  const ringRadius = Math.max(240, 45 * Math.sqrt(nodes.length));
  clusterIds
    .filter((id) => id !== ROOT_CLUSTER)
    .forEach((id, i, ring) => {
      const angle = (i / ring.length) * 2 * Math.PI - Math.PI / 2;
      clusterCenters.set(id, [ringRadius * Math.cos(angle), ringRadius * Math.sin(angle)]);
    });
  const centerOf = (n) => clusterCenters.get(n.cluster) ?? [0, 0];
  const clusterMemberCount = new Map();
  for (const n of nodes) {
    const [cx, cy] = centerOf(n);
    const i = clusterMemberCount.get(n.cluster) ?? 0;
    clusterMemberCount.set(n.cluster, i + 1);
    const a = i * 2.399963;
    const r = 12 * Math.sqrt(i);
    n.x = cx + r * Math.cos(a);
    n.y = cy + r * Math.sin(a);
  }

  let destroyed = false;
  let dpr = window.devicePixelRatio || 1;
  let zoom = 1;
  let offsetX = 0;
  let offsetY = 0;
  let placed = false;
  let userCamera = false;
  let hovered = null;
  let quadtree = null;

  let edgeProg;
  let nodeProg;
  try {
    edgeProg = link(gl, EDGE_VS, EDGE_FS);
    nodeProg = link(gl, NODE_VS, NODE_FS);
  } catch (e) {
    stage.insertAdjacentHTML(
      "beforeend",
      '<div class="alert alert-danger" role="alert"><div class="alert-content"><p class="alert-title">Graph shaders unavailable</p></div></div>',
    );
    return null;
  }
  const edgeBuffer = gl.createBuffer();
  const nodeBuffer = gl.createBuffer();
  const u = (prog, name) => gl.getUniformLocation(prog, name);
  const edgeLoc = {
    pos: gl.getAttribLocation(edgeProg, "a_pos"),
    resolution: u(edgeProg, "u_resolution"),
    scale: u(edgeProg, "u_scale"),
    offset: u(edgeProg, "u_offset"),
    color: u(edgeProg, "u_color"),
    alpha: u(edgeProg, "u_alpha"),
  };
  const nodeLoc = {
    center: gl.getAttribLocation(nodeProg, "a_center"),
    corner: gl.getAttribLocation(nodeProg, "a_corner"),
    radius: gl.getAttribLocation(nodeProg, "a_radius"),
    fill: gl.getAttribLocation(nodeProg, "a_fill"),
    border: gl.getAttribLocation(nodeProg, "a_border"),
    alpha: gl.getAttribLocation(nodeProg, "a_alpha"),
    resolution: u(nodeProg, "u_resolution"),
    scale: u(nodeProg, "u_scale"),
    offset: u(nodeProg, "u_offset"),
  };
  const parsedBg = parseCssColor(getComputedStyle(stage).backgroundColor);
  const bg = parsedBg && parsedBg[3] > 0 ? parsedBg : [0.09, 0.1, 0.12];
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  const labelEls = nodes.map((n) => {
    const el = document.createElement("div");
    el.className = "graph-label";
    el.textContent = n.id;
    labelsEl.appendChild(el);
    return el;
  });

  const simulation = d3
    .forceSimulation(nodes)
    .force("link", d3.forceLink(links).id((n) => n.id).distance(70).strength(0.5))
    .force("charge", d3.forceManyBody().strength(-220))
    .force("center", d3.forceCenter(0, 0))
    .force("x", d3.forceX((n) => centerOf(n)[0]).strength(0.14))
    .force("y", d3.forceY((n) => centerOf(n)[1]).strength(0.14))
    .force("collide", d3.forceCollide((n) => nodeRadius(n) + 6))
    .alphaDecay(0.035)
    .on("tick", () => {
      quadtree = null;
      draw();
    });

  const fit = () => {
    if (destroyed || userCamera || !nodes.length) return;
    const rect = stage.getBoundingClientRect();
    if (!rect.width || !rect.height) return;
    let x0 = Infinity;
    let y0 = Infinity;
    let x1 = -Infinity;
    let y1 = -Infinity;
    for (const n of nodes) {
      x0 = Math.min(x0, n.x);
      x1 = Math.max(x1, n.x);
      y0 = Math.min(y0, n.y);
      y1 = Math.max(y1, n.y);
    }
    const pad = 60;
    const w = Math.max(1, x1 - x0);
    const h = Math.max(1, y1 - y0);
    zoom = Math.min(
      MAX_ZOOM,
      Math.max(MIN_ZOOM, Math.min((rect.width - pad * 2) / w, (rect.height - pad * 2) / h, 1.2)),
    );
    offsetX = dpr * (rect.width / 2 - ((x0 + x1) / 2) * zoom);
    offsetY = dpr * (rect.height / 2 - ((y0 + y1) / 2) * zoom);
    draw();
  };
  simulation.on("end", fit);

  const toWorld = (mx, my) => [
    (mx * dpr - offsetX) / (zoom * dpr),
    (my * dpr - offsetY) / (zoom * dpr),
  ];

  const pick = (mx, my) => {
    if (!quadtree) {
      quadtree = d3
        .quadtree()
        .x((n) => n.x)
        .y((n) => n.y)
        .addAll(nodes);
    }
    const [wx, wy] = toWorld(mx, my);
    const found = quadtree.find(wx, wy, pickSearchRadius(ROOT_RADIUS, zoom));
    if (!found) return null;
    const sx = (found.x * zoom * dpr + offsetX) / dpr;
    const sy = (found.y * zoom * dpr + offsetY) / dpr;
    return Math.hypot(sx - mx, sy - my) <= screenHitRadius(nodeRadius(found), zoom) ? found : null;
  };

  const updateLabels = () => {
    const show = nodes.length <= LABEL_MAX_NODES && zoom >= LABEL_MIN_SCALE;
    labelsEl.style.display = show ? "" : "none";
    if (!show) return;
    for (let i = 0; i < nodes.length; i++) {
      const n = nodes[i];
      const x = (n.x * zoom * dpr + offsetX) / dpr;
      const y = (n.y * zoom * dpr + offsetY) / dpr;
      labelEls[i].style.transform = `translate(${x.toFixed(1)}px, ${(y - nodeRadius(n) - 3).toFixed(1)}px) translate(-50%, -100%)`;
    }
  };

  const realLinks = links.filter((l) => !l.virtual);
  const virtualLinks = links.filter((l) => l.virtual);
  const virtualEdgeData = new Float32Array(virtualLinks.length * 4);
  const realEdgeData = new Float32Array(realLinks.length * 4);
  const nodeData = new Float32Array(nodes.length * 6 * FLOATS_PER_VERTEX);

  const drawEdgeBatch = (batch, edgeData, rgb, alpha) => {
    if (!batch.length) return;
    gl.uniform3fv(edgeLoc.color, rgb);
    gl.uniform1f(edgeLoc.alpha, alpha);
    for (let i = 0; i < batch.length; i++) {
      const l = batch[i];
      edgeData[i * 4] = l.source.x;
      edgeData[i * 4 + 1] = l.source.y;
      edgeData[i * 4 + 2] = l.target.x;
      edgeData[i * 4 + 3] = l.target.y;
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, edgeBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, edgeData, gl.DYNAMIC_DRAW);
    gl.enableVertexAttribArray(edgeLoc.pos);
    gl.vertexAttribPointer(edgeLoc.pos, 2, gl.FLOAT, false, 0, 0);
    gl.drawArrays(gl.LINES, 0, batch.length * 2);
  };

  function draw() {
    if (destroyed) return;
    const w = canvas.width;
    const h = canvas.height;
    if (!w || !h) return;
    gl.viewport(0, 0, w, h);
    gl.clearColor(bg[0], bg[1], bg[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    const scale = zoom * dpr;

    gl.useProgram(edgeProg);
    gl.uniform2f(edgeLoc.resolution, w, h);
    gl.uniform1f(edgeLoc.scale, scale);
    gl.uniform2f(edgeLoc.offset, offsetX, offsetY);
    drawEdgeBatch(virtualLinks, virtualEdgeData, CONTAINS_RGB, CONTAINS_ALPHA);
    drawEdgeBatch(realLinks, realEdgeData, EDGE_RGB, EDGE_ALPHA);

    gl.useProgram(nodeProg);
    gl.uniform2f(nodeLoc.resolution, w, h);
    gl.uniform1f(nodeLoc.scale, scale);
    gl.uniform2f(nodeLoc.offset, offsetX, offsetY);
    let o = 0;
    for (const n of nodes) {
      const fill = n._fill ?? (n._fill = nodeFill(n));
      const border = n._border ?? (n._border = nodeBorder(n));
      const alpha = n._alpha ?? (n._alpha = nodeAlpha(n));
      const radius = nodeRadius(n) * dpr * (n === hovered ? 1.25 : 1);
      for (let c = 0; c < 6; c++) {
        nodeData[o++] = n.x;
        nodeData[o++] = n.y;
        nodeData[o++] = CORNERS[c * 2];
        nodeData[o++] = CORNERS[c * 2 + 1];
        nodeData[o++] = radius;
        nodeData[o++] = fill[0];
        nodeData[o++] = fill[1];
        nodeData[o++] = fill[2];
        nodeData[o++] = border[0];
        nodeData[o++] = border[1];
        nodeData[o++] = border[2];
        nodeData[o++] = alpha;
      }
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, nodeBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, nodeData, gl.DYNAMIC_DRAW);
    const stride = FLOATS_PER_VERTEX * 4;
    const attrib = (loc, size, offset) => {
      gl.enableVertexAttribArray(loc);
      gl.vertexAttribPointer(loc, size, gl.FLOAT, false, stride, offset);
    };
    attrib(nodeLoc.center, 2, 0);
    attrib(nodeLoc.corner, 2, 8);
    attrib(nodeLoc.radius, 1, 16);
    attrib(nodeLoc.fill, 3, 20);
    attrib(nodeLoc.border, 3, 32);
    attrib(nodeLoc.alpha, 1, 44);
    gl.drawArrays(gl.TRIANGLES, 0, nodes.length * 6);

    updateLabels();
  }

  let rafId = 0;
  const scheduleDraw = () => {
    if (destroyed || rafId) return;
    rafId = requestAnimationFrame(() => {
      rafId = 0;
      draw();
    });
  };

  const resize = () => {
    const rect = stage.getBoundingClientRect();
    dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.round(rect.width * dpr));
    canvas.height = Math.max(1, Math.round(rect.height * dpr));
    if (!placed) {
      offsetX = canvas.width / 2;
      offsetY = canvas.height / 2;
      placed = true;
    }
    draw();
  };
  const observer = new ResizeObserver(resize);
  observer.observe(stage);
  resize();

  let dprQuery = null;
  const onDprChange = () => {
    resize();
    watchDpr();
  };
  const watchDpr = () => {
    dprQuery?.removeEventListener("change", onDprChange);
    dprQuery = window.matchMedia(`(resolution: ${window.devicePixelRatio || 1}dppx)`);
    dprQuery.addEventListener("change", onDprChange);
  };
  watchDpr();

  const hideTooltip = () => {
    tooltip.hidden = true;
  };
  const showTooltip = (n, mx, my) => {
    tooltip.textContent = `${n.id} (${kindStatus(n)})${n.title ? `: ${n.title}` : ""}`;
    tooltip.hidden = false;
    const maxX = stage.clientWidth - tooltip.offsetWidth - 8;
    const maxY = stage.clientHeight - tooltip.offsetHeight - 8;
    const left = Math.max(8, Math.min(mx + 14, maxX));
    const top = Math.max(8, Math.min(my + 16, maxY));
    tooltip.style.left = `${Math.round(left)}px`;
    tooltip.style.top = `${Math.round(top)}px`;
  };
  const hideInfo = () => {
    info.hidden = true;
  };
  const showInfo = (n) => {
    root.querySelector("#graph-info-id").textContent = n.id;
    root.querySelector("#graph-info-title").textContent = n.title || "(untitled)";
    const meta = root.querySelector("#graph-info-meta");
    const chips = [metaBadge(KIND_INFO[n.kind] ?? n.kind)];
    if (n.status) {
      chips.push(metaBadge(n.status, statusVariant(n)));
    }
    if (n.wtype) chips.push(metaBadge(n.wtype));
    if (n.archived) chips.push(metaBadge("archived", "warning"));
    meta.replaceChildren(...chips);
    info.hidden = false;
  };

  let panning = false;
  let downAt = null;
  let downNode = null;
  let lastPos = null;

  const resetPointerState = () => {
    downAt = null;
    downNode = null;
    lastPos = null;
    panning = false;
    canvas.style.cursor = "default";
  };

  canvas.addEventListener("pointerdown", (e) => {
    canvas.setPointerCapture(e.pointerId);
    downAt = [e.offsetX, e.offsetY];
    downNode = pick(e.offsetX, e.offsetY);
    panning = !downNode;
    if (panning) canvas.style.cursor = "grabbing";
  });
  canvas.addEventListener("pointermove", (e) => {
    if (downAt && panning) {
      const [lx, ly] = lastPos ?? downAt;
      offsetX += (e.offsetX - lx) * dpr;
      offsetY += (e.offsetY - ly) * dpr;
      lastPos = [e.offsetX, e.offsetY];
      userCamera = true;
      scheduleDraw();
      return;
    }
    if (downAt) return;
    const n = pick(e.offsetX, e.offsetY);
    if (n !== hovered) {
      hovered = n;
      scheduleDraw();
    }
    canvas.style.cursor = n ? "pointer" : "default";
    if (n) showTooltip(n, e.offsetX, e.offsetY);
    else hideTooltip();
  });
  canvas.addEventListener("pointerup", (e) => {
    const wasPan = panning;
    const down = downAt;
    const hit = downNode;
    resetPointerState();
    if (!down) return;
    const moved = Math.hypot(e.offsetX - down[0], e.offsetY - down[1]);
    if (wasPan || moved > 4) return;
    const n = hit && pick(e.offsetX, e.offsetY);
    if (!n) {
      hideInfo();
      return;
    }
    if (n.kind === "w" && navigate) {
      navigate("packet", { id: n.id });
    } else {
      showInfo(n);
    }
  });
  canvas.addEventListener("pointercancel", resetPointerState);
  canvas.addEventListener("lostpointercapture", resetPointerState);
  canvas.addEventListener("pointerleave", () => {
    hideTooltip();
    if (hovered) {
      hovered = null;
      draw();
    }
  });
  canvas.addEventListener(
    "wheel",
    (e) => {
      e.preventDefault();
      const next = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom * Math.exp(-e.deltaY * 0.0012)));
      if (next === zoom) return;
      const px = e.offsetX * dpr;
      const py = e.offsetY * dpr;
      offsetX = px - ((px - offsetX) / zoom) * next;
      offsetY = py - ((py - offsetY) / zoom) * next;
      zoom = next;
      userCamera = true;
      scheduleDraw();
    },
    { passive: false },
  );
  reheatBtn?.addEventListener("click", () => {
    userCamera = false;
    simulation.alpha(0.6).restart();
  });
  const section = root.querySelector(".view-graph");
  const currentKind = section?.dataset.kind || "all";
  const currentFocus = section?.dataset.focus || "";
  const currentArchived = () => archivedBox?.checked ?? false;
  const cleanupFades = wireFilterTabsFades(root);
  for (const chip of root.querySelectorAll('[data-action="filter"]')) {
    chip.addEventListener("click", () =>
      navigate?.("graph", { kind: chip.dataset.status, archived: currentArchived() }),
    );
  }
  const focusWrap = root.querySelector("#graph-focus");
  let focusSelect = null;
  if (focusWrap) {
    focusSelect = new SearchableSelect({
      container: focusWrap,
      placeholder: "Focus a node...",
      emptyText: "No nodes",
      renderOption: (item) => item.label,
      onSelect: (item) =>
        navigate?.("graph", {
          kind: currentKind,
          focus: item.id,
          archived: currentArchived(),
        }),
    });
  }
  root.querySelector("#graph-focus-clear")?.addEventListener("click", () =>
    navigate?.("graph", { kind: currentKind, archived: currentArchived() }),
  );
  archivedBox?.addEventListener("change", () => {
    navigate?.("graph", {
      kind: currentKind,
      focus: currentFocus,
      archived: archivedBox.checked,
    });
  });

  return () => {
    destroyed = true;
    if (rafId) cancelAnimationFrame(rafId);
    cleanupFades();
    simulation.stop();
    observer.disconnect();
    dprQuery?.removeEventListener("change", onDprChange);
    focusSelect?.destroy();
    gl.getExtension("WEBGL_lose_context")?.loseContext();
  };
}
