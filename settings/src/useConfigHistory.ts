import { useCallback, useReducer } from "react";
import {
  createHistory,
  historyReducer,
  type History,
  type HistoryAction,
  type HistoryUpdate,
} from "./history.ts";

export function useConfigHistory<T>() {
  const [history, dispatch] = useReducer(
    (state: History<T>, action: HistoryAction<T>) =>
      historyReducer(state, action),
    createHistory<T>(null),
  );
  const setConfig = useCallback(
    (update: HistoryUpdate<T>) => dispatch({ kind: "edit", update }),
    [],
  );
  const replaceConfig = useCallback(
    (value: T | null) => dispatch({ kind: "replace", value }),
    [],
  );
  const acknowledgeConfig = useCallback(
    (update: HistoryUpdate<T>) => dispatch({ kind: "acknowledge", update }),
    [],
  );
  const undo = useCallback(() => dispatch({ kind: "undo" }), []);
  const redo = useCallback(() => dispatch({ kind: "redo" }), []);
  const beginEdit = useCallback(() => dispatch({ kind: "begin" }), []);
  const endEdit = useCallback(() => dispatch({ kind: "end" }), []);
  return {
    config: history.present,
    setConfig,
    replaceConfig,
    acknowledgeConfig,
    undo,
    redo,
    beginEdit,
    endEdit,
    editingGesture: history.grouped,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
  };
}
