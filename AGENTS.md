# Claude Code Insights

A local desktop app for reading and managing what coding agents store on this
machine: transcripts, usage, cost, and the projects registered in the settings.
Claude Code is the primary source; Codex sessions from `~/.codex/sessions` are
read into the same tables, marked by `sessions.source`. Everything stays on the
machine. Nothing is uploaded.

## Stack

| Part          | Choice                                                           |
| ------------- | ---------------------------------------------------------------- |
| Shell         | Tauri 2 (Rust), one WebView2 window                              |
| Backend       | Rust — `src-tauri/`                                              |
| Storage       | SQLite via `rusqlite` (bundled), at `%LocalAppData%\ClaudeAdmin` |
| Frontend      | SvelteKit (SPA, adapter-static), Svelte 5 runes, Tailwind v4     |
| Tables        | `@tanstack/table-core`                                           |
| Lint / format | ESLint flat config with `recommendedTypeChecked`, Prettier       |

## Commands

```bash
npm start            # tauri dev: the real window, with HMR
npm run dev          # frontend only, in a browser, no host attached
npm run app:exe      # debug exe without installer, copied to bin/
npm run lint         # prettier --check . && eslint .
npm run format       # prettier --write .
npm run check        # svelte-check
npm run verify       # fmt, clippy and the Rust tests, beside a running app
npm run inspect      # print this machine's own data — see below
cd src-tauri && cargo test
cd src-tauri && cargo lint    # clippy in its own target dir
```

`npm run app:exe` is the fastest way to see a change in the real window without
a dev server. F5 in VS Code debugs the Rust side against `npm run dev`.

`tests/inspect.rs` prints what the window would show, through the same calls the
window makes — the session list, one session with its transcript and tool calls,
every project with its transcripts and registration, every roll-up, and the
host's log file. Its tests read this machine and are `#[ignore]` for that reason,
so `npm run inspect` is what turns them on:

```bash
npm run inspect
npm run inspect -- -Test every_project_in_full
npm run inspect -- -Test one_session_in_full -Session b70e238b
```

It builds in the same separate target directory `npm run verify` uses, so it
works with the app open — which is when a log is usually worth reading. For a
breakpoint, run the test itself from the editor instead.

## Releasing

A release is a version nobody has tagged yet; there is no manual step after the merge.

1. Set `version` in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` to the same
   value. `.github/workflows/release.yml` fails when they differ.
2. Write the notes in `release-notes/v<version>.md`. They become the GitHub release body.
3. Merge to `main`. The workflow builds the NSIS installer, tags `v<version>` and
   publishes the release. A merge that leaves the version unchanged releases nothing.

## Hard rules

1. **Never write into `~/.claude/` or `~/.codex/`, except where the user
   explicitly asked for it.** Transcripts are foreign, read-only territory.
   Everything this app owns lives under `%LocalAppData%\ClaudeAdmin`.
2. **Any change to `~/.claude.json` writes a backup next to it first.** It is
   Claude Code's own configuration; a corrupt one breaks the user's tooling.
3. **A destructive action shows exactly what disappears before it runs** — file
   names and sizes, not a count.
4. **Cost is never stored.** It is derived from token counts and the price table
   at query time, so editing a rate never means rescanning.
5. **Session figures are recomputed from the turn rows, never accumulated.** An
   insert that hits the `message_id` conflict corrects a value instead of adding
   one, so a running total drifts.
6. **Sort columns come from an allowlist, and `IN` placeholders from the number
   of bound values.** Neither may be built from caller text.
7. **The API key never crosses to the frontend**, never appears in a log, never
   in an exception message. Keys live in the OS credential store; the window may
   ask whether one is set and may replace it, never read it back. Every hosted
   call is made in Rust for the same reason. Two things in this app reach the
   network, both from Rust and both only when asked: the hosted assistant, and
   the voice-pack download, which goes to a fixed address from a table in the
   source and never to one the window supplies.
8. **`src/lib/components/ui/**` is generated** by `shadcn-svelte add`. Changes
   there are lost on the next update; it is excluded from lint and format.
9. Components take props and emit events. Route pages read stores and call the
   backend. A component that needs the backend to render cannot be tested.
10. Only semantic Tailwind tokens (`bg-background`, `text-muted-foreground`).
    No raw colours, no manual `dark:` overrides.

## Where to read what

| File                                                  | Read it when                                   |
| ----------------------------------------------------- | ---------------------------------------------- |
| [01-architecture.md](.claude/docs/01-architecture.md) | Changing how the pieces fit together           |
| [02-data-model.md](.claude/docs/02-data-model.md)     | Touching the scanner, the schema, or a query   |
| [03-decisions.md](.claude/docs/03-decisions.md)       | About to reverse something — the why is here   |
| [04-roadmap.md](.claude/docs/04-roadmap.md)           | Picking up work, or looking for open questions |

## Language

Everything written into the repository is English: code, comments, commit
messages, documentation. Conversation with the user is German.
