// Entry point: wires the project controls, the income and investments views (each with an insights chart), the settings and the dashboard.
import { errorText, listen } from './shared/ipc.js';
import { showToast } from './shared/dom.js';
import { initProject, refreshProjectStatus } from './pages/project.js';
import { initIncomeOverview, reloadIncomeOverview } from './pages/income-overview.js';
import { initIncomeInsights, reloadIncomeInsights } from './pages/income-insights.js';
import { initInvestmentsInsights, reloadInvestmentsInsights } from './pages/investments-insights.js';
import { initInvestmentsOverview, reloadInvestmentsOverview } from './pages/investments-overview.js';
import { initSettings, reloadSettings } from './pages/settings.js';
import { reloadDashboard } from './pages/dashboard.js';

async function start() {
  document.querySelectorAll('dialog [data-cancel]').forEach((button) => {
    button.addEventListener('click', () => button.closest('dialog').close());
  });

  // Income changes alter the paycheck count the settings page confirms with,
  // and the dashboard figures.
  initIncomeOverview({
    onChanged: async () => {
      await refreshProjectStatus();
      await reloadSettings();
      await reloadDashboard();
    },
  });
  initIncomeInsights();
  initInvestmentsInsights();
  // Investment statements count for the currency confirmation of the settings page.
  initInvestmentsOverview({
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
      await reloadIncomeOverview();
      if (!document.getElementById('income-insights').hidden) await reloadIncomeInsights();
      await reloadInvestmentsOverview();
      if (!document.getElementById('investments-insights').hidden) await reloadInvestmentsInsights();
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
  await reloadIncomeOverview();
  await reloadInvestmentsOverview();
  await reloadDashboard();
}

start().catch((error) => showToast(errorText(error)));
