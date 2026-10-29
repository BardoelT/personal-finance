// Investment insights: the yearly stacked chart on the Investments page
// (CSD-007). Its frame (axis, bars, tooltip, loading) is shared with the other
// charts in shared/yearly-chart.js; this file supplies what is specific to
// investments: the waterfall pieces, the closing-balance marker and the growth
// percentage.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, legendItem, tooltipRow } from '../shared/chart.js';
import { createYearlyChart, levelMarker, percentLabel, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The legend items in the order of the bar from the bottom: key and label. A loss and a gain are two items. */
const LEGEND = [
  ['opening', 'Opening balance'],
  ['deposits', 'Net deposits'],
  ['growth-up', 'Capital growth (gain)'],
  ['growth-down', 'Capital growth (loss)'],
];

function describe(row) {
  if (row.statement_count === 0) return `${yearLabel(row)}: no statements`;
  const growth = isNegative(row.capital_growth) ? `${formatAmount(row.capital_growth)} (negative)` : formatAmount(row.capital_growth);
  return (
    `${yearLabel(row)}: opening balance ${formatAmount(row.opening_balance)}, ` +
    `net deposits ${formatAmount(row.net_deposits)}, capital growth ${growth}, growth ${percentLabel(row.growth_percent)}, ` +
    `closing balance ${formatAmount(row.closing_balance)}`
  );
}

/** The rows read like the bar, top to bottom: capital growth, net deposits, opening balance. */
function tooltipContent(row) {
  const growthKey = isNegative(row.capital_growth) ? 'growth-down' : 'growth-up';
  const rows =
    row.statement_count === 0
      ? []
      : [
          ...tooltipRow(growthKey, swatch(growthKey), 'Capital growth', formatAmount(row.capital_growth), isNegative(row.capital_growth) ? 'negative' : ''),
          ...tooltipRow('deposits', swatch('deposits'), 'Net deposits', formatAmount(row.net_deposits)),
          ...tooltipRow('opening', swatch('opening'), 'Opening balance', formatAmount(row.opening_balance)),
          ...tooltipRow('marker', swatch('marker'), 'Closing balance', formatAmount(row.closing_balance)),
          ...tooltipRow(null, noSwatch(), 'Growth', percentLabel(row.growth_percent), row.growth_percent !== null && isNegative(row.growth_percent) ? 'negative' : ''),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.statement_count === 0 ? h('div', { class: 'tip-note' }, 'No statements') : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/**
 * The stacked pieces of a year as a waterfall: a positive piece rises from the
 * running total and a negative one hangs down from it (CSD-007 REQ-8).
 */
function piecesOf(row) {
  let level = 0;
  const pieces = [];
  for (const [field, kind] of [['opening_balance', 'opening'], ['net_deposits', 'deposits'], ['capital_growth', 'growth']]) {
    const value = Number(row[field]);
    if (value === 0) continue;
    const next = level + value;
    const key = kind === 'growth' ? (value < 0 ? 'growth-down' : 'growth-up') : kind;
    const withdrawal = kind === 'deposits' && value < 0;
    pieces.push({
      key,
      cssClass: `bar-${key}${withdrawal ? ' withdrawal' : ''}`,
      low: Math.min(level, next),
      high: Math.max(level, next),
      hangs: value < 0,
    });
    level = next;
  }
  // Pieces that hang down are drawn last, so a later rise does not hide them.
  return [...pieces.filter((p) => !p.hangs), ...pieces.filter((p) => p.hangs)];
}

/** The legend: one item per kind of piece, and the closing-balance marker; hovering one highlights it in every year. */
function buildLegend() {
  const legendList = document.getElementById('investments-insights-legend');
  const scopes = () => [legendList, document.getElementById('investments-insights-chart')];
  legendList.replaceChildren(
    ...LEGEND.map(([key, label]) => legendItem(key, swatch(key), legendLabel(label), scopes)),
    legendItem('marker', swatch('marker'), legendLabel('Closing balance'), scopes),
  );
}

const chart = createYearlyChart({
  id: 'investments-insights',
  command: 'get_investment_insights',
  ariaLabel: 'Investments per year',
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_balance)],
  describe,
  tooltipContent,
  /** The closing-balance marker, drawn for each year that has statements. */
  decorate: (row, geometry) => (row.statement_count === 0 ? [] : levelMarker(Number(row.closing_balance), geometry)),
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
