import { describe, expect, test } from "bun:test";
import { DEFAULT_EDITOR_PREFERENCES, parseEditorPreferences } from "../src/lib/preferences";

describe("editor preferences", () => {
  test("uses defaults for missing or malformed storage", () => {
    expect(parseEditorPreferences(null)).toEqual(DEFAULT_EDITOR_PREFERENCES);
    expect(parseEditorPreferences("not json")).toEqual(DEFAULT_EDITOR_PREFERENCES);
    expect(parseEditorPreferences(JSON.stringify({ color: "not-a-color", opacity: 4 }))).toEqual(DEFAULT_EDITOR_PREFERENCES);
  });

  test("accepts valid values and keeps defaults for unknown fields", () => {
    expect(
      parseEditorPreferences(
        JSON.stringify({ color: "green", size: "xl", opacity: 0.5, dark: false, grid: true, future: "ignored" }),
      ),
    ).toEqual({
      ...DEFAULT_EDITOR_PREFERENCES,
      color: "green",
      size: "xl",
      opacity: 0.5,
      dark: false,
      grid: true,
    });
  });
});
