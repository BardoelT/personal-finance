// Ratios: the table on the Ratios page, with the years as columns (CSD-018). The
// figures come from Rust; this file only lays them out.
import { invoke, errorText } from '../shared/ipc.js';
import { h, showToast } from '../shared/dom.js';
import { formatAmount } from '../shared/format.js';
import { percentLabel, isNegative, yearLabel } from '../shared/yearly-chart.js';

const $ = (id) => document.getElementById(id);

/** An amount, or a dash when Rust has none (CSD-018 REQ-10). */
const amountCell = (value) => (value === null ? '–' : formatAmount(value));
/** A percentage with its sign, or a dash. */
const percentCell = (value) => percentLabel(value);

/** The rows of the table under their sub-headings: label, field and how to show it (CSD-018 REQ-3). */
const GROUPS = [
  ['Wealth', [
    ['Total income (gross)', 'total_income', amountCell],
    ['Wealth accumulation from cash', 'wealth_accumulation', amountCell],
    ['Percentage of income to wealth', 'wealth_percent', percentCell],
  ]],
  ['Growth', [
    ['Growth of investments', 'investment_growth_percent', percentCell],
    ['Growth of total wealth', 'wealth_growth_percent', percentCell],
  ]],
  ['Free income', [
    ['Disposable income', 'disposable_income', amountCell],
    ['Percentage of disposable income', 'disposable_percent', percentCell],
  ]],
];

function renderTable(years) {
  $('ratios-head').replaceChildren(
    h('tr', {}, h('th', { scope: 'col' }, ''), ...years.map((row) => h('th', { scope: 'col', class: 'num' }, yearLabel(row)))),
  );
  const rows = GROUPS.flatMap(([title, items]) => [
    h('tr', { class: 'group-row' }, h('th', { scope: 'colgroup', colspan: String(years.length + 1) }, title)),
    ...items.map(([label, field, show]) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, label),
        ...years.map((row) => {
          const value = row[field];
          const negative = value !== null && isNegative(value);
          return h('td', { class: `num${negative ? ' negative' : ''}` }, show(value));
        }),
      ),
    ),
  ]);
  $('ratios-rows').replaceChildren(...rows);
}

/** Loads the figures from Rust and shows them, or the note when there is nothing yet. */
export async function reloadRatios() {
  const { years } = await invoke('get_ratios');
  $('ratios-empty').hidden = years.length > 0;
  $('ratios-table').hidden = years.length === 0;
  if (years.length > 0) renderTable(years);
}

/** Reloads the table only while the page is showing. */
export async function reloadRatiosIfOpen() {
  if ($('ratios').classList.contains('active')) await reloadRatios();
}

export function initRatios() {
  // Opening the page recomputes it, so it never shows stale figures (CSD-018 REQ-12).
  document.querySelector('[data-section="ratios"]').addEventListener('click', () => reloadRatios().catch((error) => showToast(errorText(error))));
}
