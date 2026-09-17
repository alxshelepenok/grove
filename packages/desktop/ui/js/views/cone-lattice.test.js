import { describe, expect, it } from "bun:test";
import * as THREE from "../vendor/three.module.min.js";
import {
  createStrataLattice,
  createStrataLinkLines,
  createWorkLattice,
} from "./cone-lattice.js";

const footprint = { x0: -60, x1: 60, z0: -60, z1: 60 };

describe("createWorkLattice", () => {
  it("renders behind edges without writing depth", () => {
    const lattice = createWorkLattice(footprint);
    expect(lattice.mesh).toBeInstanceOf(THREE.LineSegments);
    expect(lattice.mesh.renderOrder).toBe(-2);
    expect(lattice.mesh.material.depthWrite).toBe(false);
    lattice.dispose();
  });
});

describe("createStrataLattice", () => {
  it("renders behind edges without writing depth", () => {
    const lattice = createStrataLattice(footprint);
    expect(lattice.mesh).toBeInstanceOf(THREE.LineSegments);
    expect(lattice.mesh.renderOrder).toBe(-2);
    expect(lattice.mesh.material.depthWrite).toBe(false);
    lattice.dispose();
  });
});

describe("createStrataLinkLines", () => {
  it("renders above the lattice and behind edges without writing depth", () => {
    const positionOf = new Map([
      ["W-a", [-40, 15, 0]],
      ["G-a", [0, 240, 0]],
    ]);
    const links = createStrataLinkLines([{ from: "W-a", to: "G-a" }], positionOf);
    expect(links.mesh).toBeInstanceOf(THREE.LineSegments);
    expect(links.mesh.renderOrder).toBe(-1);
    expect(links.mesh.material.depthWrite).toBe(false);
    const positions = links.mesh.geometry.attributes.position.array;
    expect([...positions]).toEqual([-40, 15, 0, 0, 240, 0]);
    links.dispose();
  });

  it("skips links whose endpoints have no position", () => {
    const links = createStrataLinkLines(
      [{ from: "W-x", to: "G-a" }],
      new Map([["G-a", [0, 240, 0]]]),
    );
    expect(links.mesh.geometry.attributes.position.array).toHaveLength(0);
    links.dispose();
  });
});
