# CubeEscape

3D Cube Game where you remove selected arrows.

Rotate the cube, find an arrow with a clear path ahead of its arrowhead, and
click it to slide it off. Clear every arrow on all six faces to finish the
level. Click one that is blocked and it shakes, the arrow in its way flashes
red, and you pick up a mistake. There is no timer.

## Requirements

- Linux platform (tested on Linux Mint 22)
- Rust 1.82 or newer
- C toolchain for linking
- Support for OpenGL and X11

## Controls

| Input | Action |
| --- | --- |
| Drag | Rotate the cube |
| Click an arrow | Slide it off (if its path is clear) |
| Mouse wheel | Zoom |
| Arrow keys / WASD | Rotate with the keyboard |
| `U` or Backspace | Undo the last removal |
| `H` | Hint: highlight an arrow you can remove right now |
| `R` | Restart the level |
| `V` | Reset the view |
| `[` / `]`, or the `-` / `+` buttons | Choose the grid size: Auto, then 4x4 up to 16x16 (the game starts on 10x10). Changing it restarts the current level on the new grid |
| Enter / Space / click | Next level, once a level is cleared |
| `Esc` | Quit |

## Build and run

You need Rust 1.82 or newer (install it with [rustup](https://rustup.rs)) and a
C toolchain for linking. On Linux Mint:

```sh
sudo apt install build-essential curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo run --release
```

The window uses OpenGL and X11, which a normal Mint desktop already has.

## Development

```sh
cargo test                                   # rules, generator, picking, animation
cargo clippy --all-targets
cargo run --example snapshot -- snapshots    # render PNGs without opening a window
```

The code is split so that nothing but `src/main.rs` touches the window:

| File | Contents |
| --- | --- |
| `src/puzzle.rs` | Grid, arrows, blocking rule, level generator |
| `src/game.rs` | Level progression, undo, mistakes, animation timers |
| `src/scene.rs` | Cube rotation, projection, picking, polygon output |
| `src/main.rs` | Window, mouse and keyboard, text (macroquad) |
| `examples/snapshot.rs` | Headless PNG renderer for checking the look |

### How levels are made

Each face is an `N x N` grid. The game starts on 10x10. The player can pick any
size from 4 to 16, or choose Auto, which starts at 6 on level 1 and grows to 12
as levels rise.

The generator first lays down a handful of long winding arrows. They prefer to
keep going straight and to hug walls and tight spots, so they sweep across much
of the face with many bends, up to about a third of the face's cells. Then it
fills every gap with shorter arrows. After that it looks for deadlocks, where
arrows block each other in a cycle, and breaks each one by turning an arrow
around or deleting the shortest arrow in the cycle. The holes that leaves are
refilled and checked again, until greedily removing whatever is free clears the
board. Removing an arrow never blocks another, so that check proves the level
is solvable. Levels come from a fixed seed, so level N is the same every time.

In this first version an arrow stays on a single face. Arrows that wrap around
the cube's edges are not implemented yet.

## License

MIT, see `LICENSE`.
