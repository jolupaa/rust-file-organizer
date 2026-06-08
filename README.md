# organizer

A small Rust CLI that sorts the files in a directory into category sub‑folders
(e.g. `images/`, `documents/`) based on their file **extension**. The mapping of
extensions to categories is stored in a `rules.json` file that you build up with
the `add-rule` command. Every real run is recorded so it can be reversed with
`undo`.

Built with [`clap`](https://docs.rs/clap) (derive) for the CLI,
[`serde`](https://serde.rs)/[`serde_json`](https://docs.rs/serde_json) for
persistence, [`dirs`](https://docs.rs/dirs) for locating config/data folders, and
[`anyhow`](https://docs.rs/anyhow) for error handling. Rust **edition 2024**.

---

## Build & install

```bash
# Debug build
cargo build

# Optimized release binary -> target/release/organizer
cargo build --release
```

Run it either through Cargo (note the `--`, everything after it goes to the
program) or by calling the compiled binary directly:

```bash
cargo run -- <subcommand> [args]      # via cargo
./target/release/organizer <subcommand> [args]   # the built binary
```

```bash
cargo install --path .    # to put organizer in the path to execute directly with organizer
```

The examples below use `organizer` as shorthand for "the binary".

---

## Quick start

```bash
# 1. Teach it which extensions belong to which category
organizer add-rule images -e jpg jpeg png gif
organizer add-rule documents -e pdf docx txt

# 2. See what it *would* do (no files are touched)
organizer preview ~/Downloads

# 3. Actually organize the folder
organizer organize ~/Downloads

# 4. Changed your mind? Reverse the last run
organizer undo
```

---

## Commands

### `organize [PATH] [flags]`

Sorts the files directly inside `PATH` into `PATH/<category>/` sub‑folders, using
the current rules. If `PATH` is omitted, the **current working directory** is
used.

| Flag              | Short | Description                                                     |
| ----------------- | ----- | --------------------------------------------------------------- |
| `--dry-run`       | `-d`  | Print the planned moves without changing anything.              |
| `--output <DIR>`  | `-o`  | Send organized output to `<DIR>` (see _Known limitations_).     |
| `--keep-original` | `-k`  | **Copy** files into the category folder instead of moving them. |
| `--by <BY>`       | `-b`  | Reserved; currently ignored (sorting is always by extension).   |
| `--recursive`     | `-r`  | Also descend into sub‑directories.                              |

```bash
organizer organize ./Downloads
organizer organize ./Downloads --dry-run        # preview only
organizer organize ./Downloads -k               # copy, keep originals
organizer organize ./Downloads -r               # recurse into sub-folders
```

A successful (non‑dry‑run) `organize` records every move so it can be undone.

### `preview [PATH] [-b BY]`

A convenience alias for a dry‑run `organize`: it prints the planned moves and
**makes no changes to disk** (no files moved, no history written). `PATH`
defaults to the current directory.

```bash
organizer preview ~/Pictures
```

### `add-rule <CATEGORY> -e <EXT...>`

Adds one or more extensions to a category, creating the category if it does not
exist. Extensions are matched **case‑insensitively** and written **without a
leading dot** (use `jpg`, not `.jpg`).

```bash
organizer add-rule images -e jpg jpeg png
organizer add-rule archives -e zip tar gz
```

### `remove-rule <CATEGORY> [-e <EXT...>]`

- With `-e/--extensions`: removes just those extensions from the category.
- Without it: removes the **entire category**. This is destructive, so it asks
  for confirmation first — you must type the literal word `yes` to proceed.

```bash
organizer remove-rule images -e gif      # drop one extension
organizer remove-rule archives           # drop the whole category (asks to confirm)
```

> The confirmation prompt is shown in Spanish:
> `... [escribe 'yes' para continuar]:` — type `yes` and press Enter to continue.

### `rules`

Lists every category and its extensions.

```bash
organizer rules
# images: jpg, jpeg, png
# documents: pdf, docx, txt
```

### `undo`

Reverses the **last** `organize` run by moving every recorded file back to its
original location, then deletes the history file.

```bash
organizer undo
```

---

## Where data is stored

Paths are resolved with the `dirs` crate, so they follow each platform's
conventions. They are **not** relative to where you run the command.

| File           | Purpose               | Location (via `dirs`)                        |
| -------------- | --------------------- | -------------------------------------------- |
| `rules.json`   | category → extensions | **config dir** `/organizer/rules.json`       |
| `history.json` | last run, for `undo`  | **local data dir** `/organizer/history.json` |

Concrete locations:

| Platform    | `rules.json`                                             | `history.json`                                                |
| ----------- | -------------------------------------------------------- | ------------------------------------------------------------- |
| **macOS**   | `~/Library/Application Support/organizer/rules.json`     | `~/Library/Application Support/organizer/history.json`        |
| **Linux**   | `~/.config/organizer/rules.json` (or `$XDG_CONFIG_HOME`) | `~/.local/share/organizer/history.json` (or `$XDG_DATA_HOME`) |
| **Windows** | `%APPDATA%\organizer\rules.json`                         | `%LOCALAPPDATA%\organizer\history.json`                       |

These directories are created automatically the first time they're needed.

`rules.json` is plain JSON you can inspect or edit by hand:

```json
{
  "categories": {
    "images": ["jpg", "jpeg", "png"],
    "documents": ["pdf", "docx", "txt"]
  }
}
```

---

## Error handling

The tool no longer panics on bad input or filesystem problems. Instead:

- Each failure prints a clear message naming the operation and path, followed by
  the specific cause (e.g. _"No such file or directory"_, _"Permission denied"_,
  _"contains invalid JSON: ..."_), and the process exits with status **1**.
- Two common situations get actionable hints:
  - running a rules command with no `rules.json` → suggests `add-rule`;
  - `undo` with no history → _"Nothing to undo"_.
- During `organize`, if a **single** file cannot be moved/copied (or a
  sub‑directory cannot be read), a `Warning:` is printed for that item and the
  run continues with the rest.

Examples:

```text
Error: No rules file found at '.../organizer/rules.json'. Add a category first, e.g. `organizer add-rule images -e jpg png`.
Error: Could not read the directory '/tmp/nope': No such file or directory (os error 2)
Error: The rules file '.../rules.json' contains invalid JSON: key must be a string at line 1 column 3
Warning: could not move '/data/locked.jpg' to '/data/images/locked.jpg': Permission denied (os error 13); skipping
```

---

## Known limitations

These are existing behaviors worth knowing before you rely on them:

- **`--by` is ignored.** Sorting is always by file extension regardless of the
  value passed.
- **`--output` is unusual.** It `rename`s the _entire category folder_ to the
  output path, and it does so once per matched file inside the loop. Review the
  result carefully before depending on this flag.
- **Recursive `undo` is partial.** With `-r`, history is written separately at
  each directory level, so after a recursive run `undo` only reverses the
  **top‑level** moves; moves made deeper in the tree are not in the final
  history.
- **`undo` leaves empty folders.** It moves files back but does not delete the
  category sub‑folders it created.
- **Lenient writes vs. strict reads.** `add-rule`/`remove-rule` treat a missing
  or unparseable `rules.json` as "no rules yet" and start fresh (so manually
  corrupting the file and then adding a rule discards the old rules). By
  contrast, `organize`/`preview`/`rules` report invalid JSON as an error.

---

## Project layout

```
src/
  main.rs       CLI definition (clap derive) + dispatch; turns errors into
                a message + exit code 1.
  commands.rs   All business logic and the serde structs (Rules, MoveRecord,
                History), plus rules/history path helpers and the confirm prompt.
test/           Sample files for manually exercising the organizer.
rules.json      A sample rules file at the repo root (the tool itself reads the
                copy in your config dir, not this one).
```

There is no automated test suite; use `test/` (or any throwaway folder) with
`preview`/`organize` to exercise the tool by hand.
