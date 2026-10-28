// Debt insights: the yearly stacked chart on the Debts page (CSD-012), one
// segment per debt. Its frame (axis, bars, tooltip, loading) is shared with the
// other charts in shared/yearly-chart.js; this file supplies what is specific
// to debts: the segments, the legend of debts and the total-debt marker.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { svg, swatch, noSwatch, legendLabel, setHighlight } from '../shared/chart.js';
import { createYearlyChart, isNegative, yearLabel, axisLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The debts of the series by id, as Rust returned them (creation order). */
let debts = new Map();

const keyOf = (debtId) => `debt-${debtId}`;
const nameOf = (segment) => debts.get(segment.debt_id)?.name ?? '';
const slotOf = (debtId) => debts.get(debtId)?.colour_slot ?? 0;
const statementsText = (count) => `${count} statement${count === 1 ? '' : 's'}`;
/** The repaid amount as shown: a dash when Rust has none. */
const repaidText = (row) => (row.repaid === null ? '–' : formatAmount(row.repaid));

function describe(row) {
  if (row.statement_count === 0) return `${yearLabel(row)}: no statements`;
  const parts = row.segments.map((segment) => `${nameOf(segment)} ${formatAmount(segment.balance)}`);
  const repaid = row.repaid === null ? 'repaid none' : `repaid ${formatAmount(row.repaid)}${isNegative(row.repaid) ? ' (negative)' : ''}`;
  return `${yearLabel(row)}: ${parts.join(', ')}, total debt ${formatAmount(row.closing_balance)}, ${repaid}, ${statementsText(row.statement_count)}`;
}

function tooltipContent(row) {
  const line = (legend, swatchNode, label, value, cssClass = '') => [
    h('dt', { 'data-legend': legend }, swatchNode, legendLabel(label)),
    h('dd', { 'data-legend': legend, class: cssClass }, value),
  ];
  const rows =
    row.statement_count === 0
      ? line(null, noSwatch(), 'Statements', '0')
      : [
          ...row.segments.flatMap((segment) =>
            line(keyOf(segment.debt_id), swatch(`slot-${slotOf(segment.debt_id)}`), nameOf(segment), formatAmount(segment.balance)),
          ),
          ...line('marker', swatch('marker'), 'Total debt', formatAmount(row.closing_balance)),
          ...line(null, noSwatch(), 'Repaid', repaidText(row), row.repaid !== null && isNegative(row.repaid) ? 'negative' : ''),
          ...line(null, noSwatch(), 'Statements', String(row.statement_count)),
        ];
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: statements so far') : null,
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

/** The legend: a swatch and the name of every debt in the series, and the total-debt marker (CSD-012 REQ-9). */
function buildLegend({ debts: list }) {
  debts = new Map(list.map((debt) => [debt.id, debt]));
  const legendList = document.getElementById('debts-insights-legend');
  const scopes = () => [legendList, document.getElementById('debts-insights-chart')];
  const items = list.map((debt) => {
    const item = h(
      'li',
      { 'data-legend': keyOf(debt.id), tabindex: '0' },
      swatch(`slot-${debt.colour_slot}`),
      h('span', { class: 'legend-label legend-name', 'data-text': debt.name }, h('span', { class: 'legend-name-text' }, debt.name)),
    );
    // Hovering or focusing a debt highlights it in every year (CSD-012 REQ-11).
    for (const on of ['mouseenter', 'focus']) item.addEventListener(on, () => setHighlight(keyOf(debt.id), scopes()));
    for (const off of ['mouseleave', 'blur']) item.addEventListener(off, () => setHighlight(null, scopes()));
    return item;
  });
  items.push(h('li', { 'data-legend': 'marker' }, swatch('marker'), legendLabel('Total debt')));
  legendList.replaceChildren(...items);
}

const chart = createYearlyChart({
  id: 'debts-insights',
  command: 'get_debt_insights',
  ariaLabel: 'Debts per year',
  plotHeight: 260,
  minSlotWidth: 64,
  zeroLine: true,
  highlightAcrossYears: true,
  onLoaded: buildLegend,
  piecesOf,
  extraLevels: (row) => [Number(row.closing_balance)],
  describe,
  tooltipContent,
  /** The total-debt marker, drawn for each year that has statements. */
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
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

export const reloadDebtsInsights = chart.reload;
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
