# gitopo

**Read-only git branch topology explorer for the terminal.**

`gitopo` is a focused TUI for *understanding* your repo's branch structure — not performing git operations. Think `gitk` for the terminal, but actually good.

```
┌─ Branches (8) ─────────┐┌─ main (47 commits) ───────────────────────────────────┐
│ * main                  ││ ● [main] a3f2c1d Add OAuth2 token refresh logic  2h ago │
│   feature/auth          ││ │ 9e1b4f2 Fix race condition in session handler  3h ago │
│   feature/payments      ││ │ c72a8d5 Merge pull request #41 from feat/auth  5h ago │
│ ↑ origin/main           ││ ● d1e9c3b Implement JWT validation middleware     1d ago │
│ ↑ origin/feature/auth   │└────────────────────────────────────────────────────────┘
│                         │┌─ Commit Details ───────────────────────────────────────┐
│                         ││ commit  a3f2c1d8b9e1...                                │
│                         ││ author  Colin                                           │
│                         ││ date    2h ago                                          │
│                         ││                                                         │
│                         ││ Add OAuth2 token refresh logic                          │
└─────────────────────────┘└────────────────────────────────────────────────────────┘
 [?] help  [/] search  [a] toggle all  [r] refresh  [q] quit
```

## Why gitopo?

Existing tools like **lazygit** and **gitui** are full git clients — powerful, but overwhelming when you just want to understand your repo's topology. `gitopo` does one thing: lets you navigate and visualise branch structure beautifully.

- **Zero write operations** — it is physically impossible to modify your repo
- **Branch-centric navigation** — branch list drives a filtered commit graph
- **Topology-aware** — merge commits and parent relationships are preserved
- **Ahead/behind tracking** — see how local branches relate to their upstreams
- **Fast search** — `/` to filter branches by name

## Install

### From source

Requires Rust 1.75+. Depends on `libgit2` (bundled via `git2` crate).

```sh
git clone https://github.com/YearningAsian/gitopo
cd gitopo
cargo build --release
# binary at target/release/gitopo
```

### Cargo

```sh
cargo install gitopo
```

## Usage

```sh
gitopo                    # open repo in current directory
gitopo /path/to/repo      # open a specific repo
gitopo -a                 # show all branches (including remotes)
gitopo -n 500             # load up to 500 commits per branch
gitopo --help
```

## Keybindings

| Key | Action |
|---|---|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Ctrl+f` / `PgDn` | Page down |
| `Ctrl+b` / `PgUp` | Page up |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `Enter` / `l` / `→` | Focus commit graph |
| `h` / `Esc` / `←` | Back to branch list |
| `/` | Search branches |
| `n` / `N` | Next / prev search match |
| `a` | Toggle local / all branches |
| `r` | Refresh repository data |
| `?` | Toggle help |
| `q` / `Ctrl+C` | Quit |

## Architecture

```
src/
├── main.rs      — CLI parsing, terminal setup, run loop
├── app.rs       — Application state, event dispatch, scroll logic
├── git.rs       — Repository reading via git2 (read-only)
├── graph.rs     — Commit graph lane assignment
├── ui.rs        — Ratatui rendering
└── events.rs    — Input event handling
```

All git operations go through `git2` in read-only mode. There are no shell exec calls, no writes to the repository.

## License

MIT
