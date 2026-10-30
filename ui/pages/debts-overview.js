// Debts: debts with a monthly or yearly statement, each a remaining balance and a note (CSD-011).
import { invoke, errorText } from '../shared/ipc.js';
import { h, iconButton, confirmAction, showToast, showError } from '../shared/dom.js';
import { formatAmount, currentYear, currentMonth } from '../shared/format.js';

const $ = (id) => document.getElementById(id);

let debts = [];
let onChanged = async () => {};

/** The first line of a note, with an ellipsis when there is more (CSD-011 REQ-20). */
function noteSummary(note) {
  const [first, ...rest] = note.split('\n');
  return rest.length > 0 ? `${first} …` : first;
}

/** An amount that Rust may leave out (a change or a repaid amount of none): a dash then. */
function optionalAmount(value) {
  return value === null ? '—' : formatAmount(value);
}

// ---- Rendering ----

function latestBalance(debt) {
  if (debt.latest_balance === null) return h('span', { class: 'account-amount' }, '—');
  return h('span', { class: 'account-amount' }, formatAmount(debt.latest_balance), h('small', {}, debt.latest_period));
}

function renderDebts(overview) {
  debts = overview.debts;
  const hasDebts = debts.length > 0;
  $('debts-empty').hidden = hasDebts;
  $('debts-content').hidden = !hasDebts;
  $('debt-list').replaceChildren(
    ...debts.map((debt) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, debt.name),
        h('span', { class: 'job-meta' }, debt.statement_period),
        latestBalance(debt),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', `Edit ${debt.name}`, () => openDebtDialog(debt)),
          iconButton('delete', `Delete ${debt.name}`, () => deleteDebt(debt)),
        ),
      ),
    ),
  );
  $('debt-total').hidden = !hasDebts;
  $('debt-total-value').textContent = overview.total === null ? '—' : formatAmount(overview.total);
}

/** Yearly totals over all debts, as returned by Rust (CSD-011 REQ-19). */
function renderTotals(totals) {
  $('debt-totals-empty').hidden = totals.length > 0;
  $('debt-totals-table').hidden = totals.length === 0;
  $('debt-totals-rows').replaceChildren(
    ...totals.map((t) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, String(t.year)),
        h('td', { class: 'num' }, formatAmount(t.closing_balance)),
        h('td', { class: 'num' }, optionalAmount(t.repaid)),
      ),
    ),
  );
}

function renderStatements(statements) {
  $('debt-statements-empty').hidden = statements.length > 0;
  $('debt-statements-table').hidden = statements.length === 0;
  $('debt-statements-rows').replaceChildren(
    ...statements.map((s) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, s.period),
        h('td', { class: 'wrap' }, s.debt_name),
        h('td', { class: 'num' }, formatAmount(s.balance)),
        h('td', { class: 'num' }, optionalAmount(s.change)),
        h('td', { class: 'note-cell' }, noteSummary(s.note)),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('edit', `Edit ${s.debt_name} ${s.period}`, () => openStatementDialog(s)),
          iconButton('delete', `Delete ${s.debt_name} ${s.period}`, () => deleteStatement(s)),
        ),
      ),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadDebtsOverview() {
  const overview = await invoke('list_debts');
  renderDebts(overview);
  if (overview.debts.length === 0) return;
  const [totals, statements] = await Promise.all([invoke('yearly_debt_totals'), invoke('list_debt_statements')]);
  renderTotals(totals);
  renderStatements(statements);
}

async function changed() {
  await reloadDebtsOverview();
  await onChanged();
}

// ---- Debt dialog ----

let editingDebt = null;

function openDebtDialog(debt) {
  editingDebt = debt;
  const form = $('debt-form');
  form.reset();
  $('debt-error').hidden = true;
  $('debt-dialog-title').textContent = debt ? 'Edit debt' : 'Add debt';
  // The statement period is locked once the debt has statements (CSD-011 REQ-5).
  const locked = Boolean(debt && debt.statement_count > 0);
  form.elements.statement_period.disabled = locked;
  $('debt-period-hint').hidden = !locked;
  if (debt) {
    form.elements.name.value = debt.name;
    form.elements.statement_period.value = debt.statement_period;
  }
  $('debt-dialog').showModal();
  form.elements.name.focus();
}

async function submitDebt(event) {
  event.preventDefault();
  const form = $('debt-form');
  const input = { name: form.elements.name.value };
  if (!form.elements.statement_period.disabled) {
    input.statement_period = form.elements.statement_period.value;
  }
  try {
    if (editingDebt) {
      await invoke('update_debt', { debtId: editingDebt.id, input });
    } else {
      await invoke('create_debt', { input });
    }
    $('debt-dialog').close();
    await changed();
  } catch (error) {
    showError('debt-error', error);
  }
}

async function deleteDebt(debt) {
  const confirmed = await confirmAction({
    title: `Delete ${debt.name}?`,
    message: 'This debt and all its statements will be deleted.',
    confirmLabel: 'Delete debt',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_debt', { debtId: debt.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Statement dialog ----

let editingStatement = null;
/** Whether the form currently asks for a year, so the period resets only when it changes. */
let formIsYearly = false;

function fillDebtChoices(selectedId) {
  // The list is alphabetical, so the first option is the first debt by name.
  const select = $('debt-statement-form').elements.debt_id;
  select.replaceChildren(...debts.map((debt) => h('option', { value: debt.id }, debt.name)));
  select.value = selectedId ?? debts[0]?.id ?? '';
}

function chosenDebt() {
  return debts.find((debt) => debt.id === $('debt-statement-form').elements.debt_id.value);
}

/** Asks for a month or a year, following the chosen debt (CSD-011 REQ-16). */
function applyStatementPeriod({ resetPeriod }) {
  const form = $('debt-statement-form');
  const yearly = chosenDebt()?.statement_period === 'Yearly';
  $('debt-statement-period-label').textContent = yearly ? 'Year' : 'Month';
  form.elements.period.placeholder = yearly ? String(new Date().getFullYear() - 1) : 'yyyy-mm';
  form.elements.period.maxLength = yearly ? 4 : 7;
  $('debt-statement-copy').textContent = yearly ? 'Copy previous year' : 'Copy previous month';
  if (resetPeriod && yearly !== formIsYearly) {
    form.elements.period.value = yearly ? currentYear() : currentMonth();
  }
  formIsYearly = yearly;
}

/** Opens the form to add a statement, or to edit `statement` when given. */
function openStatementDialog(statement = null) {
  editingStatement = statement;
  const form = $('debt-statement-form');
  form.reset();
  $('debt-statement-error').hidden = true;
  fillDebtChoices(statement?.debt_id);
  form.elements.debt_id.disabled = Boolean(statement);
  $('debt-statement-copy').hidden = Boolean(statement);
  if (statement) {
    // The period is editable too; the debt is not (CSD-011 REQ-12).
    $('debt-statement-dialog-title').textContent = `Edit statement ${statement.period}`;
    $('debt-statement-period-label').textContent = statement.period.length === 4 ? 'Year' : 'Month';
    form.elements.period.maxLength = statement.period.length;
    form.elements.period.value = statement.period;
    formIsYearly = statement.period.length === 4;
    form.elements.balance.value = statement.balance;
    form.elements.note.value = statement.note;
  } else {
    $('debt-statement-dialog-title').textContent = 'Add statement';
    formIsYearly = null; // force the period to its default for the first debt
    form.elements.period.value = '';
    applyStatementPeriod({ resetPeriod: true });
  }
  $('debt-statement-dialog').showModal();
  (statement ? form.elements.balance : form.elements.period).focus();
}

async function submitStatement(event) {
  event.preventDefault();
  const form = $('debt-statement-form');
  const args = {
    debtId: form.elements.debt_id.value,
    period: form.elements.period.value,
    balance: form.elements.balance.value,
    note: form.elements.note.value,
  };
  try {
    if (editingStatement) {
      // The statement is found by its current period; the field holds the new one (CSD-011 REQ-12).
      await invoke('update_debt_statement', { ...args, period: editingStatement.period, newPeriod: args.period });
    } else {
      await invoke('add_debt_statement', args);
    }
    $('debt-statement-dialog').close();
    await changed();
  } catch (error) {
    showError('debt-statement-error', error);
  }
}

/** Fills the balance from the latest statement of the debt chosen in the form (CSD-011 REQ-15). */
async function copyPrevious() {
  const form = $('debt-statement-form');
  try {
    const prefill = await invoke('copy_previous_debt_statement', { debtId: form.elements.debt_id.value });
    form.elements.balance.value = prefill.balance ?? '';
    form.elements.note.value = '';
  } catch (error) {
    showError('debt-statement-error', error);
  }
}

async function deleteStatement(statement) {
  const confirmed = await confirmAction({
    title: `Delete the statement of ${statement.period}?`,
    message: `The statement of ${statement.period} of ${statement.debt_name} and its note will be deleted.`,
    confirmLabel: 'Delete statement',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_debt_statement', { debtId: statement.debt_id, period: statement.period });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initDebtsOverview({ onChanged: callback }) {
  onChanged = callback;
  $('debt-add').addEventListener('click', () => openDebtDialog(null));
  $('debt-add-first').addEventListener('click', () => openDebtDialog(null));
  $('debt-form').addEventListener('submit', submitDebt);
  $('debt-statement-add').addEventListener('click', () => openStatementDialog());
  $('debt-statement-copy').addEventListener('click', copyPrevious);
  $('debt-statement-form').addEventListener('submit', submitStatement);
  $('debt-statement-form').elements.debt_id.addEventListener('change', () => applyStatementPeriod({ resetPeriod: true }));
}
