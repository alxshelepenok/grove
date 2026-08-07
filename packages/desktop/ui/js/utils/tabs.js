
const DOT_VARIANTS = ["success", "danger", "accent"];

const clearTabIndicator = (tab) => {
  const dot = tab.querySelector(".tab-dot");
  if (!dot || dot.dataset.persistent === "true") return;
  dot.hidden = true;
  delete dot.dataset.persistent;
};

export function setTabIndicator(panelId, variant, { persistent = false } = {}) {
  const tab = document.getElementById(`${panelId}-tab`);
  const dot = tab?.querySelector(".tab-dot");
  if (!dot) return;

  dot.classList.remove(...DOT_VARIANTS.map((v) => `tab-dot-${v}`));
  if (variant && DOT_VARIANTS.includes(variant)) {
    dot.classList.add(`tab-dot-${variant}`);
    dot.hidden = false;
    if (persistent) dot.dataset.persistent = "true";
    else delete dot.dataset.persistent;
  } else {
    dot.hidden = true;
    delete dot.dataset.persistent;
  }
}

export function initTabs() {
  document.addEventListener("click", (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;

    const tab = target.closest("[data-tab-target]");
    if (!tab) return;

    const list = tab.closest(".tabs");
    if (!list) return;

    for (const item of list.querySelectorAll("[data-tab-target]")) {
      const selected = item === tab;
      item.setAttribute("aria-selected", String(selected));

      const panel = document.getElementById(
        item.getAttribute("data-tab-target"),
      );
      if (panel) panel.hidden = !selected;
    }

    clearTabIndicator(tab);
  });
}
