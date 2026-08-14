# AI and Extraction Benchmarks

This directory contains the representative corpus and evaluation notes used to choose OCR, extraction, embedding, vision, and local language models.

The corpus should contain only files and URLs that we are allowed to use for local testing. Do not commit private or copyrighted material to the repository unless we have permission.

## Structure

```text
benchmarks/
  README.md
  manifest.json
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
