import { describe, expect, test } from "bun:test";
import { capturePointFromClient } from "../src/lib/nativeCapture";

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
});
