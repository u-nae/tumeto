# tumeto

> **tui** + **todo** + **pomodoro**= **tumeto**

A fast, keyboard-driven terminal todo manager built with [ratatui](https://ratatui.rs).

Groups, subtasks, per-item notes, priorities, search, and undo — all from the keyboard.

## Install

```bash
cargo install tumeto
```

## Usage

Run `tumeto` in any terminal. Data is stored at `~/.tumeto_data.json`.

### Keys

| Key | Action |
|-----|--------|
| `j` / `k` | Move down / up |
| `Space` | Toggle complete (cascades to subtasks) |
| `a` | Add todo |
| `s` | Add subtask |
| `e` | Edit |
| `d` | Delete |
| `u` | Undo delete |
| `z` | Fold / unfold subtasks |
| `m` | Edit notes |
| `p` | Cycle priority |
| `Tab` / `h` / `l` | Switch group |
| `c` | Category jump popup |
| `/` | Search |
| `?` | Help |
| `q` / `Ctrl+C` | Quit |

## License

Licensed under either of MIT or Apache-2.0 at your option.
