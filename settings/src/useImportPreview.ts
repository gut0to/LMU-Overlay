import { useCallback, useEffect, useRef, useState } from "react";

type Preview<T> = {
  source: string;
  config: T | null;
  pending: boolean;
  error: string | null;
};

export function useImportPreview<T>(
  text: string,
  validate: (text: string) => Promise<T>,
) {
  const [state, setState] = useState<Preview<T> | null>(null);
  const request = useRef(0);
  const currentText = useRef(text);
  currentText.current = text;
  useEffect(
    () => () => {
      request.current++;
    },
    [],
  );
  const clear = useCallback(() => {
    request.current++;
    setState(null);
  }, []);
  const review = useCallback(async () => {
    const id = ++request.current;
    setState({ source: text, config: null, pending: true, error: null });
    try {
      const config = await validate(text);
      if (id !== request.current || text !== currentText.current) return;
      setState({ source: text, config, pending: false, error: null });
    } catch (error) {
      if (id !== request.current || text !== currentText.current) return;
      setState({
        source: text,
        config: null,
        pending: false,
        error: String(error),
      });
    }
  }, [text, validate]);
  const active = state?.source === text ? state : null;
  return {
    config: active?.config ?? null,
    pending: active?.pending ?? false,
    error: active?.error ?? null,
    review,
    clear,
  };
}
