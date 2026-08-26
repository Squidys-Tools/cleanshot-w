import { useEffect, useState, type ComponentType } from "react";

import * as Lucide from "lucide-react";
import * as Phosphor from "@phosphor-icons/react";
import * as Tabler from "@tabler/icons-react";
import * as Heroicons from "@heroicons/react/24/outline";
import * as Remix from "@remixicon/react";

export type IconLibraryId = "svg" | "lucide" | "phosphor" | "tabler" | "heroicons" | "remix";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type IconMap = Record<string, ComponentType<any>>;

// ---- Per-library name mappings (our tool name → library export) ----

const LUCIDE: IconMap = {
  select: Lucide.MousePointer2,
  highlight: Lucide.Highlighter,
  draw: Lucide.Pencil,
  arrow: Lucide.ArrowUpRight,
  text: Lucide.Type,
  counter: Lucide.Hash,
  blur: Lucide.Droplet,
  redact: Lucide.Shield,
  eraser: Lucide.Eraser,
  pan: Lucide.Hand,
  line: Lucide.Minus,
  note: Lucide.StickyNote,
  frame: Lucide.Frame,
  laser: Lucide.Sparkle,
  crop: Lucide.Crop,
  rect: Lucide.Square,
  ellipse: Lucide.Circle,
  diamond: Lucide.Diamond,
  triangle: Lucide.Triangle,
  heart: Lucide.Heart,
  star: Lucide.Star,
  cloud: Lucide.Cloud,
  "check-box": Lucide.SquareCheck,
  "arrow-right": Lucide.ArrowRight,
  "arrow-left": Lucide.ArrowLeft,
  "arrow-up": Lucide.ArrowUp,
  "arrow-down": Lucide.ArrowDown,
  more: Lucide.Ellipsis,
  "chevron-down": Lucide.ChevronDown,
  undo: Lucide.Undo2,
  redo: Lucide.Redo2,
  rhombus: Lucide.Diamond,
  "rhombus-2": Lucide.Diamond,
  pentagon: Lucide.Pentagon,
  hexagon: Lucide.Hexagon,
  octagon: Lucide.Octagon,
};

const PHOSPHOR: IconMap = {
  select: Phosphor.CursorClick,
  highlight: Phosphor.Highlighter,
  draw: Phosphor.PencilSimple,
  arrow: Phosphor.ArrowUpRight,
  text: Phosphor.TextT,
  counter: Phosphor.NumberCircleOne,
  blur: Phosphor.EyeSlash,
  redact: Phosphor.Shield,
  eraser: Phosphor.Eraser,
  pan: Phosphor.HandGrabbing,
  line: Phosphor.Minus,
  note: Phosphor.Note,
  frame: Phosphor.FrameCorners,
  laser: Phosphor.Sparkle,
  crop: Phosphor.Crop,
  rect: Phosphor.Rectangle,
  ellipse: Phosphor.Circle,
  diamond: Phosphor.Diamond,
  triangle: Phosphor.Triangle,
  heart: Phosphor.Heart,
  star: Phosphor.Star,
  cloud: Phosphor.Cloud,
  "check-box": Phosphor.CheckSquare,
  "arrow-right": Phosphor.ArrowRight,
  "arrow-left": Phosphor.ArrowLeft,
  "arrow-up": Phosphor.ArrowUp,
  "arrow-down": Phosphor.ArrowDown,
  more: Phosphor.DotsThree,
  "chevron-down": Phosphor.CaretDown,
  undo: Phosphor.ArrowCounterClockwise,
  redo: Phosphor.ArrowClockwise,
  rhombus: Phosphor.Diamond,
  "rhombus-2": Phosphor.Diamond,
  pentagon: Phosphor.Pentagon,
  hexagon: Phosphor.Hexagon,
  octagon: Phosphor.Octagon,
};

const TABLER: IconMap = {
  select: Tabler.IconPointer,
  highlight: Tabler.IconHighlight,
  draw: Tabler.IconPencil,
  arrow: Tabler.IconArrowUpRight,
  text: Tabler.IconLetterT,
  counter: Tabler.IconNumber,
  blur: Tabler.IconEyeOff,
  redact: Tabler.IconShield,
  eraser: Tabler.IconEraser,
  pan: Tabler.IconHandGrab,
  line: Tabler.IconMinus,
  note: Tabler.IconNote,
  frame: Tabler.IconFrame,
  laser: Tabler.IconSparkle,
  crop: Tabler.IconCrop,
  rect: Tabler.IconRectangle,
  ellipse: Tabler.IconCircle,
  diamond: Tabler.IconDiamond,
  triangle: Tabler.IconTriangle,
  heart: Tabler.IconHeart,
  star: Tabler.IconStar,
  cloud: Tabler.IconCloud,
  "check-box": Tabler.IconCheckbox,
  "arrow-right": Tabler.IconArrowRight,
  "arrow-left": Tabler.IconArrowLeft,
  "arrow-up": Tabler.IconArrowUp,
  "arrow-down": Tabler.IconArrowDown,
  more: Tabler.IconDots,
  "chevron-down": Tabler.IconChevronDown,
  undo: Tabler.IconArrowBackUp,
  redo: Tabler.IconArrowForwardUp,
  rhombus: Tabler.IconDiamond,
  "rhombus-2": Tabler.IconDiamond,
  pentagon: Tabler.IconPentagon,
  hexagon: Tabler.IconHexagon,
  octagon: Tabler.IconOctagon,
};

const HEROICONS: IconMap = {
  select: Heroicons.CursorArrowRaysIcon,
  highlight: Heroicons.PaintBrushIcon,
  draw: Heroicons.PencilIcon,
  arrow: Heroicons.ArrowUpRightIcon,
  counter: Heroicons.HashtagIcon,
  blur: Heroicons.EyeSlashIcon,
  redact: Heroicons.LockClosedIcon,
  pan: Heroicons.HandRaisedIcon,
  line: Heroicons.MinusIcon,
  note: Heroicons.PencilSquareIcon,
  laser: Heroicons.SparklesIcon,
  crop: Heroicons.ViewfinderCircleIcon,
  heart: Heroicons.HeartIcon,
  star: Heroicons.StarIcon,
  cloud: Heroicons.CloudIcon,
  "check-box": Heroicons.CheckCircleIcon,
  "arrow-right": Heroicons.ArrowRightIcon,
  "arrow-left": Heroicons.ArrowLeftIcon,
  "arrow-up": Heroicons.ArrowUpIcon,
  "arrow-down": Heroicons.ArrowDownIcon,
  more: Heroicons.EllipsisHorizontalIcon,
  "chevron-down": Heroicons.ChevronDownIcon,
  undo: Heroicons.ArrowUturnLeftIcon,
  redo: Heroicons.ArrowUturnRightIcon,
};

const REMIX: IconMap = {
  select: Remix.RiCursorLine,
  highlight: Remix.RiMarkPenLine,
  draw: Remix.RiPencilLine,
  arrow: Remix.RiArrowRightUpLine,
  text: Remix.RiText,
  counter: Remix.RiNumber1,
  blur: Remix.RiEyeOffLine,
  redact: Remix.RiShieldLine,
  eraser: Remix.RiEraserLine,
  pan: Remix.RiDragMoveLine,
  line: Remix.RiScissorsLine,
  note: Remix.RiStickyNoteLine,
  laser: Remix.RiSparkling2Line,
  crop: Remix.RiCropLine,
  rect: Remix.RiRectangleLine,
  ellipse: Remix.RiCircleLine,
  diamond: Remix.RiDiamondLine,
  triangle: Remix.RiTriangleLine,
  heart: Remix.RiHeartLine,
  star: Remix.RiStarLine,
  cloud: Remix.RiCloudLine,
  "check-box": Remix.RiCheckboxLine,
  "arrow-right": Remix.RiArrowRightLine,
  "arrow-left": Remix.RiArrowLeftLine,
  "arrow-up": Remix.RiArrowUpLine,
  more: Remix.RiMoreLine,
  "chevron-down": Remix.RiArrowDownSLine,
  undo: Remix.RiArrowGoBackLine,
  redo: Remix.RiArrowGoForwardLine,
};

// ---- Lookup ----

const LIBRARY_MAP: Record<IconLibraryId, IconMap> = {
  svg: {}, // empty — fallback to hand-drawn SVG
  lucide: LUCIDE,
  phosphor: PHOSPHOR,
  tabler: TABLER,
  heroicons: HEROICONS,
  remix: REMIX,
};

export const LIBRARY_LABELS: Record<IconLibraryId, string> = {
  svg: "SVG",
  lucide: "Lucide",
  phosphor: "Phosphor",
  tabler: "Tabler",
  heroicons: "Heroicons",
  remix: "Remix",
};

export const LIBRARY_ORDER: IconLibraryId[] = ["svg", "lucide", "phosphor", "tabler", "heroicons", "remix"];

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getIconComponent(library: IconLibraryId, name: string): ComponentType<any> | null {
  const lib = LIBRARY_MAP[library];
  return lib?.[name] ?? null;
}

// ---- Shared library state (synced across components via custom event) ----

const LIB_EVENT = "cs-icon-lib-change";

export function readIconLib(): IconLibraryId {
  return (localStorage.getItem("cs-icon-lib") as IconLibraryId) || "svg";
}

function writeIconLib(lib: IconLibraryId) {
  localStorage.setItem("cs-icon-lib", lib);
  window.dispatchEvent(new CustomEvent(LIB_EVENT, { detail: lib }));
}

export function cycleIconLib(current: IconLibraryId): IconLibraryId {
  const idx = LIBRARY_ORDER.indexOf(current);
  return LIBRARY_ORDER[(idx + 1) % LIBRARY_ORDER.length];
}

export function useIconLib(): [IconLibraryId, (lib: IconLibraryId) => void] {
  const [lib, setLib] = useState<IconLibraryId>(readIconLib);
  useEffect(() => {
    const on = (e: Event) => setLib((e as CustomEvent).detail as IconLibraryId);
    window.addEventListener(LIB_EVENT, on);
    return () => window.removeEventListener(LIB_EVENT, on);
  }, []);
  return [lib, writeIconLib];
}
