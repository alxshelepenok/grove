import { describe, expect, it } from "bun:test";
import { HIT_TOLERANCE_PX, pickSearchRadius, screenHitRadius } from "./graph-math.js";

const approx = (actual, expected) => expect(actual).toBeCloseTo(expected, 6);

describe("screenHitRadius", () => {
  it("scales the world radius with zoom", () => {
    approx(screenHitRadius(9, 4), 9 * 4 + HIT_TOLERANCE_PX);
    approx(screenHitRadius(9, 0.15), 9 * 0.15 + HIT_TOLERANCE_PX);
    approx(screenHitRadius(15, 1), 15 * 1 + HIT_TOLERANCE_PX);
  });
});

describe("pickSearchRadius", () => {
  it("keeps the screen tolerance constant in world units", () => {
    approx(pickSearchRadius(15, 4), 15 + HIT_TOLERANCE_PX / 4);
    approx(pickSearchRadius(15, 0.15), 15 + HIT_TOLERANCE_PX / 0.15);
    approx(pickSearchRadius(15, 1), 15 + HIT_TOLERANCE_PX);
  });
});
