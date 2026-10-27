// Investment insights: the yearly stacked chart of the Investments page (CSD-007).
// Rust returns the figures and the growth percentage; this file only draws them,
// so the only arithmetic here is stacking and scaling values to positions.
import { invoke, errorText, h, formatAmount, showToast } from './api.js';
import { svg, niceScale, compact } from './chart.js';

const $ = (id) => document.getElementById(id);

const MARGIN = { top: 28, right: 16, bottom: 46, left: 48 };
const PLOT_HEIGHT = 260;
const MIN_PLOT_WIDTH = 560;
const MIN_SLOT_WIDTH = 64;
const MAX_BAR_WIDTH = 56;
const SEGMENTS = [
  ['opening_balance', 'Opening balance'],
  ['new_accounts', 'New accounts'],
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
    `${yearLabel(row)}: opening balance ${formatAmount(row.opening_balance)}, new accounts ${formatAmount(row.new_accounts)}, ` +
    `net deposits ${formatAmount(row.net_deposits)}, capital growth ${growth}, growth ${percentLabel(row.growth_percent)}, ` +
    `closing balance ${formatAmount(row.closing_balance)}, ${row.statement_count} statement${row.statement_count === 1 ? '' : 's'}`
  );
}

function tooltipFor(row) {
  const rows =
    row.statement_count === 0
      ? [h('dt', {}, 'Statements'), h('dd', {}, '0')]
      : [
          ...SEGMENTS.flatMap(([field, label]) => [
            h('dt', {}, label),
            h('dd', { class: field === 'capital_growth' && isNegative(row[field]) ? 'negative' : '' }, formatAmount(row[field])),
          ]),
          h('dt', {}, 'Growth'),
          h('dd', { class: row.growth_percent !== null && isNegative(row.growth_percent) ? 'negative' : '' }, percentLabel(row.growth_percent)),
          h('dt', {}, 'Closing balance'),
          h('dd', {}, formatAmount(row.closing_balance)),
          h('dt', {}, 'Statements'),
          h('dd', {}, String(row.statement_count)),
        ];
  return h(
    'div',
    { class: 'chart-tooltip', role: 'presentation', hidden: true },
    h('div', { class: 'tip-title' }, yearLabel(row)),
    h('dl', {}, ...rows),
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
    let cssClass = { opening_balance: 'bar-opening', new_accounts: 'bar-new' }[field];
    if (field === 'net_deposits') cssClass = value < 0 ? 'bar-deposits withdrawal' : 'bar-deposits';
    if (field === 'capital_growth') cssClass = value < 0 ? 'bar-growth-down' : 'bar-growth-up';
    pieces.push({ cssClass, low: Math.min(level, next), high: Math.max(level, next), hangs: value < 0 });
    level = next;
  }
  // Pieces that hang down are drawn last, so a later rise does not hide them.
  return [...pieces.filter((p) => !p.hangs), ...pieces.filter((p) => p.hangs)];
}

function hatchPattern() {
  return svg(
    'defs',
    {},
    svg(
      'pattern',
      { id: 'hatch-deposits', width: 6, height: 6, patternUnits: 'userSpaceOnUse', patternTransform: 'rotate(45)' },
      svg('rect', { class: 'hatch-fill', width: 6, height: 6 }),
      svg('line', { class: 'hatch-line', x1: 0, y1: 0, x2: 0, y2: 6 }),
    ),
  );
}

function drawChart(years) {
  const inner = $('investment-insights-chart');
  const count = years.length;
  const plotWidth = Math.max(count * MIN_SLOT_WIDTH, MIN_PLOT_WIDTH);
  const slot = plotWidth / count;
  const barWidth = Math.min(slot * 0.55, MAX_BAR_WIDTH);
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
  chart.append(hatchPattern());

  for (const tick of allTicks) {
    chart.append(
      svg('line', { class: tick === 0 ? 'chart-zero' : 'chart-grid', x1: MARGIN.left, x2: MARGIN.left + plotWidth, y1: yOf(tick), y2: yOf(tick) }),
      svg('text', { class: 'chart-axis-label', x: MARGIN.left - 8, y: yOf(tick) + 4 }, axisLabel(tick)),
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

    for (const piece of pieces[index]) {
      group.append(
        svg('rect', {
          class: piece.cssClass,
          x: centre - barWidth / 2,
          y: yOf(piece.high),
          width: barWidth,
          height: yOf(piece.low) - yOf(piece.high),
        }),
      );
    }

    if (row.statement_count > 0) {
      const y = yOf(Number(row.closing_balance));
      const half = barWidth * 0.7;
      group.append(
        svg('line', { class: 'chart-marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
        svg('text', { class: 'chart-marker-label', x: centre, y: y - 7 }, axisLabel(Number(row.closing_balance))),
      );
    }

    const negative = row.growth_percent !== null && isNegative(row.growth_percent);
    group.append(
      svg('text', { class: 'chart-year', x: centre, y: bottomEdge + 18 }, yearLabel(row)),
      svg('text', { class: `chart-percent${negative ? ' negative' : ''}`, x: centre, y: bottomEdge + 34 }, percentLabel(row.growth_percent)),
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

/** Loads the series from Rust and draws it, or shows the note when there is no statement yet. */
export async function reloadInvestmentInsights() {
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
    reloadInvestmentInsights().catch((error) => showToast(errorText(error)));
  }
}

export function initInvestmentInsights() {
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
