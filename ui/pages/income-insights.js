// Income insights: the yearly stacked chart on the Income page (CSD-006). Its
// frame (axis, bars, tooltip, loading) is shared with the investments chart in
// shared/yearly-chart.js; this file supplies what is specific to income.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, swatch, noSwatch, legendLabel } from '../shared/chart.js';
import { createYearlyChart, percentLabel, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

const SEGMENTS = [
  ['gross_salary', 'bar-salary', 'Gross salary'],
  ['holiday_pay', 'bar-holiday', 'Holiday pay'],
  ['bonus', 'bar-bonus', 'Bonus'],
];

/** The stacked pieces of a year, bottom to top. */
function piecesOf(row) {
  let level = 0;
  const pieces = [];
  for (const [field, cssClass] of SEGMENTS) {
    const value = Number(row[field]);
    if (value <= 0) continue;
    pieces.push({ key: cssClass.replace('bar-', ''), cssClass, low: level, high: level + value });
    level += value;
  }
  return pieces;
}

function describe(row) {
  const growth = row.growth_percent === null ? 'no growth figure' : `growth ${percentLabel(row.growth_percent)}`;
  return (
    `${yearLabel(row)}: gross salary ${formatAmount(row.gross_salary)}, holiday pay ${formatAmount(row.holiday_pay)}, ` +
    `bonus ${formatAmount(row.bonus)}, total gross ${formatAmount(row.total_gross)}, ${growth}`
  );
}

function tooltipContent(row) {
  const negative = row.growth_percent !== null && isNegative(row.growth_percent);
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
      ...line(null, 'Growth', percentLabel(row.growth_percent), negative ? 'negative' : ''),
    ),
  ];
}

const chart = createYearlyChart({
  id: 'income-insights',
  command: 'get_income_insights',
  ariaLabel: 'Income per year',
  plotHeight: 240,
  minSlotWidth: 56,
  zeroLine: false,
  piecesOf,
  describe,
  tooltipContent,
  /** The growth percentage above the bar. */
  decorate(row, { centre, yOf, highest }) {
    const negative = row.growth_percent !== null && isNegative(row.growth_percent);
    return [svg('text', { class: `chart-growth${negative ? ' negative' : ''}`, x: centre, y: yOf(highest) - 6 }, percentLabel(row.growth_percent))];
  },
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

export const reloadIncomeInsights = chart.reload;
/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadIncomeInsightsIfOpen = chart.reloadIfOpen;

export function initIncomeInsights() {
  chart.init();
  initTabs([
    { tab: 'income-tab-overview', panel: 'income-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (REQ-11).
    { tab: 'income-tab-insights', panel: 'income-insights', onOpen: chart.reload },
  ]);
}
