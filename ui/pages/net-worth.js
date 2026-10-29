// Net worth: the yearly stacked chart on the Net worth page (CSD-017). Its frame
// (axis, bars, tooltip, loading) is shared with the other charts in
// shared/yearly-chart.js; this file supplies what is specific to it: the four
// series (assets, investments and cash above the zero line, debts below it) and
// the net worth marker.
import { h, showToast } from '../shared/dom.js';
import { errorText } from '../shared/ipc.js';
import { formatAmount } from '../shared/format.js';
import { swatch, legendLabel, legendItem, tooltipRow } from '../shared/chart.js';
import { createYearlyChart, levelMarker, isNegative, yearLabel } from '../shared/yearly-chart.js';

/** The series from the bottom of the bar up: the Rust field, the legend key, the colour class and the label. */
const SERIES = [
  ['assets', 'assets', 'nw-assets', 'Assets'],
  ['investments', 'investments', 'nw-investments', 'Investments'],
  ['cash', 'cash', 'nw-cash', 'Cash'],
  ['debts', 'debts', 'nw-debts', 'Debts'],
];

/** The signed value of a series: debts count below the zero line (CSD-017 REQ-6). */
const signed = (row, field) => (field === 'debts' ? -Number(row.debts) : Number(row[field]));

/** A debt is shown with a minus sign (CSD-017 REQ-10); the other amounts as they are. */
const shown = (row, field) => (field === 'debts' && Number(row.debts) !== 0 ? `−${formatAmount(row.debts)}` : formatAmount(row[field]));

function describe(row) {
  if (!row.has_records) return `${yearLabel(row)}: no records`;
  const parts = SERIES.map(([field, , , label]) => `${label} ${shown(row, field)}`);
  return `${yearLabel(row)}: ${parts.join(', ')}, net worth ${formatAmount(row.net_worth)}`;
}

/**
 * The rows read like the bar, top to bottom: cash, investments, assets, then debts below
 * the zero line (CSD-017 REQ-10), and the net worth.
 */
function tooltipContent(row) {
  const rows = row.has_records
    ? [
        ...[...SERIES].reverse().flatMap(([field, key, cssClass, label]) =>
          tooltipRow(key, swatch(cssClass), label, shown(row, field), field === 'cash' && isNegative(row.cash) ? 'negative' : ''),
        ),
        ...tooltipRow('marker', swatch('marker'), 'Net worth', formatAmount(row.net_worth), isNegative(row.net_worth) ? 'negative' : ''),
      ]
    : [];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: figures so far') : null,
    row.has_records ? null : h('div', { class: 'tip-note' }, 'No records'),
    h('dl', {}, ...rows),
  ].filter(Boolean);
}

/**
 * The segments of a year: positive values rise from the zero line in the order of
 * `SERIES`, negative ones (debts, and cash of an overdrawn bank) hang down from it.
 * The outer corners of the highest and the lowest segment are rounded.
 */
function piecesOf(row) {
  let up = 0;
  let down = 0;
  const pieces = [];
  for (const [field, key, cssClass, label] of SERIES) {
    const value = signed(row, field);
    if (value === 0) continue;
    const piece = { key, cssClass: `bar-${cssClass}`, label: `${label}, ${row.year}, ${shown(row, field)}`, rounding: false };
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

/** The legend: the four series and the net worth marker; hovering one highlights it in every year. */
function buildLegend() {
  const legendList = document.getElementById('net-worth-insights-legend');
  const scopes = () => [legendList, document.getElementById('net-worth-insights-chart')];
  legendList.replaceChildren(
    ...SERIES.map(([, key, cssClass, label]) => legendItem(key, swatch(cssClass), legendLabel(label), scopes)),
    legendItem('marker', swatch('marker'), legendLabel('Net worth'), scopes),
  );
}

const chart = createYearlyChart({
  id: 'net-worth-insights',
  command: 'get_net_worth_insights',
  ariaLabel: 'Net worth per year',
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.net_worth)],
  describe,
  tooltipContent,
  /** The net worth marker, drawn for each year that has records. */
  decorate: (row, geometry) => (row.has_records ? levelMarker(Number(row.net_worth), geometry) : []),
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

export const reloadNetWorth = chart.reload;

/** Reloads the chart only while the page is showing, so a hidden chart is not drawn at zero width. */
export const reloadNetWorthIfOpen = async () => {
  if (document.getElementById('net-worth').classList.contains('active')) await chart.reload();
};

export function initNetWorth() {
  chart.init();
  // Opening the page recomputes it, so it never shows stale figures (CSD-017 REQ-13).
  document.querySelector('[data-section="net-worth"]').addEventListener('click', () => chart.reload().catch((error) => showToast(errorText(error))));
}
