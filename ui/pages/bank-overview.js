// Bank accounts: accounts with a monthly or yearly entry, each a closing balance and a note (CSD-009).
import { invoke, errorText } from '../shared/ipc.js';
import { h, iconButton, confirmAction, showToast, showError } from '../shared/dom.js';
import { formatAmount, currentYear, currentMonth } from '../shared/format.js';

const $ = (id) => document.getElementById(id);

let accounts = [];
let onChanged = async () => {};

/** The first line of a note, with an ellipsis when there is more (CSD-009 REQ-20). */
function noteSummary(note) {
  const [first, ...rest] = note.split('\n');
  return rest.length > 0 ? `${first} …` : first;
}

// ---- Rendering ----

function latestBalance(account) {
  if (account.latest_balance === null) return h('span', { class: 'account-amount' }, '—');
  return h(
    'span',
    { class: 'account-amount' },
    formatAmount(account.latest_balance),
    h('small', {}, account.latest_period),
  );
}

function renderAccounts(overview) {
  accounts = overview.accounts;
  const hasAccounts = accounts.length > 0;
  $('bank-empty').hidden = hasAccounts;
  $('bank-content').hidden = !hasAccounts;
  $('bank-account-list').replaceChildren(
    ...accounts.map((account) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, account.name),
        h('span', { class: 'job-meta' }, account.entry_period),
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
  $('bank-account-total').hidden = !hasAccounts;
  $('bank-account-total-value').textContent = overview.total === null ? '—' : formatAmount(overview.total);
}

/** Yearly totals over all accounts, as returned by Rust (CSD-009 REQ-19). */
function renderTotals(totals) {
  $('bank-totals-empty').hidden = totals.length > 0;
  $('bank-totals-table').hidden = totals.length === 0;
  $('bank-totals-rows').replaceChildren(
    ...totals.map((t) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, String(t.year)),
        h('td', { class: 'num' }, formatAmount(t.closing_balance)),
      ),
    ),
  );
}

function renderEntries(entries) {
  $('bank-entries-empty').hidden = entries.length > 0;
  $('bank-entries-table').hidden = entries.length === 0;
  $('bank-entries-rows').replaceChildren(
    ...entries.map((e) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, e.period),
        h('td', { class: 'wrap' }, e.account_name),
        h('td', { class: 'num' }, formatAmount(e.balance)),
        h('td', { class: 'note-cell' }, noteSummary(e.note)),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('edit', `Edit ${e.account_name} ${e.period}`, () => openEntryDialog(e)),
          iconButton('delete', `Delete ${e.account_name} ${e.period}`, () => deleteEntry(e)),
        ),
      ),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadBankOverview() {
  const overview = await invoke('list_bank_accounts');
  renderAccounts(overview);
  if (overview.accounts.length === 0) return;
  const [totals, entries] = await Promise.all([invoke('yearly_bank_totals'), invoke('list_bank_entries')]);
  renderTotals(totals);
  renderEntries(entries);
}

async function changed() {
  await reloadBankOverview();
  await onChanged();
}

// ---- Account dialog ----

let editingAccount = null;

function openAccountDialog(account) {
  editingAccount = account;
  const form = $('bank-account-form');
  form.reset();
  $('bank-account-error').hidden = true;
  $('bank-account-dialog-title').textContent = account ? 'Edit account' : 'Add account';
  // The entry period is locked once the account has entries (CSD-009 REQ-5).
  const locked = Boolean(account && account.entry_count > 0);
  form.elements.entry_period.disabled = locked;
  $('bank-account-period-hint').hidden = !locked;
  if (account) {
    form.elements.name.value = account.name;
    form.elements.entry_period.value = account.entry_period;
  }
  $('bank-account-dialog').showModal();
  form.elements.name.focus();
}

async function submitAccount(event) {
  event.preventDefault();
  const form = $('bank-account-form');
  const input = { name: form.elements.name.value };
  if (!form.elements.entry_period.disabled) {
    input.entry_period = form.elements.entry_period.value;
  }
  try {
    if (editingAccount) {
      await invoke('update_bank_account', { accountId: editingAccount.id, input });
    } else {
      await invoke('create_bank_account', { input });
    }
    $('bank-account-dialog').close();
    await changed();
  } catch (error) {
    showError('bank-account-error', error);
  }
}

async function deleteAccount(account) {
  const confirmed = await confirmAction({
    title: `Delete ${account.name}?`,
    message: 'This account and all its entries will be deleted.',
    confirmLabel: 'Delete account',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_bank_account', { accountId: account.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Entry dialog ----

let editingEntry = null;
/** Whether the form currently asks for a year, so the period resets only when it changes. */
let formIsYearly = false;

function fillAccountChoices(selectedId) {
  // The list is alphabetical, so the first option is the first account by name.
  const select = $('bank-entry-form').elements.account_id;
  select.replaceChildren(...accounts.map((account) => h('option', { value: account.id }, account.name)));
  select.value = selectedId ?? accounts[0]?.id ?? '';
}

function chosenAccount() {
  return accounts.find((account) => account.id === $('bank-entry-form').elements.account_id.value);
}

/** Asks for a month or a year, following the chosen account (CSD-009 REQ-16). */
function applyEntryPeriod({ resetPeriod }) {
  const form = $('bank-entry-form');
  const yearly = chosenAccount()?.entry_period === 'Yearly';
  $('bank-entry-period-label').textContent = yearly ? 'Year' : 'Month';
  form.elements.period.placeholder = yearly ? String(new Date().getFullYear() - 1) : 'yyyy-mm';
  form.elements.period.maxLength = yearly ? 4 : 7;
  $('bank-entry-copy').textContent = yearly ? 'Copy previous year' : 'Copy previous month';
  if (resetPeriod && yearly !== formIsYearly) {
    form.elements.period.value = yearly ? currentYear() : currentMonth();
  }
  formIsYearly = yearly;
}

/** Opens the form to add an entry, or to edit `entry` when given. */
function openEntryDialog(entry = null) {
  editingEntry = entry;
  const form = $('bank-entry-form');
  form.reset();
  $('bank-entry-error').hidden = true;
  fillAccountChoices(entry?.account_id);
  form.elements.account_id.disabled = Boolean(entry);
  $('bank-entry-copy').hidden = Boolean(entry);
  if (entry) {
    // The period is editable too; the account is not (CSD-009 REQ-12).
    $('bank-entry-dialog-title').textContent = `Edit entry ${entry.period}`;
    $('bank-entry-period-label').textContent = entry.period.length === 4 ? 'Year' : 'Month';
    form.elements.period.maxLength = entry.period.length;
    form.elements.period.value = entry.period;
    formIsYearly = entry.period.length === 4;
    form.elements.balance.value = entry.balance;
    form.elements.note.value = entry.note;
  } else {
    $('bank-entry-dialog-title').textContent = 'Add entry';
    formIsYearly = null; // force the period to its default for the first account
    form.elements.period.value = '';
    applyEntryPeriod({ resetPeriod: true });
  }
  $('bank-entry-dialog').showModal();
  (entry ? form.elements.balance : form.elements.period).focus();
}

async function submitEntry(event) {
  event.preventDefault();
  const form = $('bank-entry-form');
  const args = {
    accountId: form.elements.account_id.value,
    period: form.elements.period.value,
    balance: form.elements.balance.value,
    note: form.elements.note.value,
  };
  try {
    if (editingEntry) {
      // The entry is found by its current period; the field holds the new one (CSD-009 REQ-12).
      await invoke('update_bank_entry', { ...args, period: editingEntry.period, newPeriod: args.period });
    } else {
      await invoke('add_bank_entry', args);
    }
    $('bank-entry-dialog').close();
    await changed();
  } catch (error) {
    showError('bank-entry-error', error);
  }
}

/** Fills the balance from the latest entry of the account chosen in the form (CSD-009 REQ-15). */
async function copyPrevious() {
  const form = $('bank-entry-form');
  try {
    const prefill = await invoke('copy_previous_bank_entry', { accountId: form.elements.account_id.value });
    form.elements.balance.value = prefill.balance ?? '';
    form.elements.note.value = '';
  } catch (error) {
    showError('bank-entry-error', error);
  }
}

async function deleteEntry(entry) {
  const confirmed = await confirmAction({
    title: `Delete the entry of ${entry.period}?`,
    message: `The entry of ${entry.period} of ${entry.account_name} and its note will be deleted.`,
    confirmLabel: 'Delete entry',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_bank_entry', { accountId: entry.account_id, period: entry.period });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initBankOverview({ onChanged: callback }) {
  onChanged = callback;
  $('bank-account-add').addEventListener('click', () => openAccountDialog(null));
  $('bank-account-add-first').addEventListener('click', () => openAccountDialog(null));
  $('bank-account-form').addEventListener('submit', submitAccount);
  $('bank-entry-add').addEventListener('click', () => openEntryDialog());
  $('bank-entry-copy').addEventListener('click', copyPrevious);
  $('bank-entry-form').addEventListener('submit', submitEntry);
  $('bank-entry-form').elements.account_id.addEventListener('change', () => applyEntryPeriod({ resetPeriod: true }));
}
