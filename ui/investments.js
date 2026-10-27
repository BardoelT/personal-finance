// Investments: accounts with a monthly or yearly statement, each a balance, net deposits and a note (CSD-005).
import { invoke, errorText, h, iconButton, formatAmount, confirmAction, showToast } from './api.js';

const $ = (id) => document.getElementById(id);

let accounts = [];
let statementRows = [];
let onChanged = async () => {};

/** "No statements yet", "1 statement" or "3 statements". */
function statementCount(account) {
  if (account.statement_count === 0) return 'No statements yet';
  return `${account.statement_count} statement${account.statement_count === 1 ? '' : 's'}`;
}

function currentYear() {
  return String(new Date().getFullYear());
}

function currentMonth() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
}

/** The first line of a note, with an ellipsis when there is more (CSD-005 REQ-17). */
function noteSummary(note) {
  const [first, ...rest] = note.split('\n');
  return rest.length > 0 ? `${first} …` : first;
}

// ---- Rendering ----

function latestBalance(account) {
  if (account.latest_balance === null) return h('span', { class: 'account-balance' }, '—');
  return h(
    'span',
    { class: 'account-balance' },
    formatAmount(account.latest_balance),
    h('small', {}, account.latest_period),
  );
}

function renderAccounts(overview) {
  accounts = overview.accounts;
  const hasAccounts = accounts.length > 0;
  $('investments-empty').hidden = hasAccounts;
  $('investments-content').hidden = !hasAccounts;
  $('account-add').hidden = !hasAccounts;
  $('account-list').replaceChildren(
    ...accounts.map((account) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, account.name),
        h('span', { class: 'job-meta' }, `${account.statement_period} · ${statementCount(account)}`),
        latestBalance(account),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', `Edit ${account.name}`, () => openAccountDialog(account)),
          iconButton('delete', `Delete ${account.name}`, () => deleteAccount(account)),
        ),
      ),
    ),
  );
  $('account-total').hidden = !hasAccounts;
  $('account-total-value').textContent = overview.total === null ? '—' : formatAmount(overview.total);
}

/** Yearly totals over all accounts, as returned by Rust (CSD-005 REQ-26). */
function renderTotals(totals) {
  $('investment-totals-empty').hidden = totals.length > 0;
  $('investment-totals-table').hidden = totals.length === 0;
  $('investment-totals-rows').replaceChildren(
    ...totals.map((t) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, String(t.year)),
        h('td', { class: 'num' }, formatAmount(t.closing_balance)),
        h('td', { class: 'num' }, formatAmount(t.net_deposits)),
        h('td', { class: 'num' }, formatAmount(t.capital_growth)),
        h('td', { class: 'num' }, String(t.statement_count)),
      ),
    ),
  );
}

function renderStatements(statements) {
  statementRows = statements;
  $('statements-empty').hidden = statements.length > 0;
  $('statements-table').hidden = statements.length === 0;
  $('statements-rows').replaceChildren(
    ...statements.map((s) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, s.period),
        h('td', { class: 'wrap' }, s.account_name),
        h('td', { class: 'num' }, formatAmount(s.balance)),
        h('td', { class: 'num' }, formatAmount(s.net_deposits)),
        h('td', { class: 'num' }, formatAmount(s.capital_growth)),
        h('td', { class: 'note-cell' }, noteSummary(s.note)),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('edit', `Edit ${s.account_name} ${s.period}`, () => openStatementDialog(s)),
          iconButton('delete', `Delete ${s.account_name} ${s.period}`, () => deleteStatement(s)),
        ),
      ),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadInvestments() {
  const overview = await invoke('list_investment_accounts');
  renderAccounts(overview);
  if (overview.accounts.length === 0) {
    statementRows = [];
    return;
  }
  const [totals, statements] = await Promise.all([
    invoke('yearly_investment_totals'),
    invoke('list_investment_statements'),
  ]);
  renderTotals(totals);
  renderStatements(statements);
}

async function changed() {
  await reloadInvestments();
  await onChanged();
}

function showError(id, error) {
  const box = $(id);
  box.textContent = errorText(error);
  box.hidden = false;
}

// ---- Account dialog ----

let editingAccount = null;

function openAccountDialog(account) {
  editingAccount = account;
  const form = $('account-form');
  form.reset();
  $('account-error').hidden = true;
  $('account-dialog-title').textContent = account ? 'Edit account' : 'Add account';
  // The statement period is locked once the account has statements (CSD-005 REQ-4).
  const locked = Boolean(account && account.statement_count > 0);
  form.elements.statement_period.disabled = locked;
  $('account-period-hint').hidden = !locked;
  if (account) {
    form.elements.name.value = account.name;
    form.elements.statement_period.value = account.statement_period;
  }
  $('account-dialog').showModal();
  form.elements.name.focus();
}

async function submitAccount(event) {
  event.preventDefault();
  const form = $('account-form');
  const input = { name: form.elements.name.value };
  if (!form.elements.statement_period.disabled) {
    input.statement_period = form.elements.statement_period.value;
  }
  try {
    if (editingAccount) {
      await invoke('update_investment_account', { accountId: editingAccount.id, input });
    } else {
      await invoke('create_investment_account', { input });
    }
    $('account-dialog').close();
    await changed();
  } catch (error) {
    showError('account-error', error);
  }
}

async function deleteAccount(account) {
  const confirmed = await confirmAction({
    title: `Delete ${account.name}?`,
    message:
      account.statement_count === 0
        ? 'This account will be deleted.'
        : `This account and its ${statementCount(account)} will be deleted.`,
    confirmLabel: 'Delete account',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_investment_account', { accountId: account.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Statement dialog ----

let editingStatement = null;
/** Whether the form currently asks for a year, so the period resets only when it changes. */
let formIsYearly = false;

function fillAccountChoices(selectedId) {
  // The list is alphabetical, so the first option is the first account by name.
  const select = $('statement-form').elements.account_id;
  select.replaceChildren(...accounts.map((account) => h('option', { value: account.id }, account.name)));
  select.value = selectedId ?? accounts[0]?.id ?? '';
}

function chosenAccount() {
  return accounts.find((account) => account.id === $('statement-form').elements.account_id.value);
}

/** Asks for a month or a year, following the chosen account (CSD-005 REQ-14). */
function applyStatementPeriod({ resetPeriod }) {
  const form = $('statement-form');
  const yearly = chosenAccount()?.statement_period === 'Yearly';
  $('statement-period-label').textContent = yearly ? 'Year (YYYY)' : 'Month (YYYY-MM)';
  form.elements.period.placeholder = yearly ? String(new Date().getFullYear() - 1) : '2026-09';
  form.elements.period.maxLength = yearly ? 4 : 7;
  $('statement-copy').textContent = yearly ? 'Copy previous year' : 'Copy previous month';
  if (resetPeriod && yearly !== formIsYearly) {
    form.elements.period.value = yearly ? currentYear() : currentMonth();
  }
  formIsYearly = yearly;
}

// ---- Balance, net deposits and capital growth (CSD-005 REQ-24, REQ-25) ----

/** Which of balance and growth the user typed in last; the other one is calculated. */
let typedField = 'balance';
let calculationRun = 0;

function periodLooksComplete(period) {
  return (formIsYearly ? /^\d{4}$/ : /^\d{4}-\d{2}$/).test(period);
}

/**
 * Shows the note while the account has no statement before the chosen period:
 * it is taken to start at 0.00 (CSD-005 REQ-27). The note is only a hint;
 * Rust calculates every figure.
 */
function updateEarlierNote() {
  const form = $('statement-form');
  const period = form.elements.period.value.trim();
  const accountId = form.elements.account_id.value;
  const others = statementRows.filter((s) => s.account_id === accountId && s.period !== period);
  const hasEarlier = periodLooksComplete(period)
    ? others.some((s) => s.period < period)
    : others.length > 0;
  $('statement-growth-hint').hidden = hasEarlier;
}

function markCalculated(field) {
  $('balance-calculated').hidden = field !== 'balance';
  $('growth-calculated').hidden = field !== 'growth';
}

/** Asks Rust for the balance or growth that follows from the other one and the net deposits. */
async function recalculate() {
  const form = $('statement-form');
  const run = ++calculationRun;
  updateEarlierNote();
  const source = typedField;
  const target = source === 'balance' ? 'growth' : 'balance';
  const value = form.elements[source].value;
  $('statement-error').hidden = true;
  if (!periodLooksComplete(form.elements.period.value.trim()) || value.trim() === '') {
    form.elements[target].value = '';
    markCalculated(null);
    return;
  }
  try {
    const result = await invoke('calculate_investment_statement', {
      accountId: form.elements.account_id.value,
      period: form.elements.period.value.trim(),
      netDeposits: form.elements.net_deposits.value,
      input: source,
      value,
    });
    if (run !== calculationRun) return; // a newer calculation is under way
    if (target === 'growth') {
      form.elements.growth.value = result.capital_growth;
    } else {
      form.elements.balance.value = result.balance;
    }
    markCalculated(target);
  } catch (error) {
    if (run !== calculationRun) return;
    form.elements[target].value = '';
    markCalculated(null);
    showError('statement-error', error);
  }
}

/** The balance or the growth was typed in: it becomes the input and the other is calculated. */
function typedIn(field) {
  typedField = field;
  recalculate();
}

/** Opens the form to add a statement, or to edit `statement` when given. */
function openStatementDialog(statement = null) {
  editingStatement = statement;
  const form = $('statement-form');
  form.reset();
  $('statement-error').hidden = true;
  fillAccountChoices(statement?.account_id);
  form.elements.account_id.disabled = Boolean(statement);
  $('statement-copy').hidden = Boolean(statement);
  form.elements.period.readOnly = Boolean(statement);
  typedField = 'balance';
  calculationRun += 1; // drop any calculation still under way
  markCalculated(null);
  if (statement) {
    $('statement-dialog-title').textContent = `Edit statement ${statement.period}`;
    $('statement-period-label').textContent = statement.period.length === 4 ? 'Year (YYYY)' : 'Month (YYYY-MM)';
    form.elements.period.maxLength = statement.period.length;
    form.elements.period.value = statement.period;
    formIsYearly = statement.period.length === 4;
    form.elements.balance.value = statement.balance;
    form.elements.net_deposits.value = statement.net_deposits;
    form.elements.note.value = statement.note;
    recalculate();
  } else {
    $('statement-dialog-title').textContent = 'Add statement';
    formIsYearly = null; // force the period to its default for the first account
    form.elements.period.value = '';
    applyStatementPeriod({ resetPeriod: true });
    recalculate();
  }
  $('statement-dialog').showModal();
  (statement ? form.elements.balance : form.elements.period).focus();
}

async function submitStatement(event) {
  event.preventDefault();
  const form = $('statement-form');
  const args = {
    accountId: form.elements.account_id.value,
    period: form.elements.period.value,
    balance: form.elements.balance.value,
    netDeposits: form.elements.net_deposits.value,
    note: form.elements.note.value,
  };
  try {
    if (editingStatement) {
      await invoke('update_investment_statement', args);
    } else {
      await invoke('add_investment_statement', args);
    }
    $('statement-dialog').close();
    await changed();
  } catch (error) {
    showError('statement-error', error);
  }
}

/** Fills the balance from the latest statement of the account chosen in the form (CSD-005 REQ-13). */
async function copyPrevious() {
  const form = $('statement-form');
  try {
    const prefill = await invoke('copy_previous_investment_statement', {
      accountId: form.elements.account_id.value,
    });
    form.elements.balance.value = prefill.balance ?? '';
    form.elements.net_deposits.value = prefill.net_deposits;
    form.elements.note.value = '';
    typedIn('balance');
  } catch (error) {
    showError('statement-error', error);
  }
}

async function deleteStatement(statement) {
  const confirmed = await confirmAction({
    title: `Delete the statement of ${statement.period}?`,
    message: `The statement of ${statement.period} of ${statement.account_name} and its note will be deleted.`,
    confirmLabel: 'Delete statement',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_investment_statement', {
      accountId: statement.account_id,
      period: statement.period,
    });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initInvestments({ onChanged: callback }) {
  onChanged = callback;
  $('account-add').addEventListener('click', () => openAccountDialog(null));
  $('account-add-first').addEventListener('click', () => openAccountDialog(null));
  $('account-form').addEventListener('submit', submitAccount);
  $('statement-add').addEventListener('click', () => openStatementDialog());
  $('statement-copy').addEventListener('click', copyPrevious);
  $('statement-form').addEventListener('submit', submitStatement);
  const form = $('statement-form');
  form.elements.account_id.addEventListener('change', () => {
    applyStatementPeriod({ resetPeriod: true });
    recalculate();
  });
  form.elements.period.addEventListener('input', recalculate);
  form.elements.balance.addEventListener('input', () => typedIn('balance'));
  form.elements.growth.addEventListener('input', () => typedIn('growth'));
  form.elements.net_deposits.addEventListener('input', recalculate);
}
