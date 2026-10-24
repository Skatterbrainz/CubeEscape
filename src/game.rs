//! Game state on top of the puzzle: level progression, undo, mistakes and the
//! short-lived animation state (sliding lines, shakes, flashes, hints).

use crate::puzzle::{GridSize, LineId, Puzzle};

/// Seconds a removed line takes to slide off the cube.
pub const SLIDE_DURATION: f32 = 0.7;
/// Seconds a blocked line shakes for.
pub const SHAKE_DURATION: f32 = 0.35;
/// Seconds the blocking line is highlighted after a mistake.
pub const FLASH_DURATION: f32 = 0.8;
/// Seconds a hint stays highlighted.
pub const HINT_DURATION: f32 = 1.8;

/// A line that has been removed and is still sliding off.
#[derive(Clone, Copy, Debug)]
pub struct SlideAnim {
    pub id: LineId,
    pub t: f32,
}

/// A blocked line wiggling in place.
#[derive(Clone, Copy, Debug)]
pub struct Shake {
    pub id: LineId,
    pub t: f32,
}

/// The line that got in the way, highlighted so the player can see why.
#[derive(Clone, Copy, Debug)]
pub struct Flash {
    pub id: LineId,
    pub t: f32,
}

/// A suggested line to remove.
#[derive(Clone, Copy, Debug)]
pub struct HintGlow {
    pub id: LineId,
    pub t: f32,
}

/// What happened when the player clicked a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickResult {
    /// The line slid away.
    Removed,
    /// The line was blocked; carries the line standing in its way.
    Blocked(LineId),
    /// Nothing to do (already removed).
    Ignored,
}

pub struct Game {
    pub puzzle: Puzzle,
    pub level: u32,
    pub mistakes: u32,
    pub moves: u32,
    /// Total running time in seconds, used for pulsing effects.
    pub time: f32,
    pub anims: Vec<SlideAnim>,
    pub shakes: Vec<Shake>,
    pub flash: Option<Flash>,
    pub hint: Option<HintGlow>,
    /// The grid size the player chose; kept across levels and restarts.
    pub grid: GridSize,
    history: Vec<LineId>,
}

impl Game {
    pub fn new(level: u32) -> Game {
        Game::with_grid(level, GridSize::Auto)
    }

    pub fn with_grid(level: u32, grid: GridSize) -> Game {
        let level = level.max(1);
        Game {
            puzzle: Puzzle::generate_with(level, 0, grid),
            level,
            grid,
            mistakes: 0,
            moves: 0,
            time: 0.0,
            anims: Vec::new(),
            shakes: Vec::new(),
            flash: None,
            hint: None,
            history: Vec::new(),
        }
    }

    /// Advance animations by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for a in &mut self.anims {
            a.t += dt;
        }
        self.anims.retain(|a| a.t < SLIDE_DURATION);
        for s in &mut self.shakes {
            s.t += dt;
        }
        self.shakes.retain(|s| s.t < SHAKE_DURATION);
        if let Some(f) = &mut self.flash {
            f.t += dt;
            if f.t >= FLASH_DURATION {
                self.flash = None;
            }
        }
        if let Some(h) = &mut self.hint {
            h.t += dt;
            if h.t >= HINT_DURATION {
                self.hint = None;
            }
        }
    }

    pub fn is_solved(&self) -> bool {
        self.puzzle.is_solved()
    }

    /// True while any removed line is still sliding away.
    pub fn is_animating(&self) -> bool {
        !self.anims.is_empty()
    }

    /// The player clicked a line: remove it if it is free, otherwise count a
    /// mistake and show what is in the way.
    pub fn click(&mut self, id: LineId) -> ClickResult {
        if !self.puzzle.is_alive(id) {
            return ClickResult::Ignored;
        }
        if let Some(blocker) = self.puzzle.blocker(id) {
            self.mistakes += 1;
            self.shakes.retain(|s| s.id != id);
            self.shakes.push(Shake { id, t: 0.0 });
            self.flash = Some(Flash {
                id: blocker,
                t: 0.0,
            });
            return ClickResult::Blocked(blocker);
        }
        self.puzzle.remove(id);
        self.history.push(id);
        self.moves += 1;
        self.anims.push(SlideAnim { id, t: 0.0 });
        if self.hint.is_some_and(|h| h.id == id) {
            self.hint = None;
        }
        ClickResult::Removed
    }

    /// Put the most recently removed line back.
    pub fn undo(&mut self) -> bool {
        match self.history.pop() {
            Some(id) => {
                self.anims.retain(|a| a.id != id);
                self.puzzle.restore(id);
                self.moves = self.moves.saturating_sub(1);
                true
            }
            None => false,
        }
    }

    /// Highlight a line that can be removed right now.
    pub fn show_hint(&mut self) {
        if let Some(&id) = self.puzzle.removable().first() {
            self.hint = Some(HintGlow { id, t: 0.0 });
        }
    }

    /// Start the current level over.
    pub fn restart(&mut self) {
        *self = Game::with_grid(self.level, self.grid);
    }

    /// Move on to the next level.
    pub fn next_level(&mut self) {
        *self = Game::with_grid(self.level + 1, self.grid);
    }

    /// Switch grid size. The current level starts over on the new grid, since
    /// progress on the old one cannot carry across.
    pub fn set_grid(&mut self, grid: GridSize) {
        if grid != self.grid {
            *self = Game::with_grid(self.level, grid);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first_blocked(game: &Game) -> Option<LineId> {
        for f in 0..crate::puzzle::FACE_COUNT {
            for i in 0..game.puzzle.face(f).lines().len() {
                let id = LineId { face: f, index: i };
                if game.puzzle.blocker(id).is_some() {
                    return Some(id);
                }
            }
        }
        None
    }

    #[test]
    fn removing_a_free_line_counts_a_move() {
        let mut g = Game::new(3);
        let id = g.puzzle.removable()[0];
        assert_eq!(g.click(id), ClickResult::Removed);
        assert_eq!(g.moves, 1);
        assert_eq!(g.mistakes, 0);
        assert!(!g.puzzle.is_alive(id));
        assert!(g.is_animating());
        // Clicking it again does nothing.
        assert_eq!(g.click(id), ClickResult::Ignored);
    }

    #[test]
    fn clicking_a_blocked_line_is_a_mistake_and_removes_nothing() {
        let mut g = Game::new(5);
        let id = first_blocked(&g).expect("level 5 has a blocked line");
        let before = g.puzzle.remaining();
        match g.click(id) {
            ClickResult::Blocked(b) => assert!(g.puzzle.is_alive(b)),
            other => panic!("expected Blocked, got {other:?}"),
        }
        assert_eq!(g.mistakes, 1);
        assert_eq!(g.puzzle.remaining(), before);
        assert!(g.puzzle.is_alive(id));
        assert!(g.flash.is_some());
    }

    #[test]
    fn undo_restores_the_last_removed_line() {
        let mut g = Game::new(4);
        let before = g.puzzle.remaining();
        let id = g.puzzle.removable()[0];
        g.click(id);
        assert_eq!(g.puzzle.remaining(), before - 1);
        assert!(g.undo());
        assert_eq!(g.puzzle.remaining(), before);
        assert_eq!(g.moves, 0);
        assert!(!g.is_animating());
        assert!(!g.undo(), "nothing left to undo");
    }

    #[test]
    fn a_whole_level_can_be_played_and_won() {
        let mut g = Game::new(6);
        while !g.is_solved() {
            let id = g.puzzle.removable()[0];
            assert_eq!(g.click(id), ClickResult::Removed);
        }
        assert_eq!(g.mistakes, 0);
        // Let the last animations finish.
        g.update(SLIDE_DURATION + 0.1);
        assert!(!g.is_animating());
        let level = g.level;
        g.next_level();
        assert_eq!(g.level, level + 1);
        assert!(!g.is_solved());
    }

    #[test]
    fn animations_expire() {
        let mut g = Game::new(5);
        let blocked = first_blocked(&g).unwrap();
        g.click(blocked);
        g.show_hint();
        g.update(HINT_DURATION + 0.1);
        assert!(g.shakes.is_empty());
        assert!(g.flash.is_none());
        assert!(g.hint.is_none());
    }

    #[test]
    fn changing_the_grid_restarts_the_level_on_the_new_size() {
        let mut g = Game::new(3);
        let auto_size = g.puzzle.size();
        let id = g.puzzle.removable()[0];
        g.click(id);
        assert_eq!(g.moves, 1);

        g.set_grid(GridSize::Fixed(auto_size + 3));
        assert_eq!(g.puzzle.size(), auto_size + 3);
        assert_eq!(g.level, 3, "same level");
        assert_eq!(g.moves, 0, "progress is reset");
        assert_eq!(g.puzzle.remaining(), g.puzzle.total());
        assert!(!g.undo(), "old history is gone");
    }

    #[test]
    fn choosing_the_same_grid_again_does_not_wipe_progress() {
        let mut g = Game::with_grid(3, GridSize::Fixed(7));
        let id = g.puzzle.removable()[0];
        g.click(id);
        g.set_grid(GridSize::Fixed(7));
        assert_eq!(g.moves, 1);
    }

    #[test]
    fn the_chosen_grid_survives_restart_and_next_level() {
        let mut g = Game::with_grid(2, GridSize::Fixed(9));
        g.restart();
        assert_eq!((g.puzzle.size(), g.grid), (9, GridSize::Fixed(9)));
        g.next_level();
        assert_eq!(g.level, 3);
        assert_eq!((g.puzzle.size(), g.grid), (9, GridSize::Fixed(9)));
        g.set_grid(GridSize::Auto);
        g.next_level();
        assert_eq!(g.grid, GridSize::Auto);
    }

    #[test]
    fn restart_resets_progress_but_keeps_the_level() {
        let mut g = Game::new(3);
        let id = g.puzzle.removable()[0];
        g.click(id);
        g.restart();
        assert_eq!(g.level, 3);
        assert_eq!(g.moves, 0);
        assert_eq!(g.puzzle.remaining(), g.puzzle.total());
    }
}
