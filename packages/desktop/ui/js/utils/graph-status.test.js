import { describe, expect, it } from "bun:test";
import { KIND_INFO, statusVariant } from "./graph-status.js";

const EXPECTED = {
  g: { unverified: "warning", partial: "info", verified: "success", declined: "danger" },
  w: {
    proposed: "neutral",
    ready: "info",
    progress: "accent",
    done: "success",
    rejected: "danger",
    archived: "neutral",
  },
  q: { open: "warning", answered: "success", deferred: "neutral", dropped: "neutral" },
  b: {
    proposed: "neutral",
    testing: "info",
    validated: "success",
    invalidated_acceptable: "warning",
    invalidated_blocking: "danger",
  },
  t: { open: "info", done: "success" },
  y: { proposed: "warning", active: "success", stale: "danger", superseded: "neutral" },
  d: { proposed: "warning", accepted: "success", rejected: "danger", superseded: "neutral" },
};

describe("statusVariant", () => {
  it("maps every core status to its badge variant", () => {
    const wrong = [];
    for (const [kind, table] of Object.entries(EXPECTED)) {
      for (const [status, expected] of Object.entries(table)) {
        const variant = statusVariant({ kind, status });
        if (variant !== expected) wrong.push(`${kind}/${status}: ${variant} != ${expected}`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("falls back to neutral for statusless kinds", () => {
    expect(statusVariant({ kind: "a" })).toBe("neutral");
    expect(statusVariant({ kind: "root" })).toBe("neutral");
  });

  it("labels every graph kind", () => {
    for (const kind of [...Object.keys(EXPECTED), "a", "root"]) {
      expect(typeof KIND_INFO[kind]).toBe("string");
    }
  });
});
