# Research Notes: Local-First Search

Notes for the *offline search* design meeting, collected from last week's
reading.

## Key ideas

- **FTS5** is the built-in full-text engine. It handles tokenization,
  ranking, and phrase queries without a server.
- **Content-addressed storage** makes backups predictable: the same file is
  never stored twice.
- **Background processing** should never block capture. Create the card
  first, index it later.

## Questions to settle

1. Do we index `metadata` JSON as a blob, or extract fields into separate
   FTS columns?
2. What is the fallback when FTS5 is unavailable?
3. Should semantic search rerank FTS results or run as a separate query?

## Links

- [SQLite FTS5 docs](https://sqlite.org/fts5.html)
- Project TDR: `docs/technical-decision-record.md`

## Quote worth keeping

> Search is the new shelf.

That is the whole argument in five words.
