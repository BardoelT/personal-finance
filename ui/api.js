// Thin wrappers around the Tauri API plus small DOM helpers.
// The frontend only displays what Rust returns; it computes no figures.

const tauri = window.__TAURI__;

export function invoke(command, args) {
  return tauri.core.invoke(command, args);
}

export function listen(eventName, handler) {
  return tauri.event.listen(eventName, handler);
}

/** Safe, user-facing text of an error returned by a command. */
export function errorText(error) {
  if (error && typeof error.message === 'string' && error.message) {
    return error.message;
  }
  return 'Something went wrong. Please try again.';
}

/** Builds an element. Strings become text nodes, so user data is never parsed as HTML. */
export function h(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [key, value] of Object.entries(attrs)) {
    if (value === undefined || value === null || value === false) continue;
    if (key.startsWith('on')) {
      node.addEventListener(key.slice(2).toLowerCase(), value);
    } else if (key === 'class') {
      node.className = value;
    } else {
      node.setAttribute(key, value === true ? '' : value);
    }
  }
  for (const child of children.flat()) {
    if (child === null || child === undefined || child === false) continue;
    node.append(child);
  }
  return node;
}

const SVG_NS = 'http://www.w3.org/2000/svg';
const ICON_PATHS = {
  edit: ['M12 20h9', 'M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z'],
  delete: ['M3 6h18', 'M8 6V4h8v2', 'M19 6l-1 14H6L5 6', 'M10 11v6', 'M14 11v6'],
};

/** A row action as an icon button; `label` names it for screen readers and the tooltip. */
export function iconButton(kind, label, onClick) {
  const svg = document.createElementNS(SVG_NS, 'svg');
  for (const [key, value] of Object.entries({
    viewBox: '0 0 24 24', width: '16', height: '16', fill: 'none', stroke: 'currentColor',
    'stroke-width': '2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'aria-hidden': 'true',
  })) {
    svg.setAttribute(key, value);
  }
  for (const d of ICON_PATHS[kind]) {
    const path = document.createElementNS(SVG_NS, 'path');
    path.setAttribute('d', d);
    svg.append(path);
  }
  const danger = kind === 'delete' ? ' danger' : '';
  return h('button', { type: 'button', class: `icon-btn${danger}`, 'aria-label': label, title: label, onClick }, svg);
}

/** Shows an ISO date such as "2024-03-01" as dd/mm/yyyy (CSD-001 REQ-22). Display only. */
export function formatDate(iso) {
  return /^\d{4}-\d{2}-\d{2}$/.test(iso) ? iso.split('-').reverse().join('/') : String(iso);
}

/** Adds thousands separators to a decimal string such as "3000.00". Display only. */
export function formatAmount(text) {
  const [whole, fraction] = String(text).split('.');
  const grouped = whole.replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return fraction === undefined ? grouped : `${grouped}.${fraction}`;
}

let toastTimer = null;

export function showToast(message) {
  const toast = document.getElementById('toast');
  toast.textContent = message;
  toast.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.hidden = true;
  }, 6000);
}

/**
 * Asks the user to confirm. Resolves true only if they choose the confirm button.
 *
 * The answer comes from the button clicks and Escape, not from the dialog's
 * `close` event: that event can arrive late and would then answer the next
 * confirmation by mistake.
 */
export function confirmAction({ title, message, confirmLabel, danger = false }) {
  const dialog = document.getElementById('confirm-dialog');
  if (dialog.open) return Promise.resolve(false);
  document.getElementById('confirm-title').textContent = title;
  document.getElementById('confirm-message').textContent = message;
  const ok = document.getElementById('confirm-ok');
  const cancel = document.getElementById('confirm-cancel');
  ok.textContent = confirmLabel;
  ok.classList.toggle('danger', danger);
  ok.classList.toggle('primary', !danger);
  return new Promise((resolve) => {
    const finish = (confirmed) => {
      ok.onclick = null;
      cancel.onclick = null;
      dialog.oncancel = null;
      if (dialog.open) dialog.close();
      resolve(confirmed);
    };
    ok.onclick = (event) => {
      event.preventDefault();
      finish(true);
    };
    cancel.onclick = (event) => {
      event.preventDefault();
      finish(false);
    };
    dialog.oncancel = () => finish(false); // Escape
    dialog.showModal();
  });
}
