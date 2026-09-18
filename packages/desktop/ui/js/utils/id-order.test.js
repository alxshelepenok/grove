import { describe, expect, it } from "bun:test";
import { compareIds } from "./id-order.js";

const sorted = (ids) => [...ids].sort(compareIds);

describe("compareIds", () => {
  it("sorts numbers not strings", () => {
    expect(sorted(["W-100", "W-99", "W-10", "W-9"])).toEqual(["W-9", "W-10", "W-99", "W-100"]);
    expect(sorted(["G-10", "G-9"])).toEqual(["G-9", "G-10"]);
  });

  it("groups families by prefix", () => {
    expect(sorted(["Y-5", "W-100", "Q-100", "G-3", "D-99", "B-2", "A-10", "T-4"])).toEqual([
      "A-10",
      "B-2",
      "D-99",
      "G-3",
      "Q-100",
      "T-4",
      "W-100",
      "Y-5",
    ]);
  });

  it("falls back to the raw string for zero padding", () => {
    expect(sorted(["W-00", "W-0", "W-1"])).toEqual(["W-0", "W-00", "W-1"]);
  });

  it("sorts malformed ids after numeric ids of the same family", () => {
    expect(sorted(["W-bogus", "W-100", "W-!"])).toEqual(["W-100", "W-!", "W-bogus"]);
    expect(compareIds("W-bogus", "W-bogus")).toBe(0);
  });

  it("stays total for empty and lowercase families", () => {
    expect(sorted(["a-1", "Z-1", ""])).toEqual(["", "Z-1", "a-1"]);
    expect(sorted(["AB-2", "AB-1"])).toEqual(["AB-1", "AB-2"]);
  });
});
