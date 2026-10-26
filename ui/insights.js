// Income insights: the yearly stacked chart of the Income page (CSD-006).
// Rust returns the figures and the growth; this file only draws them, so the
// only arithmetic here is scaling values to positions on the chart.
import { invoke, errorText, h, formatAmount, showToast } from './api.js';

const $ = (id) => document.getElementById(id);
const SVG_NS = 'http://www.w3.org/2000/svg';

const MARGIN = { top: 28, right: 16, bottom: 30, left: 48 };
const PLOT_HEIGHT = 240;
const MIN_PLOT_WIDTH = 560;
const MIN_SLOT_WIDTH = 56;
const MAX_BAR_WIDTH = 56;
const TICK_INTERVALS = 4;
const SEGMENTS = [
  ['gross_salary', 'bar-salary', 'Gross salary'],
  ['holiday_pay', 'bar-holiday', 'Holiday pay'],
  ['bonus', 'bar-bonus', 'Bonus'],
];

function svg(tag, attrs = {}, ...children) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs)) node.setAttribute(key, value);
  node.append(...children);
  return node;
}

/** Axis top and tick values: a round step, with the top at or above `max`. */
function niceScale(max) {
  const rawStep = max / TICK_INTERVALS;
  const magnitude = 10 ** Math.floor(Math.log10(rawStep));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * magnitude).find((s) => s >= rawStep);
  const intervals = Math.ceil(max / step);
  return { top: intervals * step, ticks: Array.from({ length: intervals + 1 }, (_, i) => i * step) };
}

/** 2500 -> "2.5k", 10000 -> "10k", 1500000 -> "1.5M". */
function compact(value) {
  const trim = (n) => String(Number(n.toFixed(2)));
  if (value >= 1_000_000) return `${trim(value / 1_000_000)}M`;
  if (value >= 1_000) return `${trim(value / 1_000)}k`;
  return trim(value);
}

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

function tooltipFor(row) {
  const negative = row.growth_percent !== null && row.growth_percent.startsWith('-');
  return h(
    'div',
    { class: 'chart-tooltip', role: 'presentation', hidden: true },
    h('div', { class: 'tip-title' }, yearLabel(row)),
    h(
      'dl',
      {},
      ...SEGMENTS.flatMap(([field, , label]) => [h('dt', {}, label), h('dd', {}, formatAmount(row[field]))]),
      h('dt', {}, 'Total gross'),
      h('dd', {}, formatAmount(row.total_gross)),
      h('dt', {}, 'Growth'),
      h('dd', { class: negative ? 'negative' : '' }, growthLabel(row.growth_percent)),
    ),
  );
}

function showTooltip(inner, tooltip, hit) {
  const box = inner.getBoundingClientRect();
  const slot = hit.getBoundingClientRect();
  tooltip.hidden = false;
  const width = tooltip.offsetWidth;
  const centre = slot.left - box.left + slot.width / 2;
  const left = Math.min(Math.max(centre - width / 2, 0), Math.max(box.width - width, 0));
  tooltip.style.left = `${left}px`;
  tooltip.style.top = '0px';
}

function drawChart(years) {
  const inner = $('insights-chart');
  const count = years.length;
  const plotWidth = Math.max(count * MIN_SLOT_WIDTH, MIN_PLOT_WIDTH);
  const slot = plotWidth / count;
  const barWidth = Math.min(slot * 0.6, MAX_BAR_WIDTH);
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

  const tooltips = [];
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
    for (const [field, cssClass] of SEGMENTS) {
      const value = Number(row[field]);
      if (value <= 0) continue;
      const segTop = yOf(stacked + value);
      group.append(svg('rect', { class: cssClass, x: centre - barWidth / 2, y: segTop, width: barWidth, height: yOf(stacked) - segTop }));
      stacked += value;
    }

    const negative = row.growth_percent !== null && row.growth_percent.startsWith('-');
    group.append(
      svg('text', { class: `chart-growth${negative ? ' negative' : ''}`, x: centre, y: yOf(stacked) - 6 }, growthLabel(row.growth_percent)),
      svg('text', { class: 'chart-year', x: centre, y: baseline + 18 }, yearLabel(row)),
    );

    const tooltip = tooltipFor(row);
    tooltips.push(tooltip);
    const show = () => showTooltip(inner, tooltip, hit);
    const hide = () => { tooltip.hidden = true; };
    group.addEventListener('mouseenter', show);
    group.addEventListener('mouseleave', hide);
    group.addEventListener('focus', show);
    group.addEventListener('blur', hide);
    chart.append(group);
  });

  inner.replaceChildren(chart, ...tooltips);
}

/** Loads the series from Rust and draws it, or shows the note when there is no income yet. */
export async function reloadInsights() {
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
    reloadInsights().catch((error) => showToast(errorText(error)));
  }
}

export function initInsights() {
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
