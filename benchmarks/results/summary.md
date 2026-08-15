# Benchmark results
Generated: 2026-08-15T03:11:15.473Z

## Overall: score 0.992 — pass 36, partial 4, fail 0, skip 8

| Type | Score | Pass | Partial | Fail | Skip | Total |
| --- | --- | --- | --- | --- | --- | --- |
| article | 1.000 | 17 | 0 | 0 | 0 | 17 |
| recipe | 1.000 | 1 | 0 | 0 | 0 | 1 |
| screenshot | 0.913 | 0 | 2 | 0 | 0 | 2 |
| image | 0.961 | 2 | 2 | 0 | 8 | 12 |
| pdf | 1.000 | 6 | 0 | 0 | 0 | 6 |
| video | 1.000 | 4 | 0 | 0 | 0 | 4 |
| note | 1.000 | 4 | 0 | 0 | 0 | 4 |
| quote | 1.000 | 2 | 0 | 0 | 0 | 2 |

## Items
- [PASS] article-short-01 (score 1.000, production:ingestUrl)
- [PASS] article-long-essay-01 (score 1.000, production:ingestUrl)
- [PASS] article-news-01 (score 1.000, production:ingestUrl)
- [PASS] article-recipe-01 (score 1.000, production:ingestUrl)
- [PASS] article-docs-01 (score 1.000, production:ingestUrl)
- [PASS] article-footnotes-01 (score 1.000, production:ingestUrl)
- [PASS] article-codeblocks-01 (score 1.000, production:ingestUrl)
- [PASS] article-gallery-01 (score 1.000, production:ingestUrl)
- [PASS] article-js-heavy-01 (score 1.000, production:ingestUrl)
- [PASS] article-ads-01 (score 1.000, production:ingestUrl)
- [PASS] article-paywalled-01 (score 1.000, production:ingestUrl)
- [PARTIAL] screenshot-large-text-01 (score 0.969, windows-ocr)
- [PARTIAL] screenshot-small-text-01 (score 0.856, windows-ocr)
- [SKIP] image-photo-01 (score 0.000, none)
- [SKIP] image-design-ref-01 (score 0.000, none)
- [SKIP] image-similar-01 (score 0.000, none)
- [SKIP] image-similar-02 (score 0.000, none)
- [SKIP] image-similar-03 (score 0.000, none)
- [SKIP] image-similar-04 (score 0.000, none)
- [SKIP] image-distractor-01 (score 0.000, none)
- [SKIP] image-lowres-01 (score 0.000, none)
- [PARTIAL] image-meme-01 (score 0.929, windows-ocr)
- [PASS] image-handwriting-01 (score 1.000, windows-ocr)
- [PARTIAL] image-rotated-01 (score 0.917, windows-ocr)
- [PASS] image-columns-01 (score 1.000, tesseract.js)
- [PASS] pdf-native-text-01 (score 1.000, naive-streams)
- [PASS] pdf-scanned-01 (score 1.000, tesseract.js)
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
- [PASS] article-live-macrumors-the-macrumors-show-where-does-the-apple-watch-go-next (score 1.000, production:ingestUrl)
- [PASS] article-live-macrumors-new-things-your-iphone-can-do-ios-27 (score 1.000, production:ingestUrl)
- [PASS] article-live-macrumors-iphone-18-pro-next-month-12-rumored-features (score 1.000, production:ingestUrl)
- [PASS] article-live-macrumors-iphone-17-vs-iphone-18-buying (score 1.000, production:ingestUrl)
- [PASS] article-live-macrumors-best-apple-deals-of-the-week-8-14-26 (score 1.000, production:ingestUrl)
- [PASS] article-live-macrumors-apple-trained-own-ai-model-for-china (score 1.000, production:ingestUrl)

## Details
### screenshot-large-text-01 — partial (0.969)
- OCR recall 0.97 (32 expected tokens)
- missing tokens (sample): in
- OCR precision 0.97

### screenshot-small-text-01 — partial (0.856)
- OCR recall 0.84 (110 expected tokens)
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

### image-meme-01 — partial (0.929)
- OCR recall 0.86 (14 expected tokens)
- missing tokens (sample): i, will

### image-rotated-01 — partial (0.917)
- OCR recall 0.83 (36 expected tokens)
- missing tokens (sample): field, report, appendix, b, rotation, tolerance
