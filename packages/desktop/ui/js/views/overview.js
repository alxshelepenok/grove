import { wireGotoButtons } from "../utils/nav-wiring.js";

export const wireOverview = (root, { navigate } = {}) => {
  const section = root.querySelector(".view-overview");
  if (!section) return null;
  wireGotoButtons(section, navigate);
  return null;
};
