// Houses: things the user owns with a monthly or yearly valuation, each a value and a note (CSD-015),
// bought once and possibly sold (CSD-019).
import { invoke, errorText } from '../shared/ipc.js';
import { h, iconButton, confirmAction, showToast, showError } from '../shared/dom.js';
import { formatAmount, formatDate, currentYear, currentMonth } from '../shared/format.js';

const $ = (id) => document.getElementById(id);

let houses = [];
let debts = [];
let onChanged = async () => {};

/** The first line of a note, with an ellipsis when there is more (CSD-015 REQ-20). */
function noteSummary(note) {
  const [first, ...rest] = note.split('\n');
  return rest.length > 0 ? `${first} …` : first;
}

/** An amount that Rust may leave out (a change of none): a dash then. */
function optionalAmount(value) {
  return value === null ? '—' : formatAmount(value);
}

// ---- Rendering ----

function latestValue(house) {
  if (house.latest_value === null) return h('span', { class: 'account-amount' }, '—');
  return h('span', { class: 'account-amount' }, formatAmount(house.latest_value), h('small', {}, house.latest_period));
}

/** The purchase and the sale of a house as one line (CSD-019 REQ-9). Display only. */
function holdingText(house) {
  if (house.purchase_date === null) return `${house.valuation_period} · Held`;
  const bought = `Bought ${formatDate(house.purchase_date)} for ${formatAmount(house.purchase_amount)}`;
  const state = house.held
    ? 'Held'
    : `Sold on ${formatDate(house.sell_date)} for ${formatAmount(house.sell_amount)}, result ${formatAmount(house.result)}`;
  return `${house.valuation_period} · ${bought} · ${state}`;
}

/** The debts linked to a house with their latest balance and, once settled, the outcome (CSD-020 REQ-13). */
function linkedDebtList(house) {
  if (house.linked_debts.length === 0) return '';
  return h(
    'ul',
    { class: 'linked-debts', 'aria-label': `Debts of ${house.name}` },
    ...house.linked_debts.map((debt) =>
      h(
        'li',
        {},
        h('span', {}, `${debt.debt_name} ${debt.latest_balance === null ? '—' : formatAmount(debt.latest_balance)}`),
        debt.settlement
          ? h('span', { class: 'badge' }, `${debt.settlement.outcome} ${formatAmount(debt.settlement.amount)}`)
          : '',
      ),
    ),
  );
}

function renderHouses(overview) {
  houses = overview.houses;
  const hasHouses = houses.length > 0;
  $('houses-empty').hidden = hasHouses;
  $('houses-content').hidden = !hasHouses;
  $('house-list').replaceChildren(
    ...houses.map((house) =>
      h(
        'li',
        { class: house.linked_debts.length > 0 ? 'job-row has-linked' : 'job-row' },
        h('span', { class: 'job-name' }, house.name),
        h('span', { class: 'job-meta' }, holdingText(house)),
        latestValue(house),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', `Edit ${house.name}`, () => openHouseDialog(house)),
          iconButton('delete', `Delete ${house.name}`, () => deleteHouse(house)),
        ),
        linkedDebtList(house),
      ),
    ),
  );
  $('house-total').hidden = !hasHouses;
  $('house-total-value').textContent = overview.total === null ? '—' : formatAmount(overview.total);
}

/** Yearly values over all houses, as returned by Rust (CSD-015 REQ-19). */
function renderTotals(totals) {
  $('house-totals-empty').hidden = totals.length > 0;
  $('house-totals-table').hidden = totals.length === 0;
  $('house-totals-rows').replaceChildren(
    ...totals.map((t) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, String(t.year)),
        h('td', { class: 'num' }, formatAmount(t.closing_value)),
        h('td', { class: 'num' }, optionalAmount(t.change)),
      ),
    ),
  );
}

function renderValuations(valuations) {
  $('house-valuations-empty').hidden = valuations.length > 0;
  $('house-valuations-table').hidden = valuations.length === 0;
  $('house-valuations-rows').replaceChildren(
    ...valuations.map((s) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, s.period),
        h('td', { class: 'wrap' }, s.house_name),
        h('td', { class: 'num' }, formatAmount(s.value)),
        h('td', { class: 'num' }, optionalAmount(s.change)),
        h('td', { class: 'note-cell' }, noteSummary(s.note)),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('edit', `Edit ${s.house_name} ${s.period}`, () => openValuationDialog(s)),
          iconButton('delete', `Delete ${s.house_name} ${s.period}`, () => deleteValuation(s)),
        ),
      ),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadHousesOverview() {
  const [overview, debtOverview] = await Promise.all([invoke('list_houses'), invoke('list_debts')]);
  debts = debtOverview.debts;
  renderHouses(overview);
  if (overview.houses.length === 0) return;
  const [totals, valuations] = await Promise.all([invoke('yearly_house_values'), invoke('list_house_valuations')]);
  renderTotals(totals);
  renderValuations(valuations);
}

async function changed() {
  await reloadHousesOverview();
  await onChanged();
}

// ---- House dialog ----

let editingHouse = null;

function openHouseDialog(house) {
  editingHouse = house;
  const form = $('house-form');
  form.reset();
  $('house-error').hidden = true;
  $('house-dialog-title').textContent = house ? 'Edit house' : 'Add house';
  fillDebtChoices(house);
  // The valuation period is locked once the house has valuations (CSD-015 REQ-5).
  const locked = Boolean(house && house.valuation_count > 0);
  form.elements.valuation_period.disabled = locked;
  $('house-period-hint').hidden = !locked;
  if (house) {
    form.elements.name.value = house.name;
    form.elements.valuation_period.value = house.valuation_period;
    // A legacy house has no purchase yet, so those fields stay empty (CSD-019 REQ-6).
    form.elements.purchase_date.value = house.purchase_date === null ? '' : formatDate(house.purchase_date);
    form.elements.purchase_amount.value = house.purchase_amount ?? '';
    form.elements.sell_date.value = house.sell_date === null ? '' : formatDate(house.sell_date);
    form.elements.sell_amount.value = house.sell_amount ?? '';
  }
  $('house-dialog').showModal();
  form.elements.name.focus();
}

/** The debts a house can be linked to: unlinked ones and its own (CSD-020 REQ-13). The links of a sold house are locked. */
function fillDebtChoices(house) {
  const choices = debts.filter((debt) => !debt.house_id || (house && debt.house_id === house.id));
  const locked = Boolean(house && !house.held);
  $('house-debts').hidden = choices.length === 0;
  $('house-debt-choices').replaceChildren(
    ...choices.map((debt) =>
      h(
        'label',
        { class: 'debt-choice' },
        h('input', {
          type: 'checkbox',
          name: 'debt',
          value: debt.id,
          ...(house && debt.house_id === house.id ? { checked: true } : {}),
          ...(locked ? { disabled: true } : {}),
        }),
        debt.name,
      ),
    ),
  );
}

function checkedDebtIds() {
  return [...$('house-debt-choices').querySelectorAll('input:checked')].map((box) => box.value);
}

/** Asks Repaid or Cancelled for each open debt; resolves to the choices, or null for Back (CSD-020 REQ-15). */
function askSettlements(openDebts) {
  const dialog = $('house-settle-dialog');
  const save = $('house-settle-save');
  const rows = $('house-settle-rows');
  const chosen = new Map();
  save.disabled = true;
  rows.replaceChildren(
    ...openDebts.map((debt) =>
      h(
        'div',
        { class: 'settle-row' },
        h('span', {}, `${debt.debt_name} ${formatAmount(debt.amount)}`),
        h(
          'span',
          { class: 'settle-choices', role: 'radiogroup', 'aria-label': `Outcome of ${debt.debt_name}` },
          ...['Repaid', 'Cancelled'].map((outcome) =>
            h(
              'label',
              { class: 'debt-choice' },
              h('input', {
                type: 'radio',
                name: `settle-${debt.debt_id}`,
                value: outcome,
                onchange: () => {
                  chosen.set(debt.debt_id, outcome);
                  save.disabled = chosen.size < openDebts.length;
                },
              }),
              outcome,
            ),
          ),
        ),
      ),
    ),
  );
  return new Promise((resolve) => {
    const finish = (result) => {
      $('house-settle-form').onsubmit = null;
      $('house-settle-back').onclick = null;
      dialog.oncancel = null;
      if (dialog.open) dialog.close();
      resolve(result);
    };
    $('house-settle-form').onsubmit = (event) => {
      event.preventDefault();
      finish([...chosen].map(([debt_id, outcome]) => ({ debt_id, outcome })));
    };
    $('house-settle-back').onclick = () => finish(null);
    dialog.oncancel = () => finish(null);
    dialog.showModal();
  });
}

async function submitHouse(event) {
  event.preventDefault();
  const form = $('house-form');
  const input = {
    name: form.elements.name.value,
    purchase_date: form.elements.purchase_date.value,
    purchase_amount: form.elements.purchase_amount.value,
    sell_date: form.elements.sell_date.value,
    sell_amount: form.elements.sell_amount.value,
  };
  if (!form.elements.valuation_period.disabled) {
    input.valuation_period = form.elements.valuation_period.value;
  }
  const links = { debt_ids: checkedDebtIds(), settlements: [], confirm_undo: false };
  try {
    const sellDate = input.sell_date.trim();
    const sold = editingHouse !== null && !editingHouse.held;
    if (sellDate && !sold) {
      // A new sale: linked debts with an open balance need Repaid or Cancelled (CSD-020 REQ-3).
      const open = await invoke('list_open_debts', { debtIds: links.debt_ids, sellDate });
      if (open.length > 0) {
        const choices = await askSettlements(open);
        if (choices === null) return;
        links.settlements = choices;
      }
    } else if (!sellDate && sold && editingHouse.linked_debts.some((debt) => debt.settlement)) {
      // Undoing the sale also undoes the settled debts (CSD-020 REQ-6).
      const confirmed = await confirmAction({
        title: 'Undo the sale?',
        message: 'The debts settled on this sale go back to their earlier balances.',
        confirmLabel: 'Undo sale',
        danger: true,
      });
      if (!confirmed) return;
      links.confirm_undo = true;
    }
    if (editingHouse) {
      await invoke('update_house', { houseId: editingHouse.id, input, links });
    } else {
      await invoke('create_house', { input, links });
    }
    $('house-dialog').close();
    await changed();
  } catch (error) {
    showError('house-error', error);
  }
}

async function deleteHouse(house) {
  const confirmed = await confirmAction({
    title: `Delete ${house.name}?`,
    message: 'This house and all its valuations will be deleted.',
    confirmLabel: 'Delete house',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_house', { houseId: house.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Valuation dialog ----

let editingValuation = null;
/** Whether the form currently asks for a year, so the period resets only when it changes. */
let formIsYearly = false;

function fillHouseChoices(selectedId) {
  // The list is alphabetical, so the first option is the first house by name.
  const select = $('house-valuation-form').elements.house_id;
  select.replaceChildren(...houses.map((house) => h('option', { value: house.id }, house.name)));
  select.value = selectedId ?? houses[0]?.id ?? '';
}

function chosenHouse() {
  return houses.find((house) => house.id === $('house-valuation-form').elements.house_id.value);
}

/** Asks for a month or a year, following the chosen house (CSD-015 REQ-16). */
function applyValuationPeriod({ resetPeriod }) {
  const form = $('house-valuation-form');
  const yearly = chosenHouse()?.valuation_period === 'Yearly';
  $('house-valuation-period-label').textContent = yearly ? 'Year' : 'Month';
  form.elements.period.placeholder = yearly ? String(new Date().getFullYear() - 1) : 'yyyy-mm';
  form.elements.period.maxLength = yearly ? 4 : 7;
  $('house-valuation-copy').textContent = yearly ? 'Copy previous year' : 'Copy previous month';
  if (resetPeriod && yearly !== formIsYearly) {
    form.elements.period.value = yearly ? currentYear() : currentMonth();
  }
  formIsYearly = yearly;
}

/** Opens the form to add a valuation, or to edit `valuation` when given. */
function openValuationDialog(valuation = null) {
  editingValuation = valuation;
  const form = $('house-valuation-form');
  form.reset();
  $('house-valuation-error').hidden = true;
  fillHouseChoices(valuation?.house_id);
  form.elements.house_id.disabled = Boolean(valuation);
  $('house-valuation-copy').hidden = Boolean(valuation);
  if (valuation) {
    // The period is editable too; the house is not (CSD-015 REQ-12).
    $('house-valuation-dialog-title').textContent = `Edit valuation ${valuation.period}`;
    $('house-valuation-period-label').textContent = valuation.period.length === 4 ? 'Year' : 'Month';
    form.elements.period.maxLength = valuation.period.length;
    form.elements.period.value = valuation.period;
    formIsYearly = valuation.period.length === 4;
    form.elements.value.value = valuation.value;
    form.elements.note.value = valuation.note;
  } else {
    $('house-valuation-dialog-title').textContent = 'Add valuation';
    formIsYearly = null; // force the period to its default for the first house
    form.elements.period.value = '';
    applyValuationPeriod({ resetPeriod: true });
  }
  $('house-valuation-dialog').showModal();
  (valuation ? form.elements.value : form.elements.period).focus();
}

async function submitValuation(event) {
  event.preventDefault();
  const form = $('house-valuation-form');
  const args = {
    houseId: form.elements.house_id.value,
    period: form.elements.period.value,
    value: form.elements.value.value,
    note: form.elements.note.value,
  };
  try {
    if (editingValuation) {
      // The valuation is found by its current period; the field holds the new one (CSD-015 REQ-12).
      await invoke('update_house_valuation', { ...args, period: editingValuation.period, newPeriod: args.period });
    } else {
      await invoke('add_house_valuation', args);
    }
    $('house-valuation-dialog').close();
    await changed();
  } catch (error) {
    showError('house-valuation-error', error);
  }
}

/** Fills the value from the latest valuation of the house chosen in the form (CSD-015 REQ-15). */
async function copyPrevious() {
  const form = $('house-valuation-form');
  try {
    const prefill = await invoke('copy_previous_house_valuation', { houseId: form.elements.house_id.value });
    form.elements.value.value = prefill.value ?? '';
    form.elements.note.value = '';
  } catch (error) {
    showError('house-valuation-error', error);
  }
}

async function deleteValuation(valuation) {
  const confirmed = await confirmAction({
    title: `Delete the valuation of ${valuation.period}?`,
    message: `The valuation of ${valuation.period} of ${valuation.house_name} and its note will be deleted.`,
    confirmLabel: 'Delete valuation',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_house_valuation', { houseId: valuation.house_id, period: valuation.period });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initHousesOverview({ onChanged: callback }) {
  onChanged = callback;
  $('house-add').addEventListener('click', () => openHouseDialog(null));
  $('house-add-first').addEventListener('click', () => openHouseDialog(null));
  $('house-form').addEventListener('submit', submitHouse);
  $('house-valuation-add').addEventListener('click', () => openValuationDialog());
  $('house-valuation-copy').addEventListener('click', copyPrevious);
  $('house-valuation-form').addEventListener('submit', submitValuation);
  $('house-valuation-form').elements.house_id.addEventListener('change', () => applyValuationPeriod({ resetPeriod: true }));
}
