import { wireGotoButtons, wirePacketRows } from "../utils/nav-wiring.js";
import { wireFilterTabsFades } from "../utils/filter-tabs.js";

export const wireWork = (root, { navigate } = {}) => {
  const section = root.querySelector(".view-work");
  if (!section) return null;
  const cleanupFades = wireFilterTabsFades(section);
  const archivedBox = section.querySelector("#work-archived");
  for (const chip of section.querySelectorAll('[data-action="filter"]')) {
    chip.addEventListener("click", () =>
      navigate?.("work", {
        status: chip.dataset.status,
        archived: archivedBox?.checked ?? false,
      }),
    );
  }
  archivedBox?.addEventListener("change", () =>
    navigate?.("work", {
      status: section.dataset.filter || "all",
      archived: archivedBox.checked,
    }),
  );
  wireGotoButtons(section, navigate);
  wirePacketRows(section, navigate);
  return cleanupFades;
};
