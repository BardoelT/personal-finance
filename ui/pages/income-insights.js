// Income insights: the yearly stacked chart on the Income page (CSD-006). Its
// frame (axis, bars, tooltip, loading) is shared with the other charts in
// shared/yearly-chart.js; this file supplies what is specific to income.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, swatch, noSwatch, legendLabel, legendItem, tooltipRow } from '../shared/chart.js';
import { createYearlyChart, percentLabel, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The stacked series, bottom to top: the Rust field, the legend key and the label. */
const SEGMENTS = [
  ['gross_salary', 'salary', 'Gross salary'],
  ['holiday_pay', 'holiday', 'Holiday pay'],
  ['bonus', 'bonus', 'Bonus'],
];

/** The stacked pieces of a year, bottom to top. */
function piecesOf(row) {
  let level = 0;
  const pieces = [];
  for (const [field, key] of SEGMENTS) {
    const value = Number(row[field]);
    if (value <= 0) continue;
    pieces.push({ key, cssClass: `bar-${key}`, low: level, high: level + value });
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

/** The rows read like the bar: the top piece first. */
function tooltipContent(row) {
  const negative = row.growth_percent !== null && isNegative(row.growth_percent);
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    h(
      'dl',
      {},
      ...[...SEGMENTS].reverse().flatMap(([field, key, label]) => tooltipRow(key, swatch(key), label, formatAmount(row[field]))),
      ...tooltipRow(null, noSwatch(), 'Total gross', formatAmount(row.total_gross)),
      ...tooltipRow(null, noSwatch(), 'Growth', percentLabel(row.growth_percent), negative ? 'negative' : ''),
    ),
  ];
}

/** The legend: one item per stacked series; hovering one highlights it in every year. */
function buildLegend() {
  const legendList = document.getElementById('income-insights-legend');
  const scopes = () => [legendList, document.getElementById('income-insights-chart')];
  legendList.replaceChildren(...SEGMENTS.map(([, key, label]) => legendItem(key, swatch(key), legendLabel(label), scopes)));
}

const chart = createYearlyChart({
  id: 'income-insights',
  command: 'get_income_insights',
  ariaLabel: 'Income per year',
  onLoaded: buildLegend,
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
