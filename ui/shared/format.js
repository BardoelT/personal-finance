// Display formatting and "today" helpers. Display only: no figure is computed here.

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

/** The current calendar year as text, such as "2026". */
export function currentYear() {
  return String(new Date().getFullYear());
}

/** The current month as text, such as "2026-10". */
export function currentMonth() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
}
