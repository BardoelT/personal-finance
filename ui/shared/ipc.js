// The bridge to the Rust backend: command calls, events and error text.
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
