import { describe, expect, it } from "bun:test";
import { parseCssColor } from "./css-color.js";

const approx = (actual, expected) =>
  expect(actual).toEqual(expected.map((v) => expect.closeTo(v, 6)));

describe("parseCssColor", () => {
  it("parses legacy comma syntax with alpha", () => {
    approx(parseCssColor("rgba(24, 26, 31, 0.5)"), [24 / 255, 26 / 255, 31 / 255, 0.5]);
  });

  it("parses modern space syntax", () => {
    approx(parseCssColor("rgb(24 26 31)"), [24 / 255, 26 / 255, 31 / 255, 1]);
  });

  it("parses slash alpha syntax", () => {
    approx(parseCssColor("rgb(24 26 31 / 0.25)"), [24 / 255, 26 / 255, 31 / 255, 0.25]);
  });

  it("returns the fallback for transparent, unparsable or truncated input", () => {
    const fallback = [0.1, 0.2, 0.3, 1];
    expect(parseCssColor("transparent", fallback)).toBe(fallback);
    expect(parseCssColor("var(--bg)", fallback)).toBe(fallback);
    expect(parseCssColor("rgb(24, 26)", fallback)).toBe(fallback);
    expect(parseCssColor("", fallback)).toBe(fallback);
    expect(parseCssColor(null, fallback)).toBe(fallback);
  });

  it("keeps zero alpha explicit so callers can substitute a safe clear color", () => {
    approx(parseCssColor("rgba(0, 0, 0, 0)"), [0, 0, 0, 0]);
  });
});
