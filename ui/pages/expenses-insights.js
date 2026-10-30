// Expense insights: the yearly stacked chart on the Expenses page (CSD-014), one
// segment per category. Its frame (axis, bars, tooltip, loading) is shared with the other
// charts in shared/yearly-chart.js; this file supplies what is specific to expenses: the
// segments and the legend of the eight categories.
import { h } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { swatch, noSwatch, legendLabel, legendItem, tooltipRow, topToBottom } from '../shared/chart.js';
import { createYearlyChart, yearLabel } from '../shared/yearly-chart.js';
import { initTabs } from '../shared/tabs.js';

/** The categories as Rust returned them, in the fixed order, by name. */
let categories = new Map();

const keyOf = (category) => `category-${category}`;
const slotOf = (category) => categories.get(category)?.colour_slot ?? 0;

function describe(row) {
  if (row.expense_count === 0) return `${yearLabel(row)}: no expenses`;
  const parts = row.segments.map((segment) => `${segment.category} ${formatAmount(segment.amount)}`);
  return `${yearLabel(row)}: ${parts.join(', ')}, total ${formatAmount(row.total)}`;
}

/** The rows read like the bar: the highest segment first. */
function tooltipContent(row) {
  return [
    h('div', { class: 'tip-title' }, yearLabel(row)),
    row.is_partial ? h('div', { class: 'tip-note' }, 'Partial year: expenses so far') : null,
    row.expense_count === 0 ? h('div', { class: 'tip-note' }, 'No expenses') : null,
    h(
      'dl',
      {},
      ...topToBottom(row.segments, (segment) => segment.amount).flatMap((segment) =>
        tooltipRow(keyOf(segment.category), swatch(`slot-${slotOf(segment.category)}`), segment.category, formatAmount(segment.amount)),
      ),
      ...(row.expense_count === 0 ? [] : tooltipRow(null, noSwatch(), 'Total', formatAmount(row.total))),
    ),
  ].filter(Boolean);
}

/** The segments of a year, stacked upwards from the zero line in the order received (CSD-014 REQ-5). */
function piecesOf(row) {
  let level = 0;
  const pieces = row.segments.map((segment) => {
    const value = Number(segment.amount);
    const piece = {
      key: keyOf(segment.category),
      cssClass: `bar-slot-${slotOf(segment.category)}`,
      label: `${segment.category}, ${row.year}, ${formatAmount(segment.amount)}`,
      low: level,
      high: level + value,
      hangs: false,
      rounding: false,
    };
    level += value;
    return piece;
  });
  const highest = pieces.at(-1);
  if (highest) highest.rounding = 'top';
  return pieces;
}

/** The legend: a swatch and the name of every category, also those without an expense (CSD-014 REQ-7). */
function buildLegend({ categories: list }) {
  categories = new Map(list.map((category) => [category.name, category]));
  const legendList = document.getElementById('expenses-insights-legend');
  const scopes = () => [legendList, document.getElementById('expenses-insights-chart')];
  legendList.replaceChildren(
    ...list.map((category) => legendItem(keyOf(category.name), swatch(`slot-${category.colour_slot}`), legendLabel(category.name), scopes)),
  );
}

const chart = createYearlyChart({
  id: 'expenses-insights',
  command: 'get_expense_insights',
  ariaLabel: 'Expenses per year',
  onLoaded: buildLegend,
  piecesOf,
  describe,
  tooltipContent,
  decorate: () => [],
  labelLines: (row) => [{ text: yearLabel(row), className: 'chart-year' }],
});

/** Reloads the chart only while the Insights tab is showing (after a project is opened). */
export const reloadExpensesInsightsIfOpen = chart.reloadIfOpen;

export function initExpensesInsights() {
  chart.init();
  initTabs([
    { tab: 'expenses-tab-overview', panel: 'expenses-overview' },
    // Opening Insights recomputes it, so it never shows stale figures (CSD-014 REQ-13).
    { tab: 'expenses-tab-insights', panel: 'expenses-insights', onOpen: chart.reload },
  ]);
}
