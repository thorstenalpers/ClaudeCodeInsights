# Documentation

Entry point is [AGENTS.md](../../AGENTS.md) in the repository root — hard rules
there, details here. Written in English.

| File                                     | Content                                          |
| ---------------------------------------- | ------------------------------------------------ |
| [01-architecture.md](01-architecture.md) | How the pieces fit, startup, page state, theming |
| [02-data-model.md](02-data-model.md)     | Transcript rules, schema, derived activity       |
| [03-decisions.md](03-decisions.md)       | Why things are as they are, and traps            |
| [04-roadmap.md](04-roadmap.md)           | What works, what is next, what is open           |

## Rules for these files

- One file, one topic. Past ~250 lines, split.
- **What lives in the code does not live here.** No copied signatures, no file
  lists that go stale on the first refactor. Describe rules and intent.
- **Every rule states its why.** A rule without one gets removed by the next
  person who finds it inconvenient — which is the whole failure mode these docs
  exist to prevent.
- External facts get a checked-on date. Prices, package names and DOM shapes
  change; without a date nobody knows whether the claim still holds.
- What should _not_ be done is as valuable as the opposite. The deferred lists
  and the traps prevent more wasted work than any how-to.
