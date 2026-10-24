//! The window icon, decoded from the PNG files in `assets/icons/`.
//!
//! This module only produces raw pixels. `main.rs` hands them to the window,
//! so everything else in the library stays free of window code.
//!
//! The files are made from `assets/cube_escape_icon.png` by
//! `assets/make_icons.sh`.

use image::ImageFormat;

const ICON_16: &[u8] = include_bytes!("../assets/icons/cube_escape_16.png");
const ICON_32: &[u8] = include_bytes!("../assets/icons/cube_escape_32.png");
const ICON_64: &[u8] = include_bytes!("../assets/icons/cube_escape_64.png");

/// RGBA pixels, row by row, at the three sizes a window can show.
pub struct IconPixels {
    pub small: [u8; 16 * 16 * 4],
    pub medium: [u8; 32 * 32 * 4],
    pub big: [u8; 64 * 64 * 4],
}

/// Decode a square PNG into exactly `N` bytes of RGBA.
fn decode<const N: usize>(png: &[u8], side: u32) -> [u8; N] {
    let img = image::load_from_memory_with_format(png, ImageFormat::Png)
        .expect("embedded icon is a valid PNG")
        .to_rgba8();
    assert_eq!(
        (img.width(), img.height()),
        (side, side),
        "embedded icon has the wrong size"
    );
    let mut out = [0u8; N];
    out.copy_from_slice(img.as_raw());
    out
}

/// The game's window icon at 16, 32 and 64 pixels.
pub fn window_icon() -> IconPixels {
    IconPixels {
        small: decode(ICON_16, 16),
        medium: decode(ICON_32, 32),
        big: decode(ICON_64, 64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(px: &[u8], side: usize, x: usize, y: usize) -> u8 {
        px[(y * side + x) * 4 + 3]
    }

    #[test]
    fn all_three_sizes_decode_to_the_right_length() {
        let icon = window_icon();
        assert_eq!(icon.small.len(), 16 * 16 * 4);
        assert_eq!(icon.medium.len(), 32 * 32 * 4);
        assert_eq!(icon.big.len(), 64 * 64 * 4);
    }

    #[test]
    fn corners_are_transparent_and_the_middle_is_solid() {
        let icon = window_icon();
        for (px, side) in [
            (&icon.small[..], 16),
            (&icon.medium[..], 32),
            (&icon.big[..], 64),
        ] {
            for (x, y) in [(0, 0), (side - 1, 0), (0, side - 1), (side - 1, side - 1)] {
                assert_eq!(alpha(px, side, x, y), 0, "{side}px corner ({x},{y})");
            }
            assert_eq!(alpha(px, side, side / 2, side / 2), 255, "{side}px centre");
        }
    }

    #[test]
    fn the_icon_is_neither_blank_nor_fully_filled() {
        let icon = window_icon();
        let solid = icon.big.chunks(4).filter(|p| p[3] == 255).count();
        let total = 64 * 64;
        assert!(
            solid * 100 / total >= 35 && solid * 100 / total <= 75,
            "{solid} of {total} pixels are solid"
        );
    }

    #[test]
    fn the_three_arrows_are_present() {
        // Blue, yellow and red arrows in the artwork: this fails if a wrong
        // or washed-out file is embedded.
        let icon = window_icon();
        let px: Vec<(u8, u8, u8)> = icon
            .big
            .chunks(4)
            .filter(|p| p[3] == 255)
            .map(|p| (p[0], p[1], p[2]))
            .collect();
        assert!(
            px.iter().any(|&(r, g, b)| b > 150 && r < 100 && g > 90),
            "no blue"
        );
        assert!(
            px.iter().any(|&(r, g, b)| r > 150 && g < 80 && b < 80),
            "no red"
        );
        assert!(
            px.iter().any(|&(r, g, b)| r > 140 && g > 120 && b < 80),
            "no yellow"
        );
    }
}
