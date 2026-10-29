// Major expenses: dated expenses with a category and a note, a year and month filter and yearly totals (CSD-013).
import { invoke, errorText } from '../shared/ipc.js';
import { h, iconButton, confirmAction, showToast, showError } from '../shared/dom.js';
import { formatAmount, formatDate } from '../shared/format.js';

const $ = (id) => document.getElementById(id);

const MONTHS = ['January', 'February', 'March', 'April', 'May', 'June', 'July', 'August', 'September', 'October', 'November', 'December'];

let onChanged = async () => {};

/**
 * The chosen filter. `year` is `null` until the first load chooses the latest year with an expense,
 * 'all' for all years, or a year; `month` is `null` for all months, or 1 to 12 (CSD-013 REQ-11).
 */
const filter = { year: null, month: null };

/** The first line of a note, with an ellipsis when there is more (CSD-013 REQ-13). */
function noteSummary(note) {
  const [first, ...rest] = note.split('\n');
  return rest.length > 0 ? `${first} …` : first;
}

// ---- Rendering ----

function renderTotals(totals) {
  $('expense-totals-empty').hidden = totals.length > 0;
  $('expense-totals-table').hidden = totals.length === 0;
  $('expense-totals-rows').replaceChildren(
    ...totals.map((t) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, String(t.year)),
        h('td', { class: 'num' }, formatAmount(t.total)),
        h('td', { class: 'num' }, String(t.expense_count)),
        h('td', { class: 'num' }, formatAmount(t.largest)),
      ),
    ),
  );
}

function renderFilters(years) {
  const yearSelect = $('expense-filter-year');
  yearSelect.replaceChildren(
    h('option', { value: 'all' }, 'All years'),
    ...years.map((year) => h('option', { value: String(year) }, String(year))),
  );
  yearSelect.value = String(filter.year);
  const monthSelect = $('expense-filter-month');
  monthSelect.replaceChildren(
    h('option', { value: 'all' }, 'All months'),
    ...MONTHS.map((name, index) => h('option', { value: String(index + 1) }, name)),
  );
  monthSelect.value = filter.month === null ? 'all' : String(filter.month);
  // A month is only chosen within a year (CSD-013 REQ-11).
  monthSelect.disabled = filter.year === 'all';
}

function renderExpenses(list) {
  const none = list.expenses.length === 0;
  $('expenses-empty').hidden = !none;
  $('expenses-table').hidden = none;
  if (none) {
    $('expenses-empty').textContent =
      list.years.length === 0 ? 'No expenses yet. Add your first expense.' : 'No expenses in this period';
  }
  $('expenses-total').textContent = formatAmount(list.total);
  $('expenses-rows').replaceChildren(
    ...list.expenses.map((expense) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, formatDate(expense.date)),
        h('td', { class: 'wrap' }, expense.name),
        h('td', {}, expense.category),
        h('td', { class: 'num' }, formatAmount(expense.amount)),
        h('td', { class: 'note-cell' }, noteSummary(expense.note)),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('edit', `Edit ${expense.name}`, () => openExpenseDialog(expense)),
          iconButton('delete', `Delete ${expense.name}`, () => deleteExpense(expense)),
        ),
      ),
    ),
  );
}

const listArgs = () => ({
  year: filter.year === null || filter.year === 'all' ? null : filter.year,
  month: filter.year === null || filter.year === 'all' ? null : filter.month,
});

/**
 * Reloads everything from Rust. The chosen filter is kept while expenses change; it falls back to
 * the latest year with an expense when its year has none left, and `resetFilter` starts over (a new project).
 */
export async function reloadExpensesOverview({ resetFilter = false } = {}) {
  if (resetFilter) {
    filter.year = null;
    filter.month = null;
  }
  let list = await invoke('list_expenses', listArgs());
  const valid = filter.year === 'all' || list.years.includes(filter.year);
  if (filter.year === null || !valid) {
    filter.year = list.years[0] ?? 'all';
    filter.month = null;
    list = await invoke('list_expenses', listArgs());
  }
  renderFilters(list.years);
  renderExpenses(list);
  renderTotals(await invoke('yearly_expense_totals'));
}

async function changed() {
  await reloadExpensesOverview();
  await onChanged();
}

// ---- Expense dialog ----

let editingExpense = null;

function openExpenseDialog(expense = null) {
  editingExpense = expense;
  const form = $('expense-form');
  form.reset();
  $('expense-error').hidden = true;
  $('expense-dialog-title').textContent = expense ? 'Edit expense' : 'Add expense';
  $('expense-copy').hidden = Boolean(expense);
  if (expense) {
    form.elements.name.value = expense.name;
    form.elements.amount.value = expense.amount;
    form.elements.date.value = formatDate(expense.date);
    form.elements.category.value = expense.category;
    form.elements.note.value = expense.note;
  }
  $('expense-dialog').showModal();
  form.elements.name.focus();
}

async function submitExpense(event) {
  event.preventDefault();
  const form = $('expense-form');
  const input = {
    name: form.elements.name.value,
    amount: form.elements.amount.value,
    date: form.elements.date.value,
    category: form.elements.category.value,
    note: form.elements.note.value,
  };
  try {
    if (editingExpense) {
      await invoke('update_expense', { expenseId: editingExpense.id, input });
    } else {
      await invoke('add_expense', { input });
    }
    $('expense-dialog').close();
    await changed();
  } catch (error) {
    showError('expense-error', error);
  }
}

/** Fills the name, amount and category from the previous expense; date and note stay empty (CSD-013 REQ-9). */
async function copyPrevious() {
  const form = $('expense-form');
  try {
    const prefill = await invoke('copy_previous_expense');
    if (prefill === null) return;
    form.elements.name.value = prefill.name;
    form.elements.amount.value = prefill.amount;
    form.elements.category.value = prefill.category;
    form.elements.date.value = '';
    form.elements.note.value = '';
  } catch (error) {
    showError('expense-error', error);
  }
}

async function deleteExpense(expense) {
  const confirmed = await confirmAction({
    title: `Delete ${expense.name}?`,
    message: `The expense of ${formatDate(expense.date)} and its note will be deleted.`,
    confirmLabel: 'Delete expense',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_expense', { expenseId: expense.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initExpensesOverview({ onChanged: callback }) {
  onChanged = callback;
  $('expense-add').addEventListener('click', () => openExpenseDialog(null));
  $('expense-form').addEventListener('submit', submitExpense);
  $('expense-copy').addEventListener('click', copyPrevious);
  $('expense-filter-year').addEventListener('change', (event) => {
    filter.year = event.target.value === 'all' ? 'all' : Number(event.target.value);
    filter.month = null;
    reloadExpensesOverview().catch((error) => showToast(errorText(error)));
  });
  $('expense-filter-month').addEventListener('change', (event) => {
    filter.month = event.target.value === 'all' ? null : Number(event.target.value);
    reloadExpensesOverview().catch((error) => showToast(errorText(error)));
  });
}
