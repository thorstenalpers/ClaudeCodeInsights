# Roadmap

State as of 2026-08-01.

## Working

- Tauri shell: VS Code style rail, nine pages, Settings pinned to the footer,
  header with page title, scan button and appearance menu.
- Splash window with the handshake, plus a 15 s fallback.
- Theming: light / dark / system on one axis, five colour presets on the other.
- Transcript scanner with the incremental rules, into SQLite. Measured against a
  real history: 82 files, 74 sessions, 4171 turns, 0 failures, 0 malformed lines.
- Overview with real figures. An empty database scans on launch.
- Sessions table: server-side paging, sorting, search, activity filter, derived
  activity with a profile bar, subagent marker, tag display.
- Session detail: full transcript with messages, thinking, and tool calls paired
  with their results; chips to hide tools or reveal thinking; paged.
- 38 Rust tests, ESLint with `recommendedTypeChecked`, Prettier, CI on Windows.

## Placeholder pages

Cost & Models, Projects, Activity, Agents, Tools, Assistant, Settings. Each says
what it will show.

## Next: project management

The reason for the rename. Ordered from harmless to invasive:

1. **Read.** Projects from `~/.claude.json`: path, whether the directory still
   exists, sessions, tokens, cost, last activity. Orphans and duplicates flagged.
2. **Delete transcripts.** Remove a project's `.jsonl` files. Preview the exact
   files and their sizes before confirming.
3. **Edit or remove the registration.** Change `~/.claude.json`. A backup is
   written next to it before every change, without exception.
4. **Edit per-project settings.** The most invasive; build last.

## Open questions

**The activity threshold labels almost everything "debugging."** It is
`code_change ≥ 15% and execution ≥ 15%`. For edit-build-edit work that is
accurate, but when nearly every row carries the same label it separates nothing.
Options: raise the threshold, use a ratio instead of fixed shares, or drop the
label and show only the profile. Needs a look at the real distribution.

**The sessions table is wider than the window.** Output, cache and model scroll
off at 1296 px. Column visibility is the answer.

**Tags have no UI.** Schema and queries are done; nothing assigns them.

**The repository folder is still `ClaudeUsageAnalyzer`.** Renaming it was
deferred because it is the working directory of the session doing the work.
Rename to `ClaudeAdmin` and reopen there. Nothing inside the repo refers to the
old name.

**`%LocalAppData%\ClaudeUsageAnalyzer` is orphaned** (~12 MB) and can be deleted.

## Deliberately deferred

- Typed rendering of tool results (diffs, terminal output). The parser already
  passes the blocks through; only the display is missing.
- Subagent transcripts from `subagents/agent-*.jsonl`.
- The assistant, and with it the credential store and provider switching.
- Storybook. Planned, never set up.
- Installer, auto-update, CSV export, `win-arm64`.
- Cowork sessions — they live server-side and leave no local JSONL.
