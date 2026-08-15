# Benchmark results
Generated: 2026-08-14T23:50:29.725Z

## Overall: score 0.941 — pass 25, partial 8, fail 1, skip 8

| Type | Score | Pass | Partial | Fail | Skip | Total |
| --- | --- | --- | --- | --- | --- | --- |
| article | 0.962 | 9 | 2 | 0 | 0 | 11 |
| recipe | 0.917 | 0 | 1 | 0 | 0 | 1 |
| screenshot | 0.924 | 0 | 2 | 0 | 0 | 2 |
| image | 0.724 | 1 | 2 | 1 | 8 | 12 |
| pdf | 0.958 | 5 | 1 | 0 | 0 | 6 |
| video | 1.000 | 4 | 0 | 0 | 0 | 4 |
| note | 1.000 | 4 | 0 | 0 | 0 | 4 |
| quote | 1.000 | 2 | 0 | 0 | 0 | 2 |

## Items
- [PASS] article-short-01 (score 1.000, production:ingestUrl)
- [PASS] article-long-essay-01 (score 1.000, production:ingestUrl)
- [PARTIAL] article-news-01 (score 0.917, production:ingestUrl)
- [PARTIAL] article-recipe-01 (score 0.917, production:ingestUrl)
- [PASS] article-docs-01 (score 1.000, production:ingestUrl)
- [PASS] article-footnotes-01 (score 1.000, production:ingestUrl)
- [PASS] article-codeblocks-01 (score 1.000, production:ingestUrl)
- [PASS] article-gallery-01 (score 1.000, production:ingestUrl)
- [PARTIAL] article-js-heavy-01 (score 0.667, production:ingestUrl)
- [PASS] article-ads-01 (score 1.000, production:ingestUrl)
- [PASS] article-paywalled-01 (score 1.000, production:ingestUrl)
- [PARTIAL] screenshot-large-text-01 (score 0.966, windows-ocr)
- [PARTIAL] screenshot-small-text-01 (score 0.883, windows-ocr)
- [SKIP] image-photo-01 (score 0.000, none)
- [SKIP] image-design-ref-01 (score 0.000, none)
- [SKIP] image-similar-01 (score 0.000, none)
- [SKIP] image-similar-02 (score 0.000, none)
- [SKIP] image-similar-03 (score 0.000, none)
- [SKIP] image-similar-04 (score 0.000, none)
- [SKIP] image-distractor-01 (score 0.000, none)
- [SKIP] image-lowres-01 (score 0.000, none)
- [FAIL] image-meme-01 (score 0.000, windows-ocr)
- [PASS] image-handwriting-01 (score 1.000, windows-ocr)
- [PARTIAL] image-rotated-01 (score 0.906, windows-ocr)
- [PARTIAL] image-columns-01 (score 0.991, windows-ocr)
- [PASS] pdf-native-text-01 (score 1.000, naive-streams)
- [PARTIAL] pdf-scanned-01 (score 0.750, windows-ocr)
- [PASS] pdf-multicolumn-01 (score 1.000, naive-streams)
- [PASS] pdf-tables-01 (score 1.000, naive-streams)
- [PASS] pdf-images-captions-01 (score 1.000, naive-streams)
- [PASS] pdf-bad-metadata-01 (score 1.000, naive-streams)
- [PASS] video-youtube-01 (score 1.000, production:ingestUrl)
- [PASS] video-vimeo-01 (score 1.000, production:ingestUrl)
- [PASS] video-article-embed-01 (score 1.000, production:ingestUrl)
- [PASS] video-unsupported-iframe-01 (score 1.000, production:ingestUrl)
- [PASS] video-no-media-01 (score 1.000, production:ingestUrl)
- [PASS] note-quick-01 (score 1.000, raw-file)
- [PASS] note-long-01 (score 1.000, raw-file)
- [PASS] note-markdown-01 (score 1.000, raw-file)
- [PASS] note-todo-01 (score 1.000, raw-file)
- [PASS] quote-source-01 (score 1.000, raw-file)
- [PASS] quote-nosource-01 (score 1.000, raw-file)

## Details
### article-news-01 — partial (0.917)
- author mismatch: expected "Sam Okonkwo", got ""

### article-recipe-01 — partial (0.917)
- author mismatch: expected "June Park", got ""

### article-js-heavy-01 — partial (0.667)
- search term not found: "server side"
- search term not found: "browser"

### screenshot-large-text-01 — partial (0.966)
- OCR recall 0.97 (29 expected tokens)
- missing tokens (sample): in
- OCR precision 0.97

### screenshot-small-text-01 — partial (0.883)
- OCR recall 0.88 (77 expected tokens)
- missing tokens (sample): 18, text, 6, markdown, onboarding, 9, readme, sketchbook
- OCR precision 0.88

### image-photo-01 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-design-ref-01 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-similar-01 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-similar-02 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-similar-03 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-similar-04 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-distractor-01 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-lowres-01 — skip (0.000)
- vision/similarity scoring not implemented (embeddings benchmark)

### image-meme-01 — fail (0.000)
- OCR recall 0.00 (13 expected tokens)
- missing tokens (sample): me, i, will, just, save, one, link, 10
- OCR precision 0.00

### image-rotated-01 — partial (0.906)
- OCR recall 0.81 (32 expected tokens)
- missing tokens (sample): field, report, appendix, b, rotation, tolerance

### image-columns-01 — partial (0.991)
- OCR precision 0.98

### pdf-scanned-01 — partial (0.750)
- search term not found: "sprouted bulbs"
