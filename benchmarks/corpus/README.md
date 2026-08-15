# Corpus

Representative test material for the extraction and AI benchmarks.

## What is here

All files in this corpus are **original and generated locally** — no private,
personal, or unlicensed copyrighted material. Images, screenshots, and PDFs
are produced deterministically by `../generate-corpus.ps1`; article, video,
and note fixtures are hand-written HTML/Markdown stored in this folder.

## Layout

- `articles/` — extraction fixtures: short blog, long essay, news, recipe,
  technical docs, footnotes, code blocks, image gallery, JS-rendered page,
  ad-heavy page, and a paywalled page.
- `images/` — concept/OCR/similarity images, including two visually similar
  pairs (`image-similar-0{1,2}` in `moodboard-a`, `image-similar-0{3,4}` in
  `moodboard-b`) and a distractor (`image-distractor-01`).
- `screenshots/` — large-text and small-text (dense table) screenshots.
- `pdfs/` — native-text, scanned, multi-column, tabular, image+caption, and
  metadata-less PDFs.
- `videos/` — embed pages: YouTube, Vimeo, article-with-embed, unsupported
  iframe, and a media-free page.
- `notes/` — quick note, long note, markdown note, todo note, and quotes with
  and without a source.

## Regenerating images and PDFs

```powershell
pwsh -File benchmarks/generate-corpus.ps1
```

Re-running overwrites files and rewrites `expected/ocr/*.txt`. The
hand-written text fixtures in `articles/`, `videos/`, and `notes/` are not
touched by the generator.

## Expected outputs

OCR expectations live in `../expected/ocr/` and are referenced from
`../manifest.json` via each item's `expected.ocr_text_file`.

Keep private credentials, tokens, personal documents, and unlicensed
copyrighted material out of this directory.
