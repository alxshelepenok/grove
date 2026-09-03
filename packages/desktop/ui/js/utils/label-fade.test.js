import { describe, expect, it } from "bun:test";
import {
  LABEL_FADE_INNER,
  LABEL_FADE_OUTER,
  labelFadeOpacity,
} from "./label-fade.js";

describe("labelFadeOpacity", () => {
  it("keeps text fully opaque at and below the inner edge", () => {
    expect(labelFadeOpacity(0.25)).toBe(1);
    expect(labelFadeOpacity(1)).toBe(1);
    expect(labelFadeOpacity(LABEL_FADE_INNER)).toBe(1);
  });

  it("hides text completely at and beyond the outer edge", () => {
    expect(labelFadeOpacity(LABEL_FADE_OUTER)).toBe(0);
    expect(labelFadeOpacity(LABEL_FADE_OUTER + 1)).toBe(0);
  });

  it("passes through a semi-transparent ghost inside the window", () => {
    const mid = (LABEL_FADE_INNER + LABEL_FADE_OUTER) / 2;
    expect(labelFadeOpacity(mid)).toBeCloseTo(0.5, 6);
    const ghost = LABEL_FADE_INNER + (LABEL_FADE_OUTER - LABEL_FADE_INNER) * 0.8;
    const opacity = labelFadeOpacity(ghost);
    expect(opacity).toBeGreaterThan(0);
    expect(opacity).toBeLessThan(0.2);
  });

  it("fades monotonically as the factor grows", () => {
    let prev = 1;
    for (let f = 1; f <= 3; f += 0.1) {
      const opacity = labelFadeOpacity(f);
      expect(opacity).toBeLessThanOrEqual(prev + 1e-12);
      prev = opacity;
    }
  });

  it("degenerates to a step when the window collapses", () => {
    expect(labelFadeOpacity(1.4, 1.5, 1.5)).toBe(1);
    expect(labelFadeOpacity(1.6, 1.5, 1.5)).toBe(0);
    expect(labelFadeOpacity(1.6, 1.5, 1.2)).toBe(0);
  });
});
