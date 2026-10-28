// Bank account insights: the yearly stacked chart on the Bank accounts page
// (CSD-010), one segment per bank. Its frame (axis, bars, tooltip, loading) is
// shared with the other charts in shared/yearly-chart.js; this file supplies
// what is specific to banks: the segments, the legend of banks and the
// closing-balance marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, swatch, noSwatch, legendLabel, setHighlight } from '../shared/chart.js';
import { createYearlyChart, isNegative, yearLabel, axisLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The banks of the series by id, as Rust returned them (creation order). */
let banks = new Map();

const keyOf = (accountId) => `bank-${accountId}`;
const nameOf = (segment) => banks.get(segment.account_id)?.name ?? '';
const slotOf = (accountId) => banks.get(accountId)?.colour_slot ?? 0;
const entriesText = (count) => `${count} ${count === 1 ? 'entry' : 'entries'}`;

function describe(row) {
  if (row.entry_count === 0) return `${yearLabel(row)}: no entries`;
  const parts = row.segments.map(
    (segment) => `${nameOf(segment)} ${formatAmount(segment.balance)}${isNegative(segment.balance) ? ' (negative)' : ''}`,
  );
  return `${yearLabel(row)}: ${parts.join(', ')}, closing balance ${formatAmount(row.closing_balance)}, ${entriesText(row.entry_count)}`;
}

function tooltipContent(row) {
  const line = (legend, swatchNode, label, value, cssClass = '') => [
    h('dt', { 'data-legend': legend }, swatchNode, legendLabel(label)),
    h('dd', { 'data-legend': legend, class: cssClass }, value),
  ];
  const rows =
    row.entry_count === 0
      ? line(null, noSwatch(), 'Entries', '0')
      : [
          ...row.segments.flatMap((segment) =>
            line(
              keyOf(segment.account_id),
              swatch(`slot-${slotOf(segment.account_id)}`),
              nameOf(segment),
              formatAmount(segment.balance),
              isNegative(segment.balance) ? 'negative' : '',
            ),
          ),
          ...line('marker', swatch('marker'), 'Closing balance', formatAmount(row.closing_balance), isNegative(row.closing_balance) ? 'negative' : ''),
          ...line(null, noSwatch(), 'Entries', String(row.entry_count)),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: entries so far') : null,
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
  const items = list.map((bank) => {
    const item = h(
      'li',
      { 'data-legend': keyOf(bank.id), tabindex: '0' },
      swatch(`slot-${bank.colour_slot}`),
      h('span', { class: 'legend-label legend-name', 'data-text': bank.name }, h('span', { class: 'legend-name-text' }, bank.name)),
    );
    // Hovering or focusing a bank highlights it in every year (CSD-010 REQ-11).
    for (const on of ['mouseenter', 'focus']) item.addEventListener(on, () => setHighlight(keyOf(bank.id), scopes()));
    for (const off of ['mouseleave', 'blur']) item.addEventListener(off, () => setHighlight(null, scopes()));
    return item;
  });
  items.push(h('li', { 'data-legend': 'marker' }, swatch('marker'), legendLabel('Closing balance')));
  legendList.replaceChildren(...items);
}

const chart = createYearlyChart({
  id: 'bank-insights',
  command: 'get_bank_insights',
  ariaLabel: 'Bank accounts per year',
  plotHeight: 260,
  minSlotWidth: 64,
  zeroLine: true,
  highlightAcrossYears: true,
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_balance)],
  describe,
  tooltipContent,
  /** The closing-balance marker, drawn for each year that has entries. */
  decorate(row, { centre, barWidth, yOf }) {
    if (row.entry_count === 0) return [];
    const y = yOf(Number(row.closing_balance));
    const half = barWidth * 0.7;
    return [
      svg('line', { class: 'chart-marker-hit', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
      svg('line', { class: 'chart-marker', 'data-legend': 'marker', x1: centre - half, x2: centre + half, y1: y, y2: y }),
      svg('text', { class: 'chart-marker-label', x: centre, y: y - 7 }, axisLabel(Number(row.closing_balance))),
    ];
  },
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

export const reloadBankInsights = chart.reload;
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
