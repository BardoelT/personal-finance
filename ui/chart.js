// Small helpers shared by the SVG charts (CSD-006 and CSD-007). They only turn
// numbers into positions and labels; the figures themselves come from Rust.

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
