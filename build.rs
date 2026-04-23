//! Build-time asset pipeline.
//!
//! 1. Decode `assets/app.png`, resize it, and drop the raw RGBA + size into
//!    `OUT_DIR` so the app can `include_bytes!` it at runtime for the egui
//!    window icon and the system tray icon.
//! 2. Encode the same image at multiple sizes into an `.ico` and embed it in
//!    the exe's resource section (via `winresource`), which is what Windows
//!    Explorer / the taskbar pick up for the executable's own icon.

use std::env;
use std::fs;
use std::path::PathBuf;

use image::RgbaImage;
use image::imageops::FilterType;

const SOURCE: &str = "assets/app.png";
/// Runtime icon size — big enough for tray (32) + window icon HiDPI (64).
const RUNTIME_SIZE: u32 = 128;
/// Sizes embedded into the .ico (Explorer, taskbar, alt-tab).
const ICO_SIZES: &[u32] = &[16, 24, 32, 48, 64, 128, 256];
/// Fraction of the icon canvas left empty around the content. Windows 11
/// icon guidelines recommend filling ≥90% of the canvas; we go edge-to-edge
/// so wider-than-tall logos (ears flaring out, etc) still look big enough
/// in the taskbar — the shorter axis keeps its natural aspect-ratio gap.
const CONTENT_MARGIN: f32 = 0.0;
/// Alpha below this value is treated as fully transparent when computing
/// the content bounding box.
const ALPHA_CUTOFF: u8 = 8;

fn main() {
    println!("cargo:rerun-if-changed={SOURCE}");
    println!("cargo:rerun-if-changed=build.rs");

    let src = image::open(SOURCE)
        .unwrap_or_else(|e| panic!("open {SOURCE}: {e}"))
        .to_rgba8();

    let prepped = normalize(&src);

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Runtime RGBA buffer — a single fixed square size.
    let runtime =
        image::imageops::resize(&prepped, RUNTIME_SIZE, RUNTIME_SIZE, FilterType::Lanczos3);
    fs::write(out_dir.join("icon.rgba"), runtime.as_raw()).expect("write icon.rgba");

    // Multi-size ICO for the embedded exe resource.
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);
    for &size in ICO_SIZES {
        let resized =
            image::imageops::resize(&prepped, size, size, FilterType::Lanczos3);
        let img = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        icon_dir.add_entry(
            ico::IconDirEntry::encode(&img).expect("ico entry encode"),
        );
    }
    let ico_path = out_dir.join("app.ico");
    {
        let file = fs::File::create(&ico_path).expect("create app.ico");
        let mut file = std::io::BufWriter::new(file);
        icon_dir.write(&mut file).expect("write app.ico");
    }

    // Embed the ICO as a Win32 resource so Explorer/taskbar pick it up.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon(ico_path.to_str().expect("ico path utf8"));
        res.compile().expect("winresource compile");
    }
}

/// Crop to non-transparent content, then pad to a square canvas with a
/// uniform margin so the visible artwork fills the final icon the same way
/// Windows' stock app icons do.
fn normalize(img: &RgbaImage) -> RgbaImage {
    let (w, h) = img.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;
    for y in 0..h {
        for x in 0..w {
            let a = img.get_pixel(x, y)[3];
            if a >= ALPHA_CUTOFF {
                found = true;
                if x < min_x {
                    min_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if x > max_x {
                    max_x = x;
                }
                if y > max_y {
                    max_y = y;
                }
            }
        }
    }
    if !found {
        return img.clone();
    }

    let crop_w = max_x - min_x + 1;
    let crop_h = max_y - min_y + 1;
    let cropped =
        image::imageops::crop_imm(img, min_x, min_y, crop_w, crop_h).to_image();

    let side = crop_w.max(crop_h);
    let canvas_side = ((side as f32) / (1.0 - 2.0 * CONTENT_MARGIN)).ceil() as u32;
    let mut canvas =
        RgbaImage::from_pixel(canvas_side, canvas_side, image::Rgba([0, 0, 0, 0]));
    let ox = (canvas_side - crop_w) / 2;
    let oy = (canvas_side - crop_h) / 2;
    image::imageops::overlay(&mut canvas, &cropped, ox as i64, oy as i64);
    canvas
}
