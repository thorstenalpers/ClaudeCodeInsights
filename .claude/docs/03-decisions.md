# Decisions

Why things are the way they are. Read this before reversing something — most
entries record a reversal that already happened once.

---

## The old Python project was deleted, not ported

It was a `scanner.py` + `cli.py` + a 124 KB `dashboard.py` serving one giant
HTML string. The domain knowledge was worth keeping; the code was not. The
parsing rules in [02-data-model.md](02-data-model.md) are what survived.

Its pricing table existed in **three** hand-synced copies. That is why cost is
now derived at query time from one source and never stored.

---

## WinUI 3 was replaced by Tauri

The first host was WinUI 3 with a WebView2. It worked, and was abandoned anyway.

|                      | WinUI 3 / C#                             | Tauri / Rust              |
| -------------------- | ---------------------------------------- | ------------------------- |
| Publish artifact     | **224 MB, 528 files**                    | 12.7 MB debug binary      |
| WebView2 data folder | ~40 lines of hand-written redirect       | framework default         |
| Splash handshake     | `ReadyGate` + crossfade by hand          | documented pattern        |
| IPC                  | envelope, dispatcher, contract-sync test | `invoke`                  |
| Icon                 | own ICO writer, two failed attempts      | `tauri icon` from one PNG |

The 224 MB included `onnxruntime.dll` and `DirectML.dll` — the Windows App SDK's
AI stack, which this app never touches. The host existed only to hold a
WebView2 that Windows already ships.

The deciding evidence was the session that built it: three of the four real
problems encountered were Tauri built-ins. The fourth, the Chromium stale-colour
bug on theme switches, is WebView2 and unchanged by the move.

**Cost of the move:** Microsoft Agent Framework is .NET and Python only. The
replacement is `rig` — see below. The C#/WinUI state is reachable at tag
`winui-host-final`.

---

## `rig` for the assistant, and only because of three providers

An agent framework that only wraps HTTP would not earn its dependency. This one
does, for a specific reason: tool calling differs per provider. Anthropic sends
`tool_use` blocks and expects `tool_result` blocks; OpenAI sends `tool_calls`
and expects `role: tool`; Gemini uses `functionCall` / `functionResponse` parts.
Plus the loop — model asks for a tool, execute, feed back, repeat.

With one provider, hand-writing that is defensible. With three, it is the
abstraction that pays. Not yet implemented.

---

## Activity is derived, tags are manual, both are kept

ClaudeLens (`giulio333/ClaudeLens`, MIT, Electron) was the reference for the
sessions view and the transcript replay. Concepts taken, code not.

It stores tags in `localStorage`. Here they live in SQLite, because they are
data about sessions and belong with the sessions.

---

## No junction for the duplicated skills tree

`.agents/skills` was a byte-identical committed copy of `.claude/skills`, 314
tracked files each. A directory junction would **not** have helped: git does not
record junctions, so it would keep tracking every file and only local disk space
would be saved. Deleted instead.

---

## `@modelcontextprotocol/server-shell` removed from `.mcp.json`

It returns 404 on the npm registry — the entry could never have started. Removed
rather than replaced: a shell server in shared, committed configuration is a
decision to take deliberately, not to inherit.

---

## Traps worth not rediscovering

- **`Set-Content -Encoding utf8` writes a BOM** in Windows PowerShell 5.1. It
  made `tauri.conf.json` fail with "expected value at line 1 column 1".
- **`[IO.File]` ignores `Set-Location`** — .NET keeps its own current directory.
  Use absolute paths.
- **`cargo clippy` and `cargo build` fight over one target directory.** A
  `cargo clean -p` for either leaves the other unable to read build-script output
  it still has fingerprints for. Hence the `cargo lint` alias with its own
  directory. CI runs plain clippy — a cold runner has nothing to collide with.
- **A collapsed WebView does not composite**, so `requestAnimationFrame` never
  fires inside it. Hide with opacity, not visibility.
- **Screen-capture verification is fragile.** Other windows steal the foreground;
  a background process cannot raise itself. Start the app fresh for a capture.
