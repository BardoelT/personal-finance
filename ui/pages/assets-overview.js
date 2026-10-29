// Assets: things the user owns with a monthly or yearly valuation, each a value and a note (CSD-015).
import { invoke, errorText } from '../shared/ipc.js';
import { h, iconButton, confirmAction, showToast, showError } from '../shared/dom.js';
import { formatAmount, currentYear, currentMonth } from '../shared/format.js';

const $ = (id) => document.getElementById(id);

let assets = [];
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

function latestValue(asset) {
  if (asset.latest_value === null) return h('span', { class: 'account-amount' }, '—');
  return h('span', { class: 'account-amount' }, formatAmount(asset.latest_value), h('small', {}, asset.latest_period));
}

function renderAssets(overview) {
  assets = overview.assets;
  const hasAssets = assets.length > 0;
  $('assets-empty').hidden = hasAssets;
  $('assets-content').hidden = !hasAssets;
  $('asset-list').replaceChildren(
    ...assets.map((asset) =>
      h(
        'li',
        { class: 'job-row' },
        h('span', { class: 'job-name' }, asset.name),
        h('span', { class: 'job-meta' }, asset.valuation_period),
        latestValue(asset),
        h(
          'span',
          { class: 'job-actions' },
          iconButton('edit', `Edit ${asset.name}`, () => openAssetDialog(asset)),
          iconButton('delete', `Delete ${asset.name}`, () => deleteAsset(asset)),
        ),
      ),
    ),
  );
  $('asset-total').hidden = !hasAssets;
  $('asset-total-value').textContent = overview.total === null ? '—' : formatAmount(overview.total);
}

/** Yearly values over all assets, as returned by Rust (CSD-015 REQ-19). */
function renderTotals(totals) {
  $('asset-totals-empty').hidden = totals.length > 0;
  $('asset-totals-table').hidden = totals.length === 0;
  $('asset-totals-rows').replaceChildren(
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
  $('asset-valuations-empty').hidden = valuations.length > 0;
  $('asset-valuations-table').hidden = valuations.length === 0;
  $('asset-valuations-rows').replaceChildren(
    ...valuations.map((s) =>
      h(
        'tr',
        {},
        h('th', { scope: 'row' }, s.period),
        h('td', { class: 'wrap' }, s.asset_name),
        h('td', { class: 'num' }, formatAmount(s.value)),
        h('td', { class: 'num' }, optionalAmount(s.change)),
        h('td', { class: 'note-cell' }, noteSummary(s.note)),
        h(
          'td',
          { class: 'row-actions' },
          iconButton('edit', `Edit ${s.asset_name} ${s.period}`, () => openValuationDialog(s)),
          iconButton('delete', `Delete ${s.asset_name} ${s.period}`, () => deleteValuation(s)),
        ),
      ),
    ),
  );
}

/** Reloads everything from Rust. */
export async function reloadAssetsOverview() {
  const overview = await invoke('list_assets');
  renderAssets(overview);
  if (overview.assets.length === 0) return;
  const [totals, valuations] = await Promise.all([invoke('yearly_asset_values'), invoke('list_asset_valuations')]);
  renderTotals(totals);
  renderValuations(valuations);
}

async function changed() {
  await reloadAssetsOverview();
  await onChanged();
}

// ---- Asset dialog ----

let editingAsset = null;

function openAssetDialog(asset) {
  editingAsset = asset;
  const form = $('asset-form');
  form.reset();
  $('asset-error').hidden = true;
  $('asset-dialog-title').textContent = asset ? 'Edit asset' : 'Add asset';
  // The valuation period is locked once the asset has valuations (CSD-015 REQ-5).
  const locked = Boolean(asset && asset.valuation_count > 0);
  form.elements.valuation_period.disabled = locked;
  $('asset-period-hint').hidden = !locked;
  if (asset) {
    form.elements.name.value = asset.name;
    form.elements.valuation_period.value = asset.valuation_period;
  }
  $('asset-dialog').showModal();
  form.elements.name.focus();
}

async function submitAsset(event) {
  event.preventDefault();
  const form = $('asset-form');
  const input = { name: form.elements.name.value };
  if (!form.elements.valuation_period.disabled) {
    input.valuation_period = form.elements.valuation_period.value;
  }
  try {
    if (editingAsset) {
      await invoke('update_asset', { assetId: editingAsset.id, input });
    } else {
      await invoke('create_asset', { input });
    }
    $('asset-dialog').close();
    await changed();
  } catch (error) {
    showError('asset-error', error);
  }
}

async function deleteAsset(asset) {
  const confirmed = await confirmAction({
    title: `Delete ${asset.name}?`,
    message: 'This asset and all its valuations will be deleted.',
    confirmLabel: 'Delete asset',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_asset', { assetId: asset.id });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

// ---- Valuation dialog ----

let editingValuation = null;
/** Whether the form currently asks for a year, so the period resets only when it changes. */
let formIsYearly = false;

function fillAssetChoices(selectedId) {
  // The list is alphabetical, so the first option is the first asset by name.
  const select = $('asset-valuation-form').elements.asset_id;
  select.replaceChildren(...assets.map((asset) => h('option', { value: asset.id }, asset.name)));
  select.value = selectedId ?? assets[0]?.id ?? '';
}

function chosenAsset() {
  return assets.find((asset) => asset.id === $('asset-valuation-form').elements.asset_id.value);
}

/** Asks for a month or a year, following the chosen asset (CSD-015 REQ-16). */
function applyValuationPeriod({ resetPeriod }) {
  const form = $('asset-valuation-form');
  const yearly = chosenAsset()?.valuation_period === 'Yearly';
  $('asset-valuation-period-label').textContent = yearly ? 'Year (YYYY)' : 'Month (YYYY-MM)';
  form.elements.period.placeholder = yearly ? String(new Date().getFullYear() - 1) : '2026-09';
  form.elements.period.maxLength = yearly ? 4 : 7;
  $('asset-valuation-copy').textContent = yearly ? 'Copy previous year' : 'Copy previous month';
  if (resetPeriod && yearly !== formIsYearly) {
    form.elements.period.value = yearly ? currentYear() : currentMonth();
  }
  formIsYearly = yearly;
}

/** Opens the form to add a valuation, or to edit `valuation` when given. */
function openValuationDialog(valuation = null) {
  editingValuation = valuation;
  const form = $('asset-valuation-form');
  form.reset();
  $('asset-valuation-error').hidden = true;
  fillAssetChoices(valuation?.asset_id);
  form.elements.asset_id.disabled = Boolean(valuation);
  $('asset-valuation-copy').hidden = Boolean(valuation);
  if (valuation) {
    // The period is editable too; the asset is not (CSD-015 REQ-12).
    $('asset-valuation-dialog-title').textContent = `Edit valuation ${valuation.period}`;
    $('asset-valuation-period-label').textContent = valuation.period.length === 4 ? 'Year (YYYY)' : 'Month (YYYY-MM)';
    form.elements.period.maxLength = valuation.period.length;
    form.elements.period.value = valuation.period;
    formIsYearly = valuation.period.length === 4;
    form.elements.value.value = valuation.value;
    form.elements.note.value = valuation.note;
  } else {
    $('asset-valuation-dialog-title').textContent = 'Add valuation';
    formIsYearly = null; // force the period to its default for the first asset
    form.elements.period.value = '';
    applyValuationPeriod({ resetPeriod: true });
  }
  $('asset-valuation-dialog').showModal();
  (valuation ? form.elements.value : form.elements.period).focus();
}

async function submitValuation(event) {
  event.preventDefault();
  const form = $('asset-valuation-form');
  const args = {
    assetId: form.elements.asset_id.value,
    period: form.elements.period.value,
    value: form.elements.value.value,
    note: form.elements.note.value,
  };
  try {
    if (editingValuation) {
      // The valuation is found by its current period; the field holds the new one (CSD-015 REQ-12).
      await invoke('update_asset_valuation', { ...args, period: editingValuation.period, newPeriod: args.period });
    } else {
      await invoke('add_asset_valuation', args);
    }
    $('asset-valuation-dialog').close();
    await changed();
  } catch (error) {
    showError('asset-valuation-error', error);
  }
}

/** Fills the value from the latest valuation of the asset chosen in the form (CSD-015 REQ-15). */
async function copyPrevious() {
  const form = $('asset-valuation-form');
  try {
    const prefill = await invoke('copy_previous_asset_valuation', { assetId: form.elements.asset_id.value });
    form.elements.value.value = prefill.value ?? '';
    form.elements.note.value = '';
  } catch (error) {
    showError('asset-valuation-error', error);
  }
}

async function deleteValuation(valuation) {
  const confirmed = await confirmAction({
    title: `Delete the valuation of ${valuation.period}?`,
    message: `The valuation of ${valuation.period} of ${valuation.asset_name} and its note will be deleted.`,
    confirmLabel: 'Delete valuation',
    danger: true,
  });
  if (!confirmed) return;
  try {
    await invoke('delete_asset_valuation', { assetId: valuation.asset_id, period: valuation.period });
    await changed();
  } catch (error) {
    showToast(errorText(error));
  }
}

export function initAssetsOverview({ onChanged: callback }) {
  onChanged = callback;
  $('asset-add').addEventListener('click', () => openAssetDialog(null));
  $('asset-add-first').addEventListener('click', () => openAssetDialog(null));
  $('asset-form').addEventListener('submit', submitAsset);
  $('asset-valuation-add').addEventListener('click', () => openValuationDialog());
  $('asset-valuation-copy').addEventListener('click', copyPrevious);
  $('asset-valuation-form').addEventListener('submit', submitValuation);
  $('asset-valuation-form').elements.asset_id.addEventListener('change', () => applyValuationPeriod({ resetPeriod: true }));
}
