import { KIND_INFO, statusVariant } from "../utils/graph-status.js";
import { kindStatus } from "../utils/graph-model.js";

const metaBadge = (text, variant = "neutral") => {
  const chip = document.createElement("span");
  chip.className = `badge badge-${variant} capitalize`;
  chip.textContent = text;
  return chip;
};

export const createInfoPanel = (root) => {
  const info = root.querySelector("#graph-info");
  return {
    show(n) {
      root.querySelector("#graph-info-id").textContent = n.id;
      root.querySelector("#graph-info-title").textContent = n.title || "(untitled)";
      const meta = root.querySelector("#graph-info-meta");
      const chips = [metaBadge(KIND_INFO[n.kind] ?? n.kind)];
      if (n.status) {
        chips.push(metaBadge(n.status, statusVariant(n)));
      }
      if (n.wtype) chips.push(metaBadge(n.wtype));
      if (n.archived) chips.push(metaBadge("archived", "warning"));
      meta.replaceChildren(...chips);
      info.hidden = false;
    },
    hide() {
      info.hidden = true;
    },
  };
};

export const createTooltip = (root, stage) => {
  const tooltip = root.querySelector("#graph-tooltip");
  return {
    show(n, mx, my) {
      tooltip.textContent = `${n.id} (${kindStatus(n)})${n.title ? `: ${n.title}` : ""}`;
      tooltip.hidden = false;
      const maxX = stage.clientWidth - tooltip.offsetWidth - 8;
      const maxY = stage.clientHeight - tooltip.offsetHeight - 8;
      const left = Math.max(8, Math.min(mx + 14, maxX));
      const top = Math.max(8, Math.min(my + 16, maxY));
      tooltip.style.left = `${Math.round(left)}px`;
      tooltip.style.top = `${Math.round(top)}px`;
    },
    hide() {
      tooltip.hidden = true;
    },
  };
};
