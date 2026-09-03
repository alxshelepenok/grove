import { describe, expect, it } from "bun:test";
import * as THREE from "../vendor/three.module.min.js";
import { variantFill } from "../utils/cone-model.js";
import { CRITICAL_BEAM_THICKNESS, createEdgeLines } from "./cone-edges.js";

const MUTE_RGB = [0.12, 0.13, 0.16];
const LIT_RGB = [0.78, 0.8, 0.86];

const entities = [
  { id: "W-a", position: [-40, 0, 0] },
  { id: "W-b", position: [0, 0, 40] },
  { id: "W-c", position: [40, 0, 40] },
];
const nodeIndex = new Map(entities.map((e, i) => [e.id, i]));
const links = [
  { source: "W-a", target: "W-b" },
  { source: "W-b", target: "W-c" },
  { source: "W-a", target: "W-c" },
];
const criticalPairs = new Set(["W-a>W-b", "W-b>W-a"]);

const beamTransform = (beamMesh, k) => {
  const matrix = new THREE.Matrix4().fromArray(
    beamMesh.instanceMatrix.array.slice(k * 16, k * 16 + 16),
  );
  const position = new THREE.Vector3();
  const quaternion = new THREE.Quaternion();
  const scale = new THREE.Vector3();
  matrix.decompose(position, quaternion, scale);
  return { position, quaternion, scale };
};

const beamColorAt = (beamMesh, k) => {
  const c = beamMesh.instanceColor.array;
  return [c[k * 3], c[k * 3 + 1], c[k * 3 + 2]];
};

const srgbToWorking = (rgb) => {
  const color = new THREE.Color().setRGB(rgb[0], rgb[1], rgb[2], THREE.SRGBColorSpace);
  return [color.r, color.g, color.b];
};

describe("createEdgeLines", () => {
  it("stretches one beam per critical link between node centers", () => {
    const edges = createEdgeLines({ links, entities, nodeIndex, criticalPairs });
    expect(edges.beamMesh.count).toBe(1, "only the critical link gets a beam");
    const { position, scale } = beamTransform(edges.beamMesh, 0);
    expect(position.x).toBeCloseTo(-20);
    expect(position.y).toBeCloseTo(0);
    expect(position.z).toBeCloseTo(20);
    expect(scale.x).toBeCloseTo(Math.hypot(40, 40), 5);
    expect(scale.y).toBeCloseTo(CRITICAL_BEAM_THICKNESS, 5);
    expect(scale.z).toBeCloseTo(CRITICAL_BEAM_THICKNESS, 5);
    edges.dispose();
  });

  it("paints beams accent at rest and lit or mute on hover", () => {
    const edges = createEdgeLines({ links, entities, nodeIndex, criticalPairs });
    const accent = srgbToWorking(variantFill("accent"));
    const mute = srgbToWorking(MUTE_RGB);
    expect(beamColorAt(edges.beamMesh, 0)).toEqual(accent.map((v) => expect.closeTo(v, 5)));
    edges.update("W-c");
    expect(beamColorAt(edges.beamMesh, 0)).toEqual(mute.map((v) => expect.closeTo(v, 5)));
    edges.update("W-a");
    const lit = srgbToWorking(LIT_RGB);
    expect(beamColorAt(edges.beamMesh, 0)).toEqual(lit.map((v) => expect.closeTo(v, 5)));
    edges.update(null);
    expect(beamColorAt(edges.beamMesh, 0)).toEqual(accent.map((v) => expect.closeTo(v, 5)));
    edges.dispose();
  });

  it("keeps the line segments inside the returned group", () => {
    const edges = createEdgeLines({ links, entities, nodeIndex, criticalPairs });
    expect(edges.mesh.type).toBe("Group");
    expect(edges.mesh.children.some((c) => c instanceof THREE.LineSegments)).toBe(true);
    expect(edges.mesh.children.some((c) => c instanceof THREE.InstancedMesh)).toBe(true);
    edges.dispose();
  });
});
