// Entry point: wires the project controls and the income view together.
import { showToast, errorText } from './api.js';
import { initProject, refreshProjectStatus } from './project.js';
import { initIncome, reloadIncome } from './income.js';

async function start() {
  document.querySelectorAll('dialog [data-cancel]').forEach((button) => {
    button.addEventListener('click', () => button.closest('dialog').close());
  });
  initIncome({ onChanged: refreshProjectStatus });
  await initProject({ onReplaced: reloadIncome });
  await reloadIncome();
}

start().catch((error) => showToast(errorText(error)));
