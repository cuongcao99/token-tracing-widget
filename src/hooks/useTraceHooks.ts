import { useCallback, useEffect, useRef, useState } from "react";
import {
  getTraceHookStatus,
  updateTraceHook,
  type TraceHookStatus,
} from "../lib/trace-hooks";
import type { ProviderId } from "../lib/provider";

export interface UseTraceHooksResult {
  statuses: TraceHookStatus[];
  loading: boolean;
  error: Error | null;
  updatingProvider: ProviderId | null;
  toggle(provider: ProviderId, enabled: boolean): Promise<void>;
}

function asError(error: unknown): Error {
  return error instanceof Error ? error : new Error("hook_status_unavailable");
}

export function useTraceHooks(): UseTraceHooksResult {
  const mounted = useRef(true);
  const [statuses, setStatuses] = useState<TraceHookStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [updatingProvider, setUpdatingProvider] = useState<ProviderId | null>(null);

  useEffect(() => {
    mounted.current = true;
    void getTraceHookStatus()
      .then((snapshot) => {
        if (!mounted.current) return;
        setStatuses(snapshot.providers);
        setError(null);
      })
      .catch((loadError: unknown) => {
        if (mounted.current) setError(asError(loadError));
      })
      .finally(() => {
        if (mounted.current) setLoading(false);
      });

    return () => {
      mounted.current = false;
    };
  }, []);

  const toggle = useCallback(
    async (provider: ProviderId, enabled: boolean) => {
      if (updatingProvider) return;
      setError(null);
      setUpdatingProvider(provider);
      try {
        const snapshot = await updateTraceHook({ provider, enabled });
        if (mounted.current) setStatuses(snapshot.providers);
      } catch (toggleError) {
        if (mounted.current) setError(asError(toggleError));
      } finally {
        if (mounted.current) setUpdatingProvider(null);
      }
    },
    [updatingProvider],
  );

  return { statuses, loading, error, updatingProvider, toggle };
}

export default useTraceHooks;
