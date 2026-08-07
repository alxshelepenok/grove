const VARIANTS = ["accent", "success", "warning", "info", "danger", "neutral"];

const formatValue = (value) => Number(value || 0).toLocaleString("en-US");

export class SegmentedBar {
  constructor(el, { max = null, freeLabel = "Available" } = {}) {
    this.root = typeof el === "string" ? document.getElementById(el) : el;
    if (!this.root) return;
    this.legendEl = this.root.querySelector(".segmented-bar-legend");
    this.trackEl = this.root.querySelector(".segmented-bar-track");
    this.freeLabel = freeLabel;

    const attr = Number(this.root.dataset.segmentedBarMax);
    this.max = max ?? (Number.isFinite(attr) && attr > 0 ? attr : null);
  }

  setSegments(segments) {
    if (!this.root || !this.legendEl || !this.trackEl) return;

    const items = (segments ?? []).map((segment) => ({
      id: segment.id ?? segment.label ?? "",
      label: segment.label ?? "",
      value: Math.max(0, Number(segment.value) || 0),
      variant: VARIANTS.includes(segment.variant) ? segment.variant : "neutral",
    }));

    const total = items.reduce((sum, item) => sum + item.value, 0);
    const basis = this.max && this.max > 0 ? Math.max(this.max, total) : total;

    this.legendEl.replaceChildren();
    this.trackEl.replaceChildren();

    for (const item of items) {
      this.legendEl.appendChild(
        this._buildLegendItem(item.label, item.value, `segmented-bar-c-${item.variant}`),
      );

      if (basis > 0 && item.value > 0) {
        const segment = document.createElement("div");
        segment.className = `segmented-bar-segment segmented-bar-c-${item.variant}`;
        segment.style.width = `${(item.value / basis) * 100}%`;
        segment.title = `${item.label}: ${formatValue(item.value)}`;
        this.trackEl.appendChild(segment);
      }
    }

    if (this.max && this.max > total) {
      this.legendEl.appendChild(
        this._buildLegendItem(this.freeLabel, this.max - total, "segmented-bar-c-track"),
      );
    }
  }

  _buildLegendItem(label, value, colorClass) {
    const item = document.createElement("span");
    item.className = "segmented-bar-legend-item";

    const swatch = document.createElement("span");
    swatch.className = `segmented-bar-legend-swatch ${colorClass}`;

    const labelEl = document.createElement("span");
    labelEl.className = "segmented-bar-legend-label";
    labelEl.textContent = label;

    const valueEl = document.createElement("span");
    valueEl.className = "segmented-bar-legend-value";
    valueEl.textContent = formatValue(value);

    item.append(swatch, labelEl, valueEl);
    return item;
  }
}
