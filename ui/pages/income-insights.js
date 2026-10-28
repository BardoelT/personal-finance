// Income insights: the yearly stacked chart of the Income page (CSD-006).
// Rust returns the figures and the growth; this file only draws them, so the
// only arithmetic here is scaling values to positions on the chart.
import { invoke, errorText } from '../shared/ipc.js';
import { h, showToast } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, niceScale, compact, segmentShape, createTooltip, bindHighlight, swatch, noSwatch, legendLabel } from '../shared/chart.js';

const $ = (id) => document.getElementById(id);

const MARGIN = { top: 28, right: 16, bottom: 30, left: 48 };
const PLOT_HEIGHT = 240;
const MIN_PLOT_WIDTH = 560;
const MIN_SLOT_WIDTH = 56;
const MAX_BAR_WIDTH = 44;
const SEGMENTS = [
  ['gross_salary', 'bar-salary', 'Gross salary'],
  ['holiday_pay', 'bar-holiday', 'Holiday pay'],
  ['bonus', 'bar-bonus', 'Bonus'],
];

/** "6.4" -> "+6.4%", "-2.5" -> "−2.5%", "0.0" -> "0.0%", null -> "–". */
function growthLabel(growth) {
  if (growth === null) return '–';
  if (growth.startsWith('-')) return `−${growth.slice(1)}%`;
  return growth === '0.0' ? '0.0%' : `+${growth}%`;
}

function yearLabel(row) {
  return row.is_partial ? `${row.year} YTD` : String(row.year);
}

function describe(row) {
  const growth = row.growth_percent === null ? 'no growth figure' : `growth ${growthLabel(row.growth_percent)}`;
  return (
    `${yearLabel(row)}: gross salary ${formatAmount(row.gross_salary)}, holiday pay ${formatAmount(row.holiday_pay)}, ` +
    `bonus ${formatAmount(row.bonus)}, total gross ${formatAmount(row.total_gross)}, ${growth}`
  );
}

function tooltipContent(row) {
  const negative = row.growth_percent !== null && row.growth_percent.startsWith('-');
  const line = (key, label, value, cssClass = '') => [
    h('dt', { 'data-legend': key }, key ? swatch(key) : noSwatch(), legendLabel(label)),
    h('dd', { 'data-legend': key, class: cssClass }, value),
  ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    h(
      'dl',
      {},
      ...SEGMENTS.flatMap(([field, cssClass, label]) => line(cssClass.replace('bar-', ''), label, formatAmount(row[field]))),
      ...line(null, 'Total gross', formatAmount(row.total_gross)),
      ...line(null, 'Growth', growthLabel(row.growth_percent), negative ? 'negative' : ''),
    ),
  ];
}

let drawnYears = null;

function drawChart(years) {
  drawnYears = years;
  const inner = $('insights-chart');
  const count = years.length;
  // Draw at the real pixel width, so bars, lines and text are never scaled up.
  const available = inner.clientWidth - MARGIN.left - MARGIN.right;
  const plotWidth = Math.max(count * MIN_SLOT_WIDTH, MIN_PLOT_WIDTH, available);
  const slot = plotWidth / count;
  const barWidth = Math.min(slot * 0.5, MAX_BAR_WIDTH);
  const width = MARGIN.left + plotWidth + MARGIN.right;
  const height = MARGIN.top + PLOT_HEIGHT + MARGIN.bottom;
  const baseline = MARGIN.top + PLOT_HEIGHT;

  const largest = Math.max(...years.map((row) => Number(row.total_gross)));
  const { top, ticks } = niceScale(largest);
  const yOf = (value) => baseline - (value / top) * PLOT_HEIGHT;

  const chart = svg('svg', { viewBox: `0 0 ${width} ${height}`, role: 'group', 'aria-label': 'Income per year' });
  chart.style.minWidth = `${width}px`;

  for (const tick of ticks) {
    chart.append(
      svg('line', { class: 'chart-grid', x1: MARGIN.left, x2: MARGIN.left + plotWidth, y1: yOf(tick), y2: yOf(tick) }),
      svg('text', { class: 'chart-axis-label', x: MARGIN.left - 8, y: yOf(tick) + 4 }, compact(tick)),
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

    let stacked = 0;
    const pieces = [];
    for (const [field, cssClass] of SEGMENTS) {
      const value = Number(row[field]);
      if (value <= 0) continue;
      pieces.push({ cssClass, from: stacked, to: stacked + value });
      stacked += value;
    }
    pieces.forEach((piece, i) =>
      group.append(
        segmentShape(
          { class: piece.cssClass, 'data-legend': piece.cssClass.replace('bar-', '') },
          centre - barWidth / 2,
          barWidth,
          yOf(piece.to),
          yOf(piece.from),
          { rounded: i === pieces.length - 1 },
        ),
      ),
    );
    // An invisible box over the whole bar, so the tooltip can sit beside it.
    const bar = stacked > 0
      ? svg('rect', { class: 'chart-bar-box', x: centre - barWidth / 2, y: yOf(stacked), width: barWidth, height: baseline - yOf(stacked) })
      : null;
    if (bar) group.append(bar);

    const negative = row.growth_percent !== null && row.growth_percent.startsWith('-');
    group.append(
      svg('text', { class: `chart-growth${negative ? ' negative' : ''}`, x: centre, y: yOf(stacked) - 6 }, growthLabel(row.growth_percent)),
      svg('text', { class: 'chart-year', x: centre, y: baseline + 18 }, yearLabel(row)),
    );

    const show = () => tooltip.show(tooltipContent(row), bar, hit);
    const hide = () => tooltip.hide();
    group.addEventListener('mouseenter', show);
    group.addEventListener('mouseleave', hide);
    group.addEventListener('focus', show);
    group.addEventListener('blur', hide);
    bindHighlight(group, [$('insights-legend'), tooltip.element, group]);
    chart.append(group);
  });

  inner.replaceChildren(chart, tooltip.element);
}

/** Loads the series from Rust and draws it, or shows the note when there is no income yet. */
export async function reloadIncomeInsights() {
  const { years } = await invoke('get_income_insights');
  $('insights-empty').hidden = years.length > 0;
  $('insights-scroll').hidden = years.length === 0;
  if (years.length === 0) {
    $('insights-chart').replaceChildren();
    return;
  }
  drawChart(years);
}

// ---- Tabs ----

const TABS = [
  { tab: 'income-tab-overview', panel: 'income-overview' },
  { tab: 'income-tab-insights', panel: 'income-insights' },
];

function selectTab(index, { focus = false } = {}) {
  TABS.forEach(({ tab, panel }, i) => {
    const selected = i === index;
    $(tab).setAttribute('aria-selected', String(selected));
    $(tab).tabIndex = selected ? 0 : -1;
    $(panel).hidden = !selected;
  });
  if (focus) $(TABS[index].tab).focus();
  // Opening Insights recomputes it, so it never shows stale figures (REQ-11).
  if (TABS[index].panel === 'income-insights') {
    reloadIncomeInsights().catch((error) => showToast(errorText(error)));
  }
}

export function initIncomeInsights() {
  window.addEventListener('resize', () => {
    if (drawnYears && !$('insights-scroll').hidden && $('insights-scroll').offsetParent !== null) drawChart(drawnYears);
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
