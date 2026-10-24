//! Core puzzle model: faces, arrow lines, blocking rules and a generator that
//! only produces solvable levels. This module has no graphics dependencies.
//!
//! # Rules
//! Every face of the cube is an `N x N` grid. A [`Line`] is a path of
//! orthogonally adjacent cells. Its *head* is the last cell, and it points in
//! the direction of its final step. A line can be removed only when every cell
//! straight ahead of the head, out to the edge of the face, is empty.
//!
//! # Why generated levels are always solvable
//! Removing a line only ever frees cells, so it can never make another line
//! harder to remove. That means a board is solvable exactly when repeatedly
//! removing whatever is free eventually clears it. The generator tiles a face
//! densely with random lines, runs that check, and breaks every deadlock it
//! finds (by turning a line around or deleting it) until the check passes.

/// Number of faces on the cube.
pub const FACE_COUNT: usize = 6;

/// Cells per side of each face on level 1.
pub const MIN_FACE_SIZE: i32 = 6;

/// Cells per side of each face once levels have grown as far as they go.
pub const MAX_FACE_SIZE: i32 = 12;

/// Cells per side of each face when the game starts.
pub const DEFAULT_GRID_SIZE: i32 = 10;

/// Smallest grid the player can pick.
pub const MIN_GRID_SIZE: i32 = 4;

/// Largest grid the player can pick.
pub const MAX_GRID_SIZE: i32 = 16;

/// Number of distinct line colours the generator hands out.
pub const PALETTE_SIZE: usize = 10;

/// A direction on a face grid. `Up` is +y, `Right` is +x.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    /// The (dx, dy) step for this direction.
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, 1),
            Dir::Down => (0, -1),
            Dir::Left => (-1, 0),
            Dir::Right => (1, 0),
        }
    }

    /// The direction for a single-cell step, if it is one.
    pub fn from_delta(dx: i32, dy: i32) -> Option<Dir> {
        match (dx, dy) {
            (0, 1) => Some(Dir::Up),
            (0, -1) => Some(Dir::Down),
            (-1, 0) => Some(Dir::Left),
            (1, 0) => Some(Dir::Right),
            _ => None,
        }
    }
}

/// Identifies one line on one face.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LineId {
    pub face: usize,
    pub index: usize,
}

/// A path of adjacent cells, tail first, head last.
#[derive(Clone, Debug)]
pub struct Line {
    pub cells: Vec<(i32, i32)>,
    /// Index into the colour palette (`0..PALETTE_SIZE`).
    pub color: usize,
}

impl Line {
    /// The cell the arrowhead sits on.
    pub fn head(&self) -> (i32, i32) {
        *self.cells.last().expect("a line has at least two cells")
    }

    /// The direction the arrowhead points: the direction of the last step.
    pub fn dir(&self) -> Dir {
        let n = self.cells.len();
        let (ax, ay) = self.cells[n - 2];
        let (bx, by) = self.cells[n - 1];
        Dir::from_delta(bx - ax, by - ay).expect("line cells must be adjacent")
    }
}

/// One `size x size` face and the lines on it.
#[derive(Clone, Debug)]
pub struct Face {
    size: i32,
    lines: Vec<Line>,
    alive: Vec<bool>,
    /// Which live line occupies each cell, row-major (`y * size + x`).
    owner: Vec<Option<usize>>,
}

impl Face {
    pub fn new(size: i32, lines: Vec<Line>) -> Face {
        let mut owner = vec![None; (size * size) as usize];
        for (i, line) in lines.iter().enumerate() {
            for &(x, y) in &line.cells {
                owner[(y * size + x) as usize] = Some(i);
            }
        }
        let alive = vec![true; lines.len()];
        Face {
            size,
            lines,
            alive,
            owner,
        }
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn is_alive(&self, index: usize) -> bool {
        self.alive[index]
    }

    pub fn alive_count(&self) -> usize {
        self.alive.iter().filter(|&&a| a).count()
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.size && y < self.size
    }

    /// The live line occupying a cell, if any.
    pub fn owner_at(&self, x: i32, y: i32) -> Option<usize> {
        if self.contains(x, y) {
            self.owner[(y * self.size + x) as usize]
        } else {
            None
        }
    }

    /// The first live line standing in front of line `index`'s arrowhead.
    /// A line whose own body is in front of its head blocks itself.
    pub fn blocker(&self, index: usize) -> Option<usize> {
        let line = &self.lines[index];
        let (hx, hy) = line.head();
        let (dx, dy) = line.dir().delta();
        let (mut x, mut y) = (hx + dx, hy + dy);
        while self.contains(x, y) {
            if let Some(o) = self.owner[(y * self.size + x) as usize] {
                return Some(o);
            }
            x += dx;
            y += dy;
        }
        None
    }

    pub(crate) fn remove(&mut self, index: usize) {
        self.alive[index] = false;
        for &(x, y) in &self.lines[index].cells {
            self.owner[(y * self.size + x) as usize] = None;
        }
    }

    pub(crate) fn restore(&mut self, index: usize) {
        self.alive[index] = true;
        for &(x, y) in &self.lines[index].cells {
            self.owner[(y * self.size + x) as usize] = Some(index);
        }
    }
}

/// A small, fast, deterministic PRNG (xorshift64*). Levels must be
/// reproducible from their number alone, so we avoid external randomness.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut s = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03;
        if s == 0 {
            s = 0x2545_F491_4F6C_DD1D;
        }
        let mut rng = Rng(s);
        for _ in 0..4 {
            rng.next_u64();
        }
        rng
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in `0..n`. `n` must be non-zero.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// True with probability `p`.
    pub fn chance(&mut self, p: f32) -> bool {
        ((self.next_u64() >> 40) as f32 / (1u64 << 24) as f32) < p
    }

    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i + 1);
            items.swap(i, j);
        }
    }
}

/// How many cells each face has on a side: either grown automatically with
/// the level, or fixed by the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridSize {
    /// Start small and grow as levels rise (`MIN_FACE_SIZE` to `MAX_FACE_SIZE`).
    Auto,
    /// The same `n x n` grid on every level.
    Fixed(i32),
}

impl Default for GridSize {
    /// A fixed `DEFAULT_GRID_SIZE` grid. `Auto` is still on the selector.
    fn default() -> GridSize {
        GridSize::Fixed(DEFAULT_GRID_SIZE)
    }
}

impl GridSize {
    /// One step up the selector: Auto, then 4, 5, ... up to the largest grid.
    pub fn bigger(self) -> GridSize {
        match self {
            GridSize::Auto => GridSize::Fixed(MIN_GRID_SIZE),
            GridSize::Fixed(n) => GridSize::Fixed((n + 1).clamp(MIN_GRID_SIZE, MAX_GRID_SIZE)),
        }
    }

    /// One step down the selector, ending at Auto.
    pub fn smaller(self) -> GridSize {
        match self {
            GridSize::Auto => GridSize::Auto,
            GridSize::Fixed(n) if n <= MIN_GRID_SIZE => GridSize::Auto,
            GridSize::Fixed(n) => GridSize::Fixed(n.min(MAX_GRID_SIZE) - 1),
        }
    }

    pub fn can_grow(self) -> bool {
        self != GridSize::Fixed(MAX_GRID_SIZE)
    }

    pub fn can_shrink(self) -> bool {
        self != GridSize::Auto
    }

    /// Text for the screen; `current` is the size actually in use.
    pub fn label(self, current: i32) -> String {
        match self {
            GridSize::Auto => format!("Auto ({current}x{current})"),
            GridSize::Fixed(n) => format!("{n}x{n}"),
        }
    }
}

/// Difficulty knobs for a level.
#[derive(Clone, Copy, Debug)]
pub struct LevelParams {
    /// Cells per side of each face.
    pub size: i32,
    /// Longest of the short filler lines, in cells.
    pub max_len: usize,
    /// Longest of the long winding lines, in cells. These can run most of the
    /// way across a face.
    pub long_len: usize,
    /// How many long lines to try to lay down before filling the gaps.
    pub long_count: usize,
}

impl LevelParams {
    fn with_size(level: u32, size: i32) -> LevelParams {
        let cells = (size * size) as usize;
        LevelParams {
            size,
            max_len: (4 + (level / 2) as usize).min(12),
            long_len: (cells / 3).clamp(10, 60),
            long_count: (size / 2) as usize,
        }
    }

    /// The parameters for a level when the grid grows with the level.
    pub fn for_level(level: u32) -> LevelParams {
        let level = level.max(1);
        let size = (MIN_FACE_SIZE + ((level - 1) / 2) as i32).min(MAX_FACE_SIZE);
        LevelParams::with_size(level, size)
    }

    /// Like `for_level`, but with the grid size the player asked for.
    pub fn for_level_with(level: u32, grid: GridSize) -> LevelParams {
        match grid {
            GridSize::Auto => LevelParams::for_level(level),
            GridSize::Fixed(n) => {
                LevelParams::with_size(level.max(1), n.clamp(MIN_GRID_SIZE, MAX_GRID_SIZE))
            }
        }
    }

    /// The longest a line can be on this level.
    pub fn longest(&self) -> usize {
        self.max_len.max(self.long_len)
    }
}

/// Remove everything that can be removed, over and over, and return the face
/// in that final state. Lines still alive in it are deadlocked.
fn clear_greedy(size: i32, lines: &[Line]) -> Face {
    let mut face = Face::new(size, lines.to_vec());
    loop {
        let free: Vec<usize> = (0..face.lines().len())
            .filter(|&i| face.is_alive(i) && face.blocker(i).is_none())
            .collect();
        if free.is_empty() {
            return face;
        }
        for i in free {
            face.remove(i);
        }
    }
}

fn stuck_lines(face: &Face) -> Vec<usize> {
    (0..face.lines().len())
        .filter(|&i| face.is_alive(i))
        .collect()
}

/// Grow a self-avoiding path from `start`, marking each cell it takes in
/// `occupied`, until it has `want` cells or runs out of room.
///
/// A winding path prefers to keep going straight, then to hug cells that have
/// few free neighbours (which sweeps it along walls and into pockets instead
/// of curling up on itself). Both give long paths with many bends that cross
/// a good part of the face.
fn grow_path(
    start: (i32, i32),
    want: usize,
    occupied: &mut [bool],
    n: i32,
    winding: bool,
    rng: &mut Rng,
) -> Vec<(i32, i32)> {
    let free = |occupied: &[bool], x: i32, y: i32| {
        x >= 0 && y >= 0 && x < n && y < n && !occupied[(y * n + x) as usize]
    };
    occupied[(start.1 * n + start.0) as usize] = true;
    let mut path = vec![start];
    while path.len() < want {
        let (cx, cy) = *path.last().unwrap();
        let options: Vec<(i32, i32)> = Dir::ALL
            .iter()
            .map(|d| {
                let (dx, dy) = d.delta();
                (cx + dx, cy + dy)
            })
            .filter(|&(x, y)| free(occupied, x, y))
            .collect();
        if options.is_empty() {
            break;
        }
        let next = if winding {
            let roll = rng.below(100);
            let straight = if path.len() >= 2 {
                let (px, py) = path[path.len() - 2];
                let (dx, dy) = (cx - px, cy - py);
                options.iter().copied().find(|&o| o == (cx + dx, cy + dy))
            } else {
                None
            };
            if let (true, Some(s)) = (roll < 55, straight) {
                s
            } else if roll < 85 {
                // Tuck into the tightest spot.
                let degree = |&(x, y): &(i32, i32)| {
                    Dir::ALL
                        .iter()
                        .filter(|d| {
                            let (dx, dy) = d.delta();
                            free(occupied, x + dx, y + dy)
                        })
                        .count()
                };
                let best = options.iter().map(degree).min().unwrap();
                let tight: Vec<(i32, i32)> = options
                    .iter()
                    .copied()
                    .filter(|o| degree(o) == best)
                    .collect();
                tight[rng.below(tight.len())]
            } else {
                options[rng.below(options.len())]
            }
        } else {
            options[rng.below(options.len())]
        };
        occupied[(next.1 * n + next.0) as usize] = true;
        path.push(next);
    }
    path
}

/// Fill every free spot that can hold a line with short random lines.
fn fill_short(params: &LevelParams, occupied: &mut [bool], lines: &mut Vec<Line>, rng: &mut Rng) {
    let n = params.size;
    let mut order: Vec<usize> = (0..(n * n) as usize).collect();
    rng.shuffle(&mut order);
    for c in order {
        if occupied[c] {
            continue;
        }
        let want = 2 + rng.below(params.max_len - 1);
        let winding = rng.chance(0.5);
        let start = (c as i32 % n, c as i32 / n);
        let mut path = grow_path(start, want, occupied, n, winding, rng);
        if path.len() < 2 {
            // Nothing free next to it: leave the cell empty.
            continue;
        }
        if rng.chance(0.5) {
            path.reverse();
        }
        lines.push(Line {
            cells: path,
            color: 0,
        });
    }
}

fn occupied_cells(size: i32, lines: &[Line]) -> Vec<bool> {
    let mut occupied = vec![false; (size * size) as usize];
    for line in lines {
        for &(x, y) in &line.cells {
            occupied[(y * size + x) as usize] = true;
        }
    }
    occupied
}

/// Cover a face with lines: first a few long winding ones that cross much of
/// the face, then short ones in every gap that is left.
fn tile_face(params: &LevelParams, rng: &mut Rng) -> Vec<Line> {
    let n = params.size;
    let mut occupied = vec![false; (n * n) as usize];
    let mut lines = Vec::new();

    for _ in 0..params.long_count {
        let free: Vec<usize> = (0..occupied.len()).filter(|&c| !occupied[c]).collect();
        if free.is_empty() {
            break;
        }
        let c = free[rng.below(free.len())];
        let start = (c as i32 % n, c as i32 / n);
        // Between half and all of the longest length.
        let want = params.long_len / 2 + rng.below(params.long_len / 2 + 1);
        let mut path = grow_path(start, want.max(2), &mut occupied, n, true, rng);
        if path.len() < 2 {
            occupied[(start.1 * n + start.0) as usize] = false;
            continue;
        }
        if rng.chance(0.5) {
            path.reverse();
        }
        lines.push(Line {
            cells: path,
            color: 0,
        });
    }

    fill_short(params, &mut occupied, &mut lines, rng);
    lines
}

/// Fix deadlocks until every line can eventually be removed.
///
/// A deadlock is a cycle of lines that each block the next. We find one by
/// following blockers from any stuck line, then break it by turning one of its
/// lines around if that helps, and otherwise deleting its shortest line (so
/// the long lines survive).
fn break_deadlocks(size: i32, lines: &mut Vec<Line>, rng: &mut Rng) {
    loop {
        let face = clear_greedy(size, lines);
        let stuck = stuck_lines(&face);
        if stuck.is_empty() {
            return;
        }

        let mut walk: Vec<usize> = Vec::new();
        let mut cur = stuck[rng.below(stuck.len())];
        let cycle = loop {
            if let Some(pos) = walk.iter().position(|&x| x == cur) {
                break walk[pos..].to_vec();
            }
            walk.push(cur);
            match face.blocker(cur) {
                Some(next) => cur = next,
                None => break vec![cur],
            }
        };

        let mut order = cycle.clone();
        rng.shuffle(&mut order);
        let mut fixed = false;
        for &v in &order {
            lines[v].cells.reverse();
            if stuck_lines(&clear_greedy(size, lines)).len() < stuck.len() {
                fixed = true;
                break;
            }
            lines[v].cells.reverse();
        }
        if !fixed {
            let victim = *cycle.iter().min_by_key(|&&i| lines[i].cells.len()).unwrap();
            lines.remove(victim);
        }
    }
}

/// Give each line a colour, avoiding the colours of lines it touches.
fn assign_colors(size: i32, lines: &mut [Line], rng: &mut Rng) {
    let face = Face::new(size, lines.to_vec());
    let mut colors: Vec<Option<usize>> = vec![None; lines.len()];
    for i in 0..lines.len() {
        let mut used = [false; PALETTE_SIZE];
        for &(x, y) in &lines[i].cells {
            for d in Dir::ALL {
                let (dx, dy) = d.delta();
                if let Some(j) = face.owner_at(x + dx, y + dy) {
                    if j != i {
                        if let Some(c) = colors[j] {
                            used[c] = true;
                        }
                    }
                }
            }
        }
        let free: Vec<usize> = (0..PALETTE_SIZE).filter(|&c| !used[c]).collect();
        colors[i] = Some(if free.is_empty() {
            rng.below(PALETTE_SIZE)
        } else {
            free[rng.below(free.len())]
        });
    }
    for (line, color) in lines.iter_mut().zip(colors) {
        line.color = color.unwrap();
    }
}

fn generate_face(params: &LevelParams, rng: &mut Rng) -> Face {
    let mut lines = tile_face(params, rng);
    // Breaking deadlocks can delete lines and leave holes, so refill them and
    // check again. Each pass only ever adds short lines.
    for _ in 0..4 {
        break_deadlocks(params.size, &mut lines, rng);
        let before = lines.len();
        let mut occupied = occupied_cells(params.size, &lines);
        fill_short(params, &mut occupied, &mut lines, rng);
        if lines.len() == before {
            break;
        }
    }
    // The last refill may have added a deadlock of its own.
    break_deadlocks(params.size, &mut lines, rng);
    assign_colors(params.size, &mut lines, rng);
    Face::new(params.size, lines)
}

/// The whole cube: six faces of lines.
#[derive(Clone, Debug)]
pub struct Puzzle {
    faces: Vec<Face>,
    level: u32,
}

impl Puzzle {
    /// Build a level. The same `(level, salt)` always gives the same puzzle.
    pub fn generate(level: u32, salt: u64) -> Puzzle {
        Puzzle::generate_with(level, salt, GridSize::Auto)
    }

    /// Build a level with a chosen grid size. The same arguments always give
    /// the same puzzle.
    pub fn generate_with(level: u32, salt: u64, grid: GridSize) -> Puzzle {
        let params = LevelParams::for_level_with(level, grid);
        let faces = (0..FACE_COUNT)
            .map(|f| {
                let seed = (level as u64) * 1_000_003 + (f as u64) * 7_919 + salt * 31 + 17;
                generate_face(&params, &mut Rng::new(seed))
            })
            .collect();
        Puzzle { faces, level }
    }

    pub fn level(&self) -> u32 {
        self.level
    }

    /// Cells per side of each face.
    pub fn size(&self) -> i32 {
        self.faces[0].size()
    }

    pub fn face(&self, face: usize) -> &Face {
        &self.faces[face]
    }

    pub fn is_alive(&self, id: LineId) -> bool {
        self.faces[id.face].is_alive(id.index)
    }

    /// The live line blocking `id`, if any.
    pub fn blocker(&self, id: LineId) -> Option<LineId> {
        self.faces[id.face].blocker(id.index).map(|index| LineId {
            face: id.face,
            index,
        })
    }

    /// Remove a line. The caller is responsible for checking `blocker` first.
    pub fn remove(&mut self, id: LineId) {
        self.faces[id.face].remove(id.index);
    }

    /// Put a removed line back (used by undo, which restores in LIFO order).
    pub fn restore(&mut self, id: LineId) {
        self.faces[id.face].restore(id.index);
    }

    pub fn remaining(&self) -> usize {
        self.faces.iter().map(|f| f.alive_count()).sum()
    }

    pub fn total(&self) -> usize {
        self.faces.iter().map(|f| f.lines().len()).sum()
    }

    pub fn is_solved(&self) -> bool {
        self.remaining() == 0
    }

    /// Every live line that can be removed right now.
    pub fn removable(&self) -> Vec<LineId> {
        let mut out = Vec::new();
        for (f, face) in self.faces.iter().enumerate() {
            for i in 0..face.lines().len() {
                if face.is_alive(i) && face.blocker(i).is_none() {
                    out.push(LineId { face: f, index: i });
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(cells: &[(i32, i32)]) -> Line {
        Line {
            cells: cells.to_vec(),
            color: 0,
        }
    }

    /// Fraction of the cube's cells covered by lines.
    fn coverage(p: &Puzzle) -> f32 {
        let mut covered = 0usize;
        let mut cells = 0usize;
        for f in 0..FACE_COUNT {
            let face = p.face(f);
            cells += (face.size() * face.size()) as usize;
            covered += face.lines().iter().map(|l| l.cells.len()).sum::<usize>();
        }
        covered as f32 / cells as f32
    }

    #[test]
    fn head_and_direction_come_from_the_last_step() {
        let l = line(&[(0, 0), (1, 0), (1, 1)]);
        assert_eq!(l.head(), (1, 1));
        assert_eq!(l.dir(), Dir::Up);
        let l = line(&[(2, 2), (1, 2)]);
        assert_eq!(l.dir(), Dir::Left);
    }

    #[test]
    fn a_line_is_blocked_by_another_in_its_path() {
        // A points right along row 0; B stands in column 3 of the same row.
        let face = Face::new(5, vec![line(&[(0, 0), (1, 0)]), line(&[(3, 0), (3, 1)])]);
        assert_eq!(face.blocker(0), Some(1));
        // B points up, with nothing above it.
        assert_eq!(face.blocker(1), None);
    }

    #[test]
    fn only_cells_in_front_of_the_head_block() {
        // B is behind A's head (to the left), so it must not block.
        let face = Face::new(5, vec![line(&[(2, 0), (3, 0)]), line(&[(0, 0), (0, 1)])]);
        assert_eq!(face.blocker(0), None);
    }

    #[test]
    fn a_line_can_block_itself() {
        // A U shape whose head points down and out of the grid is free.
        let face = Face::new(4, vec![line(&[(0, 0), (0, 1), (1, 1), (1, 0)])]);
        assert_eq!(face.blocker(0), None);
        // A spiral whose head points at its own body is stuck behind itself.
        let spiral = line(&[(0, 0), (1, 0), (2, 0), (2, 1), (2, 2), (1, 2), (1, 1)]);
        assert_eq!(spiral.dir(), Dir::Down);
        let face = Face::new(4, vec![spiral]);
        assert_eq!(face.blocker(0), Some(0));
    }

    #[test]
    fn removing_the_blocker_frees_the_line() {
        let mut p = Puzzle {
            faces: vec![Face::new(
                5,
                vec![line(&[(0, 0), (1, 0)]), line(&[(3, 0), (3, 1)])],
            )],
            level: 1,
        };
        let a = LineId { face: 0, index: 0 };
        let b = LineId { face: 0, index: 1 };
        assert_eq!(p.blocker(a), Some(b));
        p.remove(b);
        assert_eq!(p.blocker(a), None);
        p.restore(b);
        assert_eq!(p.blocker(a), Some(b));
    }

    #[test]
    fn deadlocks_are_broken() {
        // Two lines pointing straight at each other can never both leave.
        let mut lines = vec![line(&[(0, 1), (1, 1)]), line(&[(3, 1), (2, 1)])];
        assert_eq!(stuck_lines(&clear_greedy(4, &lines)).len(), 2);
        break_deadlocks(4, &mut lines, &mut Rng::new(1));
        assert!(stuck_lines(&clear_greedy(4, &lines)).is_empty());
        assert!(!lines.is_empty(), "turning one around should be enough");
    }

    #[test]
    fn generation_is_deterministic() {
        let a = Puzzle::generate(7, 0);
        let b = Puzzle::generate(7, 0);
        for f in 0..FACE_COUNT {
            let (fa, fb) = (a.face(f), b.face(f));
            assert_eq!(fa.lines().len(), fb.lines().len());
            for (la, lb) in fa.lines().iter().zip(fb.lines()) {
                assert_eq!(la.cells, lb.cells);
                assert_eq!(la.color, lb.color);
            }
        }
    }

    #[test]
    fn every_generated_level_is_solvable() {
        // Removal never blocks anything, so greedily removing whatever is
        // free must clear the board if and only if the level is solvable.
        for level in 1..=40 {
            for salt in 0..3 {
                let mut p = Puzzle::generate(level, salt);
                assert!(p.total() > 0, "level {level} salt {salt} is empty");
                loop {
                    let free = p.removable();
                    if free.is_empty() {
                        break;
                    }
                    for id in free {
                        p.remove(id);
                    }
                }
                assert!(p.is_solved(), "level {level} salt {salt} is not solvable");
            }
        }
    }

    #[test]
    fn generated_lines_are_well_formed() {
        for level in [1, 4, 9, 16, 30] {
            let p = Puzzle::generate(level, 0);
            let params = LevelParams::for_level(level);
            for f in 0..FACE_COUNT {
                let face = p.face(f);
                let mut seen = std::collections::HashSet::new();
                for (i, l) in face.lines().iter().enumerate() {
                    assert!(l.cells.len() >= 2 && l.cells.len() <= params.longest());
                    assert!(l.color < PALETTE_SIZE);
                    for w in l.cells.windows(2) {
                        let d = (w[1].0 - w[0].0, w[1].1 - w[0].1);
                        assert!(Dir::from_delta(d.0, d.1).is_some(), "cells not adjacent");
                    }
                    for &(x, y) in &l.cells {
                        assert!(face.contains(x, y), "cell out of bounds");
                        assert!(seen.insert((x, y)), "cell used twice");
                    }
                    // A line must never stand in its own way.
                    assert_ne!(face.blocker(i), Some(i));
                }
            }
        }
    }

    #[test]
    fn boards_are_densely_packed() {
        for level in [1, 3, 6, 10, 20] {
            let mut sum = 0.0;
            for salt in 0..4 {
                sum += coverage(&Puzzle::generate(level, salt));
            }
            let avg = sum / 4.0;
            assert!(
                avg >= 0.7,
                "level {level} is only {:.0}% covered",
                avg * 100.0
            );
        }
    }

    fn bends(cells: &[(i32, i32)]) -> usize {
        cells
            .windows(3)
            .filter(|w| (w[1].0 - w[0].0, w[1].1 - w[0].1) != (w[2].0 - w[1].0, w[2].1 - w[1].1))
            .count()
    }

    /// Every line on every face of a few generated cubes.
    fn all_lines(size: i32, level: u32, salts: u64) -> Vec<Line> {
        let mut out = Vec::new();
        for salt in 0..salts {
            let p = Puzzle::generate_with(level, salt, GridSize::Fixed(size));
            for f in 0..FACE_COUNT {
                out.extend(p.face(f).lines().iter().cloned());
            }
        }
        out
    }

    #[test]
    fn some_lines_are_long_and_wind_a_lot() {
        for size in [8, 12, 16] {
            let params = LevelParams::for_level_with(10, GridSize::Fixed(size));
            let lines = all_lines(size, 10, 4);
            let longest = lines.iter().map(|l| l.cells.len()).max().unwrap();
            let most_bends = lines.iter().map(|l| bends(&l.cells)).max().unwrap();
            assert!(
                longest >= params.long_len / 2 + params.long_len / 4,
                "{size}x{size}: longest line is only {longest} cells"
            );
            assert!(
                most_bends >= 8,
                "{size}x{size}: most winding line has only {most_bends} bends"
            );
        }
    }

    #[test]
    fn some_lines_stretch_across_a_whole_face() {
        // A line touches both opposite edges of its face, in either direction.
        for size in [6, 8, 12, 16] {
            let lines = all_lines(size, 10, 4);
            let spans = |l: &Line| {
                let (mut x0, mut x1, mut y0, mut y1) = (size, 0, size, 0);
                for &(x, y) in &l.cells {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                }
                (x0 == 0 && x1 == size - 1) || (y0 == 0 && y1 == size - 1)
            };
            assert!(
                lines.iter().any(spans),
                "{size}x{size}: no line spans a whole face"
            );
        }
    }

    #[test]
    fn lines_are_longer_and_bendier_than_before_on_average() {
        // The old generator averaged about 4.5 cells and 1.5 bends on 12x12.
        let lines = all_lines(12, 13, 4);
        let n = lines.len() as f32;
        let avg_len = lines.iter().map(|l| l.cells.len()).sum::<usize>() as f32 / n;
        let avg_bends = lines.iter().map(|l| bends(&l.cells)).sum::<usize>() as f32 / n;
        assert!(avg_len >= 5.5, "average length is {avg_len:.1}");
        assert!(avg_bends >= 1.7, "average bends is {avg_bends:.2}");
    }

    #[test]
    fn levels_start_with_real_choices() {
        // A puzzle where everything is free to remove would not be a puzzle,
        // and one with nothing free could not be started.
        for level in [3, 6, 10] {
            let p = Puzzle::generate(level, 0);
            assert!(
                p.removable().len() * 5 < p.total() * 4,
                "level {level}: too few blocked lines"
            );
            assert!(!p.removable().is_empty(), "level {level} has no free lines");
        }
    }

    #[test]
    fn grid_selector_steps_through_every_size_and_stops_at_the_ends() {
        let mut g = GridSize::Auto;
        assert!(!g.can_shrink());
        assert_eq!(g.smaller(), GridSize::Auto);
        let mut seen = Vec::new();
        while g.can_grow() {
            g = g.bigger();
            if let GridSize::Fixed(n) = g {
                seen.push(n);
            }
        }
        assert_eq!(seen, (MIN_GRID_SIZE..=MAX_GRID_SIZE).collect::<Vec<_>>());
        assert_eq!(g.bigger(), g, "cannot grow past the largest grid");
        // And back down to Auto again.
        for n in (MIN_GRID_SIZE..MAX_GRID_SIZE).rev() {
            g = g.smaller();
            assert_eq!(g, GridSize::Fixed(n));
        }
        assert_eq!(g.smaller(), GridSize::Auto);
    }

    #[test]
    fn the_default_grid_is_ten_by_ten_and_inside_the_selectable_range() {
        assert_eq!(GridSize::default(), GridSize::Fixed(10));
        assert!((MIN_GRID_SIZE..=MAX_GRID_SIZE).contains(&DEFAULT_GRID_SIZE));
        // Auto is still reachable from the default by stepping down.
        let mut g = GridSize::default();
        while g.can_shrink() {
            g = g.smaller();
        }
        assert_eq!(g, GridSize::Auto);
    }

    #[test]
    fn grid_labels_name_the_size_in_use() {
        assert_eq!(GridSize::Auto.label(6), "Auto (6x6)");
        assert_eq!(GridSize::Fixed(9).label(6), "9x9");
    }

    #[test]
    fn a_fixed_grid_size_overrides_the_level_and_is_kept_in_range() {
        for level in [1, 9, 30] {
            let p = Puzzle::generate_with(level, 0, GridSize::Fixed(7));
            assert_eq!(p.size(), 7);
            assert_eq!(p.level(), level);
        }
        assert_eq!(
            Puzzle::generate_with(1, 0, GridSize::Fixed(1)).size(),
            MIN_GRID_SIZE
        );
        assert_eq!(
            Puzzle::generate_with(1, 0, GridSize::Fixed(99)).size(),
            MAX_GRID_SIZE
        );
        // Auto is exactly what `generate` has always done.
        for level in [1, 6, 14] {
            let a = Puzzle::generate(level, 0);
            let b = Puzzle::generate_with(level, 0, GridSize::Auto);
            assert_eq!(a.size(), b.size());
            assert_eq!(a.total(), b.total());
        }
    }

    #[test]
    fn every_selectable_grid_size_makes_solvable_well_formed_levels() {
        for n in MIN_GRID_SIZE..=MAX_GRID_SIZE {
            for level in [1, 7, 15] {
                for salt in 0..2 {
                    let mut p = Puzzle::generate_with(level, salt, GridSize::Fixed(n));
                    assert_eq!(p.size(), n);
                    assert!(p.total() > 0, "{n}x{n} level {level} is empty");
                    for f in 0..FACE_COUNT {
                        let face = p.face(f);
                        let mut seen = std::collections::HashSet::new();
                        for (i, l) in face.lines().iter().enumerate() {
                            assert!(l.cells.len() >= 2);
                            for &(x, y) in &l.cells {
                                assert!(face.contains(x, y));
                                assert!(seen.insert((x, y)), "cell used twice");
                            }
                            assert_ne!(face.blocker(i), Some(i));
                        }
                    }
                    loop {
                        let free = p.removable();
                        if free.is_empty() {
                            break;
                        }
                        for id in free {
                            p.remove(id);
                        }
                    }
                    assert!(p.is_solved(), "{n}x{n} level {level} salt {salt} is stuck");
                }
            }
        }
    }

    #[test]
    fn every_selectable_grid_size_is_reasonably_full() {
        for n in MIN_GRID_SIZE..=MAX_GRID_SIZE {
            let mut sum = 0.0;
            for salt in 0..4 {
                sum += coverage(&Puzzle::generate_with(3, salt, GridSize::Fixed(n)));
            }
            let avg = sum / 4.0;
            assert!(avg >= 0.8, "{n}x{n} is only {:.0}% covered", avg * 100.0);
        }
    }

    #[test]
    fn levels_get_bigger() {
        assert_eq!(LevelParams::for_level(1).size, MIN_FACE_SIZE);
        assert!(LevelParams::for_level(9).size > LevelParams::for_level(1).size);
        assert_eq!(LevelParams::for_level(999).size, MAX_FACE_SIZE);
        // Sizes never shrink as levels rise.
        for level in 1..60 {
            assert!(LevelParams::for_level(level + 1).size >= LevelParams::for_level(level).size);
        }
    }
}
