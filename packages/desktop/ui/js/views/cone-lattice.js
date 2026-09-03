import * as THREE from "../vendor/three.module.min.js";
import {
  AREA_Y,
  CELL_SPACING,
  FILE_Y,
  GOAL_Y,
  THEME_Y,
  WORK_Y,
} from "../utils/cone-model.js";

const LATTICE_COLOR = 0x424a5a;
const WORK_GRID_OPACITY = 0.5;
const STRATA_GRID_OPACITY = 0.22;
const STRATA_LINK_COLOR = 0x4a5568;
const STRATA_LINK_OPACITY = 0.22;

const buildPlaneGrid = (y, x0, x1, z0, z1, out) => {
  const cols = Math.max(1, Math.round((x1 - x0) / CELL_SPACING));
  const rows = Math.max(1, Math.round((z1 - z0) / CELL_SPACING));
  for (let c = 0; c <= cols; c++) {
    const x = x0 + c * CELL_SPACING;
    out.push(x, y, z0, x, y, z1);
  }
  for (let r = 0; r <= rows; r++) {
    const z = z0 + r * CELL_SPACING;
    out.push(x0, y, z, x1, y, z);
  }
};

const makeLattice = (lines, opacity) => {
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(new Float32Array(lines), 3));
  const material = new THREE.LineBasicMaterial({
    color: LATTICE_COLOR,
    transparent: true,
    opacity,
  });
  return { mesh: new THREE.LineSegments(geometry, material), geometry, material };
};

export const footprintOf = (extent) => ({
  x0: (extent.colMin - 1.5) * CELL_SPACING,
  x1: (extent.colMax + 1.5) * CELL_SPACING,
  z0: (extent.rowMin - 1.5) * CELL_SPACING,
  z1: (extent.rowMax + 1.5) * CELL_SPACING,
});

export const createWorkLattice = (footprint) => {
  const lines = [];
  buildPlaneGrid(WORK_Y, footprint.x0, footprint.x1, footprint.z0, footprint.z1, lines);
  const lattice = makeLattice(lines, WORK_GRID_OPACITY);
  return {
    mesh: lattice.mesh,
    dispose() {
      lattice.geometry.dispose();
      lattice.material.dispose();
    },
  };
};

export const createStrataLattice = (footprint) => {
  const lines = [];
  for (const y of [AREA_Y, GOAL_Y, THEME_Y, FILE_Y]) {
    buildPlaneGrid(y, footprint.x0, footprint.x1, footprint.z0, footprint.z1, lines);
  }
  const lattice = makeLattice(lines, STRATA_GRID_OPACITY);
  return {
    mesh: lattice.mesh,
    dispose() {
      lattice.geometry.dispose();
      lattice.material.dispose();
    },
  };
};

export const createStrataLinkLines = (verticalLinks, positionOf) => {
  const positions = [];
  for (const l of verticalLinks) {
    const a = positionOf.get(l.from);
    const b = positionOf.get(l.to);
    if (!a || !b) continue;
    positions.push(a[0], a[1], a[2], b[0], b[1], b[2]);
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    "position",
    new THREE.BufferAttribute(new Float32Array(positions), 3),
  );
  const material = new THREE.LineBasicMaterial({
    color: STRATA_LINK_COLOR,
    transparent: true,
    opacity: STRATA_LINK_OPACITY,
  });
  return {
    mesh: new THREE.LineSegments(geometry, material),
    dispose() {
      geometry.dispose();
      material.dispose();
    },
  };
};
