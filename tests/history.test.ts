import { describe, expect, test } from "bun:test";
import { filterCaptureRecords, normalizeCaptureTitle } from "../src/lib/history";

describe("capture history", () => {
  test("normalizes editable titles by trimming surrounding whitespace", () => {
    expect(normalizeCaptureTitle("  Bug report  ")).toBe("Bug report");
    expect(normalizeCaptureTitle("   ")).toBe("");
  });

  test("filters titles case-insensitively and preserves record order", () => {
    const records = [{ title: "Release notes" }, { title: "Bug report" }, { title: "Design review" }];
    expect(filterCaptureRecords(records, "BUG")).toEqual([{ title: "Bug report" }]);
    expect(filterCaptureRecords(records, "  ")).toEqual(records);
  });
});
