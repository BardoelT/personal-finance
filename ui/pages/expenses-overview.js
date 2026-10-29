// Expenses: dated expenses with a category and a note, and yearly totals (CSD-013).
import { invoke, errorText } from '../shared/ipc.js';
import { h, iconButton, confirmAction, showToast, showError } from '../shared/dom.js';
import { formatAmount, formatDate } from '../shared/format.js';

const $ = (id) => document.getElementById(id);

let onChanged = async () => {};

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
        h('td', { class: 'num' }, formatAmount(t.largest)),
      ),
    ),
  );
}

function renderExpenses(list) {
  const none = list.expenses.length === 0;
  $('expenses-empty').hidden = !none;
  $('expenses-table').hidden = none;
  if (none) $('expenses-empty').textContent = 'No expenses yet. Add your first expense.';
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

/** Reloads everything from Rust. */
export async function reloadExpensesOverview() {
  renderExpenses(await invoke('list_expenses'));
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
}
