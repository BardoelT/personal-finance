// Dashboard: income this year to date and last month, over all jobs (CSD-004).
// The figures come from Rust; this file only displays them.
import { invoke, h, formatAmount } from './api.js';

const $ = (id) => document.getElementById(id);

function cardsFor(totals) {
  const card = (label, value, primary = false) =>
    h(
      'div',
      { class: primary ? 'card primary' : 'card' },
      h('div', { class: 'card-label' }, label),
      h('div', { class: 'card-value' }, value),
    );
  return [
    card('Total gross', formatAmount(totals.total_gross), true),
    card('Gross salary', formatAmount(totals.gross_salary)),
    card('Holiday pay', formatAmount(totals.holiday_pay)),
    card('Bonus', formatAmount(totals.bonus)),
    card('Paychecks', String(totals.paycheck_count)),
  ];
}

/** Shows one block: its cards, or a note when the period has no paychecks. */
function renderBlock(prefix, title, totals, emptyText) {
  $(`dashboard-${prefix}-title`).textContent = title;
  $(`dashboard-${prefix}-empty`).textContent = emptyText;
  $(`dashboard-${prefix}-empty`).hidden = Boolean(totals);
  $(`dashboard-${prefix}-cards`).replaceChildren(...(totals ? cardsFor(totals) : []));
}

/** Loads the figures from Rust and shows them. */
export async function reloadDashboard() {
  const d = await invoke('get_dashboard');
  $('dashboard-empty').hidden = d.has_jobs;
  $('dashboard-content').hidden = !d.has_jobs;
  if (!d.has_jobs) return;

  renderBlock(
    'year',
    `This year · ${d.year} · ${d.currency}`,
    d.year_to_date,
    `No paychecks yet in ${d.year}.`,
  );
  const month = d.last_month ?? '';
  renderBlock(
    'month',
    `Last month · ${month} · ${d.currency}`,
    d.last_month_totals,
    `No paycheck for ${month}.`,
  );
}
