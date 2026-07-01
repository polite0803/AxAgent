// SPDX-License-Identifier: AGPL-3.0-only

import type { DataSourceConfig } from "@/types";
import { useEffect, useRef, useState } from "react";
import { resolveDataSource, subscribeDataSource } from "./DataBindingEngine";

interface UseDataSourceResult<T = unknown> {
  data: T | undefined;
  loading: boolean;
  error: Error | null;
  refresh: () => Promise<void>;
}

export function useDataSource<T = unknown>(
  config: DataSourceConfig | undefined,
): UseDataSourceResult<T> {
  const [data, setData] = useState<T | undefined>(undefined);
  const [loading, setLoading] = useState<boolean>(!!config);
  const [error, setError] = useState<Error | null>(null);
  const subscriberRef = useRef<{ unsubscribe: () => void } | null>(null);

  const refresh = async () => {
    if (!config) {
      setData(undefined);
      setLoading(false);
      setError(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const result = await resolveDataSource(config);
      setData(result as T);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let mounted = true;

    if (!config) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setData(undefined);
      setLoading(false);
      setError(null);
      return;
    }

    setLoading(true);
    setError(null);

    void (async () => {
      try {
        if (subscriberRef.current) {
          subscriberRef.current.unsubscribe();
        }

        subscriberRef.current = await subscribeDataSource(
          config,
          (newData) => {
            if (mounted) {
              setData(newData as T);
              setLoading(false);
              setError(null);
            }
          },
          (err) => {
            if (mounted) {
              setError(err);
              setLoading(false);
            }
          },
        );
      } catch (err) {
        if (mounted) {
          setError(err instanceof Error ? err : new Error(String(err)));
          setLoading(false);
        }
      }
    })();

    return () => {
      mounted = false;
      if (subscriberRef.current) {
        subscriberRef.current.unsubscribe();
        subscriberRef.current = null;
      }
    };
  }, [config]);

  return { data, loading, error, refresh };
}
