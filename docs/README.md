# CleanShot W documentation

CleanShot W is a local-first Windows screenshot and annotation app. This
directory contains the project reference docs. The repository root keeps only
the user-facing README and project files.

## Start here

1. [Roadmap and status](ROADMAP.md) for the current milestone, next work, and
   Windows gate.
2. [Product and scope](PRODUCT.md) for users, constraints, capabilities, and
   explicit gaps.
3. [Design system](DESIGN.md) for the editor layout, colors, typography,
   controls, and capture overlay rules.
4. [Roadmap](ROADMAP.md) for milestones, decisions, and next work.

## Reference docs

| Document | Use it for |
|---|---|
| [Engineering](ENGINEERING.md) | Architecture, runtime boundaries, native constraints, commands, and packaging |
| [Product and scope](PRODUCT.md) | Product brief, feature coverage, gaps, and future work |
| [Design system](DESIGN.md) | Visual rules and component guidance |
| [Roadmap](ROADMAP.md) | Milestones and product decisions |
| [Changelog](CHANGELOG.md) | Completed and release-visible changes |

## Where decisions belong

- Current milestone and next work belong in `ROADMAP.md`.
- Completed changes belong in `CHANGELOG.md`.
- Stable technical facts belong in `ENGINEERING.md`.
- Product promises and gaps belong in `PRODUCT.md`.
- Visual rules belong in `DESIGN.md`.
- Windows test evidence belongs in `ROADMAP.md`.

There is one canonical document for each topic. Do not create copies at the
repository root or under `docs/`.
