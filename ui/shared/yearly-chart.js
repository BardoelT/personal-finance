// The frame shared by the yearly insights charts (CSD-006 income, CSD-007
// investments): plot size, axis and gridlines, one slot per year, the stacked
// bars, the tooltip, the legend highlight, and loading with its empty note.
//
// A page supplies a strategy, described at `createYearlyChart`. Rust returns the
// figures; this file only stacks and scales them, so the only arithmetic here
// is turning values into positions.
import { invoke } from './ipc.js';
import { svg, niceScale, compact, segmentShape, createTooltip, bindHighlight } from './chart.js';

const MARGIN = { top: 28, right: 16, left: 48 };
const MIN_PLOT_WIDTH = 560;
const MAX_BAR_WIDTH = 44;

/** "6.4" -> "+6.4%", "-2.5" -> "−2.5%", "0.0" -> "0.0%", null -> "–". */
export function percentLabel(percent) {
  if (percent === null) return '–';
  if (percent.startsWith('-')) return `−${percent.slice(1)}%`;
  return percent === '0.0' ? '0.0%' : `+${percent}%`;
}

/** Whether a decimal string is below zero. */
export const isNegative = (text) => text.startsWith('-');

/** "2025", or "2026 YTD" for the running year. */
export function yearLabel(row) {
  return row.is_partial ? `${row.year} YTD` : String(row.year);
}

/** An axis or marker label: compact, with a real minus sign below zero. */
export function axisLabel(value) {
  return value < 0 ? `−${compact(-value)}` : compact(value);
}

/**
 * Builds a yearly chart inside the page whose root element has the id `id`.
 * Its parts are found as `${id}-legend`, `${id}-empty`, `${id}-scroll` and
 * `${id}-chart`. The strategy:
 *
 * - `command`, `ariaLabel`: the Rust command that returns `{ years }`, and the chart's name
 * - `plotHeight`, `minSlotWidth`, `zeroLine`: the size, and whether the zero line gets its own style
 * - `piecesOf(row)`: the stacked pieces `{ key, cssClass, low, high, hangs }` in drawing order;
 *   a piece may also carry `label` (its accessible name) and `rounding` (`'top'`, `'bottom'` or `false`),
 *   which otherwise rounds the top of the highest piece
 * - `extraLevels(row)`: further values the axis must reach (such as a closing balance)
 * - `describe(row)`, `tooltipContent(row)`: the screen-reader text and the tooltip rows
 * - `decorate(row, geometry)`: extra nodes for a bar (growth label, marker)
 * - `labelLines(row)`: the lines under the axis, `[{ text, className }]`
 * - `onLoaded(data)` (optional): runs with the whole response before drawing, for a legend built from it
 * - `highlightAcrossYears` (optional): a highlighted series is highlighted in every year, not only the hovered one
 */
export function createYearlyChart(strategy) {
  const part = (suffix) => document.getElementById(suffix ? `${strategy.id}-${suffix}` : strategy.id);
  let drawnYears = null;

  function draw(years) {
    drawnYears = years;
    const inner = part('chart');
    const lines = strategy.labelLines(years[0]).length;
    const margin = { ...MARGIN, bottom: 30 + 16 * (lines - 1) };
    const count = years.length;
    // Draw at the real pixel width, so bars, lines and text are never scaled up.
    inner.style.minWidth = '';
    const available = inner.clientWidth - margin.left - margin.right;
    const plotWidth = Math.max(count * strategy.minSlotWidth, MIN_PLOT_WIDTH, available);
    const slot = plotWidth / count;
    const barWidth = Math.min(slot * 0.5, MAX_BAR_WIDTH);
    const width = margin.left + plotWidth + margin.right;
    const height = margin.top + strategy.plotHeight + margin.bottom;
    const bottomEdge = margin.top + strategy.plotHeight;

    const pieces = years.map(strategy.piecesOf);
    const extras = years.flatMap((row) => strategy.extraLevels?.(row) ?? []);
    const highest = Math.max(1, ...pieces.flat().map((p) => p.high), ...extras);
    const lowest = Math.min(0, ...pieces.flat().map((p) => p.low));
    const { top, ticks } = niceScale(highest);
    const step = ticks[1] - ticks[0];
    const below = Math.ceil(-lowest / step);
    const bottom = -below * step;
    const allTicks = [...Array.from({ length: below }, (_, i) => -(below - i) * step), ...ticks];
    const yOf = (value) => margin.top + ((top - value) / (top - bottom)) * strategy.plotHeight;

    const chart = svg('svg', { viewBox: `0 0 ${width} ${height}`, role: 'group', 'aria-label': strategy.ariaLabel });
    // The container is as wide as the chart, so the tooltip is placed against the whole chart.
    inner.style.minWidth = `${width}px`;
    chart.style.minWidth = `${width}px`;

    for (const tick of allTicks) {
      const lineClass = strategy.zeroLine && tick === 0 ? 'chart-zero' : 'chart-grid';
      chart.append(
        svg('line', { class: lineClass, x1: margin.left, x2: margin.left + plotWidth, y1: yOf(tick), y2: yOf(tick) }),
        svg('text', { class: 'chart-axis-label', x: margin.left - 8, y: yOf(tick) + 4 }, axisLabel(tick)),
      );
    }

    const tooltip = createTooltip(inner);
    years.forEach((row, index) => {
      const centre = margin.left + slot * index + slot / 2;
      const group = svg('g', {
        class: `chart-year-group${row.is_partial ? ' partial' : ''}`,
        tabindex: '0',
        role: 'img',
        'aria-label': strategy.describe(row),
      });
      const hit = svg('rect', { class: 'chart-hit', x: centre - slot / 2, y: margin.top, width: slot, height: strategy.plotHeight + margin.bottom });
      group.append(hit);

      const yearPieces = pieces[index];
      const highestPiece = Math.max(...yearPieces.map((p) => p.high));
      const lowestPiece = Math.min(...yearPieces.map((p) => p.low));
      const topmost = yearPieces.reduce((best, piece, i, all) => (piece.high >= all[best].high ? i : best), 0);
      yearPieces.forEach((piece, i) =>
        group.append(
          segmentShape(
            { class: piece.cssClass, 'data-legend': piece.key, ...(piece.label ? { 'aria-label': piece.label } : {}) },
            centre - barWidth / 2,
            barWidth,
            yOf(piece.high),
            yOf(piece.low),
            { rounded: piece.rounding === undefined ? i === topmost : piece.rounding },
          ),
        ),
      );

      // An invisible box over the whole bar, so the tooltip can sit beside it.
      const bar = yearPieces.length > 0
        ? svg('rect', { class: 'chart-bar-box', x: centre - barWidth / 2, y: yOf(highestPiece), width: barWidth, height: yOf(lowestPiece) - yOf(highestPiece) })
        : null;
      if (bar) group.append(bar);

      group.append(...strategy.decorate(row, { centre, barWidth, yOf, highest: yearPieces.length > 0 ? highestPiece : 0 }));
      strategy.labelLines(row).forEach((line, i) =>
        group.append(svg('text', { class: line.className, x: centre, y: bottomEdge + 18 + 16 * i }, line.text)),
      );

      group.addEventListener('mouseenter', () => tooltip.show(strategy.tooltipContent(row), bar, hit));
      group.addEventListener('mouseleave', () => tooltip.hide());
      group.addEventListener('focus', () => tooltip.show(strategy.tooltipContent(row), bar, hit));
      group.addEventListener('blur', () => tooltip.hide());
      bindHighlight(group, strategy.highlightAcrossYears ? [part('legend'), inner] : [part('legend'), tooltip.element, group]);
      chart.append(group);
    });

    inner.replaceChildren(chart, tooltip.element);
  }

  /** Loads the series from Rust and draws it, or shows the note when there is nothing yet. */
  async function reload() {
    const data = await invoke(strategy.command);
    const { years } = data;
    strategy.onLoaded?.(data);
    part('empty').hidden = years.length > 0;
    part('scroll').hidden = years.length === 0;
    if (years.length === 0) {
      part('chart').replaceChildren();
      return;
    }
    draw(years);
  }

  /** Reloads only while the chart's page is showing, so a hidden chart is not drawn at zero width. */
  async function reloadIfOpen() {
    if (!part().hidden) await reload();
  }

  /** Redraws at the new width when the window is resized. */
  function init() {
    window.addEventListener('resize', () => {
      if (drawnYears && !part('scroll').hidden && part('scroll').offsetParent !== null) draw(drawnYears);
    });
  }

  return { reload, reloadIfOpen, init };
}
