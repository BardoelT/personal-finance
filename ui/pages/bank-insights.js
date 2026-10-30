// Bank account insights: the yearly stacked chart on the Bank accounts page
// (CSD-010), one segment per bank. Its frame (axis, bars, tooltip, loading) is
// shared with the other charts in shared/yearly-chart.js; this file supplies
// what is specific to banks: the segments, the legend of banks and the
// closing-balance marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, legendLabel, nameLabel, legendItem, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The banks of the series by id, as Rust returned them (creation order). */
let banks = new Map();

const keyOf = (accountId) => `bank-${accountId}`;
const nameOf = (segment) => banks.get(segment.account_id)?.name ?? '';
const slotOf = (accountId) => banks.get(accountId)?.colour_slot ?? 0;

function describe(row) {
  if (row.entry_count === 0) return `${yearLabel(row)}: no entries`;
  const parts = row.segments.map(
    (segment) => `${nameOf(segment)} ${formatAmount(segment.balance)}${isNegative(segment.balance) ? ' (negative)' : ''}`,
  );
  return `${yearLabel(row)}: ${parts.join(', ')}, closing balance ${formatAmount(row.closing_balance)}`;
}

/** The rows read like the bar: the highest segment first, those below the zero line last. */
function tooltipContent(row) {
  const rows =
    row.entry_count === 0
      ? []
      : [
          ...topToBottom(row.segments, (segment) => segment.balance).flatMap((segment) =>
            tooltipRow(
              keyOf(segment.account_id),
              swatch(`slot-${slotOf(segment.account_id)}`),
              nameOf(segment),
              formatAmount(segment.balance),
              isNegative(segment.balance) ? 'negative' : '',
            ),
          ),
          ...tooltipRow('marker', swatch('marker'), 'Closing balance', formatAmount(row.closing_balance), isNegative(row.closing_balance) ? 'negative' : ''),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: entries so far') : null,
    row.entry_count === 0 ? h('div', { class: 'tip-note' }, 'No entries') : null,
    h('dl', {}, ...rows),
  ].filter(Boolean);
}
/**
 * The segments of a year: positive balances rise from the zero line in the
 * order received and negative ones hang down from it (CSD-010 REQ-6). The
 * outer corners of the highest and the lowest segment are rounded (REQ-13).
 */
function piecesOf(row) {
  let up = 0;
  let down = 0;
  const pieces = [];
  for (const segment of row.segments) {
    const value = Number(segment.balance);
    if (value === 0) continue;
    const piece = {
      key: keyOf(segment.account_id),
      cssClass: `bar-slot-${slotOf(segment.account_id)}`,
      label: `${nameOf(segment)}, ${row.year}, ${formatAmount(segment.balance)}`,
      rounding: false,
    };
    if (value > 0) {
      pieces.push({ ...piece, low: up, high: up + value, hangs: false });
      up += value;
    } else {
      pieces.push({ ...piece, low: down + value, high: down, hangs: true });
      down += value;
    }
  }
  const highest = pieces.filter((piece) => !piece.hangs).at(-1);
  const lowest = pieces.filter((piece) => piece.hangs).at(-1);
  if (highest) highest.rounding = 'top';
  if (lowest) lowest.rounding = 'bottom';
  return pieces;
}

/** The legend: a swatch and the name of every bank in the series, and the closing-balance marker (CSD-010 REQ-9). */
function buildLegend({ banks: list }) {
  banks = new Map(list.map((bank) => [bank.id, bank]));
  const legendList = document.getElementById('bank-insights-legend');
  const scopes = () => [legendList, document.getElementById('bank-insights-chart')];
  legendList.replaceChildren(
    ...list.map((bank) => legendItem(keyOf(bank.id), swatch(`slot-${bank.colour_slot}`), nameLabel(bank.name), scopes)),
    legendItem('marker', swatch('marker'), legendLabel('Closing balance'), scopes),
  );
}

const chart = createYearlyChart({
  id: 'bank-insights',
  command: 'get_bank_insights',
  ariaLabel: 'Bank accounts per year',
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_balance)],
  describe,
  tooltipContent,
  /** The closing-balance marker, drawn for each year that has entries. */
  decorate: (row, geometry) => (row.entry_count === 0 ? [] : levelMarker(Number(row.closing_balance), geometry)),
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadBankInsightsIfOpen = chart.reloadIfOpen;

export function initBankInsights() {
  chart.init();
  initTabs([
    { tab: 'bank-tab-overview', panel: 'bank-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (CSD-010 REQ-15).
    { tab: 'bank-tab-insights', panel: 'bank-insights', onOpen: chart.reload },
  ]);
}
