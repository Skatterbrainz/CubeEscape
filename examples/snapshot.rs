//! Renders a few frames to PNG files without opening a window, so the look of
//! the game can be checked on a machine with no display.
//!
//!     cargo run --example snapshot -- <output-dir>

use cube_escape::game::Game;
use cube_escape::puzzle::LineId;
use cube_escape::scene::{build_scene, pick, Poly, View, BACKGROUND};
use tiny_skia::{Color, FillRule, Paint, PathBuilder, Pixmap, Transform};

fn render(polys: &[Poly], w: u32, h: u32, path: &std::path::Path) {
    let mut pm = Pixmap::new(w, h).expect("pixmap");
    pm.fill(Color::from_rgba(BACKGROUND[0], BACKGROUND[1], BACKGROUND[2], 1.0).unwrap());
    for p in polys {
        let mut pb = PathBuilder::new();
        pb.move_to(p.pts[0][0], p.pts[0][1]);
        for pt in &p.pts[1..] {
            pb.line_to(pt[0], pt[1]);
        }
        pb.close();
        let Some(shape) = pb.finish() else { continue };
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba(p.color[0], p.color[1], p.color[2], p.color[3]).unwrap());
        paint.anti_alias = true;
        pm.fill_path(
            &shape,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    pm.save_png(path).expect("save png");
    println!("wrote {}", path.display());
}

fn main() {
    let dir = std::path::PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "snapshots".into()),
    );
    std::fs::create_dir_all(&dir).unwrap();
    let (w, h) = (900u32, 700u32);

    // 1. A fresh level 1 at the default angle.
    let game = Game::new(1);
    let view = View::new(w as f32, h as f32);
    render(
        &build_scene(&game, &view, None),
        w,
        h,
        &dir.join("1_level1_default.png"),
    );

    // 2. A bigger level, turned to a different corner of the cube.
    let game = Game::new(8);
    let mut view = View::new(w as f32, h as f32);
    view.rotate(2.2, -0.5);
    render(
        &build_scene(&game, &view, None),
        w,
        h,
        &dir.join("2_level8_rotated.png"),
    );

    // 3. Mid-slide: remove a visible free line and freeze the animation.
    let mut game = Game::new(5);
    let view = View::new(w as f32, h as f32);
    let free: Vec<LineId> = game
        .puzzle
        .removable()
        .into_iter()
        .filter(|id| view.face_visible(id.face))
        .collect();
    for id in free.iter().take(3) {
        game.click(*id);
    }
    game.update(0.22);
    render(
        &build_scene(&game, &view, None),
        w,
        h,
        &dir.join("3_sliding.png"),
    );

    // 4. A blocked click: shake plus the blocker flashing red.
    let mut game = Game::new(5);
    let view = View::new(w as f32, h as f32);
    'outer: for f in 0..6 {
        if !view.face_visible(f) {
            continue;
        }
        for i in 0..game.puzzle.face(f).lines().len() {
            let id = LineId { face: f, index: i };
            if game.puzzle.blocker(id).is_some() {
                game.click(id);
                game.update(0.08);
                break 'outer;
            }
        }
    }
    let hover = pick(&game, &view, [450.0, 350.0]);
    render(
        &build_scene(&game, &view, hover),
        w,
        h,
        &dir.join("4_blocked.png"),
    );
}
