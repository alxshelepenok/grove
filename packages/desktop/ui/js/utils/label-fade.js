export const LABEL_FADE_INNER = 1.1;
export const LABEL_FADE_OUTER = 2.4;

export const labelFadeOpacity = (factor, inner = LABEL_FADE_INNER, outer = LABEL_FADE_OUTER) => {
  if (outer <= inner) return factor <= inner ? 1 : 0;
  const t = Math.min(1, Math.max(0, (factor - inner) / (outer - inner)));
  return 1 - t * t * (3 - 2 * t);
};
