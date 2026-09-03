import * as THREE from "../vendor/three.module.min.js";
import { createLayout } from "../utils/force-3d.js";
import { parseCssColor } from "../utils/css-color.js";
import {
  RAYTRACE_MAX_NODES,
  createClusterFills,
  nodeAlpha,
  nodeFillFor,
  nodeRadius,
  parseGraphModel,
} from "../utils/graph-model.js";
import { createInfoPanel, createTooltip } from "./graph-panel.js";
import { labelFadeOpacity } from "../utils/label-fade.js";

const EDGE_RGB = [0.55, 0.57, 0.62];
const CONTAINS_RGB = [0.42, 0.72, 0.62];
const EDGE_OPACITY = 0.55;
const LABEL_MAX_NODES = 200;
const HOVER_SCALE = 1.25;

const RT_VS = `
out vec2 vUv;
void main() {
  vUv = uv;
  gl_Position = vec4(position.xy, 0.0, 1.0);
}`;

const RT_FS = `
uniform highp sampler2D uNodeTex;
uniform highp sampler2D uColorTex;
uniform int uNodeCount;
uniform vec3 uCamPos;
uniform vec3 uCamRight;
uniform vec3 uCamUp;
uniform vec3 uCamFwd;
uniform float uTanHalfFov;
uniform float uAspect;
uniform float uResY;
uniform float uFragFoot;
uniform vec3 uBg;
uniform vec3 uLightDir;
uniform float uDepthA;
uniform float uDepthB;
uniform vec4 uBounds;
in vec2 vUv;
layout(location = 0) out vec4 outColor;

struct Hit {
  float t;
  vec3 n;
  vec3 albedo;
};

struct Fringe {
  float cov;
  vec3 n;
  vec3 albedo;
};

void hitSpheres(vec3 ro, vec3 rd, inout Hit h, inout Fringe f) {
  for (int i = 0; i < uNodeCount; i++) {
    vec4 s = texelFetch(uNodeTex, ivec2(i, 0), 0);
    vec3 oc = ro - s.xyz;
    float b = dot(oc, rd);
    float c = dot(oc, oc) - s.w * s.w;
    float disc = b * b - c;
    if (disc > 0.0) {
      float t = -b - sqrt(disc);
      if (t > 0.001 && t < h.t) {
        h.t = t;
        h.n = normalize(ro + rd * t - s.xyz);
        h.albedo = texelFetch(uColorTex, ivec2(i, 0), 0).rgb;
        continue;
      }
    }
    float tca = max(-b, 0.0);
    if (tca >= h.t) continue;
    float minDist2 = max(c + s.w * s.w - b * b, 0.0);
    float reach = s.w + uFragFoot;
    if (minDist2 > reach * reach) continue;
    float cov = clamp((s.w - sqrt(minDist2)) / uFragFoot + 0.5, 0.0, 1.0);
    if (cov > f.cov) {
      f.cov = cov;
      f.n = normalize(oc + rd * tca);
      f.albedo = texelFetch(uColorTex, ivec2(i, 0), 0).rgb;
    }
  }
}

bool occluded(vec3 p) {
  for (int i = 0; i < uNodeCount; i++) {
    vec4 s = texelFetch(uNodeTex, ivec2(i, 0), 0);
    vec3 oc = p - s.xyz;
    float b = dot(oc, uLightDir);
    float c = dot(oc, oc) - s.w * s.w;
    float disc = b * b - c;
    if (disc > 0.0 && (-b - sqrt(disc)) > 0.001) return true;
  }
  return false;
}

vec3 shade(vec3 n, vec3 albedo, vec3 rd, float lit) {
  float ndl = max(dot(n, uLightDir), 0.0);
  vec3 hv = normalize(uLightDir - rd);
  float spec = pow(max(dot(n, hv), 0.0), 40.0);
  return albedo * (0.22 + 0.78 * ndl * lit) + vec3(spec * 0.35 * lit);
}

void main() {
  vec2 ndc = vUv * 2.0 - 1.0;
  vec3 rd = normalize(
    uCamFwd + uCamRight * (ndc.x * uTanHalfFov * uAspect) + uCamUp * (ndc.y * uTanHalfFov));
  vec3 bOc = uCamPos - uBounds.xyz;
  float bB = dot(bOc, rd);
  float bC = dot(bOc, bOc) - uBounds.w * uBounds.w;
  if (bB * bB - bC < 0.0) {
    outColor = vec4(uBg, 1.0);
    gl_FragDepth = 1.0;
    return;
  }
  Hit h;
  h.t = 1e9;
  h.n = vec3(0.0, 1.0, 0.0);
  h.albedo = uBg;
  Fringe f;
  f.cov = 0.0;
  f.n = vec3(0.0, 1.0, 0.0);
  f.albedo = uBg;
  hitSpheres(uCamPos, rd, h, f);
  vec3 color;
  if (h.t < 1e9) {
    vec3 p = uCamPos + rd * h.t;
    float lit = 1.0;
    if (dot(h.n, uLightDir) > 0.0 && occluded(p + h.n * 0.05)) lit = 0.25;
    color = shade(h.n, h.albedo, rd, lit);
    float zview = max(dot(p - uCamPos, uCamFwd), 0.001);
    float ndcZ = uDepthA + uDepthB / zview;
    gl_FragDepth = clamp((ndcZ + 1.0) * 0.5, 0.0, 1.0);
  } else if (f.cov > 0.0) {
    color = mix(uBg, shade(f.n, f.albedo, rd, 1.0), f.cov);
    gl_FragDepth = 1.0;
  } else {
    color = uBg;
    gl_FragDepth = 1.0;
  }
  outColor = vec4(color, 1.0);
}`;

export const RT_SHADERS = { vs: RT_VS, fs: RT_FS };

export function initGraph3D(root, { navigate, model, raytrace = false } = {}) {
  const stage = root.querySelector("#graph-stage");
  const labelsEl = root.querySelector("#graph-labels");
  const reheatBtn = root.querySelector("#graph-reheat");
  if (!stage || !labelsEl || !model) return null;

  const tooltipPanel = createTooltip(root, stage);
  const infoPanel = createInfoPanel(root);

  const { nodes, links } = parseGraphModel(model);
  if (!nodes.length) return null;
  const useRaytrace = raytrace === true && nodes.length <= RAYTRACE_MAX_NODES;

  const canvas = document.createElement("canvas");
  canvas.id = "graph-canvas-3d";
  stage.insertBefore(canvas, labelsEl);

  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  } catch (e) {
    canvas.remove();
    stage.insertAdjacentHTML(
      "beforeend",
      '<div class="alert alert-danger" role="alert"><div class="alert-content"><p class="alert-title">WebGL unavailable</p><p class="alert-description">The 3D graph needs a WebGL2 context.</p></div></div>',
    );
    return null;
  }
  const parsedBg = parseCssColor(getComputedStyle(stage).backgroundColor);
  const bg = parsedBg && parsedBg[3] > 0 ? parsedBg : [0.09, 0.1, 0.12];
  renderer.setClearColor(new THREE.Color().setRGB(bg[0], bg[1], bg[2], THREE.SRGBColorSpace));
  const systemDpr = window.devicePixelRatio || 1;
  renderer.setPixelRatio(useRaytrace ? Math.min(systemDpr, nodes.length > 250 ? 1 : 1.5) : systemDpr);

  const scene = new THREE.Scene();
  scene.add(new THREE.AmbientLight(0xffffff, 0.75));
  const keyLight = new THREE.DirectionalLight(0xffffff, 1.8);
  keyLight.position.set(0.6, 1, 0.8);
  scene.add(keyLight);
  const camera = new THREE.PerspectiveCamera(50, 1, 1, 8000);

  const fillFor = nodeFillFor(createClusterFills(nodes));
  const radiusOf = new Float32Array(nodes.length);
  const baseColor = [];
  nodes.forEach((n, i) => {
    radiusOf[i] = nodeRadius(n);
    const fill = fillFor(n);
    const dim = nodeAlpha(n);
    baseColor.push([fill[0] * dim, fill[1] * dim, fill[2] * dim]);
  });

  const nodeGeometry = new THREE.SphereGeometry(1, 20, 14);
  const nodeMaterial = new THREE.MeshLambertMaterial();
  const nodeMesh = new THREE.InstancedMesh(nodeGeometry, nodeMaterial, nodes.length);
  const color = new THREE.Color();
  const matrix = new THREE.Matrix4();
  nodes.forEach((_, i) => {
    nodeMesh.setColorAt(i, color.setRGB(baseColor[i][0], baseColor[i][1], baseColor[i][2], THREE.SRGBColorSpace));
  });
  nodeMesh.instanceColor.needsUpdate = true;
  if (!useRaytrace) scene.add(nodeMesh);

  let edgeGeometry = null;
  let edgeMaterial = null;
  let edgeLines = null;
  let edgePositions = null;
  {
    const edgeCount = links.length;
    edgePositions = new Float32Array(Math.max(1, edgeCount * 6));
    const edgeColors = new Float32Array(Math.max(1, edgeCount * 6));
    links.forEach((l, i) => {
      const rgb = l.virtual ? CONTAINS_RGB : EDGE_RGB;
      for (let v = 0; v < 2; v++) {
        edgeColors[i * 6 + v * 3] = rgb[0];
        edgeColors[i * 6 + v * 3 + 1] = rgb[1];
        edgeColors[i * 6 + v * 3 + 2] = rgb[2];
      }
    });
    edgeGeometry = new THREE.BufferGeometry();
    edgeGeometry.setAttribute("position", new THREE.BufferAttribute(edgePositions, 3));
    edgeGeometry.setAttribute("color", new THREE.BufferAttribute(edgeColors, 3));
    edgeMaterial = new THREE.LineBasicMaterial({
      vertexColors: true,
      transparent: true,
      opacity: EDGE_OPACITY,
    });
    edgeLines = new THREE.LineSegments(edgeGeometry, edgeMaterial);
    scene.add(edgeLines);
  }

  const lightDir = new THREE.Vector3(0.6, 1, 0.8).normalize();
  let rtScene = null;
  let rtCamera = null;
  let rtQuadGeometry = null;
  let rtMaterial = null;
  let rtTextures = [];
  let syncTraceData = null;
  let syncTraceCamera = null;
  if (useRaytrace) {
    const n = nodes.length;
    const nodeData = new Float32Array(n * 4);
    const colorData = new Uint8Array(n * 4);
    nodes.forEach((_, i) => {
      colorData[i * 4] = Math.round(baseColor[i][0] * 255);
      colorData[i * 4 + 1] = Math.round(baseColor[i][1] * 255);
      colorData[i * 4 + 2] = Math.round(baseColor[i][2] * 255);
      colorData[i * 4 + 3] = 255;
    });
    const makeTex = (data, width, type) => {
      const tex = new THREE.DataTexture(data, width, 1, THREE.RGBAFormat, type);
      tex.magFilter = THREE.NearestFilter;
      tex.minFilter = THREE.NearestFilter;
      return tex;
    };
    const nodeTex = makeTex(nodeData, n, THREE.FloatType);
    const colorTex = makeTex(colorData, n, THREE.UnsignedByteType);
    rtTextures = [nodeTex, colorTex];
    rtMaterial = new THREE.ShaderMaterial({
      glslVersion: THREE.GLSL3,
      depthTest: false,
      depthWrite: true,
      uniforms: {
        uNodeTex: { value: nodeTex },
        uColorTex: { value: colorTex },
        uNodeCount: { value: n },
        uCamPos: { value: new THREE.Vector3() },
        uCamRight: { value: new THREE.Vector3() },
        uCamUp: { value: new THREE.Vector3() },
        uCamFwd: { value: new THREE.Vector3(0, 0, -1) },
        uTanHalfFov: { value: Math.tan((50 * Math.PI) / 360) },
        uAspect: { value: 1 },
        uResY: { value: 1 },
        uFragFoot: { value: 0.01 },
        uBg: { value: new THREE.Vector3(bg[0], bg[1], bg[2]) },
        uLightDir: { value: lightDir },
        uDepthA: { value: 1 },
        uDepthB: { value: -2 },
        uBounds: { value: new THREE.Vector4(0, 0, 0, 600) },
      },
      vertexShader: RT_VS,
      fragmentShader: RT_FS,
    });
    rtQuadGeometry = new THREE.PlaneGeometry(2, 2);
    rtScene = new THREE.Scene();
    const rtQuad = new THREE.Mesh(rtQuadGeometry, rtMaterial);
    rtQuad.frustumCulled = false;
    rtScene.add(rtQuad);
    rtCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
    syncTraceData = () => {
      let maxDist = 0;
      for (let i = 0; i < nodes.length; i++) {
        nodeData[i * 4] = layout.positions[i * 3];
        nodeData[i * 4 + 1] = layout.positions[i * 3 + 1];
        nodeData[i * 4 + 2] = layout.positions[i * 3 + 2];
        nodeData[i * 4 + 3] = radiusOf[i];
        maxDist = Math.max(
          maxDist,
          Math.hypot(
            layout.positions[i * 3],
            layout.positions[i * 3 + 1],
            layout.positions[i * 3 + 2],
          ) + radiusOf[i],
        );
      }
      for (const tex of rtTextures) tex.needsUpdate = true;
      rtMaterial.uniforms.uBounds.value.set(0, 0, 0, maxDist + 8);
    };
    syncTraceCamera = () => {
      camera.updateMatrixWorld();
      const m = camera.matrixWorld.elements;
      rtMaterial.uniforms.uCamPos.value.copy(camera.position);
      rtMaterial.uniforms.uCamRight.value.set(m[0], m[1], m[2]);
      rtMaterial.uniforms.uCamUp.value.set(m[4], m[5], m[6]);
      rtMaterial.uniforms.uCamFwd.value.set(-m[8], -m[9], -m[10]);
      rtMaterial.uniforms.uAspect.value = camera.aspect;
      rtMaterial.uniforms.uResY.value = renderer.domElement.height;
      rtMaterial.uniforms.uFragFoot.value = Math.max(
        (2 * spherical.radius * Math.tan((50 * Math.PI) / 360)) /
          Math.max(1, renderer.domElement.height),
        0.001,
      );
      const p = camera.projectionMatrix.elements;
      rtMaterial.uniforms.uDepthA.value = -p[10];
      rtMaterial.uniforms.uDepthB.value = p[14];
    };
  }

  const layout = createLayout(nodes, links);
  let preSteps = 0;
  while (layout.alpha > 0.6 && preSteps++ < 120) layout.step();
  const nodeIndex = new Map(nodes.map((n, i) => [n.id, i]));

  let destroyed = false;
  let hovered = null;
  let rafId = 0;
  let fitted = false;
  let userOrbit = false;

  const target = new THREE.Vector3(0, 0, 0);
  const fitTargetRadius = () => {
    let maxDist = 0;
    for (let i = 0; i < nodes.length; i++) {
      maxDist = Math.max(
        maxDist,
        Math.hypot(
          layout.positions[i * 3],
          layout.positions[i * 3 + 1],
          layout.positions[i * 3 + 2],
        ),
      );
    }
    return Math.max(260, maxDist * 2.4);
  };
  const spherical = { radius: fitTargetRadius(), theta: Math.PI / 4, phi: Math.PI / 3 };
  let labelBaseRadius = spherical.radius;
  const applyCamera = () => {
    camera.position.setFromSphericalCoords(spherical.radius, spherical.phi, spherical.theta).add(target);
    camera.lookAt(target);
  };

  const labelEls = nodes.map((n) => {
    const el = document.createElement("div");
    el.className = "graph-label";
    el.textContent = n.id;
    labelsEl.appendChild(el);
    return el;
  });
  const labelsAllowed = nodes.length <= LABEL_MAX_NODES;
  let labelsOn = false;
  let labelFade = 0;
  labelsEl.style.display = "none";
  const updateLabelVisibility = () => {
    const fade = labelsAllowed ? labelFadeOpacity(spherical.radius / labelBaseRadius) : 0;
    labelFade = fade;
    const on = fade > 0;
    if (on === labelsOn) return;
    labelsOn = on;
    labelsEl.style.display = on ? "" : "none";
  };

  const projected = new THREE.Vector3();
  const updateLabels = () => {
    if (!labelsOn) return;
    const w = stage.clientWidth;
    const h = stage.clientHeight;
    for (let i = 0; i < nodes.length; i++) {
      projected.set(layout.positions[i * 3], layout.positions[i * 3 + 1], layout.positions[i * 3 + 2]);
      projected.project(camera);
      const el = labelEls[i];
      if (projected.z > 1 || projected.z < -1) {
        el.hidden = true;
        continue;
      }
      el.hidden = false;
      el.style.opacity = labelFade.toFixed(3);
      const x = ((projected.x + 1) / 2) * w;
      const y = ((1 - projected.y) / 2) * h - radiusOf[i] - 3;
      el.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) translate(-50%, -100%)`;
    }
  };

  const scaleOf = (i) => radiusOf[i] * (nodes[i] === hovered ? HOVER_SCALE : 1);

  const syncInstances = () => {
    for (let i = 0; i < nodes.length; i++) {
      matrix.makeScale(scaleOf(i), scaleOf(i), scaleOf(i));
      matrix.setPosition(layout.positions[i * 3], layout.positions[i * 3 + 1], layout.positions[i * 3 + 2]);
      nodeMesh.setMatrixAt(i, matrix);
    }
    nodeMesh.instanceMatrix.needsUpdate = true;
  };

  const syncEdges = () => {
    links.forEach((l, i) => {
      const a = nodeIndex.get(l.source);
      const b = nodeIndex.get(l.target);
      if (a === undefined || b === undefined) return;
      edgePositions[i * 6] = layout.positions[a * 3];
      edgePositions[i * 6 + 1] = layout.positions[a * 3 + 1];
      edgePositions[i * 6 + 2] = layout.positions[a * 3 + 2];
      edgePositions[i * 6 + 3] = layout.positions[b * 3];
      edgePositions[i * 6 + 4] = layout.positions[b * 3 + 1];
      edgePositions[i * 6 + 5] = layout.positions[b * 3 + 2];
    });
    edgeGeometry.attributes.position.needsUpdate = true;
  };

  let instancesDirty = true;
  let lastSyncedHover = null;

  const tick = () => {
    if (destroyed) return;
    const moving = layout.alpha > 0.002;
    if (moving) layout.step();
    if (!fitted && !userOrbit) {
      const goal = fitTargetRadius();
      labelBaseRadius = goal;
      spherical.radius += (goal - spherical.radius) * 0.12;
      if (!moving && Math.abs(goal - spherical.radius) < 2) fitted = true;
    }
    if (moving || hovered !== lastSyncedHover) {
      syncInstances();
      syncEdges();
      if (useRaytrace) syncTraceData();
      lastSyncedHover = hovered;
      instancesDirty = true;
    }
    applyCamera();
    renderer.autoClear = false;
    renderer.clear();
    if (useRaytrace) {
      syncTraceCamera();
      renderer.render(rtScene, rtCamera);
    }
    renderer.render(scene, camera);
    updateLabelVisibility();
    updateLabels();
    rafId = requestAnimationFrame(tick);
  };

  const resize = () => {
    const w = Math.max(1, stage.clientWidth);
    const h = Math.max(1, stage.clientHeight);
    renderer.setSize(w, h, false);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  };
  const observer = new ResizeObserver(resize);
  observer.observe(stage);
  resize();

  const raycaster = new THREE.Raycaster();
  const ndc = new THREE.Vector2();
  const pick = (mx, my) => {
    const w = stage.clientWidth;
    const h = stage.clientHeight;
    ndc.set((mx / w) * 2 - 1, -(my / h) * 2 + 1);
    raycaster.setFromCamera(ndc, camera);
    if (instancesDirty) {
      nodeMesh.computeBoundingSphere();
      instancesDirty = false;
    }
    const hits = raycaster.intersectObject(nodeMesh, false);
    return hits.length ? nodes[hits[0].instanceId] : null;
  };

  let dragging = null;
  let downAt = null;
  let downNode = null;

  const onPointerDown = (e) => {
    canvas.setPointerCapture(e.pointerId);
    downAt = [e.offsetX, e.offsetY];
    downNode = pick(e.offsetX, e.offsetY);
    const mode = e.button === 2 || e.shiftKey ? "pan" : "rotate";
    dragging = { mode, x: e.offsetX, y: e.offsetY };
    canvas.style.cursor = downNode ? "pointer" : dragging.mode === "pan" ? "move" : "grabbing";
  };

  const onPointerMove = (e) => {
    if (dragging) {
      const dx = e.offsetX - dragging.x;
      const dy = e.offsetY - dragging.y;
      dragging.x = e.offsetX;
      dragging.y = e.offsetY;
      userOrbit = true;
      if (dragging.mode === "rotate") {
        spherical.theta -= dx * 0.006;
        spherical.phi = Math.max(0.05, Math.min(Math.PI - 0.05, spherical.phi - dy * 0.006));
      } else {
        const panScale = spherical.radius * 0.0016;
        const right = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 0);
        const up = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 1);
        target.addScaledVector(right, -dx * panScale).addScaledVector(up, dy * panScale);
      }
      return;
    }
    const n = pick(e.offsetX, e.offsetY);
    if (n !== hovered) {
      hovered = n;
    }
    canvas.style.cursor = n ? "pointer" : "default";
    if (n) tooltipPanel.show(n, e.offsetX, e.offsetY);
    else tooltipPanel.hide();
  };

  const resetDrag = () => {
    dragging = null;
    downAt = null;
    downNode = null;
    canvas.style.cursor = "default";
  };

  const onPointerUp = (e) => {
    const down = downAt;
    const hit = downNode;
    resetDrag();
    if (!down) return;
    const moved = Math.hypot(e.offsetX - down[0], e.offsetY - down[1]);
    if (moved > 4) return;
    const n = hit && pick(e.offsetX, e.offsetY);
    if (!n) {
      infoPanel.hide();
      return;
    }
    if (n.kind === "w" && navigate) {
      navigate("packet", { id: n.id });
    } else {
      infoPanel.show(n);
    }
  };

  const onWheel = (e) => {
    e.preventDefault();
    userOrbit = true;
    spherical.radius = Math.max(
      80,
      Math.min(6000, spherical.radius * Math.exp(e.deltaY * 0.0012)),
    );
  };

  const onReheat = () => {
    fitted = false;
    layout.reheat(0.6);
  };

  canvas.addEventListener("pointerdown", onPointerDown);
  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerup", onPointerUp);
  canvas.addEventListener("pointercancel", resetDrag);
  canvas.addEventListener("lostpointercapture", resetDrag);
  canvas.addEventListener("pointerleave", () => {
    tooltipPanel.hide();
    hovered = null;
  });
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  canvas.addEventListener("wheel", onWheel, { passive: false });
  reheatBtn?.addEventListener("click", onReheat);

  rafId = requestAnimationFrame(tick);
  canvas.graphNodes = nodes;

  return () => {
    destroyed = true;
    if (rafId) cancelAnimationFrame(rafId);
    observer.disconnect();
    canvas.removeEventListener("pointerdown", onPointerDown);
    canvas.removeEventListener("pointermove", onPointerMove);
    canvas.removeEventListener("pointerup", onPointerUp);
    canvas.removeEventListener("pointercancel", resetDrag);
    canvas.removeEventListener("lostpointercapture", resetDrag);
    canvas.removeEventListener("wheel", onWheel);
    reheatBtn?.removeEventListener("click", onReheat);
    tooltipPanel.hide();
    infoPanel.hide();
    for (const el of labelEls) el.remove();
    labelsEl.style.display = "";
    nodeGeometry.dispose();
    nodeMaterial.dispose();
    if (edgeGeometry) edgeGeometry.dispose();
    if (edgeMaterial) edgeMaterial.dispose();
    for (const tex of rtTextures) tex.dispose();
    if (rtQuadGeometry) rtQuadGeometry.dispose();
    if (rtMaterial) rtMaterial.dispose();
    renderer.dispose();
    canvas.remove();
  };
}
