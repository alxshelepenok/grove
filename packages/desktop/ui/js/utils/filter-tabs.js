const FADE_EPSILON = 1;

export const wireFilterTabsFades = (root) => {
  const cleanups = [];
  for (const menu of root.querySelectorAll(".filter-tabs")) {
    const update = () => {
      menu.dataset.fadeLeft = menu.scrollLeft > FADE_EPSILON ? "true" : "false";
      menu.dataset.fadeRight =
        menu.scrollLeft + menu.clientWidth < menu.scrollWidth - FADE_EPSILON ? "true" : "false";
    };
    menu.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(menu);
    update();
    cleanups.push(() => {
      menu.removeEventListener("scroll", update);
      observer.disconnect();
    });
  }
  return () => {
    for (const fn of cleanups) fn();
  };
};
