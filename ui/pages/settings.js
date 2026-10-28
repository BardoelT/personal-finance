// Project settings page (CSD-003): the first setting is the currency of the whole project.
import { invoke, errorText } from '../shared/ipc.js';
import { showToast, confirmAction } from '../shared/dom.js';

const select = () => document.getElementById('setting-currency');

let settings = { currency: 'EUR', entry_count: 0 };

/** Loads the settings from Rust and shows them on the page. */
export async function reloadSettings() {
  settings = await invoke('get_settings');
  select().value = settings.currency;
}

async function onCurrencyChosen(onChanged) {
  const requested = select().value;
  if (requested === settings.currency) return;

  // With paychecks, a changed currency silently relabels every amount (REQ-5).
  if (settings.entry_count > 0) {
    const confirmed = await confirmAction({
      title: `Change the currency to ${requested}?`,
      message: `The ${settings.entry_count === 1 ? 'entry keeps its amount' : `${settings.entry_count} entries keep their amounts`} and only the currency changes. Amounts are not converted.`,
      confirmLabel: `Change to ${requested}`,
    });
    if (!confirmed) {
      select().value = settings.currency;
      return;
    }
  }

  try {
    settings = await invoke('set_currency', { currency: requested });
  } catch (error) {
    select().value = settings.currency;
    showToast(errorText(error));
    return;
  }
  await onChanged();
}

export function initSettings({ onChanged }) {
  select().addEventListener('change', () => onCurrencyChosen(onChanged));
}
