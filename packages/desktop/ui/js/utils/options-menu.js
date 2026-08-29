export const wireOptionsMenu = (root) => {
  const button = root.querySelector("#graph-options-btn");
  const menu = root.querySelector("#graph-options-menu");
  if (!button || !menu) return null;

  const setOpen = (open) => {
    menu.hidden = !open;
    button.setAttribute("aria-expanded", String(open));
  };

  const onButtonClick = (e) => {
    e.stopPropagation();
    setOpen(menu.hidden);
  };
  const onDocClick = (e) => {
    if (!menu.contains(e.target) && !button.contains(e.target)) setOpen(false);
  };
  const onKey = (e) => {
    if (e.key === "Escape") setOpen(false);
  };

  button.addEventListener("click", onButtonClick);
  document.addEventListener("click", onDocClick);
  document.addEventListener("keydown", onKey);
  return () => {
    button.removeEventListener("click", onButtonClick);
    document.removeEventListener("click", onDocClick);
    document.removeEventListener("keydown", onKey);
    setOpen(false);
  };
};
