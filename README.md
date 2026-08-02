# ClaudeAdmin

A desktop app for the data Claude Code leaves on your machine. It reads the
transcripts under `~/.claude/`, works out where your tokens and money went, lets
you replay any past conversation, and manages the projects registered in Claude
Code's settings.

**Everything stays local, unless you ask otherwise.** The app reads `~/.claude/`
and writes only to `%LocalAppData%\ClaudeAdmin`. There is no telemetry, and
nothing is sent anywhere on its own.

The one exception is the Assistant, and only when you point it at a hosted
model: then the summary shown on that page — token counts and totals, never a
transcript — is sent to the provider you chose. Its default source is the Claude
Code binary already on this machine, which sends nothing from this app at all.
API keys live in the Windows credential store and never reach the window.

## What it does

- **Overview** — sessions, turns, tokens by class, active days. Cache reads
  usually dwarf everything else, so they get their own figure rather than being
  folded into a total.
- **Sessions** — every conversation in a sortable, searchable table, with the
  activity it was mostly spent on derived from its tool calls.
- **Transcript replay** — click a session for the full conversation: messages,
  thinking blocks, and tool calls paired with their results.
- **Projects** _(in progress)_ — analyse, tidy and remove the projects Claude
  Code has registered.

## Requirements

- Windows 10 1809 or newer
- WebView2 runtime (preinstalled on Windows 11; the installer fetches it
  otherwise)
- Claude Code, used at least once, so `~/.claude/` exists

## Build from source

Needs Node 24+, Rust with the MSVC toolchain, and the Windows SDK.
`scripts/verify-dev-machine.ps1 -Probe` checks all of it and proves the linker
works by compiling a throwaway binary — a green checklist alone does not.
`scripts/setup-dev-machine.ps1` installs what is missing.

```bash
npm install
npm run start        # the real window, with hot reload
npm run app:exe      # a runnable exe in bin/, without the installer
npm run app:build    # installer
```

## A note on cost

Costs are **estimates** from published API prices. They are wrong for Pro and
Max subscribers, who pay a subscription rather than per token. A model with no
price entry shows `n/a`, never `$0.00` — an unknown cost is not a zero one.

## Contributing

Architecture and conventions are in [AGENTS.md](AGENTS.md) and
[.claude/docs/](.claude/docs/). [03-decisions.md](.claude/docs/03-decisions.md)
records why things are as they are; worth reading before changing something that
looks odd.

## License

MIT.
