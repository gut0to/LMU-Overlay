/** Configuration snapshots are immutable JSON values. */
export const HISTORY_LIMIT = 40;
export type History<T> = { present: T | null; past: T[]; future: T[]; grouped: boolean; groupRecorded: boolean };
export function createHistory<T>(present: T | null): History<T> {
  return { present, past: [], future: [], grouped: false, groupRecorded: false };
}
export function editHistory<T>(state: History<T>, present: T | null): History<T> {
  if (JSON.stringify(state.present) === JSON.stringify(present)) return state;
  if (present === null) return createHistory<T>(null);
  return { ...state, present, past: state.present === null || (state.grouped && state.groupRecorded) ? state.past : [...state.past, state.present].slice(-HISTORY_LIMIT), future: [], groupRecorded: state.grouped };
}
export function undoHistory<T>(state: History<T>): History<T> {
  if (state.past.length === 0) return state;
  return { ...state, present: state.past[state.past.length - 1], past: state.past.slice(0, -1), future: state.present === null ? state.future : [state.present, ...state.future], grouped: false, groupRecorded: false };
}

export function redoHistory<T>(state: History<T>): History<T> {
  if (state.future.length === 0) return state;
  return { ...state, present: state.future[0], past: state.present === null ? state.past : [...state.past, state.present].slice(-HISTORY_LIMIT), future: state.future.slice(1), grouped: false, groupRecorded: false };
}

export function beginHistory<T>(state: History<T>): History<T> { return state.grouped ? state : { ...state, grouped: true, groupRecorded: false }; }
export function endHistory<T>(state: History<T>): History<T> { return !state.grouped ? state : { ...state, grouped: false, groupRecorded: false }; }

export function acknowledgeHistory<T>(state: History<T>, present: T | null): History<T> {
  return state.present === present ? state : { ...state, present };
}

export type HistoryUpdate<T> = T | null | ((current: T | null) => T | null);
export type HistoryAction<T> = { kind: "edit" | "acknowledge"; update: HistoryUpdate<T> } | { kind: "replace"; value: T | null } | { kind: "undo" | "redo" | "begin" | "end" };
export function historyReducer<T>(state: History<T>, action: HistoryAction<T>): History<T> {
  switch (action.kind) {
    case "edit": case "acknowledge": {
      const present = typeof action.update === "function" ? (action.update as (current: T | null) => T | null)(state.present) : action.update;
      return action.kind === "edit" ? editHistory(state, present) : acknowledgeHistory(state, present);
    }
    case "replace": return createHistory(action.value);
    case "undo": return undoHistory(state);
    case "redo": return redoHistory(state);
    case "begin": return beginHistory(state);
    case "end": return endHistory(state);
  }
}
