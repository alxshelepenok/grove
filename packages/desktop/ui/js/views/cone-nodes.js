import * as THREE from "../vendor/three.module.min.js";
import { nodeAlpha } from "../utils/graph-model.js";
import { NODE_SIZE, variantFill, zoneFill } from "../utils/cone-model.js";

export const HOVER_SCALE = 1.25;

const FINISHED_DIM = 0.45;
const FINISHED_STATUSES = new Set(["done", "rejected", "archived"]);

const surfaceDim = (file, statusOf) => {
  const touchers = (file.touchers ?? []).filter((t) => statusOf.has(t));
  return touchers.length && touchers.every((t) => FINISHED_STATUSES.has(statusOf.get(t)))
    ? FINISHED_DIM
    : 1;
};

export const createNodeMeshes = (entities) => {
  const sizes = new Float32Array(entities.length);
  const statusOf = new Map(entities.filter((n) => n.kind === "w").map((n) => [n.id, n.status]));
  const baseColor = [];
  entities.forEach((n, i) => {
    sizes[i] = NODE_SIZE;
    const fill =
      n.kind === "a" || n.kind === "g" || n.kind === "t"
        ? variantFill("neutral")
        : zoneFill(n.zone);
    const dim = n.kind === "f" ? surfaceDim(n, statusOf) : nodeAlpha(n);
    baseColor.push([fill[0] * dim, fill[1] * dim, fill[2] * dim]);
  });

  const nodeGeometry = new THREE.BoxGeometry(1, 1, 1);
  const sphereGeometry = new THREE.SphereGeometry(0.5, 24, 16);
  const material = new THREE.MeshLambertMaterial();
  const sphereIds = [];
  const cubeIds = [];
  entities.forEach((n, i) => (n.kind === "f" ? cubeIds : sphereIds).push(i));
  const sphereOf = new Map(sphereIds.map((entityIdx, inst) => [entityIdx, inst]));
  const cubeOf = new Map(cubeIds.map((entityIdx, inst) => [entityIdx, inst]));
  const sphereMesh = new THREE.InstancedMesh(sphereGeometry, material, sphereIds.length);
  const cubeMesh = new THREE.InstancedMesh(nodeGeometry, material, cubeIds.length);
  const color = new THREE.Color();
  for (const [mesh, ids] of [
    [sphereMesh, sphereIds],
    [cubeMesh, cubeIds],
  ]) {
    ids.forEach((entityIdx, inst) => {
      mesh.setColorAt(
        inst,
        color.setRGB(
          baseColor[entityIdx][0],
          baseColor[entityIdx][1],
          baseColor[entityIdx][2],
          THREE.SRGBColorSpace,
        ),
      );
    });
    if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
  }

  const matrix = new THREE.Matrix4();
  const sync = (hovered, linked) => {
    for (let i = 0; i < entities.length; i++) {
      const s = sizes[i] * (entities[i] === hovered || linked.has(entities[i].id) ? HOVER_SCALE : 1);
      matrix.makeScale(s, s, s);
      matrix.setPosition(
        entities[i].position[0],
        entities[i].position[1],
        entities[i].position[2],
      );
      const si = sphereOf.get(i);
      if (si !== undefined) sphereMesh.setMatrixAt(si, matrix);
      const ci = cubeOf.get(i);
      if (ci !== undefined) cubeMesh.setMatrixAt(ci, matrix);
    }
    sphereMesh.instanceMatrix.needsUpdate = true;
    cubeMesh.instanceMatrix.needsUpdate = true;
    sphereMesh.computeBoundingSphere();
    cubeMesh.computeBoundingSphere();
  };

  return {
    sphereMesh,
    cubeMesh,
    sphereIds,
    cubeIds,
    sizes,
    sync,
    dispose() {
      sphereGeometry.dispose();
      nodeGeometry.dispose();
      material.dispose();
    },
  };
};
