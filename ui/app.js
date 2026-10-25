// Entry point: wires the project controls, the income view and the settings together.
import { showToast, errorText, listen } from './api.js';
import { initProject, refreshProjectStatus } from './project.js';
import { initIncome, reloadIncome } from './income.js';
import { initSettings, reloadSettings } from './settings.js';

async function start() {
  document.querySelectorAll('dialog [data-cancel]').forEach((button) => {
    button.addEventListener('click', () => button.closest('dialog').close());
  });

  // Income changes alter the paycheck count the settings page confirms with.
  initIncome({
    onChanged: async () => {
      await refreshProjectStatus();
      await reloadSettings();
    },
  });
  // A new currency shows up in the titles of the income view.
  initSettings({
    onChanged: async () => {
      await refreshProjectStatus();
      await reloadIncome();
    },
  });
  await initProject({
    onReplaced: async () => {
      await reloadSettings();
      await reloadIncome();
    },
  });

  // Settings… in the app menu, and ⌘, (CSD-003 REQ-2).
  await listen('menu-action', (event) => {
    if (event.payload === 'open-settings') {
      document.querySelector('[data-section="settings"]').click();
    }
  });

  await reloadSettings();
  await reloadIncome();
}

start().catch((error) => showToast(errorText(error)));
