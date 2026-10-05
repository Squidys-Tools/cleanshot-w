import { describe, expect, test } from "bun:test";
import { readIconLib } from "../src/components/editor/IconLibrary";

/* A Storage that throws the way a packaged WebView2 profile can. Both a
   SecurityError (storage unavailable) and a QuotaExceededError are modelled,
   because the second also throws from getItem in some engines. */
function throwingStorage(error: Error): Storage {
  const boom = () => {
    throw error;
  };
  return {
    get length(): number {
      return boom();
    },
    clear: boom,
    getItem: boom,
    key: boom,
    removeItem: boom,
    setItem: boom,
  } as unknown as Storage;
}

describe("readIconLib", () => {
  test("returns the stored library when storage works", () => {
    const working = {
      getItem: () => "phosphor",
    } as unknown as Storage;
    expect(readIconLib(working)).toBe("phosphor");
  });

  test("defaults to svg when nothing is stored", () => {
    const empty = { getItem: () => null } as unknown as Storage;
    expect(readIconLib(empty)).toBe("svg");
  });

  /* This is the regression. readIconLib runs in a useState initializer inside
     the tldraw subtree (IconLibrary.tsx useIconLib, consumed by Toolbar in
     TldrawCanvas). An unguarded throw there is a render-time throw inside
     <Tldraw>, which tldraw's own OptionalErrorBoundary contains: the canvas,
     dock and status bar all disappear while the rest of the app keeps running.
     That is the exact shape recorded as release-gate Finding 1. */
  test("returns svg instead of throwing when localStorage is unavailable", () => {
    expect(readIconLib(throwingStorage(new Error("SecurityError: storage is disabled")))).toBe("svg");
  });

  test("survives a quota error from getItem", () => {
    expect(readIconLib(throwingStorage(new Error("QuotaExceededError")))).toBe("svg");
  });
});
