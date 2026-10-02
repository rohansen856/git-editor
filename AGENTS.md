# Using git-editor from an agent or script

This guide is for LLM agents (Claude Code, Codex, …) and automation that need to drive
`git-editor` without a human at the keyboard. Every mode can run non-interactively, prints
a single JSON document on stdout with `--json`, and reports success or failure through its
exit status.

## The safe workflow

1. **Inspect** the branch. Commit numbers (`1` = newest) are the ones `--commit` and
   `--select` accept.

   ```bash
   git-editor -r path/to/repo -s --json
   ```

2. **Preview** the exact change. Nothing is written.

   ```bash
   git-editor -r path/to/repo --name "New Name" --email new@example.com --keep-dates --simulate --json
   ```

3. **Apply** it, confirming with `--yes`:

   ```bash
   git-editor -r path/to/repo --name "New Name" --email new@example.com --keep-dates --yes --json
   ```

4. **Undo**, if needed. Every rewrite saves the previous tip first:

   ```bash
   git -C path/to/repo reset --keep refs/git-editor/backup/<branch>
   ```

Without `--yes`, and when stdin is not a terminal, the confirmation is read as a plain line
from stdin. If stdin is closed, the command fails with exit code 1 and writes nothing. It
never hangs waiting for a terminal.

## Commands

| Goal | Command |
|---|---|
| Rewrite author on every commit, keep dates | `--name N --email E --keep-dates` |
| Rewrite author and spread dates | `--name N --email E --begin "2024-01-01 09:00:00" --end "2024-03-01 18:00:00"` |
| Edit one commit | `-p --commit <number or hash prefix> [--name N] [--email E] [--set-date D] [--set-message M]` |
| Edit a range | `-x --select 2-5 [--name N] [--email E] [--begin D --end D]` (dates are spread oldest to newest) |
| Keep committer untouched | add `--committer keep` (default `match-author`) |
| Show per-commit details in a preview | add `--show-diff` |
| Rewrite a remote repository | `-r <url> --clone-dir <dir> …` then push from `<dir>` |
| Write the HTML docs to a file | `--docs --docs-out docs.html` |

`--json` with `-p` or `-x` requires `--commit` / `--select`, because the interactive menu
and table cannot run in JSON mode. Combining several mode flags (`-s`, `-p`, `-x`, `--docs`)
is rejected with exit code 2.

### Dates

`YYYY-MM-DD HH:MM:SS` is UTC. Add an offset (`2024-01-01 09:00:00 +05:30`) or use RFC 3339
(`2024-01-01T09:00:00+05:30`) to write commits in that time zone. With `--begin`/`--end`,
generated dates use the offset given with `--begin`.

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success (including "nothing to change") |
| 1 | Error; nothing was written unless the JSON result says otherwise |
| 2 | Invalid or conflicting arguments (clap usage error, printed to stderr) |
| 130 | Cancelled (declined confirmation, Esc, Ctrl+C) |

## JSON documents

All documents carry `"ok"`. Errors look like this:

```json
{ "ok": false, "error": "HEAD is detached; …", "cancelled": false, "exit_code": 1 }
```

**`-s --json`** returns the history:

```json
{ "ok": true, "command": "history", "branch": "main", "head": "<oid>", "total_commits": 3,
  "commits": [ { "number": 1, "oid": "…", "short": "…",
                 "author": { "name": "…", "email": "…", "date": "2020-01-03T10:00:00+05:30" },
                 "committer": { … }, "subject": "…", "message": "…",
                 "message_is_utf8": true, "parents": 1 } ] }
```

**`--simulate --json`** returns the preview. Only commits that change are listed; unchanged
fields are `null`:

```json
{ "ok": true, "command": "simulate", "mode": "Full Repository Rewrite",
  "total_commits": 3, "commits_to_change": 3,
  "changes": [ { "oid": "…", "short": "…",
                 "author_name": { "from": "Old", "to": "New" }, "author_email": null,
                 "date": { "from": "2020-01-01T04:30:00Z", "to": "2024-01-01T00:00:00Z" },
                 "message": null } ] }
```

**A write with `--yes --json`** returns the result:

```json
{ "ok": true, "command": "rewrite", "changed": true, "branch": "main",
  "old_head": "…", "new_head": "…", "backup_ref": "refs/git-editor/backup/main",
  "rewritten": [ { "old": "…", "new": "…" } ], "reused": 2,
  "signatures_dropped": 0, "stale_refs": ["refs/heads/feature", "refs/tags/v1"] }
```

- `reused` counts commits kept byte-identical (same id).
- `stale_refs` lists other branches and tags that still point at the old history.
- `signatures_dropped` counts recreated commits whose GPG/SSH signature had to be removed.

## What a rewrite changes

- Only commits that are edited, plus their descendants, are recreated. Older commits keep
  their ids, signatures and every header.
- Recreated commits keep their `encoding`, `mergetag` and other headers, raw message bytes
  (including non-UTF-8 and CRLF) and time-zone offsets. Signatures are dropped because they
  would no longer verify.
- Only the checked-out branch moves. A detached HEAD, an unborn branch or a branch that moved
  during the command is refused with exit code 1, and nothing is written.
