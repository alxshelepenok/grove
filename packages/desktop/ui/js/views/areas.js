import { wireGotoButtons } from "../utils/nav-wiring.js";

export const wireAreas = (root, { navigate } = {}) => {
  const section = root.querySelector(".view-areas");
  if (!section) return null;
  wireGotoButtons(section, navigate);
  return null;
};
