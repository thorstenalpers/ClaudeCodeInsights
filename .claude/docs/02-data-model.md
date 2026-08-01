# Data model

## Input

`~/.claude/projects/**/*.jsonl`, one JSON object per line. Scan roots are
configurable; the default list skips directories that do not exist.

Fields that matter: `type` (only `assistant`, `user`, `custom-title`,
`ai-title`), `sessionId`, `uuid`, `timestamp`, `cwd`, `gitBranch`,
`isSidechain`, `agentId` (top level or under `data`), `message.id`,
`message.model`, `message.usage.{input_tokens, output_tokens,
cache_read_input_tokens, cache_creation_input_tokens}`,
`message.content[].type == "tool_use"` → `.name`, `toolUseResult.*` on `user`
records, `customTitle` / `aiTitle`.

## Parsing rules, and why each exists

Every one of these was learned from real transcripts. Removing one produces
plausible-looking but wrong numbers, which is the worst failure mode here.

| Rule                                                                     | Why                                                                                                              |
| ------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------- |
| Dedup on `message.id`, **last record wins**                              | Claude Code writes several streaming records per response; only the last carries the final usage tally           |
| Dedup on `uuid` as well                                                  | A session resumed headlessly and reopened in the CLI rewrites the same line range verbatim — every turn twice    |
| Drop assistant turns whose four counters sum to 0                        | Streaming artefacts, no content                                                                                  |
| Turns without `message.id` are kept individually                         | They cannot be attributed to a tool call; they are the `reasoning` bucket and must not collapse into one another |
| Subagent = `isSidechain` **or** `agentId` **or** path under `subagents/` | Three independent signals; none is reliable alone                                                                |
| `custom-title` outranks `ai-title`, in either file order                 | An inferred title must never overwrite one the user wrote                                                        |
| `toolUseResult` read defensively                                         | Sometimes an object, sometimes a bare string                                                                     |
| A user message with **only** `tool_result` blocks is not a user turn     | It is the harness reporting back; showing it puts words in the user's mouth                                      |
| `toolUseId → tool_result` indexed across the whole session               | With parallel tools each `tool_use` and each result is on its own line, often out of order                       |

## Incremental scan

Keyed on path, mtime, size and line count:

- unchanged → skipped
- grew → read from the previous line count
- **shrank → rows for that file deleted, then read in full** (rewritten or truncated)

After every scan, everything derived is recomputed from the turn rows. Never
accumulated: an insert that hits the `message_id` conflict corrects a value
instead of adding one, so a running total drifts.

## Schema

`scan_files`, `sessions`, `turns`, `turn_tools`, `agents`,
`session_tool_counts`, `session_activity`, `tags`, `session_tags`.
Forward-only migrations keyed on `user_version`; the database is a cache and can
always be rebuilt from the transcripts.

Three deliberate choices:

- **A partial unique index on `turns(message_id)`** where it is non-empty, so the
  reasoning bucket is not constrained. `ON CONFLICT(message_id)` **must repeat
  that WHERE clause** or SQLite silently matches nothing — this cost a debugging
  session where every insert quietly did nothing.
- **`turn_tools` keys on `turn_id`**, so a turn without a `message.id` still has
  first-class rows instead of being a query-time residual.
- **No cost column anywhere.** Derived at query time.

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
