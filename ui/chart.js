// Small helpers shared by the SVG charts (CSD-006 and CSD-007). They turn
// numbers into positions and labels and handle the tooltip and the legend
// highlight; the figures themselves come from Rust.

const SVG_NS = 'http://www.w3.org/2000/svg';
const TICK_INTERVALS = 4;

export function svg(tag, attrs = {}, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  node.append(...children);
  return node;
}

/** Axis top and tick values: a round step, with the top at or above `max`. */
export function niceScale(max) {
  const rawStep = max / TICK_INTERVALS;
  const magnitude = 10 ** Math.floor(Math.log10(rawStep));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * magnitude).find((s) => s >= rawStep);
  const intervals = Math.ceil(max / step);
  return { top: intervals * step, ticks: Array.from({ length: intervals + 1 }, (_, i) => i * step) };
}

/** 2500 -> "2.5k", 10000 -> "10k", 1500000 -> "1.5M". */
export function compact(value) {
  const trim = (n) => String(Number(n.toFixed(2)));
  if (value >= 1_000_000) return `${trim(value / 1_000_000)}M`;
  if (value >= 1_000) return `${trim(value / 1_000)}k`;
  return trim(value);
}

const clamp = (value, low, high) => Math.min(Math.max(value, low), Math.max(high, low));

/**
 * Places a year's tooltip beside its bar: to the right with its top level with
 * the bar, to the left when there is no room, and inside the chart. Without a
 * bar (a gap year) it is centred over the slot (CSD-006 REQ-12, CSD-007 REQ-14).
 */
export function placeTooltip(inner, tooltip, bar, slot) {
  const box = inner.getBoundingClientRect();
  tooltip.hidden = false;
  const width = tooltip.offsetWidth;
  const height = tooltip.offsetHeight;
  if (!bar) {
    const slotBox = slot.getBoundingClientRect();
    const centre = slotBox.left - box.left + slotBox.width / 2;
    tooltip.style.left = `${clamp(centre - width / 2, 0, box.width - width)}px`;
    tooltip.style.top = '0px';
    return;
  }
  const edge = bar.getBoundingClientRect();
  const gap = 8;
  let left = edge.right - box.left + gap;
  if (left + width > box.width) left = edge.left - box.left - gap - width;
  tooltip.style.left = `${clamp(left, 0, box.width - width)}px`;
  tooltip.style.top = `${clamp(edge.top - box.top, 0, box.height - height)}px`;
}

/**
 * Highlights the legend item `key`, or nothing for `null`: in the panel legend,
 * the tooltip and the bar. Items are found by `data-legend` (CSD-006 REQ-13,
 * CSD-007 REQ-15).
 */
export function setHighlight(key, scopes) {
  for (const scope of scopes) {
    for (const item of scope.querySelectorAll('[data-legend]')) {
      item.classList.toggle('hl', key !== null && item.dataset.legend === key);
      item.classList.toggle('dim', key !== null && item.dataset.legend !== key);
    }
  }
}

/** Highlights the legend item of the segment under the mouse; nothing elsewhere or on leaving. */
export function bindHighlight(group, scopes) {
  group.addEventListener('mouseover', (event) => {
    const target = event.target.closest?.('[data-legend]');
    setHighlight(target && group.contains(target) ? target.dataset.legend : null, scopes);
  });
  group.addEventListener('mouseleave', () => setHighlight(null, scopes));
  group.addEventListener('focus', () => setHighlight(null, scopes));
}
