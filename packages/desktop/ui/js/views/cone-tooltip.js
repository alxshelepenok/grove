export const createConeTooltip = (section, stage) => {
  const tooltip = section.querySelector("#graph-tooltip");
  return {
    show(n, mx, my) {
      tooltip.textContent =
        n.kind === "f" ? n.id : `${n.id} (${n.status})${n.title ? `: ${n.title}` : ""}`;
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
