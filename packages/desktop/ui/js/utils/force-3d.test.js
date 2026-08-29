import { describe, expect, it } from "bun:test";
import { clusterCenters3d, createLayout } from "./force-3d.js";

const synthetic = (nodeCount, clusterCount) => {
  const nodes = [];
  const links = [];
  for (let i = 0; i < nodeCount; i++) {
    nodes.push({ id: `N-${i}`, cluster: `A-${String((i % clusterCount) + 1).padStart(2, "0")}` });
    if (i > 0) links.push({ source: `N-${i - 1}`, target: `N-${i}` });
  }
  return { nodes, links };
};

describe("clusterCenters3d", () => {
  it("pins root at the origin and spreads the rest on a sphere", () => {
    const centers = clusterCenters3d(["root", "A-01", "A-02", "A-03"], 50);
    expect(centers.get("root")).toEqual([0, 0, 0]);
    const r = centers.get("A-02");
    expect(Math.hypot(r[0], r[1], r[2])).toBeCloseTo(Math.max(240, 45 * Math.sqrt(50)), 5);
  });
});

describe("createLayout", () => {
  it("settles without NaN coordinates", () => {
    const { nodes, links } = synthetic(200, 4);
    const layout = createLayout(nodes, links);
    let speed = Infinity;
    for (let t = 0; t < 300; t++) speed = layout.step();
    expect(speed).toBeLessThan(1.0);
    expect(layout.positions.every((v) => Number.isFinite(v))).toBe(true);
  });

  it("keeps clusters separated after settling", () => {
    const { nodes, links } = synthetic(120, 3);
    const layout = createLayout(nodes, links);
    for (let t = 0; t < 300; t++) layout.step();
    const groups = new Map();
    nodes.forEach((n, i) => {
      const p = [
        layout.positions[i * 3],
        layout.positions[i * 3 + 1],
        layout.positions[i * 3 + 2],
      ];
      groups.set(n.cluster, [...(groups.get(n.cluster) ?? []), p]);
    });
    const centroids = new Map();
    for (const [cluster, pts] of groups) {
      const sum = [0, 0, 0];
      for (const p of pts) {
        sum[0] += p[0];
        sum[1] += p[1];
        sum[2] += p[2];
      }
      centroids.set(cluster, sum.map((v) => v / pts.length));
    }
    let spreadSum = 0;
    let spreadCount = 0;
    for (const [cluster, pts] of groups) {
      const c = centroids.get(cluster);
      for (const p of pts) {
        spreadSum += Math.hypot(p[0] - c[0], p[1] - c[1], p[2] - c[2]);
        spreadCount++;
      }
    }
    const spread = spreadSum / spreadCount;
    const list = [...centroids.values()];
    let minCentroid = Infinity;
    for (let a = 0; a < list.length; a++) {
      for (let b = a + 1; b < list.length; b++) {
        minCentroid = Math.min(
          minCentroid,
          Math.hypot(list[a][0] - list[b][0], list[a][1] - list[b][1], list[a][2] - list[b][2]),
        );
      }
    }
    expect(minCentroid).toBeGreaterThan(spread * 1.5);
  });

  it("spaces nodes evenly after settling", () => {
    const { nodes, links } = synthetic(120, 3);
    const layout = createLayout(nodes, links);
    for (let t = 0; t < 300; t++) layout.step();
    const nn = [];
    for (let i = 0; i < nodes.length; i++) {
      let best = Infinity;
      for (let j = 0; j < nodes.length; j++) {
        if (i === j) continue;
        const d = Math.hypot(
          layout.positions[i * 3] - layout.positions[j * 3],
          layout.positions[i * 3 + 1] - layout.positions[j * 3 + 1],
          layout.positions[i * 3 + 2] - layout.positions[j * 3 + 2],
        );
        if (d < best) best = d;
      }
      nn.push(best);
    }
    nn.sort((a, b) => a - b);
    const min = nn[0];
    const median = nn[Math.floor(nn.length / 2)];
    const p95 = nn[Math.floor(nn.length * 0.95)];
    expect(min).toBeGreaterThan(26);
    expect(median).toBeGreaterThan(30);
    expect(p95 / median).toBeLessThan(2);
  });

  it("keeps 1000-node ticks under the frame budget (B-07)", () => {
    const { nodes, links } = synthetic(1000, 8);
    const layout = createLayout(nodes, links);
    layout.step();
    const started = performance.now();
    const ticks = 100;
    for (let t = 0; t < ticks; t++) layout.step();
    const perTick = (performance.now() - started) / ticks;
    expect(perTick).toBeLessThan(16);
  });
});
