/** Configuration snapshots are immutable JSON values. */
export const HISTORY_LIMIT = 40;
export type History<T> = { present: T | null; past: T[]; future: T[]; grouped: boolean; groupRecorded: boolean };
export function createHistory<T>(present: T | null): History<T> {
  return { present, past: [], future: [], grouped: false, groupRecorded: false };
}
export function editHistory<T>(state: History<T>, present: T | null): History<T> {
  if (JSON.stringify(state.present) === JSON.stringify(present)) return state;
  if (present === null) return createHistory<T>(null);
  return { ...state, present, past: state.present === null ? state.past : [...state.past, state.present].slice(-HISTORY_LIMIT), future: [] };
}
export function undoHistory<T>(state: History<T>): History<T> {
  if (state.past.length === 0) return state;
  return { ...state, present: state.past[state.past.length - 1], past: state.past.slice(0, -1), future: state.present === null ? state.future : [state.present, ...state.future], grouped: false, groupRecorded: false };
}

export function redoHistory<T>(state: History<T>): History<T> {
  if (state.future.length === 0) return state;
  return { ...state, present: state.future[0], past: state.present === null ? state.past : [...state.past, state.present].slice(-HISTORY_LIMIT), future: state.future.slice(1), grouped: false, groupRecorded: false };
}
