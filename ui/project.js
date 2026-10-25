// Project file controls: New, Open, Save, Save As, and the unsaved-changes indicator (CSD-002).
import { invoke, listen, confirmAction, showToast, errorText } from './api.js';

let status = { unsaved_changes: false, has_path: false };

function render() {
  const box = document.getElementById('project-state');
  const text = document.getElementById('project-state-text');
  box.classList.toggle('is-unsaved', status.unsaved_changes);
  if (status.unsaved_changes) {
    text.textContent = 'Unsaved changes';
  } else if (status.has_path) {
    text.textContent = 'All changes saved';
  } else {
    text.textContent = 'New project';
  }
}

export async function refreshProjectStatus() {
  status = await invoke('project_status');
  render();
}

async function attempt(action) {
  try {
    await action();
  } catch (error) {
    showToast(errorText(error));
  }
}

async function confirmDiscard(action) {
  await refreshProjectStatus();
  if (!status.unsaved_changes) return true;
  return confirmAction({
    title: 'Discard unsaved changes?',
    message: `You have unsaved changes. If you ${action}, they will be lost.`,
    confirmLabel: 'Discard changes',
    danger: true,
  });
}

export async function initProject({ onReplaced }) {
  document.getElementById('project-new').addEventListener('click', () =>
    attempt(async () => {
      if (!(await confirmDiscard('start a new project'))) return;
      status = await invoke('new_project');
      render();
      await onReplaced();
    }),
  );

  document.getElementById('project-open').addEventListener('click', () =>
    attempt(async () => {
      if (!(await confirmDiscard('open another file'))) return;
      const result = await invoke('open_project');
      status = result.status;
      render();
      if (result.done) await onReplaced();
    }),
  );

  document.getElementById('project-save').addEventListener('click', () =>
    attempt(async () => {
      const result = await invoke('save_project');
      status = result.status;
      render();
    }),
  );

  document.getElementById('project-save-as').addEventListener('click', () =>
    attempt(async () => {
      const result = await invoke('save_project_as');
      status = result.status;
      render();
    }),
  );

  // Rust asks for a confirmation when the window or app is closed with unsaved changes.
  await listen('close-requested', () =>
    attempt(async () => {
      const quit = await confirmAction({
        title: 'Quit without saving?',
        message: 'You have unsaved changes. They will be lost if you quit.',
        confirmLabel: 'Quit without saving',
        danger: true,
      });
      if (quit) await invoke('quit_app');
    }),
  );

  await refreshProjectStatus();
}
