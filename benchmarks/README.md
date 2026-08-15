# AI and Extraction Benchmarks

This directory contains the representative corpus and evaluation notes used to choose OCR, extraction, embedding, vision, and local language models.

The corpus should contain only files and URLs that we are allowed to use for local testing. Do not commit private or copyrighted material to the repository unless we have permission.

## Structure

```text
benchmarks/
  README.md
  manifest.json
  generate-corpus.ps1
  harness/
    index.ts
    manifest.ts
    server.ts
    extract.ts
    pdf.ts
    ocr.ts
    score.ts
    win-ocr.ps1
  corpus/
    articles/
    screenshots/
    images/
    pdfs/
    videos/
    notes/
    edge-cases/
  expected/
    ocr/
    extraction/
    search/
    similarity/
  results/
```

## Corpus checklist

### Articles

- Short blog post
- Long-form essay
- News article
- Recipe
- Technical documentation page
- Article with footnotes
- Article with code blocks
- Article with an image gallery
- JavaScript-heavy article
- Page with ads and sticky navigation
- Paywalled or inaccessible page for failure handling

### Images and screenshots

- Photograph
- Design reference
- Screenshot with large text
- Screenshot with small text
- Meme
- Handwriting
- Multiple text columns
- Low-resolution image
- Rotated text
- Image with a visually similar partner
- Image with a visually different distractor

### PDFs

- Native-text PDF
- Scanned PDF
- Multi-column PDF
- PDF with tables
- PDF with images and captions
- PDF with bad or missing metadata

### Video and embeds

- YouTube page
- Vimeo page
- Article with an embedded video
- Page with an unsupported iframe
- Page with no playable media

### Notes and quotes

- Quick note
- Long note
- Note with Markdown
- Note with a todo
- Quote with a source URL
- Quote without a source URL

## Required manifest fields

Each corpus item should have a stable ID, relative path or URL, content type, language, expected extraction behavior, expected OCR text when applicable, search terms that should match, search terms that should not match, similarity group when applicable, and notes about known edge cases.

## Evaluation metrics

- OCR character and word accuracy
- Article extraction completeness
- Metadata accuracy
- Search recall and precision
- Semantic search relevance
- Image similarity relevance
- Summary usefulness
- CPU time and peak memory
- Model download size
- Cold-start time
- Batch processing time

## Workflow

1. Add files and URLs to `corpus/`.
2. Record them in `manifest.json`.
3. Add expected outputs under `expected/`.
4. Run the benchmark harness.
5. Store machine and model information with results.
6. Compare candidates before selecting production models.

## Running the harness

Requires [Bun](https://bun.sh) (the harness and the ingestion pipeline are
TypeScript). From the repo root:

```sh
bun benchmarks/harness/index.ts   # or: bun run bench
```

What it does:

1. Serves `corpus/` over a local HTTP server (`harness/server.ts`) so the
   fixture pages are fetched exactly like live pages.
2. For `article` / `recipe` / `video` items, runs the production extraction
   pipeline (`ingestUrl` → Defuddle → sanitize) via `harness/extract.ts`.
3. For PDFs, runs a minimal content-stream text extractor
   (`harness/pdf.ts`; label `naive-streams`). Real-world PDFs need a full
   parser in a later milestone. Scanned PDFs fall back to OCR on their
   embedded JPEG (`extractFirstEmbeddedJpeg`).
4. For OCR items (screenshots, text images, scanned PDFs), uses a pluggable
   OCR engine (`harness/ocr.ts`). The default is **Windows built-in OCR**
   (`Windows.Media.Ocr`), invoked through Windows PowerShell 5.1 via
   `harness/win-ocr.ps1` — zero-install on any Windows machine with the OCR
   language pack. A `tesseract.js` runner is the fallback.
5. Scores each item against `expected` (title/author/search terms/must-not
   match/embeds/image counts) and writes `results/results-latest.json`, a
   timestamped copy, and `results/summary.md`.

Scoring notes: search terms are matched across the extracted
title + description + text, mirroring how a saved item would be searched.
OCR items are scored with token recall + precision against `expected/ocr/`.

## Initial findings

Baseline run on the 42-item corpus with the `windows-ocr` engine:

- **25 pass, 8 partial, 1 fail, 8 skip — overall 0.941.**
- 8 skips: vision / similarity items (photos, design refs, similar pairs,
  distractor, low-res) — these need the embeddings benchmark.
- Genuine gaps surfaced by the partials and fail:
  - **Byline author parsing** (`article-news-01`, `article-recipe-01`):
    `By Sam Okonkwo, City Desk` and `Recipe by June Park · Serves 2` are not
    recognized as authors, while `By Devon Ruiz` is. Defuddle's byline
    handling is brittle around suffixes and prefixes.
  - **JS-rendered pages** (`article-js-heavy-01`): sanitization strips
    `<noscript>`, so a client-side-rendered page yields only title +
    description and an empty body. Executing JS (or preserving noscript
    fallback text) is required to recover content.
  - **Windows OCR on stylized text** (`image-meme-01`, fail): returns no
    text for an Impact-style all-caps meme even at high contrast. A
    different engine (Tesseract) would likely read it — a useful engine
    comparison signal.
  - **OCR noise on real content**: small-text screenshot 0.88 recall,
    rotated headline 0.81 recall, scanned typewritten PDF 0.75 (term
    "sprouted bulbs" mangled), two-column 0.98 precision.

These are documented as findings, not fixture errors — the fixtures
deliberately model realistic pages.

