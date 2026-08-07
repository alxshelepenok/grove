export const addDotIfNeeded = (text) => {
  if (!text) return "";
  return text.endsWith(".") ? text : `${text}.`;
};

const buttonLabelTarget = (btn) => btn.querySelector(".gradient-inner") ?? btn;

const buttonLabelTextNodes = (btn) =>
  [...buttonLabelTarget(btn).childNodes].filter(
    (node) => node.nodeType === Node.TEXT_NODE,
  );

export const getButtonLabel = (btn) => {
  if (!btn) return "";
  return buttonLabelTextNodes(btn)
    .map((node) => node.textContent)
    .join("")
    .trim();
};

export const setButtonLabel = (btn, text) => {
  if (!btn) return;
  const target = buttonLabelTarget(btn);
  for (const node of buttonLabelTextNodes(btn)) {
    target.removeChild(node);
  }
  target.appendChild(document.createTextNode(text));
};
