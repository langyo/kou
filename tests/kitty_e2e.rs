//! End-to-end test: feed a kitty APC with a small PNG into a Screen,
//! render it to a real PNG, and verify the inline image is visible.
use image::GenericImageView;
use kou::{FontCache, Screen, theme_by_name};

/// Build a minimal 4×4 pixel PNG (red square).
fn red_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgba, ImageEncoder};
    use image::codecs::png::PngEncoder;
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(4, 4, Rgba([255, 0, 0, 255]));
    let mut buf = Vec::new();
    PngEncoder::new(&mut buf)
        .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8)
        .unwrap();
    buf
}

/// Build a 128×128 noisy PNG (varies per-pixel so PNG can't compress it,
/// producing a multi-KB base64 payload for split-feed testing).
fn large_red_png() -> Vec<u8> {
    use image::{ImageBuffer, Rgba, ImageEncoder};
    let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(128, 128);
    for y in 0..128 {
        for x in 0..128 {
            let r = ((x as u32 * 7 + y as u32 * 13) % 256) as u8;
            let g = ((x as u32 * 3 + y as u32 * 5) % 256) as u8;
            let b = ((x as u32 * 11 + y as u32 * 17) % 256) as u8;
            img[(x, y)] = Rgba([r, g, b, 255]);
        }
    }
    let mut buf = Vec::new();
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8)
        .unwrap();
    buf
}

#[test]
fn kitty_apc_renders_visible_image() {
    let png = red_png();
    let b64 = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        &png,
    );

    // Build a kitty APC: place a 4×4-cell image at cursor position.
    let kitty = format!(
        "\x1b_Ga=t,f=100,s=4,v=4,c=4,r=4,i=1;{}\x1b\\",
        b64
    );

    // Move cursor to row 2, col 3 BEFORE the APC.
    let mut screen = Screen::new(20, 10);
    screen.feed(b"\x1b[3;4H");
    screen.feed(kitty.as_bytes());

    // Verify placement.
    let placements = screen.image_store.placements();
    assert!(!placements.is_empty(), "no placement; feed may not have decoded the APC");
    let p = &placements[0];
    assert_eq!(p.row, 2);
    assert_eq!(p.col, 3);
    assert_eq!(p.cells_w, 4);

    // Render to PNG.
    let theme = theme_by_name("campbell");
    let empty_fonts = FontCache::empty();
    let rendered = kou::render::render_png_supersampled(
        &screen, &empty_fonts, 8.0, 1, theme,
    ).unwrap();

    let img = image::load_from_memory(&rendered).unwrap();
    let (w, h) = img.dimensions();
    let cell_w = 8u32;  // empty font cache: 8px
    let cell_h = 16u32;

    // Center of the red image block: col 3..7, row 2..6
    let cx = (3 + 2) as u32 * cell_w;
    let cy = (2 + 2) as u32 * cell_h;
    assert!(cx < w && cy < h, "sample {cx}x{cy} out of bounds ({w}x{h})");
    let pixel = img.get_pixel(cx, cy);
    let is_red = pixel[0] > 200 && pixel[1] < 50 && pixel[2] < 50;
    assert!(is_red, "red region center should be red, got {:?}", pixel);

    // Top-left corner: dark canvas bg.
    let bg = img.get_pixel(0, 0);
    assert!(bg[0] < 30 && bg[1] < 30 && bg[2] < 30,
            "canvas bg should be dark, got {:?}", bg);

    std::fs::write("/tmp/kitty_e2e_test.png", &rendered).unwrap();
    eprintln!("wrote /tmp/kitty_e2e_test.png ({w}x{h})");
}

#[test]
fn kitty_apc_split_across_feed() {
    let png = red_png();
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png);
    let kitty = format!("\x1b_Ga=t,f=100,c=4,r=4,i=99;{}\x1b\\", b64);

    // Split the APC at an arbitrary mid-point to simulate PTY read chunking.
    let mid = kitty.len() / 2;
    let part1 = &kitty.as_bytes()[..mid];
    let part2 = &kitty.as_bytes()[mid..];

    let mut screen = Screen::new(20, 10);
    screen.feed(part1);
    screen.feed(part2);

    let placements = screen.image_store.placements();
    assert!(!placements.is_empty(),
            "split APC should still produce a placement after reassembly");
    assert_eq!(placements[0].image_id, 99);
}

#[test]
fn iterm2_osc1337_renders_visible_image() {
    let png = red_png();
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png);

    // iTerm2 OSC 1337 inline image: 4×2 cells at cursor (2, 3).
    let seq = format!(
        "\x1b]1337;File=inline=1;width=4cells;height=2cells:{}\x07",
        b64
    );

    let mut screen = Screen::new(20, 10);
    screen.feed(b"\x1b[3;4H"); // cursor → row 2, col 3
    screen.feed(seq.as_bytes());

    let placements = screen.image_store.placements();
    assert!(!placements.is_empty(), "iTerm2 OSC 1337 should produce a placement");
    let p = &placements[0];
    assert_eq!(p.row, 2);
    assert_eq!(p.col, 3);
    assert_eq!(p.cells_w, 4);
    assert_eq!(p.cells_h, 2);

    // Render and verify the red region.
    let theme = theme_by_name("campbell");
    let empty_fonts = FontCache::empty();
    let rendered = kou::render::render_png_supersampled(
        &screen, &empty_fonts, 8.0, 1, theme,
    ).unwrap();
    let img = image::load_from_memory(&rendered).unwrap();
    let cell_w = 8u32;
    let cell_h = 16u32;
    let cx = (3 + 2) as u32 * cell_w;
    let cy = (2 + 1) as u32 * cell_h;
    let pixel = img.get_pixel(cx, cy);
    assert!(pixel[0] > 200 && pixel[1] < 50 && pixel[2] < 50,
            "iTerm2 image region should be red, got {:?}", pixel);
}

#[test]
fn iterm2_osc1337_split_across_feed() {
    // Large iTerm2 OSC 1337 sequences get split by PTY reads. Verify
    // the sliding apc_buf reassembles them across feed() boundaries.
    // Use a 64×64 image for a naturally long base64 payload (no invalid
    // padding from concatenation).
    let png = large_red_png();
    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png);
    let seq = format!(
        "\x1b]1337;File=inline=1;width=8cells;height=8cells:{}\x07",
        b64
    );
    assert!(seq.len() > 4000, "test needs a large OSC to exercise the buffer");
    let mid = seq.len() / 2;
    let mut screen = Screen::new(20, 10);
    screen.feed(b"\x1b[3;4H");
    screen.feed(&seq.as_bytes()[..mid]);
    screen.feed(&seq.as_bytes()[mid..]);
    let n = screen.image_store.placements().len();
    assert!(n > 0, "split iTerm2 OSC should still produce a placement (got {n})");
}
