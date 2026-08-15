# AI and Extraction Benchmarks

This directory contains the representative corpus and evaluation notes used to choose OCR, extraction, embedding, vision, and local language models.

The corpus should contain only files and URLs that we are allowed to use for local testing. Do not commit private or copyrighted material to the repository unless we have permission.

## Structure

```text
benchmarks/
  README.md
  manifest.json
  generate-corpus.ps1
  fetch-macrumors.ts
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
    live/macrumors/
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
- Latest MacRumors article (live web page, fetched by `fetch-macrumors.ts`)

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
4. For OCR items (screenshots, text images, scanned PDFs), runs **every
   available engine** (`harness/ocr.ts`) and keeps the best-scoring result:
   **Windows built-in OCR** (`Windows.Media.Ocr`, invoked via
   `harness/win-ocr.ps1` — zero-install on Windows) and a **tesseract.js**
   runner (multi-PSM 3/6/7, highest-confidence pass wins).
5. Scores each item against `expected` (title/author/search terms/must-not
   match/embeds/image counts) and writes `results/results-latest.json`, a
   timestamped copy, and `results/summary.md`.

Scoring notes: search terms are matched across the extracted
title + description + text (whitespace-normalized on both sides), mirroring
how a saved item would be searched. OCR items are scored with token recall +
precision against `expected/ocr/`.

## Initial findings

Baseline run on the 42-item corpus with the `windows-ocr` engine:

- **25 pass, 8 partial, 1 fail, 8 skip — overall 0.940.**
- 8 skips: vision / similarity items (photos, design refs, similar pairs,
  distractor, low-res) — these need the embeddings benchmark.

After fixes (JSON-LD article metadata, `<noscript>` fallback text, author
byline heuristics, a multi-engine OCR harness, and more legible generated
meme/scanned-PDF fixtures), the same 42 items plus 6 live MacRumors pages:

- **36 pass, 4 partial, 0 fail, 8 skip — overall 0.992.**
- All 6 live-article fixtures pass at 1.000, verifying extraction against
  real-world macrumors.com markup (title, author, and search terms all match;
  authors come from the articles' own JSON-LD).
- Fixed by the above:
  - `article-news-01` / `article-recipe-01` (byline authors now recognized),
    `article-js-heavy-01` (noscript fallback text restored), `image-meme-01`
    (0.93, was fail), `pdf-scanned-01` (full memo OCR'd in order).
- Remaining partials are genuine engine limits, not fixture errors:
  - `screenshot-large-text-01` 0.97 and `screenshot-small-text-01` 0.86
    (Windows OCR misses a few dense tokens; the small-text one is a dense
    table, 110 tokens).
  - `image-rotated-01` 0.92 (Windows OCR misses 6 of 36 rotated tokens).
  - `image-meme-01` 0.93 (OCR merges some words; stylized Impact text).
- Engine split observed: tesseract.js wins on the two-column layout and the
  scanned PDF; Windows OCR wins on the meme, handwriting, rotated image, and
  screenshots. Best-of-multi-engine is therefore the right default.

Fixtures deliberately model realistic pages; these residual gaps are
documented as findings.

## Refreshing fixtures

```sh
# Regenerate deterministic images/screenshots/PDFs and expected/ocr/*.txt
pwsh -File benchmarks/generate-corpus.ps1

# Fetch the N (default 6) newest MacRumors articles as live fixtures
bun benchmarks/fetch-macrumors.ts 6
```

`fetch-macrumors.ts` re-runs are incremental: it skips articles already in
the manifest, rewrites only the `article-live-macrumors-*` entries, and
verifies every fixture through the production ingestion pipeline before
updating the manifest. Delete the `corpus/live/macrumors/*.html` files and
the `article-live-macrumors-*` manifest entries to drop them.

