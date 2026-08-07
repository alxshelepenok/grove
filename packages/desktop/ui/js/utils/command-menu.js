const setChildren = (el, nodes) => {
  el.replaceChildren(...nodes);
};

const el = (tag, className, text) => {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
};

const searchInput = ({ placeholder = "", icon = "search" } = {}) => {
  const wrap = el("span", "search-input");
  const iconEl = el(
    "span",
    `search-input-icon command-menu-icon command-menu-icon-${icon}`,
  );
  const input = el("input", "command-menu-input");
  input.type = "text";
  input.placeholder = placeholder;
  input.setAttribute("aria-label", placeholder);
  input.spellcheck = false;
  input.autocomplete = "off";
  wrap.append(iconEl, input);
  const setIcon = (slug) => {
    iconEl.className = `search-input-icon command-menu-icon command-menu-icon-${slug}`;
  };
  return { wrap, input, setIcon };
};

const openCommandMenu = ({
  sections = [],
  placeholder = "Search...",
  emptyText = "No results",
  preselect,
  onClose,
} = {}) => {
  let currentSections = sections;
  let activeIndex = -1;
  let visible = [];
  let closed = false;
  let form = null;
  let extraInputs = [];

  const overlay = el("div", "modal-overlay active command-menu-overlay");
  overlay.setAttribute("role", "dialog");
  overlay.setAttribute("aria-modal", "true");

  const panel = el("div", "command-menu-panel");

  const inputRow = el("div", "command-menu-input-row");
  const search = searchInput({ placeholder });
  const input = search.input;
  const browseBtn = el("button", "command-menu-browse");
  browseBtn.type = "button";
  browseBtn.title = "Browse for a folder";
  browseBtn.setAttribute("aria-label", "Browse for a folder");
  browseBtn.append(
    el("span", "command-menu-icon command-menu-icon-folder-open"),
  );
  browseBtn.hidden = true;
  inputRow.append(search.wrap, browseBtn);

  const extraRow = el("div", "command-menu-extra");
  const hintEl = el("p", "command-menu-hint");
  hintEl.hidden = true;
  const errorEl = el("p", "command-menu-error");
  errorEl.hidden = true;
  const list = el("div", "command-menu-list");
  list.setAttribute("role", "listbox");

  panel.append(inputRow, extraRow, hintEl, errorEl, list);
  overlay.append(panel);
  document.body.append(overlay);

  const setActive = (index, scroll = true) => {
    activeIndex = index;
    visible.forEach(({ el: itemEl }, i) => {
      const on = i === activeIndex;
      itemEl.classList.toggle("active", on);
      itemEl.setAttribute("aria-selected", on ? "true" : "false");
    });
    if (scroll && visible[activeIndex]) {
      visible[activeIndex].el.scrollIntoView({ block: "nearest" });
    }
  };

  const renderItem = (item) => {
    const itemEl = el("div", "command-menu-item");
    itemEl.dataset.id = item.id ?? "";
    itemEl.setAttribute("role", "option");
    if (item.disabled) {
      itemEl.classList.add("disabled");
      itemEl.setAttribute("aria-disabled", "true");
    }
    if (item.avatar) {
      const tile = el("span", "command-menu-item-avatar");
      tile.dataset.avatar = "";
      if (typeof item.avatar === "string") {
        tile.innerHTML = item.avatar;
      } else {
        tile.append(el("span", `command-menu-icon command-menu-icon-${item.icon}`));
      }
      itemEl.append(tile);
    } else if (item.icon) {
      itemEl.append(el("span", `command-menu-icon command-menu-icon-${item.icon}`));
    }
    const text = el("span", "command-menu-item-text");
    text.append(el("span", "command-menu-item-label", item.label ?? ""));
    if (item.secondary) {
      text.append(el("span", "command-menu-item-secondary", item.secondary));
    }
    itemEl.append(text);
    if (item.badge) {
      itemEl.append(el("span", "badge badge-accent command-menu-item-badge", item.badge));
    }
    itemEl.append(el("span", "command-menu-item-hint", item.hint ?? ""));
    if (typeof item.remove === "function") {
      const removeBtn = el("button", "command-menu-item-remove");
      removeBtn.type = "button";
      removeBtn.title = "Remove from recent";
      removeBtn.setAttribute(
        "aria-label",
        `Remove ${item.label ?? ""} from recent`,
      );
      removeBtn.append(
        el("span", "command-menu-icon command-menu-icon-cancel"),
      );
      itemEl.append(removeBtn);
    }
    return itemEl;
  };

  const render = () => {
    const q = form ? "" : input.value.trim().toLowerCase();
    const nodes = [];
    visible = [];
    for (const section of currentSections) {
      const items = (section.items ?? []).filter(
        (item) => !q || (item.label ?? "").toLowerCase().includes(q),
      );
      if (!items.length) continue;
      const sectionEl = el("div", "command-menu-section");
      const title = el("div", "command-menu-section-title");
      if (section.icon) {
        title.append(el("span", `command-menu-icon command-menu-icon-${section.icon}`));
      }
      title.append(el("span", "", section.title ?? ""));
      sectionEl.append(title);
      for (const item of items) {
        const itemEl = renderItem(item);
        sectionEl.append(itemEl);
        if (!item.disabled) visible.push({ item, el: itemEl });
      }
      nodes.push(sectionEl);
    }
    if (!nodes.length) {
      nodes.push(el("div", "command-menu-empty", emptyText));
    }
    setChildren(list, nodes);
    if (!visible.length) {
      activeIndex = -1;
      return;
    }
    let next = 0;
    if (preselect) {
      const found = visible.findIndex(({ item }) => item.id === preselect);
      if (found >= 0) next = found;
      preselect = null;
    }
    setActive(next, false);
  };

  const close = () => {
    if (closed) return;
    closed = true;
    document.removeEventListener("keydown", onKeydown, true);
    overlay.remove();
    onClose?.();
  };

  const showError = (message) => {
    errorEl.textContent = String(message ?? "");
    errorEl.hidden = !errorEl.textContent;
  };

  const clearError = () => showError("");

  const activate = async ({ item }) => {
    if (!item.action) return;
    const keep = await item.action(item, handle);
    if (!keep) close();
  };

  const moveActive = (delta) => {
    if (!visible.length) return;
    const next =
      activeIndex < 0
        ? delta > 0
          ? 0
          : visible.length - 1
        : Math.min(Math.max(activeIndex + delta, 0), visible.length - 1);
    setActive(next);
  };

  const submitForm = async () => {
    if (!form) return;
    clearError();
    const values = {};
    form.fields.forEach((field, i) => {
      values[field.key] = (i === 0 ? input : extraInputs[i - 1])?.value ?? "";
    });
    let err = null;
    try {
      err = await form.submit(values);
    } catch (e) {
      err = String(e?.message ?? e);
    }
    if (err) {
      showError(err);
      input.focus();
      return;
    }
    close();
  };

  const showForm = ({ icon, fields = [], hint, submit, browse } = {}) => {
    form = { fields, submit, browse: typeof browse === "function" ? browse : null };
    clearError();
    if (icon) search.setIcon(icon);
    const [first, ...rest] = fields;
    input.value = first?.value ?? "";
    if (first?.placeholder) {
      input.placeholder = first.placeholder;
      input.setAttribute("aria-label", first.placeholder);
    }
    extraInputs = rest.map((field) => {
      const extra = el("input", "command-menu-field");
      extra.type = "text";
      extra.placeholder = field.placeholder ?? field.key;
      extra.setAttribute("aria-label", field.placeholder ?? field.key);
      extra.spellcheck = false;
      extra.autocomplete = "off";
      return extra;
    });
    setChildren(extraRow, extraInputs);
    hintEl.textContent = hint ?? "";
    hintEl.hidden = !hint;
    browseBtn.hidden = !form.browse;
    list.hidden = true;
    setChildren(list, []);
    visible = [];
    activeIndex = -1;
    input.focus();
    return handle;
  };

  const setSections = (next) => {
    currentSections = next ?? [];
    if (!form) render();
  };

  const onKeydown = (e) => {
    if (closed) return;
    switch (e.key) {
      case "ArrowDown":
        if (form) return;
        e.preventDefault();
        moveActive(1);
        break;
      case "ArrowUp":
        if (form) return;
        e.preventDefault();
        moveActive(-1);
        break;
      case "Enter":
        e.preventDefault();
        if (form) {
          submitForm();
        } else if (visible[activeIndex]) {
          activate(visible[activeIndex]);
        }
        break;
      case "Escape":
        e.preventDefault();
        close();
        break;
    }
  };

  input.addEventListener("input", () => {
    clearError();
    if (!form) render();
  });

  browseBtn.addEventListener("click", async () => {
    if (!form?.browse) return;
    let picked = null;
    try {
      picked = await form.browse();
    } catch {
      picked = null;
    }
    if (picked) {
      input.value = String(picked);
      clearError();
    }
    input.focus();
  });

  list.addEventListener("mousedown", (e) => {
    const itemEl = e.target.closest(".command-menu-item");
    if (!itemEl) return;
    e.preventDefault();
    const entry = visible.find(({ el: candidate }) => candidate === itemEl);
    if (!entry) return;
    if (e.target.closest(".command-menu-item-remove")) {
      entry.item.remove?.(entry.item, handle);
      return;
    }
    activate(entry);
  });

  list.addEventListener("mouseover", (e) => {
    const itemEl = e.target.closest(".command-menu-item");
    if (!itemEl) return;
    const index = visible.findIndex(({ el: candidate }) => candidate === itemEl);
    if (index >= 0 && index !== activeIndex) setActive(index, false);
  });

  overlay.addEventListener("mousedown", (e) => {
    if (e.target === overlay) close();
  });

  document.addEventListener("keydown", onKeydown, true);

  const handle = {
    close,
    setSections,
    showForm,
    showError,
    clearError,
    isOpen: () => !closed,
  };

  render();
  setTimeout(() => input.focus(), 0);
  return handle;
};

export { openCommandMenu };
