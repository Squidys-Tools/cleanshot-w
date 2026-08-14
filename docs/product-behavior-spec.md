# Personal Library Product Behavior Specification

## Product definition

This is a private, visual personal library for saving anything worth remembering. The primary behavior is:

> Save without organizing; retrieve through memory, search, and visual association.

The product should feel quiet, immediate, and visual. Organization happens automatically in the background. Users should not be asked to manage AI output.

## Product principles

- Capture must be faster than deciding where something belongs.
- Search is the primary organizing system.
- Every saved item should receive a visual treatment appropriate to its content.
- AI classification is invisible implementation detail. Do not label tags as AI-generated.
- Do not provide tag correction, tag deletion, confidence scores, or AI-management workflows.
- Manual tags and Spaces are available for intentional projects, not required maintenance.
- Local processing is explained during onboarding and settings, not repeatedly in the main interface.
- User-authored notes and content must never be silently rewritten by AI.
- The library should encourage rediscovery, not only storage.

## Supported content

The initial product should support:

- URLs and web pages
- Articles
- Images and screenshots
- PDFs
- Quick notes
- Longer notes
- Quotes and text clippings
- Video links and selected local video files

Content types are first-class item types. A product, article, image, quote, PDF, and note should not all render as generic bookmarks.

## Capture behavior

### URL capture

When a user pastes or saves a URL:

1. Create the item immediately.
2. Display a provisional card using the URL.
3. Fetch metadata and page content in the background.
4. Classify the page type.
5. Extract readable content when appropriate.
6. Generate the final card presentation.
7. Index extracted text and derived concepts.

If extraction fails, retain the URL, title, domain, and any metadata that was successfully found.

### Image capture

Images can be added by file picker, drag-and-drop, clipboard, screenshot capture, or browser extension.

The system should automatically process images for searchable text, concepts, colors, and similarity. The resulting metadata should improve search and Spaces without exposing an AI-management surface.

### Notes and quotes

Quick Notes should open instantly and remain lightweight. A note can expand into Focus Mode for longer writing.

Quoted text should be stored with its source URL when available and displayed as a quote card.

## Card types

Minimum card types:

- Image
- Screenshot
- Article
- Website
- Product
- Recipe
- Book
- PDF
- Video
- Note
- Quote

Each card may show a title, source, preview, date, tags, summary, and relevant metadata. The exact fields depend on the type.

## Search

Search should support natural-language-like queries and structured filters. Examples:

```text
shoes
shoes blue
object:car
text:invoice
type:image
format:pdf
site:youtube
tag:research
-red
```

Pressing Enter after a term creates a deeper search step. Search should combine:

- Titles and descriptions
- Notes and article text
- OCR text
- Domains and authors
- Manual tags
- Derived image concepts
- Colors
- Semantic similarity
- Dates and item types

Search results should be useful even when the query does not exactly match the saved content.

## Spaces

Spaces are saved searches presented as visual collections.

### Smart Spaces

Smart Spaces update automatically whenever an item matches their query. They can use type, date, tag, domain, search terms, OCR text, image concepts, colors, or semantic criteria.

### Regular Spaces

Regular Spaces contain items manually added by the user.

Users should be able to create, rename, reorder, recolor, and delete Spaces. Deleting a Space must not delete its items.

## Reading and writing

Articles should open in a distraction-free reader with clean typography, cached images, headings and links, code blocks, footnotes where available, safe supported video embeds, and an original source link.

Focus Mode should provide a sparse writing environment with headings, bold text, links, and simple interactive todos.

## Rediscovery

### Serendipity

Serendipity presents older items in a slow, visual browsing mode. The user can keep an item or forget it. Forgetting should move the item to a recoverable trash/archive state rather than permanently destroy it immediately.

### Top of Mind

Top of Mind is a small, user-curated set of pinned items shown when opening the library.

## Onboarding

First-run onboarding should explain:

- The library is local-first.
- AI processing runs on the device where supported.
- Content may still require network access when fetching a URL or remote media.
- The user can export and back up their library.

After onboarding, this information should not dominate the main experience.

## Explicit non-goals

The first releases should not include social feeds, likes, public profiles, collaboration, complex project-management features, manual AI tag administration, mobile applications, mandatory cloud accounts, or cloud-only processing.
