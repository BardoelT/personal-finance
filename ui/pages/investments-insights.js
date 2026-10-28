// Investment insights: the yearly stacked chart of the Investments page (CSD-007).
// Rust returns the figures and the growth percentage; this file only draws them,
// so the only arithmetic here is stacking and scaling values to positions.
import { invoke, errorText } from '../shared/ipc.js';
import { h, showToast } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, niceScale, compact, segmentShape, createTooltip, bindHighlight, swatch, noSwatch, legendLabel } from '../shared/chart.js';

const $ = (id) => document.getElementById(id);

const MARGIN = { top: 28, right: 16, bottom: 46, left: 48 };
const PLOT_HEIGHT = 260;
const MIN_PLOT_WIDTH = 560;
const MIN_SLOT_WIDTH = 64;
const MAX_BAR_WIDTH = 44;
const SEGMENTS = [
  ['opening_balance', 'Opening balance'],
  ['net_deposits', 'Net deposits'],
  ['capital_growth', 'Capital growth'],
];

/** "6.1" -> "+6.1%", "-2.5" -> "−2.5%", "0.0" -> "0.0%", null -> "–". */
function percentLabel(percent) {
  if (percent === null) return '–';
  if (percent.startsWith('-')) return `−${percent.slice(1)}%`;
  return percent === '0.0' ? '0.0%' : `+${percent}%`;
}

const isNegative = (text) => text.startsWith('-');

function yearLabel(row) {
  return row.is_partial ? `${row.year} YTD` : String(row.year);
}

/** Axis labels: compact, with a real minus sign below zero. */
function axisLabel(value) {
  return value < 0 ? `−${compact(-value)}` : compact(value);
}

function describe(row) {
  if (row.statement_count === 0) return `${yearLabel(row)}: no statements`;
  const growth = isNegative(row.capital_growth) ? `${formatAmount(row.capital_growth)} (negative)` : formatAmount(row.capital_growth);
  return (
    `${yearLabel(row)}: opening balance ${formatAmount(row.opening_balance)}, ` +
    `net deposits ${formatAmount(row.net_deposits)}, capital growth ${growth}, growth ${percentLabel(row.growth_percent)}, ` +
    `closing balance ${formatAmount(row.closing_balance)}, ${row.statement_count} statement${row.statement_count === 1 ? '' : 's'}`
  );
}

function tooltipContent(row) {
  const line = (key, label, value, cssClass = '') => [
    h('dt', { 'data-legend': key }, key ? swatch(key) : noSwatch(), legendLabel(label)),
    h('dd', { 'data-legend': key, class: cssClass }, value),
  ];
  const growthKey = isNegative(row.capital_growth) ? 'growth-down' : 'growth-up';
  const rows =
    row.statement_count === 0
      ? [...line(null, 'Statements', '0')]
      : [
          ...line('opening', 'Opening balance', formatAmount(row.opening_balance)),
          ...line('deposits', 'Net deposits', formatAmount(row.net_deposits)),
          ...line(growthKey, 'Capital growth', formatAmount(row.capital_growth), isNegative(row.capital_growth) ? 'negative' : ''),
          ...line(null, 'Growth', percentLabel(row.growth_percent), row.growth_percent !== null && isNegative(row.growth_percent) ? 'negative' : ''),
          ...line('marker', 'Closing balance', formatAmount(row.closing_balance)),
          ...line(null, 'Statements', String(row.statement_count)),
        ];
  return [h('div', { class: 'tip-title' }, yearLabel(row)), h('dl', {}, ...rows)];
}

/**
 * The stacked pieces of a year as a waterfall: a positive piece rises from the
 * running total and a negative one hangs down from it (CSD-007 REQ-8).
 */
function piecesOf(row) {
  let level = 0;
  const pieces = [];
  for (const [field] of SEGMENTS) {
    const value = Number(row[field]);
    if (value === 0) continue;
    const next = level + value;
    let key = { opening_balance: 'opening' }[field];
    if (field === 'net_deposits') key = 'deposits';
    if (field === 'capital_growth') key = value < 0 ? 'growth-down' : 'growth-up';
    const cssClass = { opening: 'bar-opening', deposits: value < 0 ? 'bar-deposits withdrawal' : 'bar-deposits', 'growth-up': 'bar-growth-up', 'growth-down': 'bar-growth-down' }[key];
    pieces.push({ key, cssClass, low: Math.min(level, next), high: Math.max(level, next), hangs: value < 0 });
    level = next;
  }
  // Pieces that hang down are drawn last, so a later rise does not hide them.
  return [...pieces.filter((p) => !p.hangs), ...pieces.filter((p) => p.hangs)];
}

let drawnYears = null;

function drawChart(years) {
  drawnYears = years;
  const inner = $('investment-insights-chart');
  const count = years.length;
  // Draw at the real pixel width, so bars, lines and text are never scaled up.
  const available = inner.clientWidth - MARGIN.left - MARGIN.right;
  const plotWidth = Math.max(count * MIN_SLOT_WIDTH, MIN_PLOT_WIDTH, available);
  const slot = plotWidth / count;
  const barWidth = Math.min(slot * 0.5, MAX_BAR_WIDTH);
  const width = MARGIN.left + plotWidth + MARGIN.right;
  const height = MARGIN.top + PLOT_HEIGHT + MARGIN.bottom;
  const bottomEdge = MARGIN.top + PLOT_HEIGHT;

  const pieces = years.map(piecesOf);
  const highest = Math.max(1, ...pieces.flat().map((p) => p.high), ...years.map((row) => Number(row.closing_balance)));
  const lowest = Math.min(0, ...pieces.flat().map((p) => p.low));
  const { top, ticks } = niceScale(highest);
  const step = ticks[1] - ticks[0];
  const below = Math.ceil(-lowest / step);
  const bottom = -below * step;
  const allTicks = [...Array.from({ length: below }, (_, i) => -(below - i) * step), ...ticks];
  const yOf = (value) => MARGIN.top + ((top - value) / (top - bottom)) * PLOT_HEIGHT;

  const chart = svg('svg', { viewBox: `0 0 ${width} ${height}`, role: 'group', 'aria-label': 'Investments per year' });
  chart.style.minWidth = `${width}px`;

  for (const tick of allTicks) {
    chart.append(
      svg('line', { class: tick === 0 ? 'chart-zero' : 'chart-grid', x1: MARGIN.left, x2: MARGIN.left + plotWidth, y1: yOf(tick), y2: yOf(tick) }),
      svg('text', { class: 'chart-axis-label', x: MARGIN.left - 8, y: yOf(tick) + 4 }, axisLabel(tick)),
    );
  }

  const tooltip = createTooltip(inner);
  years.forEach((row, index) => {
    const centre = MARGIN.left + slot * index + slot / 2;
    const group = svg('g', {
      class: `chart-year-group${row.is_partial ? ' partial' : ''}`,
      tabindex: '0',
      role: 'img',
      'aria-label': describe(row),
    });
    const hit = svg('rect', { class: 'chart-hit', x: centre - slot / 2, y: MARGIN.top, width: slot, height: PLOT_HEIGHT + MARGIN.bottom });
    group.append(hit);

    const topmost = pieces[index].reduce((best, piece, i, all) => (piece.high >= all[best].high ? i : best), 0);
    pieces[index].forEach((piece, i) =>
      group.append(
        segmentShape(
          { class: piece.cssClass, 'data-legend': piece.key },
          centre - barWidth / 2,
          barWidth,
          yOf(piece.high),
          yOf(piece.low),
          { rounded: i === topmost },
        ),
      ),
    );

    // An invisible box over the whole bar, so the tooltip can sit beside it.
    const bar = pieces[index].length > 0
      ? svg('rect', {
          class: 'chart-bar-box',
          x: centre - barWidth / 2,
          y: yOf(Math.max(...pieces[index].map((p) => p.high))),
          width: barWidth,
          height: yOf(Math.min(...pieces[index].map((p) => p.low))) - yOf(Math.max(...pieces[index].map((p) => p.high))),
        })
      : null;
    if (bar) group.append(bar);

    if (row.statement_count > 0) {
      const y = yOf(Number(row.closing_balance));
      const half = barWidth * 0.7;
      group.append(
        svg('line', { class: 'chart-marker-hit', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
        svg('line', { class: 'chart-marker', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
        svg('text', { class: 'chart-marker-label', x: centre, y: y - 7 }, axisLabel(Number(row.closing_balance))),
      );
    }

    const negative = row.growth_percent !== null && isNegative(row.growth_percent);
    group.append(
      svg('text', { class: 'chart-year', x: centre, y: bottomEdge + 18 }, yearLabel(row)),
      svg('text', { class: `chart-percent${negative ? ' negative' : ''}`, x: centre, y: bottomEdge + 34 }, percentLabel(row.growth_percent)),
    );

    const show = () => tooltip.show(tooltipContent(row), bar, hit);
    const hide = () => tooltip.hide();
    group.addEventListener('mouseenter', show);
    group.addEventListener('mouseleave', hide);
    group.addEventListener('focus', show);
    group.addEventListener('blur', hide);
    bindHighlight(group, [$('investment-insights-legend'), tooltip.element, group]);
    chart.append(group);
  });

  inner.replaceChildren(chart, tooltip.element);
}

/** Loads the series from Rust and draws it, or shows the note when there is no statement yet. */
export async function reloadInvestmentsInsights() {
  const { years } = await invoke('get_investment_insights');
  $('investment-insights-empty').hidden = years.length > 0;
  $('investment-insights-scroll').hidden = years.length === 0;
  if (years.length === 0) {
    $('investment-insights-chart').replaceChildren();
    return;
  }
  drawChart(years);
}

// ---- Tabs ----

const TABS = [
  { tab: 'investments-tab-overview', panel: 'investments-overview' },
  { tab: 'investments-tab-insights', panel: 'investments-insights' },
];

function selectTab(index, { focus = false } = {}) {
  TABS.forEach(({ tab, panel }, i) => {
    const selected = i === index;
    $(tab).setAttribute('aria-selected', String(selected));
    $(tab).tabIndex = selected ? 0 : -1;
    $(panel).hidden = !selected;
  });
  if (focus) $(TABS[index].tab).focus();
  // Opening Insights recomputes it, so it never shows stale figures (REQ-13).
  if (TABS[index].panel === 'investments-insights') {
    reloadInvestmentsInsights().catch((error) => showToast(errorText(error)));
  }
}

export function initInvestmentsInsights() {
  window.addEventListener('resize', () => {
    if (drawnYears && !$('investment-insights-scroll').hidden && $('investment-insights-scroll').offsetParent !== null) drawChart(drawnYears);
  });
  TABS.forEach(({ tab }, index) => {
    $(tab).addEventListener('click', () => selectTab(index));
    $(tab).addEventListener('keydown', (event) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
      if (step === undefined) return;
      event.preventDefault();
      selectTab((index + step + TABS.length) % TABS.length, { focus: true });
    });
  });
}
