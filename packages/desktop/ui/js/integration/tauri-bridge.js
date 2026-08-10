class TauriBridge {
  constructor() {
    this.available = typeof window.__TAURI_INTERNALS__ !== "undefined";
    this._invoke = this.available
      ? window.__TAURI_INTERNALS__.invoke.bind(window.__TAURI_INTERNALS__)
      : null;
  }

  minimize() {
    return this._invoke?.("plugin:window|minimize");
  }

  toggleMaximize() {
    return this._invoke?.("plugin:window|toggle_maximize");
  }

  show() {
    return this._invoke?.("plugin:window|show");
  }

  close() {
    return this._invoke?.("plugin:window|close");
  }

  startDragging() {
    return this._invoke?.("plugin:window|start_dragging");
  }

  async isMaximized() {
    if (!this._invoke) return false;
    return this._invoke("plugin:window|is_maximized");
  }

  invoke(cmd, args) {
    return this._invoke?.(cmd, args) ?? Promise.reject(new Error("tauri bridge unavailable"));
  }

  async writeClipboard(text) {
    if (!this._invoke) return;
    return this._invoke("plugin:clipboard-manager|write_text", { text });
  }

  async clearClipboard() {
    if (!this._invoke) return;
    return this._invoke("plugin:clipboard-manager|clear");
  }

  async openExternal(url) {
    if (!this._invoke) return;
    return this._invoke("plugin:shell|open", { path: url });
  }
}

export const tauriBridge = new TauriBridge();
