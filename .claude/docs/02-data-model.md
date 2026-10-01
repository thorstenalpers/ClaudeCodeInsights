# Data model

## Input

`.jsonl` transcripts, one JSON object per line, from every agent that leaves
some behind. Scan roots are configurable; the default list skips directories
that do not exist. Each root belongs to one source (`ingest/source.rs`), the
source picks the parser, and it is stamped on `sessions.source` — everything
downstream reads the same tables whoever wrote the file.

- **Claude Code** — `~/.claude/projects/**/*.jsonl`, one directory per project.
- **Codex** — `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`, one directory per
  day; the session id is in the file name.

Fields that matter: `type` (only `assistant`, `user`, `system`, `custom-title`,
`ai-title`), `subtype` and `compactMetadata.{trigger, preTokens}` on `system`,
`sessionId`, `uuid`, `timestamp`, `cwd`, `gitBranch`, `isSidechain`, `agentId`
(top level or under `data`), `message.id`, `message.model`,
`message.usage.{input_tokens, output_tokens, cache_read_input_tokens,
cache_creation_input_tokens}`, `message.content[].type == "tool_use"` → `.id`,
`.name` and `.input.{file_path, notebook_path, path}`, `... == "tool_result"` →
`.tool_use_id` and `.is_error`, `toolUseResult.*` on `user` records,
`customTitle` / `aiTitle`.

## Parsing rules, and why each exists

Every one of these was learned from real transcripts. Removing one produces
plausible-looking but wrong numbers, which is the worst failure mode here.

| Rule                                                                     | Why                                                                                                                                                         |
| ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dedup on `message.id`, **last record wins**                              | Claude Code writes several streaming records per response; only the last carries the final usage tally                                                      |
| Dedup on `uuid` as well                                                  | A session resumed headlessly and reopened in the CLI rewrites the same line range verbatim — every turn twice                                               |
| Drop assistant turns whose four counters sum to 0                        | Streaming artefacts, no content                                                                                                                             |
| Turns without `message.id` are kept individually                         | They cannot be attributed to a tool call; they are the `reasoning` bucket and must not collapse into one another                                            |
| Subagent = `isSidechain` **or** `agentId` **or** path under `subagents/` | Three independent signals; none is reliable alone                                                                                                           |
| `custom-title` outranks `ai-title`, in either file order                 | An inferred title must never overwrite one the user wrote                                                                                                   |
| `toolUseResult` read defensively                                         | Sometimes an object, sometimes a bare string                                                                                                                |
| A user message with **only** `tool_result` blocks is not a user turn     | It is the harness reporting back; showing it puts words in the user's mouth                                                                                 |
| `toolUseId → tool_result` indexed across the whole session               | With parallel tools each `tool_use` and each result is on its own line, often out of order                                                                  |
| A compaction is read from `type: system`, `subtype: compact_boundary`    | The record states its trigger (`auto` / `manual`) and `preTokens` outright, so nothing is inferred from the prefix of the summary that follows              |
| Any trigger but `manual` counts as an overflow                           | A future build could add a third one; whatever is not the user's own `/compact` was the machine's decision                                                  |
| A slash command is read from the `<command-name>` element                | The only place an invocation is named. Arguments are dropped, so `/compact focus on auth` is `/compact`                                                     |
| Tool outcomes are stored on their own, keyed on `tool_use_id`            | The result lands on a later record than the call, and routinely in a later scan; pairing them at parse time loses every outcome that straddles the boundary |
| A call with no `tool_use_id` has an unknown outcome, not a good one      | Older transcripts wrote none. Counting them as successes would report a failure rate of zero for a tool nothing is known about                              |

Codex rollouts record events, not messages, so their rules differ
(`ingest/codex.rs`):

| Rule                                                             | Why                                                                                                                                     |
| ---------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| A turn is the `token_count` event that reports it                | Codex writes the model's text, its calls and its usage as separate lines; the tally is the one line that says a call happened           |
| Dedup on the cumulative total, which also names the turn         | The last tally is repeated verbatim at the start of the next turn; the running total is the one value a repeat carries unchanged        |
| `cached_input_tokens` is subtracted from `input_tokens`          | Codex counts the cached part inside the input; this schema keeps them apart, and adding them back together would charge the cache twice |
| The session id comes from the file name                          | The head of the file names it too, but an incremental scan reads only the tail                                                          |
| `function_call` lines belong to the next tally                   | A Codex tool call is its own line, ahead of the usage that pays for it                                                                  |
| An outcome is read from `Exit code:` and from `rejected by user` | Codex writes shell outcomes as text; a refusal failed as surely as a non-zero exit                                                      |
| `apply_patch` file paths are read out of the patch               | The one call that changes code names its files inside the patch body, not in a field                                                    |
| The first user message becomes the topic                         | Codex writes no title of its own, and a list of untitled sessions is one no one can find anything in                                    |

## Incremental scan

Keyed on path, mtime, size and line count:

- unchanged → skipped
- grew → read from the previous line count
- **shrank → rows for that file deleted, then read in full** (rewritten or truncated)

After every scan, everything derived is recomputed from the turn rows. Never
accumulated: an insert that hits the `message_id` conflict corrects a value
instead of adding one, so a running total drifts.

## Schema

`scan_files`, `sessions`, `turns`, `turn_tools`, `tool_results`,
`session_events`, `agents`, `session_tool_counts`, `session_activity`, `tags`,
`session_tags`. Forward-only migrations keyed on `user_version`; the database is
a cache and can always be rebuilt from the transcripts — which is what V4 does,
clearing `scan_files` so the next scan fills columns no backfill could. V5 adds
`sessions.source` (`claude`, `codex`), defaulted to `claude` because every
session already stored came from Claude Code.

Five deliberate choices:

- **A partial unique index on `turns(message_id)`** where it is non-empty, so the
  reasoning bucket is not constrained. `ON CONFLICT(message_id)` **must repeat
  that WHERE clause** or SQLite silently matches nothing — this cost a debugging
  session where every insert quietly did nothing.
- **`turn_tools` keys on `turn_id`**, so a turn without a `message.id` still has
  first-class rows instead of being a query-time residual.
- **No cost column anywhere.** Derived at query time.
- **`session_events` holds rows, not counters.** Compactions and slash commands
  are counted in `recompute_derived` for the same reason the turn figures are: a
  tally written while a file is read in two passes drifts.
- **`turn_tools.file_path` keeps the path as the transcript wrote it.** Case and
  separator are folded in the query, the way `session_tool_counts` keeps raw
  tool names and leaves the mapping to the join.

## Derived activity

`session_tool_counts` holds **raw tool names** and encodes no mapping.
`session_activity` is rebuilt from it whenever the tool-to-category mapping
changes — milliseconds, no transcript touched. That is what makes "editing the
mapping applies immediately" true rather than aspirational.

Classification is ordered, not largest-bucket-wins: editing interleaved with
running things is debugging, not coding. The share per category travels with the
label because most sessions do several things. A tool may belong to several
categories, so shares can sum above 1.

## Transcript replay

Read from the `.jsonl` on demand, not mirrored into the database. The database
holds figures; the transcript stays the source for what was said. Keeps the
database small and the replay current, at the cost of re-reading — hence paging.

Tool payloads are truncated before crossing to the frontend (a single `Read`
result can be hundreds of kilobytes) and the cut walks back to a char boundary,
which a byte slice would panic on.
