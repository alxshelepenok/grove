import * as THREE from "../vendor/three.module.min.js";
import {
  AREA_Y,
  CELL_SPACING,
  FILE_Y,
  GOAL_Y,
  STRATUM_GAP,
  THEME_Y,
  WORK_Y,
  screenSpaceSpriteHeight,
} from "../utils/cone-model.js";
import { HOVER_SCALE } from "./cone-nodes.js";

const LABEL_CANVAS_PX = 48;
const AXIS_LABEL_Y = FILE_Y - STRATUM_GAP / 2;
const STRATUM_LABELS = [
  ["Files", FILE_Y],
  ["Work", WORK_Y],
  ["Themes", THEME_Y],
  ["Goals", GOAL_Y],
  ["Areas", AREA_Y],
];
const ZONE_LABELS = [
  ["Dependencies", "backward"],
  ["Seed", "seed"],
  ["Impact", "forward"],
];
const ZONE_LABEL_RGB = {
  backward: "152,179,240",
  seed: "237,242,247",
  forward: "245,181,83",
};

const makeLabelSprite = (text, fill, font) => {
  const canvas = document.createElement("canvas");
  const measure = canvas.getContext("2d");
  measure.font = font;
  const padding = Math.ceil(LABEL_CANVAS_PX * 0.25);
  canvas.width = Math.ceil(measure.measureText(text).width) + 2 * padding;
  canvas.height = LABEL_CANVAS_PX + 2 * padding;
  const ctx = canvas.getContext("2d");
  ctx.font = font;
  ctx.fillStyle = fill;
  ctx.textBaseline = "middle";
  ctx.fillText(text, padding, canvas.height / 2);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  const material = new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: false });
  const sprite = new THREE.Sprite(material);
  sprite.renderOrder = 10;
  return {
    sprite,
    texture,
    material,
    aspect: canvas.width / canvas.height,
    emFraction: LABEL_CANVAS_PX / canvas.height,
  };
};

export const graphLabelStyle = (labelsEl) => {
  const probe = document.createElement("div");
  probe.className = "graph-label";
  probe.hidden = true;
  labelsEl.appendChild(probe);
  const cs = getComputedStyle(probe);
  const style = {
    font: `400 ${LABEL_CANVAS_PX}px ${cs.fontFamily || "monospace"}`,
    px: parseFloat(cs.fontSize) || 10,
    color: cs.color || "rgb(154, 163, 178)",
  };
  probe.remove();
  return style;
};

export const createSceneLabels = (scene, footprint, labelStyle, occupiedZones) => {
  const labels = [];
  const labelX = footprint.x0 - CELL_SPACING;
  const labelZ = footprint.z1 + CELL_SPACING;
  for (const [text, y] of STRATUM_LABELS) {
    const label = makeLabelSprite(text, labelStyle.color, labelStyle.font);
    label.sprite.position.set(labelX, y, labelZ);
    scene.add(label.sprite);
    labels.push(label);
  }
  const seedL = -0.5 * CELL_SPACING;
  const seedR = 0.5 * CELL_SPACING;
  const zoneSpans = {
    backward: [footprint.x0, seedL],
    seed: [seedL, seedR],
    forward: [seedR, footprint.x1],
  };
  for (const [text, zone] of ZONE_LABELS) {
    if (!occupiedZones.has(zone)) continue;
    const [from, to] = zoneSpans[zone];
    if (to - from < CELL_SPACING / 2) continue;
    const label = makeLabelSprite(text, `rgb(${ZONE_LABEL_RGB[zone]})`, labelStyle.font);
    label.sprite.position.set((from + to) / 2, AXIS_LABEL_Y, labelZ);
    scene.add(label.sprite);
    labels.push(label);
  }
  return {
    update(camera, stageH, fade = 1) {
      const h = Math.max(1, stageH);
      for (const label of labels) {
        const dist = camera.position.distanceTo(label.sprite.position);
        const height = screenSpaceSpriteHeight(labelStyle.px, label.emFraction, camera.fov, h, dist);
        label.sprite.scale.set(height * label.aspect, height, 1);
        label.sprite.visible = fade > 0;
        label.material.opacity = fade;
      }
    },
    dispose() {
      for (const label of labels) {
        scene.remove(label.sprite);
        label.texture.dispose();
        label.material.dispose();
      }
    },
  };
};

export const createWorkLabels = (labelsEl, entities) =>
  entities.map((n) => {
    if (n.kind !== "w") return null;
    const el = document.createElement("div");
    el.className = n.critical ? "graph-label graph-label-critical" : "graph-label";
    el.textContent = n.id;
    labelsEl.appendChild(el);
    return el;
  });

const projected = new THREE.Vector3();

export const updateWorkLabels = ({ els, entities, camera, stage, hovered, sizes, fade = 1 }) => {
  const w = stage.clientWidth;
  const h = stage.clientHeight;
  const fovScale = h / (2 * Math.tan((camera.fov * Math.PI) / 360));
  for (let i = 0; i < entities.length; i++) {
    const el = els[i];
    if (!el) continue;
    projected.set(
      entities[i].position[0],
      entities[i].position[1],
      entities[i].position[2],
    );
    const dist = camera.position.distanceTo(projected);
    projected.project(camera);
    if (fade <= 0 || projected.z > 1 || projected.z < -1) {
      el.hidden = true;
      continue;
    }
    el.hidden = false;
    el.style.opacity = fade.toFixed(3);
    const radiusPx =
      ((sizes[i] * (entities[i] === hovered ? HOVER_SCALE : 1)) / 2 / dist) * fovScale;
    const x = ((projected.x + 1) / 2) * w;
    const y = ((1 - projected.y) / 2) * h - radiusPx - 3;
    el.style.transform = `translate(${x.toFixed(1)}px, ${y.toFixed(1)}px) translate(-50%, -100%)`;
  }
};
