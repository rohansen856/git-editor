# Git Editor: Git History Rewriting Tool

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

Git Editor is a Rust CLI that rewrites Git commit metadata (author, email, dates, messages) for a whole branch, one commit, or a range. You can drive it interactively or entirely from flags. Commits that don't need to change are left byte-for-byte untouched.

## Features

- **Full rewrite**: give every commit a new identity, and either spread new dates across a range or keep each commit's original date (`--keep-dates`).
- **Pick one commit** (`-p`): edit name, email, date or message, from a menu or with `--commit`.
- **Range editing** (`-x`): an interactive table, or `--select 2-5` from flags; limit editable fields with `--message`, `--author` or `--time`.
- **Exact previews**: `--simulate` shows precisely what will be written; `--show-diff` adds per-commit details.
- **Minimal rewrites**: only edited commits and their descendants are recreated. Older commits keep their ids and signatures. Recreated commits keep their encoding, time zone, raw message bytes and extra headers.
- **Safe by default**: refuses detached or unborn HEADs and branches that moved mid-run, saves the previous tip to `refs/git-editor/backup/<branch>`, and warns about other refs that still point at the old history.
- **Agent and script friendly**: `--yes`, `--json`, a plain-stdin fallback, and documented exit codes. See [AGENTS.md](AGENTS.md).
- **Signoff trailers**: matching `Signed-off-by` / `Co-authored-by` / `Authored-by` lines in the trailer block follow the new identity.
- **Remote repositories**: read-only inspection of URLs, or `--clone-dir` to keep a rewritten clone. Uses ssh-agent and Git credential helpers.
- **Cross-platform**: Linux, macOS and Windows; Docker image included.

## Installation

### Prerequisites

- Rust 1.87+ ([Install Rust](https://www.rust-lang.org/tools/install))
- A C compiler, plus `pkg-config` and OpenSSL headers on Linux/macOS (libgit2 is built from source)
- The `git` command is **not** required at runtime

### From Source

```bash
git clone https://github.com/rohansen856/git-editor.git
cd git-editor
cargo build --release
# The binary is at target/release/git-editor
```

## Documentation

📚 **Online documentation:** [rohansen856.github.io/git-editor](https://rohansen856.github.io/git-editor)

```bash
git-editor --docs                      # open the bundled HTML docs in a browser
git-editor --docs --docs-out docs.html # write them to a file instead
```

`--docs` writes a uniquely named temporary file unless `--docs-out` is given. Set `NO_BROWSER` or `GIT_EDITOR_NO_BROWSER` to skip opening a browser.

## Usage

Only one mode flag may be given (`-s`, `-p`, `-x`, `--docs`); combining them is an error. `--simulate` turns a full rewrite, `-p` or `-x` into a dry run.

### 1. Full History Rewrite (Default)
```bash
# New identity, dates spread between --begin and --end (oldest commit gets --begin, newest gets --end)
git-editor --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" \
  --begin "2023-01-01 09:00:00" --end "2023-01-31 18:00:00"

# New identity, every commit keeps its own date and time zone
git-editor --repo-path "/path/to/repo" --email "user@example.com" --name "Author Name" --keep-dates
```

The tool prints a summary and asks for confirmation; add `--show-diff` for the per-commit list and `--yes` to skip the question. Missing `--name`/`--email`/dates are prompted for, with your Git `user.name`/`user.email` offered as defaults. Pressing Enter on both date prompts is the same as `--keep-dates`.

### 2. Show History
```bash
git-editor --repo-path "/path/to/repo" -s
git-editor --repo-path "/path/to/repo" -s --json   # machine-readable
```

Commits are numbered from 1 (newest). `--commit` and `--select` use these numbers.

### 3. Edit One Commit
```bash
git-editor -p                                       # interactive menus
git-editor -p --commit 3 --set-message "Fix typo" --yes
git-editor -p --commit a1b2c3d --name "Jane" --email "jane@example.com" --set-date "2024-05-01 10:00:00 +02:00" --yes
```

In the menu, finish a new message with a line containing only `.`. This lets you put a blank line between the subject and the body.

### 4. Edit a Range
```bash
git-editor -x                    # interactive table
git-editor -x --message          # table with only messages editable (also --author, --time)
git-editor -x --select 2-5 --name "Jane" --email "jane@example.com" --yes
git-editor -x --select 2-5 --begin "2024-01-01 09:00:00" --end "2024-01-05 18:00:00" --yes
```

Table keys:
- arrows or `h/j/k/l` to move
- Enter to edit a cell
- Esc to save and exit
- `q` or Ctrl+C to cancel without saving

Range input is `start-end` (e.g. `5-11`), a single number, or `*` for all commits.

### 5. Simulation (Dry Run)
```bash
git-editor --simulate --email "user@example.com" --name "Author Name" --begin "2023-01-01 00:00:00" --end "2023-12-31 23:59:59"
git-editor --simulate --show-diff -x --select 1-3 --name "Jane"
git-editor --simulate --json -p --commit 2 --set-message "Reworded"
```

The preview is built from the same plan that a real run applies.

### 6. Remote Repositories
```bash
git-editor -s --repo-path "https://github.com/user/repo"            # read-only, temporary clone
git-editor --simulate --repo-path "git@github.com:user/repo.git" ...  # SSH via ssh-agent
git-editor --repo-path "https://github.com/user/repo" --clone-dir ./repo-rewritten \
  --email "user@example.com" --name "Author Name" --keep-dates
git -C ./repo-rewritten push --force-with-lease
```

Rewriting a URL without `--clone-dir` is refused, because the temporary clone would be deleted on exit. Authentication uses ssh-agent for SSH and your Git credential helpers for HTTPS. Credentials embedded in a URL are masked in all output.

### Arguments

| Option | Short | Description |
| ------ | ----- | ----------- |
| `--repo-path` | `-r` | Repository path (any directory inside it, or a bare repo) or clone URL; default: current directory |
| `--email` | | New author email (full rewrite, `-p --commit`, `-x --select`) |
| `--name` | `-n` | New author name |
| `--begin` | `-b` | First new date: `YYYY-MM-DD HH:MM:SS` (UTC), optionally with an offset like `+05:30`, or RFC 3339 |
| `--end` | `-e` | Last new date (same formats) |
| `--keep-dates` | | Full rewrite: keep every commit's original date and time zone |
| `--committer` | | `match-author` (default: committer follows the edited author fields) or `keep` |
| `--show-history` | `-s` | Show commit history (read-only) |
| `--pick-specific-commits` | `-p` | Edit one commit (menu, or `--commit`) |
| `--commit` | | With `-p`: commit number (1 = newest) or hash prefix |
| `--set-date` / `--set-message` | | With `-p --commit`: new date / message |
| `--range` | `-x` | Edit a range (table, or `--select`) |
| `--select` | | With `-x`: `N-M`, `N` or `*` |
| `--message` / `--author` / `--time` | | With `-x`: limit which fields the table can edit |
| `--simulate` | | Dry run: preview, write nothing |
| `--show-diff` | | Per-commit details in previews |
| `--skip-range-check` | | Allow ranges shorter than 3h per commit (5-minute gaps, then even spacing; never duplicate seconds) |
| `--yes` | `-y` | Confirm automatically |
| `--json` | | One JSON document on stdout (history, preview, result or error); progress on stderr |
| `--clone-dir` | | Keep the clone of a URL in DIR (required to rewrite a URL) |
| `--docs` / `--docs-out` | | Open the docs / write them to a file |

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success (including "nothing to change") |
| 1 | Error |
| 2 | Invalid or conflicting arguments |
| 130 | Cancelled (declined, Esc, Ctrl+C) |

## How It Works

1. **Repository access**: the path is resolved to its repository (subdirectories and bare repositories work). URLs are cloned with libgit2.
2. **Plan**: the selected mode builds a plan of edits keyed by commit id; previews and `--simulate` render that plan.
3. **Rewrite**: commits are walked oldest-first. A commit with no edit whose parents did not change is **reused as-is**. Every other commit is rebuilt from its raw object:
   - parents are updated;
   - edited author fields, and the committer according to `--committer`, are replaced;
   - everything else is kept: encoding, mergetag and other headers, raw message bytes, time-zone offsets.
4. **Trailers**: in the trailer block of each message, `Signed-off-by` / `Co-authored-by` / `Authored-by` lines with the old email follow the new identity.
5. **Reference update**: the previous tip is saved as `refs/git-editor/backup/<branch>`. The current branch is then moved with a compare-and-swap, so it fails cleanly if the branch moved in the meantime.

## Behavior & limitations

- **Scope**: only commits reachable from `HEAD` are rewritten, and only the checked-out branch moves. Other branches and tags that share the history are reported (`stale_refs` in JSON) but not moved. Undo with `git reset --keep refs/git-editor/backup/<branch>`.
- **Signatures**: recreated commits lose their GPG/SSH signature, because it would no longer verify. The count is reported. Commits that are reused keep theirs.
- **Committer**: with the default `--committer match-author`, editing an author name, email or date also sets the committer's name, email or date to the new values. Message-only edits never touch the committer.
- **Dates**: dates without an offset are UTC. Displayed dates in tables are UTC; JSON shows each commit's own offset.
- **Detached HEAD / empty repository**: refused with a clear error; nothing is written.
- **Interactive use** needs a terminal. Without one, prompts read plain lines from stdin, or pass every value as flags plus `--yes`.
- **Identity defaults** come from your Git config, through libgit2. That covers repository, global, XDG and system files, but not `GIT_CONFIG_*` environment overrides.
- **Repository content** shown in the terminal has control characters escaped.

## Docker

```bash
docker build -t git-editor .
docker run --rm -it --user "$(id -u):$(id -g)" -e HOME=/tmp \
  -v /path/to/repo:/workspace git-editor -s
```

Run the container with your user id (`--user`) so that libgit2's repository-ownership check passes and no root-owned files end up in your repository.

## Warning

**This tool rewrites Git history.** Rewritten commits have new ids; anyone sharing the branch must re-sync. Pushing requires `git push --force-with-lease`. The backup ref makes local undo easy, but back up important repositories anyway.

## Development

### Testing

```bash
cargo test                              # all tests (unit, engine, end-to-end CLI, integration)
cargo test --lib                        # unit tests
cargo test --test engine                # rewrite engine
cargo test --test e2e                   # runs the binary against throwaway repositories
cargo clippy --all-targets --all-features -- -D warnings
```

The Makefile has helpers: `make run` (read-only history of `REPO_PATH`), `make run-custom` (preview), `make rewrite REPO_PATH=…` (real rewrite), and `make install PREFIX=~/.local`.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
