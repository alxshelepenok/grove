const FLOAT_SELECTOR =
  ".modal-overlay .tooltip:not(.tooltip-right):not(.tooltip-left):not(.tooltip-bottom)";
const VIEWPORT_PAD = 8;

let floated = null;

const clearFloat = () => {
  if (!floated) return;
  const { bubble } = floated;
  bubble.style.position = "";
  bubble.style.left = "";
  bubble.style.top = "";
  bubble.style.right = "";
  bubble.style.bottom = "";
  bubble.style.transform = "";
  bubble.classList.remove("tooltip-bubble-flipped");
  floated = null;
};

const placeFloat = (tip) => {
  const bubble = tip.querySelector(":scope > .tooltip-bubble");
  if (!bubble) return;
  if (floated && floated.tip !== tip) clearFloat();
  floated = { tip, bubble };
  bubble.style.position = "fixed";
  bubble.style.right = "auto";
  bubble.style.bottom = "auto";
  bubble.style.transform = "none";
  const rect = tip.getBoundingClientRect();
  const w = bubble.offsetWidth;
  const h = bubble.offsetHeight;
  let left = Math.round(rect.left + rect.width / 2 - w / 2);
  left = Math.max(
    VIEWPORT_PAD,
    Math.min(left, window.innerWidth - w - VIEWPORT_PAD),
  );
  let top = Math.round(rect.top - h - 8);
  const flipped = top < VIEWPORT_PAD;
  if (flipped) top = Math.round(rect.bottom + 8);
  bubble.classList.toggle("tooltip-bubble-flipped", flipped);
  bubble.style.left = `${left}px`;
  bubble.style.top = `${top}px`;
};

export const initFloatingTooltips = () => {
  document.addEventListener("mouseover", (e) => {
    const tip = e.target.closest?.(FLOAT_SELECTOR);
    if (tip) placeFloat(tip);
  });
  document.addEventListener("focusin", (e) => {
    const tip = e.target.closest?.(FLOAT_SELECTOR);
    if (tip) placeFloat(tip);
  });
  document.addEventListener("mouseout", (e) => {
    if (!floated) return;
    if (
      e.target.closest?.(FLOAT_SELECTOR) === floated.tip &&
      !floated.tip.contains(e.relatedTarget)
    ) {
      clearFloat();
    }
  });
  document.addEventListener("focusout", (e) => {
    if (!floated) return;
    if (
      e.target.closest?.(FLOAT_SELECTOR) === floated.tip &&
      !floated.tip.contains(e.relatedTarget)
    ) {
      clearFloat();
    }
  });
  document.addEventListener(
    "scroll",
    () => {
      if (floated) placeFloat(floated.tip);
    },
    true,
  );
  window.addEventListener("resize", () => {
    if (floated) placeFloat(floated.tip);
  });
};
