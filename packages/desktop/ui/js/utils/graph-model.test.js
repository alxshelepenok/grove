import { describe, expect, it } from "bun:test";
import {
  createClusterFills,
  hexToRgb,
  hslToRgb,
  nodeAlpha,
  nodeFillFor,
  nodeRadius,
  parseGraphModel,
} from "./graph-model.js";

describe("parseGraphModel", () => {
  it("maps server edges to links and clones nodes", () => {
    const { nodes, links } = parseGraphModel({
      nodes: [{ id: "W-01", kind: "w" }],
      edges: [{ from: "G-01", to: "W-01", label: "implements", virtual: true }],
    });
    expect(nodes).toEqual([{ id: "W-01", kind: "w" }]);
    expect(links).toEqual([
      { source: "G-01", target: "W-01", label: "implements", virtual: true },
    ]);
  });

  it("tolerates missing collections", () => {
    expect(parseGraphModel({})).toEqual({ nodes: [], links: [] });
    expect(parseGraphModel(null)).toEqual({ nodes: [], links: [] });
  });
});

describe("colors", () => {
  it("converts hex and hsl triples", () => {
    expect(hexToRgb("#ffffff")).toEqual([1, 1, 1]);
    expect(hexToRgb("#000000")).toEqual([0, 0, 0]);
    expect(hslToRgb(0, 0, 50)).toEqual([0.5, 0.5, 0.5]);
  });

  it("fills clusters deterministically with a root and palette fallback", () => {
    const nodes = [{ cluster: "A-01" }, { cluster: "A-02" }, { cluster: "root" }];
    const fills = createClusterFills(nodes);
    expect(fills.get("root")).toEqual([0.45, 0.45, 0.48]);
    const again = createClusterFills(nodes);
    expect(fills.get("A-01")).toEqual(again.get("A-01"));
    expect(nodeFillFor(fills)({ cluster: "nope" })).toEqual(hexToRgb("#5a5a5a"));
  });
});

describe("node metrics", () => {
  it("dims archived and finished work, enlarges the root", () => {
    expect(nodeAlpha({ archived: true })).toBe(0.3);
    expect(nodeAlpha({ kind: "w", status: "done" })).toBe(0.45);
    expect(nodeAlpha({ kind: "w", status: "progress" })).toBe(1.0);
    expect(nodeRadius({ kind: "root" })).toBe(15);
    expect(nodeRadius({ kind: "w" })).toBe(9);
  });
});
