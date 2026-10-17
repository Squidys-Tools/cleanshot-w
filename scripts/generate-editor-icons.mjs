import { copyFile, mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { chromium } from "playwright";
import * as Lucide from "lucide-react";
import * as Phosphor from "@phosphor-icons/react";
import * as Tabler from "@tabler/icons-react";
import * as Heroicons from "@heroicons/react/24/outline";
import * as Remix from "@remixicon/react";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = resolve(root, "app/cleanshot/assets/icons");

const glyphs = {
  arrow_right: {
    label: "Arrow right",
    lucide: Lucide.ArrowRight,
    phosphor: Phosphor.ArrowRight,
    tabler: Tabler.IconArrowRight,
    heroicons: Heroicons.ArrowRightIcon,
    remix: Remix.RiArrowRightLine,
  },
  check: {
    label: "Check",
    lucide: Lucide.Check,
    phosphor: Phosphor.Check,
    tabler: Tabler.IconCheck,
    heroicons: Heroicons.CheckIcon,
    remix: Remix.RiCheckLine,
  },
  close: {
    label: "Close",
    lucide: Lucide.X,
    phosphor: Phosphor.X,
    tabler: Tabler.IconX,
    heroicons: Heroicons.XMarkIcon,
    remix: Remix.RiCloseLine,
  },
  info: {
    label: "Information",
    lucide: Lucide.Info,
    phosphor: Phosphor.Info,
    tabler: Tabler.IconInfoCircle,
    heroicons: Heroicons.InformationCircleIcon,
    remix: Remix.RiInformationLine,
  },
  warning: {
    label: "Warning",
    lucide: Lucide.TriangleAlert,
    phosphor: Phosphor.WarningCircle,
    tabler: Tabler.IconAlertTriangle,
    heroicons: Heroicons.ExclamationTriangleIcon,
    remix: Remix.RiErrorWarningLine,
  },
  question: {
    label: "Question",
    lucide: Lucide.CircleHelp,
    phosphor: Phosphor.Question,
    tabler: Tabler.IconHelpCircle,
    heroicons: Heroicons.QuestionMarkCircleIcon,
    remix: Remix.RiQuestionLine,
  },
  star: {
    label: "Star",
    lucide: Lucide.Star,
    phosphor: Phosphor.Star,
    tabler: Tabler.IconStar,
    heroicons: Heroicons.StarIcon,
    remix: Remix.RiStarLine,
  },
  heart: {
    label: "Heart",
    lucide: Lucide.Heart,
    phosphor: Phosphor.Heart,
    tabler: Tabler.IconHeart,
    heroicons: Heroicons.HeartIcon,
    remix: Remix.RiHeartLine,
  },
  lock: {
    label: "Lock",
    lucide: Lucide.Lock,
    phosphor: Phosphor.Lock,
    tabler: Tabler.IconLock,
    heroicons: Heroicons.LockClosedIcon,
    remix: Remix.RiLockLine,
  },
  eye: {
    label: "Eye",
    lucide: Lucide.Eye,
    phosphor: Phosphor.Eye,
    tabler: Tabler.IconEye,
    heroicons: Heroicons.EyeIcon,
    remix: Remix.RiEyeLine,
  },
  camera: {
    label: "Camera",
    lucide: Lucide.Camera,
    phosphor: Phosphor.Camera,
    tabler: Tabler.IconCamera,
    heroicons: Heroicons.CameraIcon,
    remix: Remix.RiCameraLine,
  },
  user: {
    label: "Person",
    lucide: Lucide.User,
    phosphor: Phosphor.User,
    tabler: Tabler.IconUser,
    heroicons: Heroicons.UserIcon,
    remix: Remix.RiUserLine,
  },
};

const licenses = {
  lucide: "node_modules/lucide-react/LICENSE",
  phosphor: "node_modules/@phosphor-icons/react/LICENSE",
  tabler: "node_modules/@tabler/icons-react/LICENSE",
  heroicons: "node_modules/@heroicons/react/LICENSE",
};

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({
    viewport: { width: 64, height: 64 },
    deviceScaleFactor: 1,
  });
  await page.setContent(
    "<html><head><style>html,body{margin:0;width:64px;height:64px}body{display:grid;place-items:center;background:transparent}svg{display:block;width:52px!important;height:52px!important}</style></head><body></body></html>",
  );

  for (const [pack, licensePath] of Object.entries(licenses)) {
    const path = resolve(output, "licenses", `${pack}.LICENSE`);
    await mkdir(dirname(path), { recursive: true });
    await copyFile(resolve(root, licensePath), path);
  }

  for (const [glyph, packs] of Object.entries(glyphs)) {
    for (const [pack, Icon] of Object.entries(packs)) {
      if (pack === "label") continue;
      const props = {
        width: 56,
        height: 56,
        size: 56,
        color: "#111827",
        strokeWidth: 2,
        weight: "regular",
        style: { color: "#111827" },
      };
      const markup = renderToStaticMarkup(React.createElement(Icon, props));
      await page.locator("body").evaluate((body, svg) => {
        body.innerHTML = svg;
      }, markup);
      const path = resolve(output, pack, `${glyph}.png`);
      await mkdir(dirname(path), { recursive: true });
      await page.locator("svg").screenshot({ path, omitBackground: true });
    }
  }

  const packVariants = ["lucide", "phosphor", "tabler", "heroicons", "remix"];
  const assetRows = packVariants.flatMap((pack) =>
    Object.keys(glyphs).map((glyph) => {
      const packVariant = pack[0].toUpperCase() + pack.slice(1);
      const glyphVariant = glyph
        .split("_")
        .map((part) => part[0].toUpperCase() + part.slice(1))
        .join("");
      return `    (IconPack::${packVariant}, IconGlyph::${glyphVariant}, include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/${pack}/${glyph}.png"))),`;
    }),
  );
  const generated = [
    "pub(super) const ICON_ASSETS: &[(IconPack, IconGlyph, &[u8])] = &[",
    ...assetRows,
    "];",
    "",
  ].join("\n");
  await writeFile(resolve(root, "app/cleanshot/src/editor/icon_assets.rs"), generated);
} finally {
  await browser.close();
}
