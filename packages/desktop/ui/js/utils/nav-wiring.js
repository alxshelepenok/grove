export const wireGotoButtons = (section, navigate) => {
  for (const btn of section.querySelectorAll('[data-action="goto"]')) {
    btn.addEventListener("click", () =>
      navigate?.(btn.dataset.level, btn.dataset.id ? { id: btn.dataset.id } : {}),
    );
  }
};

export const wirePacketRows = (section, navigate) => {
  for (const row of section.querySelectorAll("tr[data-id]")) {
    row.addEventListener("click", (e) => {
      if (e.target.closest('[data-action="goto"]')) return;
      navigate?.("packet", { id: row.dataset.id });
    });
  }
};
