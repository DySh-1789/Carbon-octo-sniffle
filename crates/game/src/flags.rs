use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sim::Nation;

/// Native pixel dimensions for procedural nation flag UI textures (`16:10` aspect ratio).
pub const FLAG_WIDTH: u32 = 64;
pub const FLAG_HEIGHT: u32 = 40;

/// GPU texture handles for crisp procedural WWII nation flags shown in the Hangar & Tech Tree.
#[derive(Resource, Clone)]
pub struct FlagTextures {
    pub usa: Handle<Image>,
    pub germany: Handle<Image>,
    pub ussr: Handle<Image>,
}

impl FlagTextures {
    /// Generates all three `64x40` RGBA nation flag textures and registers them in `Assets<Image>`.
    pub fn from_images(images: &mut Assets<Image>) -> Self {
        let usa = images.add(rgba_to_flag_image(build_usa_flag_rgba()));
        let germany = images.add(rgba_to_flag_image(build_germany_flag_rgba()));
        let ussr = images.add(rgba_to_flag_image(build_ussr_flag_rgba()));
        Self { usa, germany, ussr }
    }

    /// Returns a cloned `Handle<Image>` for the requested nation's flag texture.
    pub fn handle(&self, nation: Nation) -> Handle<Image> {
        match nation {
            Nation::Usa => self.usa.clone(),
            Nation::Germany => self.germany.clone(),
            Nation::Ussr => self.ussr.clone(),
        }
    }
}

/// Generates all three `64x40` RGBA nation flag textures on Startup if not already created.
pub fn setup_flag_textures(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    existing: Option<Res<FlagTextures>>,
) {
    if existing.is_none() {
        commands.insert_resource(FlagTextures::from_images(&mut images));
    }
}

fn rgba_to_flag_image(rgba: Vec<u8>) -> Image {
    debug_assert_eq!(rgba.len(), (FLAG_WIDTH * FLAG_HEIGHT * 4) as usize);
    Image::new(
        Extent3d {
            width: FLAG_WIDTH,
            height: FLAG_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[inline]
fn set_px(buf: &mut [u8], x: u32, y: u32, rgba: [u8; 4]) {
    if x < FLAG_WIDTH && y < FLAG_HEIGHT {
        let idx = ((y * FLAG_WIDTH + x) * 4) as usize;
        buf[idx..idx + 4].copy_from_slice(&rgba);
    }
}

/// Applies subtle 1px top/left highlight and bottom/right bevel shading so flags sit crisply in UI frames.
fn apply_subtle_border_shading(buf: &mut [u8]) {
    for y in 0..FLAG_HEIGHT {
        for x in 0..FLAG_WIDTH {
            let factor = if y == FLAG_HEIGHT - 1 || x == FLAG_WIDTH - 1 {
                0.86_f32
            } else if y == 0 || x == 0 {
                1.06_f32
            } else {
                continue;
            };
            let idx = ((y * FLAG_WIDTH + x) * 4) as usize;
            for c in 0..3 {
                let v = (buf[idx + c] as f32 * factor).round().clamp(0.0, 255.0) as u8;
                buf[idx + c] = v;
            }
        }
    }
}

/// Generates the `64x40` RGBA Stars & Stripes US flag:
/// - 13 alternating Crimson Red (`#B22234`) & Pure White (`#F5F5F7`) horizontal stripes (red top & bottom)
/// - Deep Navy Blue (`#172B59`) canton over the top 7 stripes (`x = 0..27, y = 0..21`)
/// - Alternating 5-star and 4-star crisp white rows inside the canton
pub fn build_usa_flag_rgba() -> Vec<u8> {
    const CRIMSON: [u8; 4] = [0xB2, 0x22, 0x34, 0xFF];
    const WHITE: [u8; 4] = [0xF5, 0xF5, 0xF7, 0xFF];
    const NAVY: [u8; 4] = [0x17, 0x2B, 0x59, 0xFF];
    const STAR_CORE: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];
    const STAR_ARM: [u8; 4] = [0x96, 0x9F, 0xB4, 0xFF];

    const CANTON_W: u32 = 27;
    const CANTON_H: u32 = 21;

    let mut buf = vec![0u8; (FLAG_WIDTH * FLAG_HEIGHT * 4) as usize];

    for y in 0..FLAG_HEIGHT {
        // First 7 stripes are exactly 3px each (y = 0..21) so the canton aligns flush with stripe 7 (white).
        let stripe = if y < CANTON_H {
            y / 3
        } else {
            7 + ((y - CANTON_H) * 6 / (FLAG_HEIGHT - CANTON_H))
        };
        let stripe_col = if stripe.is_multiple_of(2) {
            CRIMSON
        } else {
            WHITE
        };

        for x in 0..FLAG_WIDTH {
            let col = if x < CANTON_W && y < CANTON_H {
                NAVY
            } else {
                stripe_col
            };
            set_px(&mut buf, x, y, col);
        }
    }

    // 5 alternating rows (5, 4, 5, 4, 5) of crisp symmetric white stars inside the navy canton.
    let star_rows: [(u32, &[u32]); 5] = [
        (2, &[3, 8, 13, 18, 23]),
        (6, &[5, 10, 16, 21]),
        (10, &[3, 8, 13, 18, 23]),
        (14, &[5, 10, 16, 21]),
        (18, &[3, 8, 13, 18, 23]),
    ];

    for (cy, xs) in star_rows {
        for &cx in xs {
            set_px(&mut buf, cx, cy, STAR_CORE);
            set_px(&mut buf, cx - 1, cy, STAR_ARM);
            set_px(&mut buf, cx + 1, cy, STAR_ARM);
            set_px(&mut buf, cx, cy - 1, STAR_ARM);
            set_px(&mut buf, cx, cy + 1, STAR_ARM);
        }
    }

    apply_subtle_border_shading(&mut buf);
    buf
}

/// Generates the `64x40` RGBA War Thunder WWII German Armored Forces flag:
/// - Rich Crimson Red (`#B8161E`) field
/// - Crisp black-and-white Nordic cross (white outer fimbriation, bold black inner cross bars)
/// - Centered black-rimmed white medallion housing a flared black Iron Cross (Balkenkreuz)
pub fn build_germany_flag_rgba() -> Vec<u8> {
    const CRIMSON: [u8; 4] = [0xB8, 0x16, 0x1E, 0xFF];
    const WHITE: [u8; 4] = [0xF4, 0xF4, 0xF6, 0xFF];
    const BLACK: [u8; 4] = [0x14, 0x15, 0x18, 0xFF];

    const CX: f32 = 28.5;
    const CY: f32 = 19.5;

    let mut buf = vec![0u8; (FLAG_WIDTH * FLAG_HEIGHT * 4) as usize];

    for y in 0..FLAG_HEIGHT {
        let dy = y as f32 - CY;
        let hy = dy.abs();

        for x in 0..FLAG_WIDTH {
            let dx = x as f32 - CX;
            let hx = dx.abs();

            // 1. Base crimson field + white-fimbriated black Nordic cross
            let mut col = if hx <= 3.0 || hy <= 3.0 {
                BLACK
            } else if hx <= 5.0 || hy <= 5.0 {
                WHITE
            } else {
                CRIMSON
            };

            // 2. Centered circular medallion + flared Iron Cross
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= 11.2 {
                col = if dist > 9.8 {
                    BLACK
                } else {
                    let h_arm = hx <= 7.2 && hy <= (1.65 + (hx / 7.2).powf(1.5) * 1.75);
                    let v_arm = hy <= 7.2 && hx <= (1.65 + (hy / 7.2).powf(1.5) * 1.75);
                    if h_arm || v_arm { BLACK } else { WHITE }
                };
            }

            set_px(&mut buf, x, y, col);
        }
    }

    apply_subtle_border_shading(&mut buf);
    buf
}

/// Generates the `64x40` RGBA Soviet Union flag:
/// - Deep Crimson Red (`#B81218`) field
/// - Crisp Gold (`#F8CE28`) 5-point Star above crossed Hammer & Sickle in upper-left (`x = 6..24, y = 5..21`)
pub fn build_ussr_flag_rgba() -> Vec<u8> {
    const CRIMSON: [u8; 4] = [0xB8, 0x12, 0x18, 0xFF];
    const GOLD: [u8; 4] = [0xF8, 0xCE, 0x28, 0xFF];

    const EMBLEM_X0: u32 = 6;
    const EMBLEM_Y0: u32 = 5;

    // 18x16 crisp pixel art for the Soviet 5-point Star + crossed Hammer & Sickle.
    const EMBLEM_BITMAP: [&str; 16] = [
        "........##........",
        "......######......",
        ".......####.......",
        "......##..##......",
        "..................",
        "........####......",
        "..........####....",
        ".....###....###...",
        "....#####....###..",
        ".....####.....##..",
        ".......##.....##..",
        "........##...###..",
        ".........######...",
        "......########....",
        "....###....###....",
        "...##.......###...",
    ];

    let mut buf = vec![0u8; (FLAG_WIDTH * FLAG_HEIGHT * 4) as usize];

    for y in 0..FLAG_HEIGHT {
        for x in 0..FLAG_WIDTH {
            set_px(&mut buf, x, y, CRIMSON);
        }
    }

    for (ry, row) in EMBLEM_BITMAP.iter().enumerate() {
        for (rx, ch) in row.bytes().enumerate() {
            if ch == b'#' {
                set_px(&mut buf, EMBLEM_X0 + rx as u32, EMBLEM_Y0 + ry as u32, GOLD);
            }
        }
    }

    apply_subtle_border_shading(&mut buf);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel_at(rgba: &[u8], x: u32, y: u32) -> [u8; 4] {
        let idx = ((y * FLAG_WIDTH + x) * 4) as usize;
        [rgba[idx], rgba[idx + 1], rgba[idx + 2], rgba[idx + 3]]
    }

    #[test]
    fn flag_rgba_buffers_have_exact_dimensions_and_signature_colors() {
        let expected_len = (FLAG_WIDTH * FLAG_HEIGHT * 4) as usize;

        // 1. USA: Stars & Stripes (navy canton top-left, red top-right & bottom-right — NOT Yugoslavia!)
        let usa = build_usa_flag_rgba();
        assert_eq!(usa.len(), expected_len);

        // Top-left canton interior is deep navy blue
        let canton_navy = pixel_at(&usa, 1, 1);
        assert!(
            canton_navy[2] > canton_navy[0] * 2 && canton_navy[2] > canton_navy[1],
            "USA top-left canton must be deep navy blue, got {canton_navy:?}"
        );
        // Star pixel inside canton is pure bright white
        let star_px = pixel_at(&usa, 13, 10);
        assert_eq!(star_px, [0xFF, 0xFF, 0xFF, 0xFF]);

        // Top-right stripe (stripe 0) and bottom-right stripe (stripe 12) are crimson red
        let top_right = pixel_at(&usa, 50, 1);
        let bottom_right = pixel_at(&usa, 50, 38);
        assert!(
            top_right[0] > 150 && top_right[1] < 60 && top_right[2] < 80,
            "USA top-right stripe must be crimson red, got {top_right:?}"
        );
        assert!(
            bottom_right[0] > 150 && bottom_right[1] < 60 && bottom_right[2] < 80,
            "USA bottom-right stripe must be crimson red, got {bottom_right:?}"
        );
        // White stripe directly under canton (y = 22)
        let mid_white = pixel_at(&usa, 10, 22);
        assert!(
            mid_white[0] > 230 && mid_white[1] > 230 && mid_white[2] > 230,
            "USA stripe below canton must be crisp white, got {mid_white:?}"
        );

        // 2. Germany: crimson red field + black & white Nordic cross + centered Iron Cross
        let ger = build_germany_flag_rgba();
        assert_eq!(ger.len(), expected_len);

        let ger_corner = pixel_at(&ger, 6, 6);
        assert!(
            ger_corner[0] > 160 && ger_corner[1] < 40 && ger_corner[2] < 50,
            "Germany field corners must be crimson red, got {ger_corner:?}"
        );
        let ger_cross_black = pixel_at(&ger, 52, 20);
        assert!(
            ger_cross_black[0] < 40 && ger_cross_black[1] < 40 && ger_cross_black[2] < 40,
            "Germany horizontal Nordic cross core must be black, got {ger_cross_black:?}"
        );
        let ger_cross_white = pixel_at(&ger, 52, 15);
        assert!(
            ger_cross_white[0] > 220 && ger_cross_white[1] > 220 && ger_cross_white[2] > 220,
            "Germany Nordic cross fimbriation must be crisp white, got {ger_cross_white:?}"
        );
        // Center of Iron Cross is black, surrounded by white medallion inside
        let ger_iron_center = pixel_at(&ger, 29, 20);
        let ger_medallion_white = pixel_at(&ger, 24, 15);
        assert!(ger_iron_center[0] < 40 && ger_iron_center[1] < 40 && ger_iron_center[2] < 40);
        assert!(ger_medallion_white[0] > 220 && ger_medallion_white[1] > 220 && ger_medallion_white[2] > 220);

        // 3. USSR: deep crimson red field + bright gold star & hammer/sickle in upper-left
        let ussr = build_ussr_flag_rgba();
        assert_eq!(ussr.len(), expected_len);

        let ussr_field = pixel_at(&ussr, 50, 28);
        assert!(
            ussr_field[0] > 160 && ussr_field[1] < 40 && ussr_field[2] < 40,
            "USSR field must be deep crimson red, got {ussr_field:?}"
        );
        let ussr_ref = ussr.as_slice();
        let gold_pixels = (6..24u32)
            .flat_map(|x| (5..21u32).map(move |y| pixel_at(ussr_ref, x, y)))
            .filter(|px| px[0] > 220 && px[1] > 180 && px[2] < 80)
            .count();
        assert!(
            gold_pixels >= 60,
            "USSR upper-left quadrant must contain crisp gold star & hammer/sickle pixels, found {gold_pixels}"
        );
    }
}
