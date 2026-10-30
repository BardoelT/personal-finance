// Types the separators of a date or month for the user. Display only: whether the
// text is a valid date is decided in Rust.

const DATE = { groups: [2, 2, 4], separator: '/' };
const MONTH = { groups: [4, 2], separator: '-' };
const YEAR = { groups: [4], separator: '' };

/**
 * Lays digits out in groups, adding a separator once a group is full and more
 * typing follows. `keepTrailing` also adds it after the last full group, which
 * is what typing wants and deleting does not.
 */
export function maskDigits(digits, { groups, separator }, keepTrailing) {
  const parts = [];
  let rest = digits;
  for (const size of groups) {
    if (rest === '') break;
    parts.push(rest.slice(0, size));
    rest = rest.slice(size);
  }
  let text = parts.join(separator);
  const lastIsFull = parts.length > 0 && parts[parts.length - 1].length === groups[parts.length - 1];
  if (keepTrailing && lastIsFull && parts.length < groups.length) text += separator;
  return text;
}

function layoutOf(input) {
  if (input.dataset.mask === 'date') return DATE;
  // A month field asks for a year only when the page limits it to 4 characters.
  return input.maxLength === 4 ? YEAR : MONTH;
}

function onInput(event) {
  const input = event.target;
  const layout = layoutOf(input);
  const deleting = String(event.inputType ?? '').startsWith('delete');
  const caret = input.selectionStart ?? input.value.length;
  const atEnd = caret >= input.value.length;
  const digitsBeforeCaret = input.value.slice(0, caret).replace(/\D/g, '').length;
  const digits = input.value.replace(/\D/g, '');
  const text = maskDigits(digits, layout, !deleting);
  if (text === input.value) return;
  input.value = text;
  // Keep the caret after the same digit, so editing in the middle still works.
  if (atEnd) {
    input.setSelectionRange(text.length, text.length);
    return;
  }
  let seen = 0;
  let position = 0;
  while (position < text.length && seen < digitsBeforeCaret) {
    if (/\d/.test(text[position])) seen += 1;
    position += 1;
  }
  input.setSelectionRange(position, position);
}

/** Masks every `<input data-mask="date|month">` on the page. */
export function initDateMasks() {
  document.querySelectorAll('input[data-mask]').forEach((input) => {
    input.addEventListener('input', onInput);
  });
}
