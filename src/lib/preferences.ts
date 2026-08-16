import {
  ArrowShapeArrowheadEndStyle,
  ArrowShapeArrowheadStartStyle,
  ArrowShapeKindStyle,
  DefaultColorStyle,
  DefaultDashStyle,
  DefaultFillStyle,
  DefaultFontStyle,
  DefaultSizeStyle,
  GeoShapeGeoStyle,
  type Editor,
  type SharedStyle,
} from "@tldraw/editor";
import type {
  TLArrowShapeArrowheadStyle,
  TLArrowShapeKind,
  TLDefaultColorStyle,
  TLDefaultDashStyle,
  TLDefaultFillStyle,
  TLDefaultFontStyle,
  TLDefaultSizeStyle,
  TLGeoShapeGeoStyle,
} from "@tldraw/tlschema";

const STORAGE_KEY = "cleanshotw.editor-preferences.v1";

const COLORS = ["red", "orange", "yellow", "green", "blue", "violet", "black", "grey"] as const;
const SIZES = ["s", "m", "l", "xl"] as const;
const FILLS = ["none", "semi", "solid", "pattern"] as const;
const DASHES = ["solid", "dashed", "dotted", "draw"] as const;
const FONTS = ["sans", "serif", "mono", "draw"] as const;
const ARROWHEADS = ["none", "arrow", "triangle", "bar", "dot", "pipe", "diamond", "inverted", "square"] as const;
const ARROW_KINDS = ["arc", "elbow"] as const;
const GEO_SHAPES = [
  "rectangle",
  "ellipse",
  "oval",
  "diamond",
  "triangle",
  "trapezoid",
  "rhombus",
  "rhombus-2",
  "pentagon",
  "hexagon",
  "octagon",
  "star",
  "heart",
  "cloud",
  "check-box",
  "arrow-right",
  "arrow-left",
  "arrow-up",
  "arrow-down",
  "x-box",
] as const;

export type EditorPreferences = {
  color: TLDefaultColorStyle;
  size: TLDefaultSizeStyle;
  fill: TLDefaultFillStyle;
  dash: TLDefaultDashStyle;
  font: TLDefaultFontStyle;
  opacity: number;
  arrowStart: TLArrowShapeArrowheadStyle;
  arrowEnd: TLArrowShapeArrowheadStyle;
  arrowKind: TLArrowShapeKind;
  geo: TLGeoShapeGeoStyle;
  snap: boolean;
  grid: boolean;
  dark: boolean;
};

export const DEFAULT_EDITOR_PREFERENCES: EditorPreferences = {
  color: "red",
  size: "m",
  fill: "none",
  dash: "solid",
  font: "sans",
  opacity: 1,
  arrowStart: "none",
  arrowEnd: "arrow",
  arrowKind: "arc",
  geo: "rectangle",
  snap: false,
  grid: false,
  dark: true,
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function oneOf<const T extends readonly string[]>(values: T, value: unknown): value is T[number] {
  return typeof value === "string" && values.includes(value);
}

function booleanOr(value: unknown, fallback: boolean): boolean {
  return typeof value === "boolean" ? value : fallback;
}

function opacityOr(value: unknown, fallback: number): number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1 ? value : fallback;
}

export function parseEditorPreferences(serialized: string | null): EditorPreferences {
  if (!serialized) return { ...DEFAULT_EDITOR_PREFERENCES };
  try {
    const value: unknown = JSON.parse(serialized);
    if (!isRecord(value)) return { ...DEFAULT_EDITOR_PREFERENCES };
    return {
      color: oneOf(COLORS, value.color) ? value.color : DEFAULT_EDITOR_PREFERENCES.color,
      size: oneOf(SIZES, value.size) ? value.size : DEFAULT_EDITOR_PREFERENCES.size,
      fill: oneOf(FILLS, value.fill) ? value.fill : DEFAULT_EDITOR_PREFERENCES.fill,
      dash: oneOf(DASHES, value.dash) ? value.dash : DEFAULT_EDITOR_PREFERENCES.dash,
      font: oneOf(FONTS, value.font) ? value.font : DEFAULT_EDITOR_PREFERENCES.font,
      opacity: opacityOr(value.opacity, DEFAULT_EDITOR_PREFERENCES.opacity),
      arrowStart: oneOf(ARROWHEADS, value.arrowStart) ? value.arrowStart : DEFAULT_EDITOR_PREFERENCES.arrowStart,
      arrowEnd: oneOf(ARROWHEADS, value.arrowEnd) ? value.arrowEnd : DEFAULT_EDITOR_PREFERENCES.arrowEnd,
      arrowKind: oneOf(ARROW_KINDS, value.arrowKind) ? value.arrowKind : DEFAULT_EDITOR_PREFERENCES.arrowKind,
      geo: oneOf(GEO_SHAPES, value.geo) ? value.geo : DEFAULT_EDITOR_PREFERENCES.geo,
      snap: booleanOr(value.snap, DEFAULT_EDITOR_PREFERENCES.snap),
      grid: booleanOr(value.grid, DEFAULT_EDITOR_PREFERENCES.grid),
      dark: booleanOr(value.dark, DEFAULT_EDITOR_PREFERENCES.dark),
    };
  } catch {
    return { ...DEFAULT_EDITOR_PREFERENCES };
  }
}

export function readEditorPreferences(storage: Storage = window.localStorage): EditorPreferences {
  try {
    return parseEditorPreferences(storage.getItem(STORAGE_KEY));
  } catch {
    return { ...DEFAULT_EDITOR_PREFERENCES };
  }
}

export function writeEditorPreferences(preferences: EditorPreferences, storage: Storage = window.localStorage): void {
  try {
    storage.setItem(STORAGE_KEY, JSON.stringify(preferences));
  } catch {
    // Private browsing and blocked storage should not stop editing.
  }
}

export function readEditorPreferencesFromEditor(editor: Editor): EditorPreferences {
  const opacity = editor.getSharedOpacity();
  const saved = readEditorPreferences();
  return {
    color: editor.getStyleForNextShape(DefaultColorStyle),
    size: editor.getStyleForNextShape(DefaultSizeStyle),
    fill: editor.getStyleForNextShape(DefaultFillStyle),
    dash: editor.getStyleForNextShape(DefaultDashStyle),
    font: editor.getStyleForNextShape(DefaultFontStyle),
    // A mixed selection describes the selected shapes, not the next-shape
    // default. Keep the saved default when the selection has mixed opacity.
    opacity: opacityForPreference(opacity, saved.opacity),
    arrowStart: editor.getStyleForNextShape(ArrowShapeArrowheadStartStyle),
    arrowEnd: editor.getStyleForNextShape(ArrowShapeArrowheadEndStyle),
    arrowKind: editor.getStyleForNextShape(ArrowShapeKindStyle),
    geo: editor.getStyleForNextShape(GeoShapeGeoStyle),
    snap: editor.user.getIsSnapMode(),
    grid: editor.getInstanceState().isGridMode,
    dark: editor.user.getUserPreferences().colorScheme === "dark",
  };
}

export function persistEditorPreferences(editor: Editor): void {
  writeEditorPreferences(readEditorPreferencesFromEditor(editor));
}

export function opacityForPreference(shared: SharedStyle<number>, savedOpacity: number): number {
  return shared.type === "mixed" ? savedOpacity : shared.value;
}

export function applyEditorPreferences(editor: Editor, preferences: EditorPreferences): void {
  editor.run(() => {
    editor.setStyleForNextShapes(DefaultColorStyle, preferences.color);
    editor.setStyleForNextShapes(DefaultSizeStyle, preferences.size);
    editor.setStyleForNextShapes(DefaultFillStyle, preferences.fill);
    editor.setStyleForNextShapes(DefaultDashStyle, preferences.dash);
    editor.setStyleForNextShapes(DefaultFontStyle, preferences.font);
    editor.setOpacityForNextShapes(preferences.opacity);
    editor.setStyleForNextShapes(ArrowShapeArrowheadStartStyle, preferences.arrowStart);
    editor.setStyleForNextShapes(ArrowShapeArrowheadEndStyle, preferences.arrowEnd);
    editor.setStyleForNextShapes(ArrowShapeKindStyle, preferences.arrowKind);
    editor.setStyleForNextShapes(GeoShapeGeoStyle, preferences.geo);
    editor.user.updateUserPreferences({ colorScheme: preferences.dark ? "dark" : "light", isSnapMode: preferences.snap });
    editor.updateInstanceState({ isGridMode: preferences.grid });
  });
}
