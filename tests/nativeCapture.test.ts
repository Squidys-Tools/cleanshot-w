import { describe, expect, test } from "bun:test";
import { capturePointFromClient, nativeErrorMessage } from "../src/lib/nativeCapture";

describe("native capture coordinates", () => {
  test("maps viewport CSS coordinates into physical capture pixels", () => {
    expect(
      capturePointFromClient(
        490,
        290,
        { left: 10, top: 20, width: 960, height: 540 },
        { width: 1920, height: 1080 },
      ),
    ).toEqual({ x: 960, y: 540 });
  });

  test("clamps pointer coordinates to the captured surface", () => {
    expect(
      capturePointFromClient(
        -100,
        900,
        { left: 10, top: 20, width: 960, height: 540 },
        { width: 1920, height: 1080 },
      ),
    ).toEqual({ x: 0, y: 1080 });
  });

  test("normalizes native errors from common Tauri rejection shapes", () => {
    expect(nativeErrorMessage("The overlay could not open", "fallback")).toBe("The overlay could not open");
    expect(nativeErrorMessage({ message: "The window is unavailable" }, "fallback")).toBe("The window is unavailable");
    expect(nativeErrorMessage({ error: "Clipboard access was denied" }, "fallback")).toBe("Clipboard access was denied");
    expect(nativeErrorMessage({ reason: "unknown" }, "fallback")).toBe("fallback");
  });
});
