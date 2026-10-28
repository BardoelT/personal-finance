// Entry point: wires the project controls, the income and investments views (each with an insights chart), the settings and the dashboard.
import { errorText, listen } from './shared/ipc.js';
import { showToast } from './shared/dom.js';
import { initProject, refreshProjectStatus } from './pages/project.js';
import { initIncomeOverview, reloadIncomeOverview } from './pages/income-overview.js';
import { initIncomeInsights, reloadIncomeInsightsIfOpen } from './pages/income-insights.js';
import { initInvestmentsInsights, reloadInvestmentsInsightsIfOpen } from './pages/investments-insights.js';
import { initInvestmentsOverview, reloadInvestmentsOverview } from './pages/investments-overview.js';
import { initSettings, loadPreferences, reloadSettings } from './pages/settings.js';
import { reloadDashboard } from './pages/dashboard.js';

async function start() {
  // The theme comes first: the page stays hidden until it is applied (CSD-008 REQ-6).
  // If it cannot be loaded the app still starts, with the theme macOS gives it.
  try {
    await loadPreferences();
  } catch (error) {
    showToast(errorText(error));
  } finally {
    document.documentElement.classList.add('theme-ready');
  }

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
      await reloadIncomeInsightsIfOpen();
      await reloadInvestmentsOverview();
      await reloadInvestmentsInsightsIfOpen();
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
