// Investment insights: the yearly stacked chart on the Investments page
// (CSD-007). Its frame (axis, bars, tooltip, loading) is shared with the income
// chart in shared/yearly-chart.js; this file supplies what is specific to
// investments: the waterfall pieces, the closing-balance marker and the growth
// percentage.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, swatch, noSwatch, legendLabel } from '../shared/chart.js';
import { createYearlyChart, percentLabel, isNegative, yearLabel, axisLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

const SEGMENTS = [
  ['opening_balance', 'Opening balance'],
  ['net_deposits', 'Net deposits'],
  ['capital_growth', 'Capital growth'],
];

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

const chart = createYearlyChart({
  id: 'investments-insights',
  command: 'get_investment_insights',
  ariaLabel: 'Investments per year',
  plotHeight: 260,
  minSlotWidth: 64,
  zeroLine: true,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_balance)],
  describe,
  tooltipContent,
  /** The closing-balance marker, drawn for each year that has statements. */
  decorate(row, { centre, barWidth, yOf }) {
    if (row.statement_count === 0) return [];
    const y = yOf(Number(row.closing_balance));
    const half = barWidth * 0.7;
    return [
      svg('line', { class: 'chart-marker-hit', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
      svg('line', { class: 'chart-marker', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
      svg('text', { class: 'chart-marker-label', x: centre, y: y - 7 }, axisLabel(Number(row.closing_balance))),
    ];
  },
  /** The year, and the growth percentage on a second line. */
  labelLines: (row) => [
    { text: yearLabel(row), className: 'chart-year' },
    {
      text: percentLabel(row.growth_percent),
      className: `chart-percent${row.growth_percent !== null && isNegative(row.growth_percent) ? ' negative' : ''}`,
    },
  ],
});

export const reloadInvestmentsInsights = chart.reload;
/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadInvestmentsInsightsIfOpen = chart.reloadIfOpen;

export function initInvestmentsInsights() {
  chart.init();
  initTabs([
    { tab: 'investments-tab-overview', panel: 'investments-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (REQ-13).
    { tab: 'investments-tab-insights', panel: 'investments-insights', onOpen: chart.reload },
  ]);
}
