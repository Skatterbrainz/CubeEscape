# Cube Escape: task list

Each task is small enough to finish in one short session and leaves the repo
working (`cargo fmt`, `cargo clippy --all-targets` and `cargo test` all pass).
Do them in order unless a task says otherwise. Tick the box when it is done.

Sizes: **S** = one short session, **M** = one normal session, **L** = split it up first.

**To start a session**, say: "Do task <ID> from TASKS.md in Skatterbrainz/CubeEscape."
Each task lists what it touches and a "Done when" check, so a session can stop
cleanly after one task.

## Done (v0.1 prototype)

- [x] Rules, blocking logic and a level generator that only makes solvable levels (`src/puzzle.rs`)
- [x] Game state: levels, undo, hints, mistakes, animation timers (`src/game.rs`)
- [x] Rotatable 3D cube, perspective, click-picking from any angle (`src/scene.rs`)
- [x] Window front end with drag, click, wheel, keyboard (`src/main.rs`)
- [x] Headless PNG renderer for checking the look (`examples/snapshot.rs`)
- [x] 27 unit tests; the real window was driven with synthetic mouse and keys

## Ship it

- [ ] **A. Get v0.1 into the repo** (S)
  Copy the project files into your clone, `cargo run --release` on Mint, commit, push.
  *Done when:* the code is on GitHub and the game runs on your machine.

- [ ] **B. Settle the name** (S)
  "Cube Escape" is also the name of an existing game series, which could cause
  trouble on Flathub. Pick a final name. The title lives in `GAME_TITLE`
  (`src/lib.rs`); also update `Cargo.toml`, `README.md` and the repo name.
  *Done when:* no leftover references to the old name and tests pass.

- [ ] **C. Continuous integration** (S)
  Add `.github/workflows/ci.yml` running `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`.
  *Done when:* a push shows a green check on GitHub.

## Gameplay polish (any order, all independent)

- [ ] **D. Save progress** (S)
  Remember the highest level reached in `$XDG_DATA_HOME` (or `~/.local/share`) as a tiny text file, and resume there. Add a round-trip test that uses a temp dir.
  Touches: new `src/save.rs`, `src/main.rs`.
  *Done when:* quitting and relaunching continues from the same level.

- [ ] **E. Title screen and level picker** (M)
  Start screen with Continue, New game and a grid of unlocked levels.
  Touches: `src/main.rs` (screen state), possibly a new `src/ui.rs`.
  *Done when:* you can jump to any level you have already reached.

- [ ] **F. Difficulty tuning** (S)
  Add a metric for how deep the removal chains run and how many arrows start free. Print it per level with a small example, then adjust `LevelParams` in `src/puzzle.rs` so difficulty rises steadily. Add tests that lock the curve in.
  *Done when:* levels 1 to 30 show a smooth increase and the tests pass.

- [ ] **G. Colour-blind palette** (S)
  Second palette chosen with the `C` key and saved with progress (after D). Palette lives in `src/scene.rs`.
  *Done when:* arrows stay distinguishable in a colour-blindness simulator.

- [ ] **H. Level-clear celebration** (M)
  A short effect when the last arrow leaves (glow, particles, or the cube spinning once). It should only add polygons in `src/scene.rs`, so the snapshot tool can still render it.
  *Done when:* it plays once per cleared level and never blocks input.

## Packaging for the Mint Software Manager (do in order)

- [ ] **I. App icon** (S)
  An original SVG icon plus PNGs. It must not resemble any other game's art.
  Touches: new `packaging/` folder.
  *Done when:* SVG and 128px PNG exist and look right at small sizes.

- [ ] **J. Desktop entry and AppStream metadata** (M)
  `.desktop` file and `.metainfo.xml` with description, screenshots (use `examples/snapshot.rs`), content rating and release entry.
  *Done when:* `desktop-file-validate` and `appstreamcli validate` both pass.

- [ ] **K. Flatpak manifest** (M)
  Manifest using the freedesktop runtime and Rust SDK extension, with dependencies vendored through `flatpak-cargo-generator` so the build works offline. Needs window and GPU access (X11, Wayland, DRI).
  *Done when:* `flatpak-builder` produces a build and `flatpak run` launches the game on Mint.

- [ ] **L. Submit to Flathub** (M)
  Open the submission pull request following the current Flathub guidelines (check them for the exact app ID format) and answer review comments.
  *Done when:* the app is merged and shows up in the Software Manager.

## Stretch (after a 1.0)

Arrows that wrap around cube edges are the hardest feature. Split into steps; each one leaves single-face play working.

- [ ] **M1. Cube surface map** (M): for each face and direction, which face you reach next and how coordinates and direction turn. Test that walking around the cube returns to the start.
- [ ] **M2. Blocking across edges** (M): follow a line's ray onto the next face.
- [ ] **M3. Generator for wrapping lines** (M): tile the whole cube surface, then break deadlocks as now.
- [ ] **M4. Rendering and picking** (M): draw and click lines that span two faces; slide them along the surface.
- [ ] **M5. Rebalance difficulty** (S): redo task F for the new level shapes.

Sound is deliberately not on the list: it needs an audio library, which makes the Flatpak build heavier.
