const CSS_COLOR_RE = /rgba?\(([^)]+)\)/;

export const parseCssColor = (value, fallback = null) => {
  const m = CSS_COLOR_RE.exec(value || "");
  if (!m) return fallback;
  const parts = m[1]
    .split(/[\s,/]+/)
    .filter(Boolean)
    .map((v) => parseFloat(v));
  if (parts.length < 3 || parts.slice(0, 3).some((v) => Number.isNaN(v))) return fallback;
  return [parts[0] / 255, parts[1] / 255, parts[2] / 255, parts.length > 3 ? parts[3] : 1];
};
