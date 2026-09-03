import * as THREE from "../vendor/three.module.min.js";
import { variantFill } from "../utils/cone-model.js";

const EDGE_LIT_RGB = [0.78, 0.8, 0.86];
const EDGE_MUTE_RGB = [0.12, 0.13, 0.16];
const REST_EDGE_RGB = [0.3, 0.34, 0.41];
const CRITICAL_EDGE_RGB = variantFill("accent");
const EDGE_OPACITY = 0.9;
const REST_EDGE_OPACITY = 0.55;
const BEAM_OPACITY = 1;
export const CRITICAL_BEAM_THICKNESS = 1.5;

const beamMatrix = (pa, pb) => {
  const a = new THREE.Vector3(pa[0], pa[1], pa[2]);
  const b = new THREE.Vector3(pb[0], pb[1], pb[2]);
  const dir = new THREE.Vector3().subVectors(b, a);
  const length = dir.length();
  const mid = new THREE.Vector3().addVectors(a, b).multiplyScalar(0.5);
  const quat = new THREE.Quaternion().setFromUnitVectors(
    new THREE.Vector3(1, 0, 0),
    dir.normalize(),
  );
  return new THREE.Matrix4().compose(
    mid,
    quat,
    new THREE.Vector3(length, CRITICAL_BEAM_THICKNESS, CRITICAL_BEAM_THICKNESS),
  );
};

export const createEdgeLines = ({ links, entities, nodeIndex, criticalPairs }) => {
  const edgeCount = links.length;
  const edgePositions = new Float32Array(Math.max(1, edgeCount * 6));
  const edgeColors = new Float32Array(Math.max(1, edgeCount * 6));
  const criticalLinks = [];
  links.forEach((l, i) => {
    const a = nodeIndex.get(l.source);
    const b = nodeIndex.get(l.target);
    if (a === undefined || b === undefined) return;
    const pa = entities[a].position;
    const pb = entities[b].position;
    edgePositions[i * 6] = pa[0];
    edgePositions[i * 6 + 1] = pa[1];
    edgePositions[i * 6 + 2] = pa[2];
    edgePositions[i * 6 + 3] = pb[0];
    edgePositions[i * 6 + 4] = pb[1];
    edgePositions[i * 6 + 5] = pb[2];
    if (criticalPairs.has(`${l.source}>${l.target}`)) {
      criticalLinks.push({ linkIndex: i, pa, pb });
    }
  });
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute("position", new THREE.BufferAttribute(edgePositions, 3));
  geometry.setAttribute("color", new THREE.BufferAttribute(edgeColors, 3));
  const material = new THREE.LineBasicMaterial({
    vertexColors: true,
    transparent: true,
    opacity: REST_EDGE_OPACITY,
  });
  const mesh = new THREE.LineSegments(geometry, material);

  const beamGeometry = new THREE.BoxGeometry(1, 1, 1);
  const beamMaterial = new THREE.MeshBasicMaterial({
    transparent: true,
    opacity: BEAM_OPACITY,
  });
  const beamMesh = new THREE.InstancedMesh(
    beamGeometry,
    beamMaterial,
    Math.max(1, criticalLinks.length),
  );
  beamMesh.count = criticalLinks.length;
  criticalLinks.forEach((c, k) => {
    beamMesh.setMatrixAt(k, beamMatrix(c.pa, c.pb));
  });
  beamMesh.computeBoundingSphere();

  const group = new THREE.Group();
  group.add(mesh);
  group.add(beamMesh);

  const paintEdge = (i, rgb) => {
    for (let v = 0; v < 2; v++) {
      edgeColors[i * 6 + v * 3] = rgb[0];
      edgeColors[i * 6 + v * 3 + 1] = rgb[1];
      edgeColors[i * 6 + v * 3 + 2] = rgb[2];
    }
  };
  const repaint = (i, hoverId) => {
    const l = links[i];
    if (hoverId) {
      return l.source === hoverId || l.target === hoverId ? EDGE_LIT_RGB : EDGE_MUTE_RGB;
    }
    return criticalPairs.has(`${l.source}>${l.target}`) ? CRITICAL_EDGE_RGB : REST_EDGE_RGB;
  };
  const beamColor = new THREE.Color();
  const update = (hoverId) => {
    material.opacity = hoverId ? EDGE_OPACITY : REST_EDGE_OPACITY;
    beamMaterial.opacity = hoverId ? EDGE_OPACITY : BEAM_OPACITY;
    mesh.visible = true;
    links.forEach((l, i) => paintEdge(i, repaint(i, hoverId)));
    geometry.attributes.color.needsUpdate = true;
    criticalLinks.forEach((c, k) => {
      const rgb = hoverId ? repaint(c.linkIndex, hoverId) : CRITICAL_EDGE_RGB;
      beamMesh.setColorAt(
        k,
        beamColor.setRGB(rgb[0], rgb[1], rgb[2], THREE.SRGBColorSpace),
      );
    });
    if (beamMesh.instanceColor) beamMesh.instanceColor.needsUpdate = true;
  };
  update(null);
  return {
    mesh: group,
    beamMesh,
    update,
    dispose() {
      geometry.dispose();
      material.dispose();
      beamGeometry.dispose();
      beamMaterial.dispose();
    },
  };
};
