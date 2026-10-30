// Debt insights: the yearly stacked chart on the Debts page (CSD-012), one
// segment per debt. Its frame (axis, bars, tooltip, loading) is shared with the
// other charts in shared/yearly-chart.js; this file supplies what is specific
// to debts: the segments, the legend of debts and the total debt marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, nameLabel, legendItem, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The debts of the series by id, as Rust returned them (creation order). */
let items = new Map();

const keyOf = (id) => `debt-${id}`;
const nameOf = (segment) => items.get(segment.debt_id)?.name ?? '';
const slotOf = (id) => items.get(id)?.colour_slot ?? 0;
/** The repaid amount as shown: a dash when Rust has none. */
const deltaText = (row) => (row.repaid === null ? '–' : formatAmount(row.repaid));

function describe(row) {
  if (row.statement_count === 0) return `${yearLabel(row)}: no statements`;
  const parts = row.segments.map((segment) => `${nameOf(segment)} ${formatAmount(segment.balance)}`);
  const delta = row.repaid === null ? 'repaid none' : `repaid ${formatAmount(row.repaid)}${isNegative(row.repaid) ? ' (negative)' : ''}`;
  return `${yearLabel(row)}: ${parts.join(', ')}, total debt ${formatAmount(row.closing_balance)}, ${delta}`;
}

/** The rows read like the bar: the highest segment first. */
function tooltipContent(row) {
  const rows =
    row.statement_count === 0
      ? []
      : [
          ...topToBottom(row.segments, (segment) => segment.balance).flatMap((segment) =>
            tooltipRow(keyOf(segment.debt_id), swatch(`slot-${slotOf(segment.debt_id)}`), nameOf(segment), formatAmount(segment.balance)),
          ),
          ...tooltipRow('marker', swatch('marker'), 'Total debt', formatAmount(row.closing_balance)),
          ...tooltipRow(null, noSwatch(), 'Repaid', deltaText(row), row.repaid !== null && isNegative(row.repaid) ? 'negative' : ''),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: statements so far') : null,
    row.statement_count === 0 ? h('div', { class: 'tip-note' }, 'No statements') : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/**
 * The segments of a year, stacked upwards from the zero line in the order
 * received (CSD-012 REQ-6). A balance is never negative, so nothing hangs down;
 * a debt at 0.00 adds no height. The top corners of the highest segment are rounded.
 */
function piecesOf(row) {
  let level = 0;
  const pieces = [];
  for (const segment of row.segments) {
    const value = Number(segment.balance);
    if (value === 0) continue;
    pieces.push({
      key: keyOf(segment.debt_id),
      cssClass: `bar-slot-${slotOf(segment.debt_id)}`,
      label: `${nameOf(segment)}, ${row.year}, ${formatAmount(segment.balance)}`,
      low: level,
      high: level + value,
      hangs: false,
      rounding: false,
    });
    level += value;
  }
  const highest = pieces.at(-1);
  if (highest) highest.rounding = 'top';
  return pieces;
}

/** The legend: a swatch and the name of every debt in the series, and the total debt marker (CSD-012 REQ-9). */
function buildLegend({ debts: list }) {
  items = new Map(list.map((item) => [item.id, item]));
  const legendList = document.getElementById('debts-insights-legend');
  const scopes = () => [legendList, document.getElementById('debts-insights-chart')];
  legendList.replaceChildren(
    ...list.map((item) => legendItem(keyOf(item.id), swatch(`slot-${item.colour_slot}`), nameLabel(item.name), scopes)),
    legendItem('marker', swatch('marker'), legendLabel('Total debt'), scopes),
  );
}

const chart = createYearlyChart({
  id: 'debts-insights',
  command: 'get_debt_insights',
  ariaLabel: 'Debts per year',
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_balance)],
  describe,
  tooltipContent,
  /** The total debt marker, drawn for each year that has statements. */
  decorate: (row, geometry) => (row.statement_count === 0 ? [] : levelMarker(Number(row.closing_balance), geometry)),
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadDebtsInsightsIfOpen = chart.reloadIfOpen;

export function initDebtsInsights() {
  chart.init();
  initTabs([
    { tab: 'debts-tab-overview', panel: 'debts-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (CSD-012 REQ-15).
    { tab: 'debts-tab-insights', panel: 'debts-insights', onOpen: chart.reload },
  ]);
}
