"use client";

import { useCallback, useEffect, useState } from "react";

export function useRemoteData<T>(
  loader: () => Promise<T | null>,
  cacheKey: string,
) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [generation, setGeneration] = useState(0);

  const reload = useCallback(() => {
    setIsLoading(true);
    setGeneration((current) => current + 1);
  }, []);

  const beginLoad = reload;

  useEffect(() => {
    let cancelled = false;
    loader()
      .then((result) => {
        if (cancelled) {
          return;
        }
        setData(result);
        setError(null);
        setIsLoading(false);
      })
      .catch((loadError: unknown) => {
        if (cancelled) {
          return;
        }
        setData(null);
        setError(loadError);
        setIsLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [cacheKey, generation, loader]);

  return { data, error, isLoading, reload, beginLoad, setData };
}
