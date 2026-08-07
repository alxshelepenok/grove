import { wirePacketRows } from "../utils/nav-wiring.js";

export const wireThemes = (root, { navigate } = {}) => {
  const section = root.querySelector(".view-themes");
  if (!section) return null;
  wirePacketRows(section, navigate);
  return null;
};
