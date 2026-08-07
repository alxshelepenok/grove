class ScrollDispatcher {
  constructor() {
    this._listeners = new Set();
    this._boundOnScroll = this._onScroll.bind(this);
    this._attachedElement = null;
  }

  get container() {
    return document.getElementById("app-scrollable-area") || null;
  }

  attach(callback) {
    const container = this.container || window;
    this._listeners.add(callback);
    if (this._listeners.size === 1) {
      container.addEventListener("scroll", this._boundOnScroll, { passive: true });
      this._attachedElement = container;
    }
    return () => this.detach(callback);
  }

  detach(callback) {
    this._listeners.delete(callback);
    if (this._listeners.size === 0 && this._attachedElement) {
      this._attachedElement.removeEventListener("scroll", this._boundOnScroll, { passive: true });
      this._attachedElement = null;
    }
  }

  _onScroll(event) {
    for (const listener of this._listeners) {
      listener(event);
    }
  }
}

export const scrollDispatcher = new ScrollDispatcher();
