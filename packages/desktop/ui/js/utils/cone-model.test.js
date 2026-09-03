import { describe, expect, it } from "bun:test";
import {
  AREA_Y,
  CELL_SPACING,
  FILE_Y,
  GOAL_Y,
  LATTICE_LIFT,
  THEME_Y,
  WORK_Y,
  parseConeModel,
  screenSpaceSpriteHeight,
  variantFill,
  zoneFill,
} from "./cone-model.js";

const diamond = {
  seed: "W-0",
  backward: ["W-1", "W-2"],
  order: ["W-2", "W-1"],
  forward: ["W-3"],
  truncated: false,
  strata: {
    goals: [
      { id: "G-01", kind: "g", title: "Goal one", status: "verified", members: ["W-0", "W-1"] },
    ],
    areas: [{ id: "A-01", kind: "a", title: "Area", status: "present", goals: ["G-01"] }],
    themes: [{ id: "T-01", kind: "t", title: "Theme", status: "open", members: ["W-1"] }],
    files: [
      { id: "src/a.rs", kind: "f", title: "src/a.rs", status: "present", touchers: ["W-0"] },
      { id: "src/b.rs", kind: "f", title: "src/b.rs", status: "present", touchers: ["W-3"] },
    ],
  },
  nodes: [
    { id: "W-0", kind: "w", title: "Seed", status: "progress" },
    { id: "W-1", kind: "w", title: "Dep one", status: "done" },
    { id: "W-2", kind: "w", title: "Dep two", status: "proposed" },
    { id: "W-3", kind: "w", title: "Impact", status: "proposed" },
  ],
  edges: [
    { from: "W-1", to: "W-0" },
    { from: "W-2", to: "W-1" },
    { from: "W-0", to: "W-3" },
  ],
};

const isOnLattice = (p) =>
  [p[0], p[2]].every((v) => Math.abs(v / CELL_SPACING - Math.round(v / CELL_SPACING)) < 1e-9);

describe("parseConeModel", () => {
  it("places work members on integer hop columns with zones", () => {
    const { nodes } = parseConeModel(diamond);
    const byId = Object.fromEntries(nodes.map((n) => [n.id, n]));
    expect(byId["W-0"].position).toEqual([0, WORK_Y + LATTICE_LIFT, 0]);
    expect(byId["W-1"].position[0]).toBe(-CELL_SPACING);
    expect(byId["W-2"].position[0]).toBe(-2 * CELL_SPACING);
    expect(byId["W-3"].position[0]).toBe(CELL_SPACING);
    expect(byId["W-0"].zone).toBe("seed");
    expect(byId["W-1"].zone).toBe("backward");
    expect(byId["W-3"].zone).toBe("forward");
  });

  it("spreads a wide layer on integer rows in contraction order", () => {
    const blob = {
      ...diamond,
      backward: ["W-a", "W-b", "W-c", "W-d"],
      order: ["W-a", "W-b", "W-c", "W-d"],
      forward: [],
      nodes: [
        { id: "W-0", kind: "w", title: "Seed", status: "progress" },
        ...["W-a", "W-b", "W-c", "W-d"].map((id) => ({
          id,
          kind: "w",
          title: id,
          status: "proposed",
        })),
      ],
      edges: ["W-a", "W-b", "W-c", "W-d"].map((id) => ({ from: id, to: "W-0" })),
    };
    const { nodes } = parseConeModel(blob);
    const layer = nodes.filter((n) => n.hop === -1).sort((a, b) => a.order - b.order);
    expect(layer.map((n) => n.position[2])).toEqual(
      [-2, -1, 0, 1].map((k) => k * CELL_SPACING),
    );
    for (const n of layer) expect(n.position[0]).toBe(-CELL_SPACING);
  });

  it("keeps every entity on the integer x/z lattice with no shared cell per plane", () => {
    const { nodes, strata } = parseConeModel(diamond);
    for (const e of [...nodes, ...strata]) {
      expect(isOnLattice(e.position)).toBe(true, `${e.id} sits on an integer cell`);
    }
    const planes = new Map();
    for (const e of [...nodes, ...strata]) {
      const key = `${e.position[1]}`;
      const cell = `${e.position[0]},${e.position[2]}`;
      if (!planes.has(key)) planes.set(key, new Set());
      expect(planes.get(key).has(cell)).toBe(false, `${e.id} collides on plane ${key}`);
      planes.get(key).add(cell);
    }
  });

  it("stacks strata on their own planes, lifted above their lattices", () => {
    const { strata } = parseConeModel(diamond);
    const byId = Object.fromEntries(strata.map((n) => [n.id, n]));
    expect(byId["G-01"].position[1]).toBe(GOAL_Y + LATTICE_LIFT);
    expect(byId["A-01"].position[1]).toBe(AREA_Y + LATTICE_LIFT);
    expect(byId["T-01"].position[1]).toBe(THEME_Y + LATTICE_LIFT);
    expect(byId["src/a.rs"].position[1]).toBe(FILE_Y + LATTICE_LIFT);
    expect(byId["src/b.rs"].position[1]).toBe(FILE_Y + LATTICE_LIFT);
  });

  it("resolves colliding strata to free cells of the shared lattice", () => {
    const blob = {
      ...diamond,
      strata: {
        ...diamond.strata,
        goals: [
          ...diamond.strata.goals,
          { id: "G-02", kind: "g", title: "Goal two", status: "verified", members: ["W-0", "W-1"] },
        ],
      },
    };
    const { strata } = parseConeModel(blob);
    const cells = strata
      .filter((e) => e.kind === "g")
      .map((e) => `${e.position[0]},${e.position[2]}`);
    expect(new Set(cells).size).toBe(2, "both goals keep distinct cells");
    for (const e of strata.filter((e) => e.kind === "g")) {
      expect(isOnLattice(e.position)).toBe(true);
    }
  });

  it("emits the shared footprint extent", () => {
    const { extent } = parseConeModel(diamond);
    expect(extent.colMin).toBe(-2);
    expect(extent.colMax).toBe(1);
    expect(extent.rowMin).toBe(0);
    expect(extent.rowMax).toBe(0);
  });

  it("emits membership links that pierce the planes", () => {
    const { verticalLinks } = parseConeModel(diamond);
    const kinds = verticalLinks.map((l) => l.kind).sort();
    expect(kinds).toEqual(["area", "member", "member", "surface", "surface", "theme"]);
    expect(verticalLinks.filter((l) => l.kind === "member")).toEqual([
      { from: "W-0", to: "G-01", kind: "member" },
      { from: "W-1", to: "G-01", kind: "member" },
    ]);
  });

  it("carries the critical path and its consecutive pairs", () => {
    const blob = {
      ...diamond,
      critical: ["W-2", "W-1", "W-0", "W-missing"],
      nodes: diamond.nodes.map((n) => ({ ...n, critical: n.id === "W-1" })),
    };
    const { critical, criticalPairs, nodes } = parseConeModel(blob);
    expect(critical).toEqual(["W-2", "W-1", "W-0"], "unknown ids drop out of the path");
    expect(criticalPairs.has("W-2>W-1")).toBe(true);
    expect(criticalPairs.has("W-1>W-2")).toBe(true);
    expect(criticalPairs.has("W-1>W-0")).toBe(true);
    expect(criticalPairs.has("W-0>W-3")).toBe(false, "non-consecutive pairs stay unmarked");
    const byId = Object.fromEntries(nodes.map((n) => [n.id, n]));
    expect(byId["W-1"].critical).toBe(true);
    expect(byId["W-3"].critical).toBe(false);
  });

  it("drops edges that leave the member set", () => {
    const blob = {
      ...diamond,
      edges: [...diamond.edges, { from: "W-9", to: "W-0" }],
    };
    const { links } = parseConeModel(blob);
    expect(links.length).toBe(3);
  });

  it("marks truncation and maps variant fills", () => {
    const { truncated } = parseConeModel({ ...diamond, truncated: true });
    expect(truncated).toBe(true);
    expect(variantFill("danger")).toEqual([
      expect.closeTo(0.886, 2),
      expect.closeTo(0.354, 2),
      expect.closeTo(0.354, 2),
    ]);
    expect(variantFill("unknown-variant")).toEqual(variantFill("neutral"));
  });

  it("maps distinct zone fills for the file cubes", () => {
    const backward = zoneFill("backward");
    const seed = zoneFill("seed");
    const forward = zoneFill("forward");
    expect(backward).not.toEqual(seed);
    expect(backward).not.toEqual(forward);
    expect(seed).not.toEqual(forward);
    expect(zoneFill("unknown-zone")).toEqual(seed);
  });

  it("lands the sprite em box at the probed identifier pixels at every distance", () => {
    const px = 10;
    const emFraction = 48 / 72;
    const viewportH = 800;
    const fov = 50;
    const project = (worldHeight, dist) =>
      (worldHeight * (viewportH / (2 * Math.tan((fov * Math.PI) / 360)))) / dist;
    for (const dist of [200, 500, 900]) {
      const worldHeight = screenSpaceSpriteHeight(px, emFraction, fov, viewportH, dist);
      expect(project(worldHeight, dist) * emFraction).toBeCloseTo(px, 6);
    }
  });
});
