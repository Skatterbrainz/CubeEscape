//! Turns game state into a flat list of coloured convex polygons in screen
//! space, and maps mouse positions back onto cube faces.
//!
//! The renderer is deliberately tiny: a rotation, a perspective divide, and
//! back-face culling. Because only front-facing faces of a convex cube are
//! drawn, there is no depth sorting to do. Keeping the output as plain
//! polygons means the window front end and the headless snapshot tool draw
//! exactly the same picture.

use crate::game::{Game, FLASH_DURATION, HINT_DURATION, SHAKE_DURATION, SLIDE_DURATION};
use crate::puzzle::{Dir, LineId, FACE_COUNT};
use glam::{Quat, Vec2, Vec3};

/// Window clear colour.
pub const BACKGROUND: [f32; 4] = [0.055, 0.07, 0.11, 1.0];

const PALETTE: [[f32; 3]; 10] = [
    [1.00, 0.42, 0.42], // coral
    [1.00, 0.72, 0.30], // amber
    [0.96, 0.88, 0.30], // lemon
    [0.55, 0.88, 0.31], // lime
    [0.24, 0.86, 0.59], // mint
    [0.30, 0.82, 0.88], // cyan
    [0.36, 0.61, 1.00], // sky
    [0.61, 0.48, 1.00], // violet
    [1.00, 0.48, 0.84], // pink
    [1.00, 0.54, 0.24], // orange
];

/// Distance from the camera to the cube centre, in cube half-widths.
const CAM_DIST: f32 = 4.4;

/// A convex polygon in screen pixels.
#[derive(Clone, Debug)]
pub struct Poly {
    pub pts: Vec<[f32; 2]>,
    /// Non-premultiplied RGBA, each 0..1.
    pub color: [f32; 4],
}

/// Outward normal and the in-face "right" and "up" axes of a face. For every
/// face, `right x up == normal`, so each face reads correctly from outside.
pub fn face_basis(face: usize) -> (Vec3, Vec3, Vec3) {
    match face {
        0 => (Vec3::Z, Vec3::X, Vec3::Y),
        1 => (-Vec3::Z, -Vec3::X, Vec3::Y),
        2 => (Vec3::X, -Vec3::Z, Vec3::Y),
        3 => (-Vec3::X, Vec3::Z, Vec3::Y),
        4 => (Vec3::Y, Vec3::X, -Vec3::Z),
        _ => (-Vec3::Y, Vec3::X, Vec3::Z),
    }
}

/// Camera: how the cube is turned, zoomed and where it sits on screen.
#[derive(Clone, Debug)]
pub struct View {
    pub rot: Quat,
    pub zoom: f32,
    pub width: f32,
    pub height: f32,
}

impl View {
    pub fn new(width: f32, height: f32) -> View {
        View {
            rot: Self::default_rot(),
            zoom: 1.0,
            width,
            height,
        }
    }

    /// A pleasant starting angle that shows the front, right and top faces.
    pub fn default_rot() -> Quat {
        Quat::from_rotation_x(0.5) * Quat::from_rotation_y(-0.6)
    }

    pub fn reset(&mut self) {
        self.rot = Self::default_rot();
        self.zoom = 1.0;
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    /// Turn the cube about the screen's vertical axis by `yaw` and horizontal
    /// axis by `pitch` (radians). Positive yaw drags the front towards the
    /// right; positive pitch drags it down.
    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.rot =
            (Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch) * self.rot).normalize();
    }

    /// Pixels per cube half-width at the centre depth.
    pub fn scale(&self) -> f32 {
        self.width.min(self.height) * 0.21 * self.zoom
    }

    fn center(&self) -> Vec2 {
        Vec2::new(self.width * 0.5, self.height * 0.5)
    }

    /// Project a point in cube space to the screen.
    pub fn project(&self, p: Vec3) -> Vec2 {
        let r = self.rot * p;
        let persp = CAM_DIST / (CAM_DIST - r.z);
        self.center() + Vec2::new(r.x, -r.y) * self.scale() * persp
    }

    /// Whether a face is turned towards the camera.
    pub fn face_visible(&self, face: usize) -> bool {
        let (n, _, _) = face_basis(face);
        let camera = self.rot.inverse() * Vec3::new(0.0, 0.0, CAM_DIST);
        n.dot(camera) > 1.0
    }

    /// Where a screen position lands on a face's plane, in that face's local
    /// coordinates (the face spans -1..1 on both axes).
    pub fn unproject_to_face(&self, screen: Vec2, face: usize) -> Option<Vec2> {
        let s = self.scale();
        let off = screen - self.center();
        let a = off.x / (s * CAM_DIST);
        let b = -off.y / (s * CAM_DIST);
        let inv = self.rot.inverse();
        // The ray in cube space is  p(e) = e * ray_dir + ray_origin.
        let ray_dir = inv * Vec3::new(a, b, -1.0);
        let ray_origin = inv * Vec3::new(0.0, 0.0, CAM_DIST);
        let (n, u, v) = face_basis(face);
        let denom = n.dot(ray_dir);
        if denom.abs() < 1e-6 {
            return None;
        }
        let e = (1.0 - n.dot(ray_origin)) / denom;
        if e <= 0.0 {
            return None;
        }
        let p = ray_dir * e + ray_origin;
        Some(Vec2::new(p.dot(u), p.dot(v)))
    }
}

fn dir_vec(dir: Dir) -> Vec2 {
    let (dx, dy) = dir.delta();
    Vec2::new(dx as f32, dy as f32)
}

/// Centre of grid cell (x, y) in face-local coordinates.
fn cell_center(n: i32, x: i32, y: i32) -> Vec2 {
    let cell = 2.0 / n as f32;
    Vec2::new(
        -1.0 + (x as f32 + 0.5) * cell,
        -1.0 + (y as f32 + 0.5) * cell,
    )
}

fn to_screen(view: &View, face: usize, lift: f32, p: Vec2) -> [f32; 2] {
    let (n, u, v) = face_basis(face);
    let s = view.project(n * (1.0 + lift) + u * p.x + v * p.y);
    [s.x, s.y]
}

fn push_poly(
    out: &mut Vec<Poly>,
    view: &View,
    face: usize,
    lift: f32,
    pts: &[Vec2],
    color: [f32; 4],
) {
    out.push(Poly {
        pts: pts
            .iter()
            .map(|&p| to_screen(view, face, lift, p))
            .collect(),
        color,
    });
}

fn octagon(center: Vec2, r: f32) -> [Vec2; 8] {
    let mut pts = [Vec2::ZERO; 8];
    for (k, p) in pts.iter_mut().enumerate() {
        let a = (k as f32 + 0.5) * std::f32::consts::TAU / 8.0;
        *p = center + Vec2::new(a.cos(), a.sin()) * r;
    }
    pts
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Draw one arrow: a thick polyline with rounded joints and a triangular
/// head on its final point, pointing along `dir`. `pad` fattens the whole
/// shape (used for the dark outline drawn beneath the coloured pass).
#[allow(clippy::too_many_arguments)]
fn arrow_polys(
    out: &mut Vec<Poly>,
    view: &View,
    face: usize,
    lift: f32,
    pts: &[Vec2],
    dir: Vec2,
    cell: f32,
    pad: f32,
    color: [f32; 4],
) {
    if pts.len() < 2 {
        return;
    }
    let hw = cell * 0.13 + pad;
    for w in pts.windows(2) {
        let d = w[1] - w[0];
        if d.length_squared() < 1e-10 {
            continue;
        }
        let d = d.normalize();
        let p = Vec2::new(-d.y, d.x) * hw;
        push_poly(
            out,
            view,
            face,
            lift,
            &[w[0] - p, w[1] - p, w[1] + p, w[0] + p],
            color,
        );
    }
    for &pt in &pts[..pts.len() - 1] {
        push_poly(out, view, face, lift, &octagon(pt, hw), color);
    }
    let last = *pts.last().unwrap();
    let perp = Vec2::new(-dir.y, dir.x);
    let tip = last + dir * (cell * 0.42 + pad * 1.6);
    let base = last - dir * (pad * 0.5);
    let half = cell * 0.27 + pad;
    push_poly(
        out,
        view,
        face,
        lift,
        &[tip, base + perp * half, base - perp * half],
        color,
    );
}

fn with_alpha(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

const OUTLINE: [f32; 3] = [0.03, 0.04, 0.07];

#[allow(clippy::too_many_arguments)]
fn draw_arrow(
    out: &mut Vec<Poly>,
    view: &View,
    face: usize,
    lift: f32,
    pts: &[Vec2],
    dir: Vec2,
    cell: f32,
    color: [f32; 3],
    alpha: f32,
) {
    arrow_polys(
        out,
        view,
        face,
        lift,
        pts,
        dir,
        cell,
        cell * 0.05,
        with_alpha(OUTLINE, 0.85 * alpha),
    );
    arrow_polys(
        out,
        view,
        face,
        lift,
        pts,
        dir,
        cell,
        0.0,
        with_alpha(color, alpha),
    );
}

fn face_base(out: &mut Vec<Poly>, view: &View, face: usize, game: &Game) {
    let (n, _, _) = face_basis(face);
    let light = Vec3::new(-0.35, 0.6, 0.72).normalize();
    let shade = 0.72 + 0.38 * (view.rot * n).dot(light).max(0.0);
    let quad = |s: f32| {
        [
            Vec2::new(-s, -s),
            Vec2::new(s, -s),
            Vec2::new(s, s),
            Vec2::new(-s, s),
        ]
    };
    push_poly(
        out,
        view,
        face,
        0.0,
        &quad(1.0),
        [0.34 * shade, 0.42 * shade, 0.56 * shade, 1.0],
    );
    push_poly(
        out,
        view,
        face,
        0.0,
        &quad(0.955),
        [0.14 * shade, 0.19 * shade, 0.29 * shade, 1.0],
    );
    // A faint dot in every cell, so empty spots still read as part of the grid.
    let size = game.puzzle.face(face).size();
    let cell = 2.0 / size as f32;
    for y in 0..size {
        for x in 0..size {
            push_poly(
                out,
                view,
                face,
                0.0,
                &octagon(cell_center(size, x, y), cell * 0.05),
                [1.0, 1.0, 1.0, 0.10],
            );
        }
    }
}

fn static_color(game: &Game, id: LineId, palette: usize, hover: Option<LineId>) -> [f32; 3] {
    let mut c = PALETTE[palette % PALETTE.len()];
    if hover == Some(id) {
        c = mix3(c, [1.0, 1.0, 1.0], 0.35);
    }
    if let Some(h) = game.hint {
        if h.id == id {
            let k = 0.5 + 0.5 * (game.time * 9.0).sin();
            let fade = 1.0 - (h.t / HINT_DURATION).powi(4);
            c = mix3(c, [1.0, 1.0, 1.0], (0.25 + 0.6 * k) * fade);
        }
    }
    if let Some(f) = game.flash {
        if f.id == id {
            let fade = 1.0 - f.t / FLASH_DURATION;
            let k = (0.5 + 0.5 * (f.t * 26.0).sin()) * fade;
            c = mix3(c, [1.0, 0.15, 0.15], 0.4 + 0.6 * k);
        }
    }
    c
}

fn shake_offset(game: &Game, id: LineId, cell: f32) -> f32 {
    game.shakes
        .iter()
        .find(|s| s.id == id)
        .map(|s| (s.t * 48.0).sin() * (1.0 - s.t / SHAKE_DURATION) * cell * 0.14)
        .unwrap_or(0.0)
}

/// The part of a polyline between arclengths `a` and `b`.
fn subpath(points: &[Vec2], a: f32, b: f32) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::new();
    let mut acc = 0.0;
    for w in points.windows(2) {
        let seg = (w[1] - w[0]).length();
        if seg < 1e-6 {
            continue;
        }
        let (s0, s1) = (acc, acc + seg);
        if s1 > a && s0 < b {
            let ta = ((a - s0) / seg).clamp(0.0, 1.0);
            let tb = ((b - s0) / seg).clamp(0.0, 1.0);
            if out.is_empty() {
                out.push(w[0].lerp(w[1], ta));
            }
            out.push(w[0].lerp(w[1], tb));
        }
        acc = s1;
    }
    out
}

fn path_length(points: &[Vec2]) -> f32 {
    points.windows(2).map(|w| (w[1] - w[0]).length()).sum()
}

fn draw_sliding(out: &mut Vec<Poly>, view: &View, game: &Game, anim: &crate::game::SlideAnim) {
    let id = anim.id;
    let face = game.puzzle.face(id.face);
    let n = face.size();
    let cell = 2.0 / n as f32;
    let line = &face.lines()[id.index];
    let body: Vec<Vec2> = line
        .cells
        .iter()
        .map(|&(x, y)| cell_center(n, x, y))
        .collect();
    let dir = dir_vec(line.dir());
    let len = path_length(&body);

    // Cells between the arrowhead and the edge of the face.
    let (mut x, mut y) = line.head();
    let (dx, dy) = line.dir().delta();
    let mut steps = 0;
    while face.contains(x + dx, y + dy) {
        x += dx;
        y += dy;
        steps += 1;
    }
    // Slide far enough that the whole line is clear of the face and then some.
    let travel = len + (steps as f32 + 1.0) * cell + 0.9;

    let mut full = body.clone();
    full.push(*body.last().unwrap() + dir * (travel + 1.0));

    let u = (anim.t / SLIDE_DURATION).clamp(0.0, 1.0);
    let eased = 0.35 * u + 0.65 * u * u;
    let s = travel * eased;
    let alpha = 1.0 - smoothstep((u - 0.45) / 0.55);
    let lift = 0.01 + 0.06 * u;

    let visible = subpath(&full, s, s + len);
    if visible.len() < 2 || alpha <= 0.01 {
        return;
    }
    let color = PALETTE[line.color % PALETTE.len()];
    draw_arrow(out, view, id.face, lift, &visible, dir, cell, color, alpha);
}

/// Build the polygons for one frame, back to front.
pub fn build_scene(game: &Game, view: &View, hover: Option<LineId>) -> Vec<Poly> {
    let mut out = Vec::new();
    let visible: Vec<usize> = (0..FACE_COUNT).filter(|&f| view.face_visible(f)).collect();

    for &f in &visible {
        face_base(&mut out, view, f, game);
    }

    for &f in &visible {
        let face = game.puzzle.face(f);
        let n = face.size();
        let cell = 2.0 / n as f32;
        for (i, line) in face.lines().iter().enumerate() {
            if !face.is_alive(i) {
                continue;
            }
            let id = LineId { face: f, index: i };
            let dir = dir_vec(line.dir());
            let shift = dir * shake_offset(game, id, cell);
            let pts: Vec<Vec2> = line
                .cells
                .iter()
                .map(|&(x, y)| cell_center(n, x, y) + shift)
                .collect();
            let color = static_color(game, id, line.color, hover);
            draw_arrow(&mut out, view, f, 0.012, &pts, dir, cell, color, 1.0);
        }
    }

    for anim in &game.anims {
        if view.face_visible(anim.id.face) {
            draw_sliding(&mut out, view, game, anim);
        }
    }
    out
}

fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-12 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// The line under a screen position, if any. Only front-facing faces count,
/// and only lines that are still on the cube.
pub fn pick(game: &Game, view: &View, mouse: [f32; 2]) -> Option<LineId> {
    let m = Vec2::new(mouse[0], mouse[1]);
    for f in 0..FACE_COUNT {
        if !view.face_visible(f) {
            continue;
        }
        let Some(local) = view.unproject_to_face(m, f) else {
            continue;
        };
        if local.x.abs() > 1.0 || local.y.abs() > 1.0 {
            continue;
        }
        // At most one front face can contain the pointer, so we are done here
        // whether or not a line is close enough.
        let face = game.puzzle.face(f);
        let n = face.size();
        let cell = 2.0 / n as f32;
        let mut best: Option<(f32, usize)> = None;
        for (i, line) in face.lines().iter().enumerate() {
            if !face.is_alive(i) {
                continue;
            }
            let mut pts: Vec<Vec2> = line
                .cells
                .iter()
                .map(|&(x, y)| cell_center(n, x, y))
                .collect();
            let tip = *pts.last().unwrap() + dir_vec(line.dir()) * cell * 0.42;
            pts.push(tip);
            let d = pts
                .windows(2)
                .map(|w| dist_to_segment(local, w[0], w[1]))
                .fold(f32::MAX, f32::min);
            if d <= cell * 0.55 && best.is_none_or(|(bd, _)| d < bd) {
                best = Some((d, i));
            }
        }
        return best.map(|(_, index)| LineId { face: f, index });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_axes_are_right_handed() {
        for f in 0..FACE_COUNT {
            let (n, u, v) = face_basis(f);
            assert!((u.cross(v) - n).length() < 1e-6, "face {f} is mirrored");
        }
    }

    #[test]
    fn default_view_shows_front_right_and_top() {
        let view = View::new(1000.0, 800.0);
        let visible: Vec<usize> = (0..FACE_COUNT).filter(|&f| view.face_visible(f)).collect();
        assert_eq!(visible, vec![0, 2, 4]);
    }

    #[test]
    fn every_orientation_shows_between_one_and_three_faces() {
        let mut view = View::new(1000.0, 800.0);
        for i in 0..200 {
            view.rotate(0.37, 0.23 + (i % 7) as f32 * 0.1);
            let n = (0..FACE_COUNT).filter(|&f| view.face_visible(f)).count();
            assert!((1..=3).contains(&n), "{n} faces visible");
        }
    }

    #[test]
    fn unproject_inverts_project() {
        let mut view = View::new(1000.0, 800.0);
        view.rotate(0.4, -0.3);
        for f in 0..FACE_COUNT {
            if !view.face_visible(f) {
                continue;
            }
            let (n, u, v) = face_basis(f);
            for &(x, y) in &[(0.0, 0.0), (0.5, -0.25), (-0.8, 0.7)] {
                let screen = view.project(n + u * x + v * y);
                let back = view.unproject_to_face(screen, f).unwrap();
                assert!(
                    (back - Vec2::new(x, y)).length() < 1e-3,
                    "face {f}: {back:?}"
                );
            }
        }
    }

    #[test]
    fn clicking_a_line_picks_that_line_from_any_angle() {
        let mut game = Game::new(5);
        game.update(0.0);
        let mut view = View::new(1000.0, 800.0);
        let mut checked = 0;
        for step in 0..12 {
            view.rotate(0.55, 0.31);
            for f in 0..FACE_COUNT {
                if !view.face_visible(f) {
                    continue;
                }
                let face = game.puzzle.face(f);
                let n = face.size();
                let (nrm, u, v) = face_basis(f);
                for (i, line) in face.lines().iter().enumerate() {
                    let (x, y) = line.cells[0];
                    let c = cell_center(n, x, y);
                    let s = view.project(nrm + u * c.x + v * c.y);
                    let got = pick(&game, &view, [s.x, s.y]);
                    assert_eq!(
                        got,
                        Some(LineId { face: f, index: i }),
                        "step {step}, face {f}, line {i}"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 50, "only {checked} lines were checked");
    }

    #[test]
    fn clicking_empty_space_picks_nothing() {
        let game = Game::new(5);
        let view = View::new(1000.0, 800.0);
        assert_eq!(pick(&game, &view, [5.0, 5.0]), None);
    }

    #[test]
    fn removed_lines_cannot_be_picked() {
        let mut game = Game::new(4);
        let view = View::new(1000.0, 800.0);
        let id = game
            .puzzle
            .removable()
            .into_iter()
            .find(|id| view.face_visible(id.face))
            .expect("a removable line on a visible face");
        let face = game.puzzle.face(id.face);
        let n = face.size();
        let (nrm, u, v) = face_basis(id.face);
        let (x, y) = face.lines()[id.index].cells[0];
        let c = cell_center(n, x, y);
        let s = view.project(nrm + u * c.x + v * c.y);
        assert_eq!(pick(&game, &view, [s.x, s.y]), Some(id));
        game.click(id);
        assert_eq!(pick(&game, &view, [s.x, s.y]), None);
    }

    #[test]
    fn scene_output_is_finite_and_non_empty() {
        let mut game = Game::new(6);
        let id = game.puzzle.removable()[0];
        game.click(id);
        game.update(0.25);
        let view = View::new(1000.0, 800.0);
        let polys = build_scene(&game, &view, None);
        assert!(polys.len() > 100);
        for p in &polys {
            assert!(p.pts.len() >= 3);
            for pt in &p.pts {
                assert!(pt[0].is_finite() && pt[1].is_finite());
            }
        }
    }

    #[test]
    fn subpath_clips_to_the_requested_window() {
        let pts = [
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
        ];
        let sub = subpath(&pts, 1.0, 3.0);
        assert_eq!(sub.first().copied(), Some(Vec2::new(1.0, 0.0)));
        assert_eq!(sub.last().copied(), Some(Vec2::new(2.0, 1.0)));
        assert!((path_length(&sub) - 2.0).abs() < 1e-5);
    }
}
