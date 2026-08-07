import { debounce } from "./debounce.js";

class SearchableSelect {
  constructor({
    container,
    fetchOptions,
    renderOption,
    onSelect,
    placeholder = "Select...",
    emptyText = "No results",
    debounceMs = 200,
    initialItems = null,
  }) {
    this.container = container;
    this.fetchOptions = fetchOptions;
    this.renderOption = renderOption;
    this.onSelect = onSelect;
    this.placeholder = placeholder;
    this.emptyText = emptyText;
    this._initialItems = initialItems;

    this._items = [];
    this._activeIndex = -1;
    this._isOpen = false;
    this._selectedId = null;
    this._selectedLabel = null;

    this._hydrateFromDOM();
    this._setupDOM();

    this._boundOnSearch = debounce((q) => this._doSearch(q), debounceMs);
    this._boundOnKeydown = (e) => this._onKeydown(e);
    this._boundOnDocumentClick = (e) => this._onDocumentClick(e);
    this._boundOnListClick = (e) => this._onListClick(e);
    this._boundOnTriggerClick = (e) => this._onTriggerClick(e);

    this._attachListeners();
  }

  _hydrateFromDOM() {
    if (this._initialItems) return;
    const script = this.container.querySelector(`script[type="application/json"]`);
    if (script) {
      try {
        this._initialItems = JSON.parse(script.textContent);
      } catch {
        this._initialItems = null;
      }
    }
  }

  _setupDOM() {
    this.trigger = this.container.querySelector(":scope > .searchable-select-trigger");
    this.dropdown = this.container.querySelector(":scope > .searchable-select-dropdown");
    this.searchInput = this.dropdown.querySelector(":scope > .searchable-select-search");
    this.list = this.dropdown.querySelector(":scope > .searchable-select-list");
  }

  _attachListeners() {
    this.trigger.addEventListener("click", this._boundOnTriggerClick);
    this.searchInput.addEventListener("input", (e) => this._boundOnSearch(e.target.value));
    this.searchInput.addEventListener("keydown", this._boundOnKeydown);
    this.list.addEventListener("mousedown", this._boundOnListClick);
    document.addEventListener("click", this._boundOnDocumentClick);
  }

  destroy() {
    this.trigger.removeEventListener("click", this._boundOnTriggerClick);
    this.searchInput.removeEventListener("input", this._boundOnSearch);
    this.searchInput.removeEventListener("keydown", this._boundOnKeydown);
    this.list.removeEventListener("mousedown", this._boundOnListClick);
    document.removeEventListener("click", this._boundOnDocumentClick);
    this._close();
  }

  _onTriggerClick(e) {
    e.stopPropagation();
    if (this._isOpen) {
      this._close();
    } else {
      this._open();
      this._doSearch("");
      setTimeout(() => this.searchInput.focus(), 0);
    }
  }

  async _doSearch(query) {
    const q = (query || "").trim().toLowerCase();

    if (this._initialItems) {
      let items = this._initialItems;
      if (q) {
        items = this._initialItems.filter((item) => {
          const text = `${item.name || ""} ${item.label || ""} ${item.publicKey || ""} ${item.id || ""}`.toLowerCase();
          return text.includes(q);
        });
      }
      if (items.length > 0 || !this.fetchOptions) {
        this._items = items;
        this._activeIndex = -1;
        this._render(items);
        return;
      }
    }

    try {
      const items = await this.fetchOptions(query);
      this._items = items;
      this._activeIndex = -1;
      this._render(items);
    } catch (err) {
      console.error("[SearchableSelect] fetch failed:", err);
    }
  }

  _render(items) {
    if (items.length === 0) {
      this.list.innerHTML = `<div class="searchable-select-empty">${this.emptyText}</div>`;
      return;
    }
    this.list.innerHTML = items.map((item, i) =>
      `<div class="searchable-select-option" data-index="${i}" data-id="${item.id}">${this.renderOption(item)}</div>`
    ).join("");
  }

  _onKeydown(e) {
    if (!this._isOpen) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        this._activeIndex = Math.min(this._activeIndex + 1, this._items.length - 1);
        this._highlight();
        break;
      case "ArrowUp":
        e.preventDefault();
        this._activeIndex = Math.max(this._activeIndex - 1, 0);
        this._highlight();
        break;
      case "Enter":
        e.preventDefault();
        if (this._activeIndex >= 0) {
          this._select(this._items[this._activeIndex]);
        }
        break;
      case "Escape":
        e.preventDefault();
        this._close();
        break;
    }
  }

  _highlight() {
    const options = this.list.querySelectorAll(".searchable-select-option");
    options.forEach((el, i) => {
      el.classList.toggle("active", i === this._activeIndex);
    });
    const active = options[this._activeIndex];
    if (active) active.scrollIntoView({ block: "nearest" });
  }

  _onListClick(e) {
    const option = e.target.closest(".searchable-select-option");
    if (!option) return;
    const index = parseInt(option.dataset.index, 10);
    if (!isNaN(index) && this._items[index]) {
      this._select(this._items[index]);
    }
  }

  _select(item) {
    this._selectedId = item.id;
    this._selectedLabel = this.renderOption(item);
    this.trigger.textContent = this._selectedLabel;
    this.trigger.classList.add("has-value");
    this._close();
    this.onSelect(item);
  }

  _open() {
    this._isOpen = true;
    this.dropdown.removeAttribute("hidden");
  }

  _close() {
    this._isOpen = false;
    this.dropdown.setAttribute("hidden", "");
    this._activeIndex = -1;
  }

  _onDocumentClick(e) {
    if (!this._isOpen) return;
    if (this.container.contains(e.target)) return;
    this._close();
  }

  setValue(value, label) {
    this._selectedId = value;
    this._selectedLabel = label || value;
    if (value) {
      this.trigger.textContent = this._selectedLabel;
      this.trigger.classList.add("has-value");
    } else {
      this.trigger.textContent = this.placeholder;
      this.trigger.classList.remove("has-value");
    }
  }

  clear() {
    this._selectedId = null;
    this._selectedLabel = null;
    this.trigger.textContent = this.placeholder;
    this.trigger.classList.remove("has-value");
    this._close();
  }
}

export { SearchableSelect };
