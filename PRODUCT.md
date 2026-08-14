# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Stack

delegated: Windows-first Tauri desktop application with a React/TypeScript frontend, Rust native core, and SQLite local storage; keep domain boundaries portable for a future web version.

## Users

People who collect articles, images, screenshots, notes, PDFs, quotes, and references and want to retrieve them later without maintaining a filing system.

## Product Purpose

Provide a private visual library for saving anything worth remembering and finding it later through search, visual browsing, semantic connections, and automatic collections.

## Positioning

Save without organizing. The product treats search and associative memory as the organizing system instead of requiring users to maintain folders and metadata.

## Operating Context

The primary experience is a Windows desktop application used during browsing, research, creative work, writing, and everyday information capture. Users should be able to capture quickly, continue working while processing happens in the background, and rediscover older material through visual browsing.

## Capabilities and Constraints

- Support URLs, articles, images, screenshots, PDFs, notes, quotes, and video links.
- Render specialized card types rather than generic bookmarks.
- Provide exact, OCR, structured, semantic, and visual search over local content.
- Provide manual and automatically updated Spaces.
- Provide article reading, Focus Mode, Serendipity, and Top of Mind over time.
- Use Tesseract for OCR and Defuddle for article extraction.
- Prefer local processing on Windows and support CPU-only operation.
- AI-derived concepts are invisible implementation details; do not expose correction, deletion, confidence, or AI-management workflows.
- Explain local-first behavior during onboarding and settings without making it dominant in the main interface.

## Brand Commitments

- Product name: mymind library.
- Quiet, visual, private, and low-maintenance experience.
- Inspired by the product behavior and experience of mymind, while using original implementation and assets.

## Evidence on Hand

- Product behavior specification: `docs/product-behavior-spec.md`
- Technical decisions: `docs/technical-decision-record.md`
- Benchmark plan: `benchmarks/README.md`
- The current repository is an early Tauri scaffold, not an established visual system.

## Product Principles

- Capture must be faster than deciding where something belongs.
- Search is the primary organizing system.
- Content should receive a visual treatment appropriate to its type.
- AI should improve retrieval quietly in the background.
- The library should encourage rediscovery, not only storage.

## Accessibility & Inclusion

- Support keyboard-first operation for capture, navigation, search, and focus states.
- Maintain readable contrast and visible focus indicators.
- Do not rely on color alone to communicate item type or state.
