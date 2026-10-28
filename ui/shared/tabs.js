// Tabs of a page, such as Overview and Insights: one tab selected at a time,
// arrow keys to move between them, and an optional action when a tab opens.
import { errorText } from './ipc.js';
import { showToast } from './dom.js';

const $ = (id) => document.getElementById(id);

/**
 * Wires the tabs `[{ tab, panel, onOpen }]`, where `tab` and `panel` are element
 * ids and `onOpen` (optional) runs each time the tab is opened, for example to
 * recompute a chart so it never shows stale figures.
 */
export function initTabs(tabs) {
  function select(index, { focus = false } = {}) {
    tabs.forEach(({ tab, panel }, i) => {
      const selected = i === index;
      $(tab).setAttribute('aria-selected', String(selected));
      $(tab).tabIndex = selected ? 0 : -1;
      $(panel).hidden = !selected;
    });
    if (focus) $(tabs[index].tab).focus();
    const { onOpen } = tabs[index];
    if (onOpen) Promise.resolve(onOpen()).catch((error) => showToast(errorText(error)));
  }

  tabs.forEach(({ tab }, index) => {
    $(tab).addEventListener('click', () => select(index));
    $(tab).addEventListener('keydown', (event) => {
      const step = { ArrowRight: 1, ArrowLeft: -1 }[event.key];
      if (step === undefined) return;
      event.preventDefault();
      select((index + step + tabs.length) % tabs.length, { focus: true });
    });
  });
}
