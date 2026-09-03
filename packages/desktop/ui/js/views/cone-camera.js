import * as THREE from "../vendor/three.module.min.js";
import { NODE_SIZE } from "../utils/cone-model.js";

const ROTATE_SPEED = 0.006;
const PAN_SPEED = 0.0016;
const ZOOM_SPEED = 0.0012;
const MIN_RADIUS = 80;
const MAX_RADIUS = 6000;
const MIN_PHI = 0.05;

export const createCameraRig = (entities) => {
  const camera = new THREE.PerspectiveCamera(50, 1, 1, 8000);
  const bounds = entities.reduce(
    (acc, n) => {
      for (let a = 0; a < 3; a++) {
        acc.min[a] = Math.min(acc.min[a], n.position[a]);
        acc.max[a] = Math.max(acc.max[a], n.position[a]);
      }
      return acc;
    },
    { min: [Infinity, Infinity, Infinity], max: [-Infinity, -Infinity, -Infinity] },
  );
  const target = new THREE.Vector3(
    (bounds.min[0] + bounds.max[0]) / 2,
    (bounds.min[1] + bounds.max[1]) / 2,
    (bounds.min[2] + bounds.max[2]) / 2,
  );
  let maxDist = 0;
  for (const n of entities) {
    maxDist = Math.max(maxDist, target.distanceTo(new THREE.Vector3(...n.position)));
  }
  const spherical = {
    radius: Math.max(320, (maxDist + NODE_SIZE) * 2.2),
    theta: 0,
    phi: 0.95,
  };
  const fitRadius = spherical.radius;
  return {
    camera,
    target,
    fitRadius,
    get radius() {
      return spherical.radius;
    },
    apply() {
      camera.position
        .setFromSphericalCoords(spherical.radius, spherical.phi, spherical.theta)
        .add(target);
      camera.lookAt(target);
    },
    rotateBy(dx, dy) {
      spherical.theta -= dx * ROTATE_SPEED;
      spherical.phi = Math.max(
        MIN_PHI,
        Math.min(Math.PI - MIN_PHI, spherical.phi - dy * ROTATE_SPEED),
      );
    },
    panBy(dx, dy) {
      const panScale = spherical.radius * PAN_SPEED;
      const right = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 0);
      const up = new THREE.Vector3().setFromMatrixColumn(camera.matrix, 1);
      target.addScaledVector(right, -dx * panScale).addScaledVector(up, dy * panScale);
    },
    zoomBy(deltaY) {
      spherical.radius = Math.max(
        MIN_RADIUS,
        Math.min(MAX_RADIUS, spherical.radius * Math.exp(deltaY * ZOOM_SPEED)),
      );
    },
  };
};
