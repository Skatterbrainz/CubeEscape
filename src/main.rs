//! Window, input and text for Cube Escape. All game rules live in the
//! library; this file only turns mouse and keyboard events into game calls and
//! draws the polygons the scene produces.

use cube_escape::game::{ClickResult, Game};
use cube_escape::scene::{build_scene, pick, Poly, View, BACKGROUND};
use cube_escape::GAME_TITLE;
use macroquad::prelude::*;

/// Pixels the pointer must travel before a press counts as a drag.
const DRAG_THRESHOLD: f32 = 6.0;
/// Radians of rotation per pixel of drag.
const DRAG_SENSITIVITY: f32 = 0.0065;
/// Radians per second for keyboard rotation.
const KEY_TURN_SPEED: f32 = 1.8;

fn window_conf() -> Conf {
    Conf {
        window_title: GAME_TITLE.to_owned(),
        window_width: 1000,
        window_height: 760,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

fn draw_poly(p: &Poly) {
    if p.pts.len() < 3 {
        return;
    }
    let color = Color::new(p.color[0], p.color[1], p.color[2], p.color[3]);
    let v0 = vec2(p.pts[0][0], p.pts[0][1]);
    for i in 1..p.pts.len() - 1 {
        draw_triangle(
            v0,
            vec2(p.pts[i][0], p.pts[i][1]),
            vec2(p.pts[i + 1][0], p.pts[i + 1][1]),
            color,
        );
    }
}

/// The two buttons beside the grid-size readout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GridButton {
    Smaller,
    Bigger,
}

const BUTTON_SIZE: Vec2 = vec2(30.0, 28.0);
const BUTTON_Y: f32 = 88.0;

fn button_rect(button: GridButton) -> Rect {
    let x = match button {
        GridButton::Smaller => 78.0,
        GridButton::Bigger => 114.0,
    };
    Rect::new(x, BUTTON_Y, BUTTON_SIZE.x, BUTTON_SIZE.y)
}

fn button_at(p: Vec2) -> Option<GridButton> {
    [GridButton::Smaller, GridButton::Bigger]
        .into_iter()
        .find(|&b| button_rect(b).contains(p))
}

fn change_grid(game: &mut Game, button: GridButton) {
    let grid = game.grid;
    match button {
        GridButton::Smaller if grid.can_shrink() => game.set_grid(grid.smaller()),
        GridButton::Bigger if grid.can_grow() => game.set_grid(grid.bigger()),
        _ => {}
    }
}

fn draw_grid_button(button: GridButton, enabled: bool, hovered: bool) {
    let r = button_rect(button);
    let fill = match (enabled, hovered) {
        (false, _) => Color::new(0.12, 0.15, 0.22, 1.0),
        (true, false) => Color::new(0.20, 0.26, 0.38, 1.0),
        (true, true) => Color::new(0.30, 0.38, 0.54, 1.0),
    };
    draw_rectangle(r.x, r.y, r.w, r.h, fill);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color::new(0.34, 0.42, 0.56, 1.0));
    let ink = if enabled {
        Color::new(0.90, 0.94, 1.0, 1.0)
    } else {
        Color::new(0.35, 0.40, 0.52, 1.0)
    };
    // Drawn as shapes so it does not depend on the font having a minus sign.
    let (cx, cy) = (r.x + r.w * 0.5, r.y + r.h * 0.5);
    draw_line(cx - 6.0, cy, cx + 6.0, cy, 3.0, ink);
    if button == GridButton::Bigger {
        draw_line(cx, cy - 6.0, cx, cy + 6.0, 3.0, ink);
    }
}

fn draw_centered(text: &str, y: f32, size: f32, color: Color) {
    let dims = measure_text(text, None, size as u16, 1.0);
    draw_text(text, (screen_width() - dims.width) * 0.5, y, size, color);
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new(1);
    let mut view = View::new(screen_width(), screen_height());

    let mut press_pos: Option<Vec2> = None;
    let mut dragging = false;
    let mut last_pos = Vec2::ZERO;
    // Rotation speed carried on after a drag is released, in rad/s.
    let mut spin = Vec2::ZERO;
    // A grid button the pointer went down on, finished when it comes up.
    let mut ui_press: Option<GridButton> = None;

    loop {
        let dt = get_frame_time().min(0.05);
        view.set_size(screen_width(), screen_height());
        let (mx, my) = mouse_position();
        let mouse = vec2(mx, my);
        let level_done = game.is_solved() && !game.is_animating();

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        // Keyboard.
        if is_key_pressed(KeyCode::U) || is_key_pressed(KeyCode::Backspace) {
            game.undo();
        }
        if is_key_pressed(KeyCode::R) {
            game.restart();
        }
        if is_key_pressed(KeyCode::H) {
            game.show_hint();
        }
        if is_key_pressed(KeyCode::LeftBracket) {
            change_grid(&mut game, GridButton::Smaller);
        }
        if is_key_pressed(KeyCode::RightBracket) {
            change_grid(&mut game, GridButton::Bigger);
        }
        if is_key_pressed(KeyCode::V) {
            view.reset();
            spin = Vec2::ZERO;
        }
        if level_done
            && (is_key_pressed(KeyCode::Enter)
                || is_key_pressed(KeyCode::Space)
                || is_key_pressed(KeyCode::N))
        {
            game.next_level();
        }
        let turn = KEY_TURN_SPEED * dt;
        if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            view.rotate(-turn, 0.0);
        }
        if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            view.rotate(turn, 0.0);
        }
        if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            view.rotate(0.0, -turn);
        }
        if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            view.rotate(0.0, turn);
        }

        // Mouse wheel zoom.
        let (_, wheel) = mouse_wheel();
        if wheel != 0.0 {
            view.zoom = (view.zoom * (1.0 + wheel.signum() * 0.08)).clamp(0.5, 3.0);
        }

        // Mouse: a short press is a click, a longer move is a drag-to-rotate.
        if is_mouse_button_pressed(MouseButton::Left) {
            if let Some(button) = button_at(mouse) {
                // Over a button: this press belongs to the button, not the cube.
                ui_press = Some(button);
            } else {
                press_pos = Some(mouse);
                dragging = false;
                last_pos = mouse;
                spin = Vec2::ZERO;
            }
        }
        if is_mouse_button_down(MouseButton::Left) {
            if let Some(start) = press_pos {
                if !dragging && (mouse - start).length() > DRAG_THRESHOLD {
                    dragging = true;
                }
                if dragging {
                    let d = (mouse - last_pos) * DRAG_SENSITIVITY;
                    view.rotate(d.x, d.y);
                    let rate = d / dt.max(1e-3);
                    spin = vec2(rate.x.clamp(-5.0, 5.0), rate.y.clamp(-5.0, 5.0));
                }
            }
            last_pos = mouse;
        } else if spin.length() > 0.02 {
            view.rotate(spin.x * dt, spin.y * dt);
            spin *= (-4.5 * dt).exp();
        } else {
            spin = Vec2::ZERO;
        }
        if is_mouse_button_released(MouseButton::Left) {
            if let Some(button) = ui_press.take() {
                if button_at(mouse) == Some(button) {
                    change_grid(&mut game, button);
                }
            } else if press_pos.is_some() && !dragging {
                if level_done {
                    game.next_level();
                } else if let Some(id) = pick(&game, &view, [mx, my]) {
                    let _: ClickResult = game.click(id);
                }
            }
            press_pos = None;
            dragging = false;
        }

        game.update(dt);

        // Draw.
        clear_background(Color::new(
            BACKGROUND[0],
            BACKGROUND[1],
            BACKGROUND[2],
            BACKGROUND[3],
        ));
        let hover = if press_pos.is_none() && button_at(mouse).is_none() {
            pick(&game, &view, [mx, my])
        } else {
            None
        };
        for poly in build_scene(&game, &view, hover) {
            draw_poly(&poly);
        }

        // HUD.
        let soft = Color::new(0.80, 0.85, 0.95, 1.0);
        let dim = Color::new(0.55, 0.60, 0.72, 1.0);
        draw_text(GAME_TITLE.to_uppercase(), 24.0, 44.0, 38.0, soft);
        draw_text(
            format!(
                "Level {}    Arrows left: {}    Mistakes: {}",
                game.level,
                game.puzzle.remaining(),
                game.mistakes
            ),
            24.0,
            74.0,
            24.0,
            dim,
        );
        draw_text("Grid", 24.0, BUTTON_Y + 21.0, 24.0, dim);
        let over = button_at(mouse);
        draw_grid_button(
            GridButton::Smaller,
            game.grid.can_shrink(),
            over == Some(GridButton::Smaller),
        );
        draw_grid_button(
            GridButton::Bigger,
            game.grid.can_grow(),
            over == Some(GridButton::Bigger),
        );
        draw_text(
            game.grid.label(game.puzzle.size()),
            156.0,
            BUTTON_Y + 21.0,
            24.0,
            soft,
        );
        draw_centered(
            "Drag: rotate    Click an arrow: slide it off    Wheel: zoom",
            screen_height() - 44.0,
            20.0,
            dim,
        );
        draw_centered(
            "U: undo    H: hint    R: restart    V: reset view    [ ]: grid size    Esc: quit",
            screen_height() - 20.0,
            20.0,
            dim,
        );

        if level_done {
            draw_rectangle(
                0.0,
                0.0,
                screen_width(),
                screen_height(),
                Color::new(0.03, 0.04, 0.08, 0.72),
            );
            draw_centered(
                &format!("Level {} cleared!", game.level),
                screen_height() * 0.45,
                64.0,
                Color::new(0.55, 0.88, 0.31, 1.0),
            );
            draw_centered(
                &format!("{} moves, {} mistakes", game.moves, game.mistakes),
                screen_height() * 0.45 + 40.0,
                26.0,
                soft,
            );
            draw_centered(
                "Click or press Enter for the next level",
                screen_height() * 0.45 + 84.0,
                26.0,
                soft,
            );
        }

        next_frame().await;
    }
}
