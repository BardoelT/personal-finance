// The "Settle linked debts" prompt: Repaid or Cancelled for each open debt (CSD-020 REQ-15).
import { h } from './dom.js';
import { formatAmount } from './format.js';

const $ = (id) => document.getElementById(id);

/** Asks Repaid or Cancelled for each open debt; resolves to the choices, or null for Back (CSD-020 REQ-15). */
export function askSettlements(openDebts) {
  const dialog = $('house-settle-dialog');
  const save = $('house-settle-save');
  const rows = $('house-settle-rows');
  const chosen = new Map();
  save.disabled = true;
  rows.replaceChildren(
    ...openDebts.map((debt) =>
      h(
        'div',
        { class: 'settle-row' },
        h('span', {}, `${debt.debt_name} ${formatAmount(debt.amount)}`),
        h(
          'span',
          { class: 'settle-choices', role: 'radiogroup', 'aria-label': `Outcome of ${debt.debt_name}` },
          ...['Repaid', 'Cancelled'].map((outcome) =>
            h(
              'label',
              { class: 'debt-choice' },
              h('input', {
                type: 'radio',
                name: `settle-${debt.debt_id}`,
                value: outcome,
                onchange: () => {
                  chosen.set(debt.debt_id, outcome);
                  save.disabled = chosen.size < openDebts.length;
                },
              }),
              outcome,
            ),
          ),
        ),
      ),
    ),
  );
  return new Promise((resolve) => {
    const finish = (result) => {
      $('house-settle-form').onsubmit = null;
      $('house-settle-back').onclick = null;
      dialog.oncancel = null;
      if (dialog.open) dialog.close();
      resolve(result);
    };
    $('house-settle-form').onsubmit = (event) => {
      event.preventDefault();
      finish([...chosen].map(([debt_id, outcome]) => ({ debt_id, outcome })));
    };
    $('house-settle-back').onclick = () => finish(null);
    dialog.oncancel = () => finish(null);
    dialog.showModal();
  });
}
