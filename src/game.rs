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

/// Share of a level's arrows the player may click wrongly before losing.
pub const MISTAKE_ALLOWANCE: f32 = 0.10;
/// The fewest failed clicks a level ever allows, however small it is.
pub const MIN_MISTAKE_LIMIT: u32 = 3;

/// How many failed clicks lose a level with `arrows` arrows on it: a tenth of
/// them, rounded up, and never fewer than `MIN_MISTAKE_LIMIT`.
pub fn mistake_limit_for(arrows: usize) -> u32 {
    ((arrows as f32 * MISTAKE_ALLOWANCE).ceil() as u32).max(MIN_MISTAKE_LIMIT)
}

/// How the current level stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Still being played.
    Playing,
    /// Every arrow is off.
    Won,
    /// The failed clicks reached the limit before the cube was cleared.
    Lost,
}

/// Running tally of finished levels. It outlives restarts, new levels and
/// grid changes; it is not saved between launches yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Score {
    pub wins: u32,
    pub losses: u32,
}

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
    /// Failed clicks that lose this level.
    pub mistake_limit: u32,
    pub outcome: Outcome,
    pub score: Score,
    history: Vec<LineId>,
}

impl Game {
    pub fn new(level: u32) -> Game {
        Game::with_grid(level, GridSize::default())
    }

    pub fn with_grid(level: u32, grid: GridSize) -> Game {
        Game::with_score(level, grid, Score::default())
    }

    /// A fresh level that carries an existing score along.
    pub fn with_score(level: u32, grid: GridSize, score: Score) -> Game {
        let level = level.max(1);
        let puzzle = Puzzle::generate_with(level, 0, grid);
        Game {
            mistake_limit: mistake_limit_for(puzzle.total()),
            puzzle,
            level,
            grid,
            outcome: Outcome::Playing,
            score,
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

    pub fn is_lost(&self) -> bool {
        self.outcome == Outcome::Lost
    }

    /// Failed clicks the player can still make before losing.
    pub fn tries_left(&self) -> u32 {
        self.mistake_limit.saturating_sub(self.mistakes)
    }

    /// True while any removed line is still sliding away.
    pub fn is_animating(&self) -> bool {
        !self.anims.is_empty()
    }

    /// The player clicked a line: remove it if it is free, otherwise count a
    /// mistake and show what is in the way.
    pub fn click(&mut self, id: LineId) -> ClickResult {
        // Once a level is won or lost, the result stands.
        if self.outcome != Outcome::Playing || !self.puzzle.is_alive(id) {
            return ClickResult::Ignored;
        }
        if let Some(blocker) = self.puzzle.blocker(id) {
            self.mistakes += 1;
            if self.mistakes >= self.mistake_limit {
                self.outcome = Outcome::Lost;
                self.score.losses += 1;
            }
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
        if self.puzzle.is_solved() {
            self.outcome = Outcome::Won;
            self.score.wins += 1;
        }
        ClickResult::Removed
    }

    /// Put the most recently removed line back. Failed clicks are not
    /// refunded, and a finished level cannot be undone.
    pub fn undo(&mut self) -> bool {
        if self.outcome != Outcome::Playing {
            return false;
        }
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
        if self.outcome != Outcome::Playing {
            return;
        }
        if let Some(&id) = self.puzzle.removable().first() {
            self.hint = Some(HintGlow { id, t: 0.0 });
        }
    }

    /// Start the current level over. The score is kept; walking away from a
    /// level that was still in play counts as neither a win nor a loss.
    pub fn restart(&mut self) {
        *self = Game::with_score(self.level, self.grid, self.score);
    }

    /// Move on to the next level.
    pub fn next_level(&mut self) {
        *self = Game::with_score(self.level + 1, self.grid, self.score);
    }

    /// Switch grid size. The current level starts over on the new grid, since
    /// progress on the old one cannot carry across. The score is kept.
    pub fn set_grid(&mut self, grid: GridSize) {
        if grid != self.grid {
            *self = Game::with_score(self.level, grid, self.score);
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

    /// Click blocked arrows until the level is decided or `n` clicks are made.
    fn make_mistakes(g: &mut Game, n: u32) {
        for _ in 0..n {
            let id = first_blocked(g).expect("a blocked arrow");
            g.click(id);
        }
    }

    #[test]
    fn the_mistake_limit_is_a_tenth_of_the_arrows_but_never_below_three() {
        assert_eq!(mistake_limit_for(0), 3);
        assert_eq!(mistake_limit_for(20), 3);
        assert_eq!(mistake_limit_for(30), 3);
        assert_eq!(mistake_limit_for(31), 4, "rounded up");
        assert_eq!(mistake_limit_for(130), 13);
        assert_eq!(mistake_limit_for(1000), 100);
        let g = Game::new(1);
        assert_eq!(g.mistake_limit, mistake_limit_for(g.puzzle.total()));
        assert_eq!(g.tries_left(), g.mistake_limit);
    }

    #[test]
    fn running_out_of_tries_loses_the_level_and_counts_one_loss() {
        let mut g = Game::new(3);
        assert_eq!(g.outcome, Outcome::Playing);
        let limit = g.mistake_limit;
        make_mistakes(&mut g, limit - 1);
        assert_eq!(g.outcome, Outcome::Playing, "one try still left");
        assert_eq!(g.tries_left(), 1);
        make_mistakes(&mut g, 1);
        assert_eq!(g.outcome, Outcome::Lost);
        assert!(g.is_lost());
        assert_eq!(g.tries_left(), 0);
        assert_eq!(g.score, Score { wins: 0, losses: 1 });
        // Further clicks neither change anything nor count a second loss.
        let before = (g.mistakes, g.puzzle.remaining());
        let free = g.puzzle.removable()[0];
        assert_eq!(g.click(free), ClickResult::Ignored);
        make_mistakes(&mut g, 5);
        assert_eq!((g.mistakes, g.puzzle.remaining()), before);
        assert_eq!(g.score.losses, 1);
    }

    #[test]
    fn clearing_every_arrow_wins_once() {
        let mut g = Game::new(4);
        while !g.is_solved() {
            let id = g.puzzle.removable()[0];
            assert_eq!(g.outcome, Outcome::Playing);
            g.click(id);
        }
        assert_eq!(g.outcome, Outcome::Won);
        assert_eq!(g.score, Score { wins: 1, losses: 0 });
        // A won level is final: no undo that would let it be won again.
        assert!(!g.undo());
        assert!(g.puzzle.is_solved());
        g.show_hint();
        assert!(g.hint.is_none());
        assert_eq!(g.score.wins, 1);
    }

    #[test]
    fn a_win_with_mistakes_below_the_limit_is_still_a_win() {
        let mut g = Game::new(4);
        let limit = g.mistake_limit;
        make_mistakes(&mut g, limit - 1);
        while !g.is_solved() {
            let id = g.puzzle.removable()[0];
            g.click(id);
        }
        assert_eq!(g.outcome, Outcome::Won);
        assert_eq!(g.score, Score { wins: 1, losses: 0 });
    }

    #[test]
    fn the_score_survives_restart_next_level_and_grid_changes() {
        let mut g = Game::new(2);
        let limit = g.mistake_limit;
        make_mistakes(&mut g, limit);
        assert_eq!(g.score.losses, 1);
        g.restart();
        assert_eq!(
            (g.outcome, g.mistakes, g.score.losses),
            (Outcome::Playing, 0, 1)
        );
        while !g.is_solved() {
            let id = g.puzzle.removable()[0];
            g.click(id);
        }
        assert_eq!(g.score, Score { wins: 1, losses: 1 });
        g.next_level();
        assert_eq!(g.level, 3);
        assert_eq!(g.score, Score { wins: 1, losses: 1 });
        g.set_grid(GridSize::Fixed(6));
        assert_eq!(g.score, Score { wins: 1, losses: 1 });
        assert_eq!(g.outcome, Outcome::Playing);
    }

    #[test]
    fn walking_away_from_a_level_in_play_is_not_a_loss() {
        let mut g = Game::new(3);
        make_mistakes(&mut g, 1);
        g.restart();
        g.set_grid(GridSize::Fixed(5));
        assert_eq!(g.score, Score::default());
    }

    #[test]
    fn failed_clicks_are_not_refunded_by_undo() {
        let mut g = Game::new(3);
        let id = g.puzzle.removable()[0];
        g.click(id);
        make_mistakes(&mut g, 2);
        assert!(g.undo());
        assert_eq!(g.mistakes, 2);
    }

    #[test]
    fn a_new_level_gets_a_limit_that_fits_its_own_arrow_count() {
        let small = Game::with_grid(1, GridSize::Fixed(4));
        let large = Game::with_grid(1, GridSize::Fixed(16));
        assert_eq!(small.mistake_limit, mistake_limit_for(small.puzzle.total()));
        assert_eq!(large.mistake_limit, mistake_limit_for(large.puzzle.total()));
        assert!(large.mistake_limit > small.mistake_limit);
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
    fn a_new_game_starts_on_a_ten_by_ten_grid_at_every_level() {
        for level in [1, 2, 9, 25] {
            let g = Game::new(level);
            assert_eq!(g.grid, GridSize::Fixed(10));
            assert_eq!(g.puzzle.size(), 10, "level {level}");
        }
    }

    #[test]
    fn auto_can_still_be_chosen_and_then_grows_with_the_level() {
        let mut g = Game::new(1);
        g.set_grid(GridSize::Auto);
        let first = g.puzzle.size();
        for _ in 0..10 {
            g.next_level();
        }
        assert_eq!(g.grid, GridSize::Auto);
        assert!(g.puzzle.size() > first, "Auto should grow as levels rise");
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
