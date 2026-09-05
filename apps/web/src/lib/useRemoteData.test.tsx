import { act, renderHook, waitFor } from "@testing-library/react";
import { useRemoteData } from "@/lib/useRemoteData";

test("should_refetch_when_beginLoad_is_called_with_the_same_cache_key", async () => {
  let resolveCurrent: (value: string) => void = () => undefined;
  let calls = 0;
  const loader = () => {
    calls += 1;
    return new Promise<string>((resolve) => {
      resolveCurrent = resolve;
    });
  };

  const { result } = renderHook(() => useRemoteData(loader, "catalog"));

  expect(result.current.isLoading).toBe(true);
  await act(async () => {
    resolveCurrent("first");
  });
  await waitFor(() => expect(result.current.isLoading).toBe(false));
  expect(result.current.data).toBe("first");

  act(() => {
    result.current.beginLoad();
  });
  expect(result.current.isLoading).toBe(true);

  await act(async () => {
    resolveCurrent("second");
  });
  await waitFor(() => expect(result.current.isLoading).toBe(false));

  expect(calls).toBe(2);
  expect(result.current.data).toBe("second");
});
