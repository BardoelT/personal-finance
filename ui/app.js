// Entry point: wires the project controls, the income and investments views, the settings and the dashboard.
import { showToast, errorText, listen } from './api.js';
import { initProject, refreshProjectStatus } from './project.js';
import { initIncome, reloadIncome } from './income.js';
import { initInvestments, reloadInvestments } from './investments.js';
import { initSettings, reloadSettings } from './settings.js';
import { reloadDashboard } from './dashboard.js';

async function start() {
  document.querySelectorAll('dialog [data-cancel]').forEach((button) => {
    button.addEventListener('click', () => button.closest('dialog').close());
  });

  // Income changes alter the paycheck count the settings page confirms with,
  // and the dashboard figures.
  initIncome({
    onChanged: async () => {
      await refreshProjectStatus();
      await reloadSettings();
      await reloadDashboard();
    },
  });
  // Investment statements count for the currency confirmation of the settings page.
  initInvestments({
    onChanged: async () => {
      await refreshProjectStatus();
      await reloadSettings();
    },
  });
  initSettings({
    onChanged: async () => {
      await refreshProjectStatus();
    },
  });
  await initProject({
    onReplaced: async () => {
      await reloadSettings();
      await reloadIncome();
      await reloadInvestments();
      await reloadDashboard();
    },
  });

  // Settings… in the app menu, and ⌘, (CSD-003 REQ-2).
  await listen('menu-action', (event) => {
    if (event.payload === 'open-settings') {
      document.querySelector('[data-section="settings"]').click();
    }
  });

  // Opening the dashboard recomputes it, so it never shows stale months.
  document
    .querySelector('[data-section="dashboard"]')
    .addEventListener('click', () => reloadDashboard().catch((e) => showToast(errorText(e))));

  await reloadSettings();
  await reloadIncome();
  await reloadInvestments();
  await reloadDashboard();
}

start().catch((error) => showToast(errorText(error)));
