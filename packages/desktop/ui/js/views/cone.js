import * as THREE from "../vendor/three.module.min.js";
import { SearchableSelect } from "../utils/searchable-select.js";
import { parseCssColor } from "../utils/css-color.js";
import { parseConeModel } from "../utils/cone-model.js";
import { createConeTooltip } from "./cone-tooltip.js";
import {
  createStrataLattice,
  createStrataLinkLines,
  createWorkLattice,
  footprintOf,
} from "./cone-lattice.js";
import {
  createSceneLabels,
  createWorkLabels,
  graphLabelStyle,
  updateWorkLabels,
} from "./cone-labels.js";
import { labelFadeOpacity } from "../utils/label-fade.js";
import { createCameraRig } from "./cone-camera.js";
import { createEdgeLines } from "./cone-edges.js";
import { createNodeMeshes } from "./cone-nodes.js";
import { wireConeInputs } from "./cone-inputs.js";

const initConeScene = (section, stage, blob, { onActivate } = {}) => {
  const labelsEl = section.querySelector("#cone-labels");
  if (!labelsEl) return null;

  const tooltip = createConeTooltip(section, stage);

  const model = parseConeModel(blob);
  const { nodes, strata, links, verticalLinks, extent, criticalPairs } = model;
  const entities = [...nodes, ...strata];
  if (!entities.length) return null;

  const canvas = document.createElement("canvas");
  stage.insertBefore(canvas, labelsEl);

  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
  } catch (e) {
    canvas.remove();
    stage.insertAdjacentHTML(
      "beforeend",
      '<div class="alert alert-danger" role="alert"><div class="alert-content"><p class="alert-title">WebGL unavailable</p><p class="alert-description">The 3D cone needs a WebGL2 context.</p></div></div>',
    );
    return null;
  }
  const parsedBg = parseCssColor(getComputedStyle(stage).backgroundColor);
  const bg = parsedBg && parsedBg[3] > 0 ? parsedBg : [0.09, 0.1, 0.12];
  renderer.setClearColor(new THREE.Color().setRGB(bg[0], bg[1], bg[2], THREE.SRGBColorSpace));
  renderer.setPixelRatio(window.devicePixelRatio || 1);

  const scene = new THREE.Scene();
  scene.add(new THREE.AmbientLight(0xffffff, 0.75));
  const keyLight = new THREE.DirectionalLight(0xffffff, 1.8);
  keyLight.position.set(0.6, 1, 0.8);
  scene.add(keyLight);

  const footprint = footprintOf(extent);
  const workLattice = createWorkLattice(footprint);
  scene.add(workLattice.mesh);
  const strataLattice = createStrataLattice(footprint);
  scene.add(strataLattice.mesh);
  const positionOf = new Map(entities.map((e) => [e.id, e.position]));
  const strataLinks = createStrataLinkLines(verticalLinks, positionOf);
  scene.add(strataLinks.mesh);

  const nodeIndex = new Map(entities.map((e, i) => [e.id, i]));
  const edges = createEdgeLines({ links, entities, nodeIndex, criticalPairs });
  scene.add(edges.mesh);

  const nodeMeshes = createNodeMeshes(entities);
  scene.add(nodeMeshes.sphereMesh);
  scene.add(nodeMeshes.cubeMesh);
  nodeMeshes.sync(null, new Set());

  const labelStyle = graphLabelStyle(labelsEl);
  const sceneLabels = createSceneLabels(
    scene,
    footprint,
    labelStyle,
    new Set(entities.map((e) => e.zone)),
  );
  const workLabelEls = createWorkLabels(labelsEl, entities);
  const rig = createCameraRig(entities);

  const inputs = wireConeInputs({
    section,
    canvas,
    stage,
    entities,
    verticalLinks,
    nodeMeshes,
    rig,
    edges,
    tooltip,
    onActivate,
  });

  let destroyed = false;
  let rafId = 0;

  const tick = () => {
    if (destroyed) return;
    rig.apply();
    const fade = labelFadeOpacity(rig.radius / rig.fitRadius);
    sceneLabels.update(rig.camera, stage.clientHeight, fade);
    renderer.render(scene, rig.camera);
    updateWorkLabels({
      els: workLabelEls,
      entities,
      camera: rig.camera,
      stage,
      hovered: inputs.hovered,
      sizes: nodeMeshes.sizes,
      fade,
    });
    rafId = requestAnimationFrame(tick);
  };

  const resize = () => {
    const w = Math.max(1, stage.clientWidth);
    const h = Math.max(1, stage.clientHeight);
    renderer.setSize(w, h, false);
    rig.camera.aspect = w / h;
    rig.camera.updateProjectionMatrix();
  };
  const observer = new ResizeObserver(resize);
  observer.observe(stage);
  resize();

  rafId = requestAnimationFrame(tick);

  return () => {
    destroyed = true;
    if (rafId) cancelAnimationFrame(rafId);
    observer.disconnect();
    inputs.dispose();
    tooltip.hide();
    for (const el of workLabelEls) el?.remove();
    workLattice.dispose();
    strataLattice.dispose();
    strataLinks.dispose();
    nodeMeshes.dispose();
    edges.dispose();
    sceneLabels.dispose();
    renderer.dispose();
    canvas.remove();
  };
};

export const initCone = (root, { navigate } = {}) => {
  const section = root.querySelector(".view-cone");
  if (!section) return null;
  const container = document.getElementById("cone-select");
  if (!container) return null;
  const select = new SearchableSelect({
    container,
    placeholder: "Select a work item...",
    emptyText: "No work items",
    renderOption: (item) => item.label,
    onSelect: (item) => navigate?.("cone", { id: item.id }),
  });
  const readControls = () => ({
    depth: Math.max(1, Number(section.querySelector("#cone-depth")?.value) || 1),
    max: Math.max(1, Number(section.querySelector("#cone-max")?.value) || 1),
  });
  for (const input of section.querySelectorAll("#cone-depth, #cone-max")) {
    input.addEventListener("change", () => {
      const id = section.dataset.seed;
      if (id) navigate?.("cone", { id, ...readControls() });
    });
  }
  const onNavigate = (e) => {
    const el = e.target.closest("[data-navigate]");
    if (!el) return;
    e.preventDefault();
    navigate?.(el.dataset.navigate, {});
  };
  section.addEventListener("click", onNavigate);
  const blobScript = section.querySelector("#cone-data");
  const stage = section.querySelector("#cone-stage");
  let disposeScene = null;
  if (blobScript && stage) {
    try {
      disposeScene = initConeScene(section, stage, JSON.parse(blobScript.textContent), {
        onActivate: (id) => navigate?.("packet", { id }),
      });
    } catch (e) {
      stage.insertAdjacentHTML(
        "beforeend",
        `<div class="alert alert-danger" role="alert"><div class="alert-content"><p class="alert-title">Cone failed</p><p class="alert-description">${String(
          e?.message ?? e,
        ).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c])}</p></div></div>`,
      );
    }
  }
  return () => {
    disposeScene?.();
    section.removeEventListener("click", onNavigate);
    select.destroy();
  };
};
