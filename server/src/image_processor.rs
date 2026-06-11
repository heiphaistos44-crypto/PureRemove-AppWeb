/// image_processor.rs — Décodage, manipulation et encodage des images.
/// Adapté de PureRemove desktop (audit v1.2.0) — version 100% bytes, zéro accès fichier.

use anyhow::{anyhow, Result};
use image::{DynamicImage, GrayImage, RgbaImage};
use rayon::prelude::*;
use std::io::Cursor;

// ─── Chargement ──────────────────────────────────────────────────────────────

/// Charge depuis un buffer brut (upload HTTP).
/// SVG détecté par contenu → rasterisé. Images > 4K → smart-downscale.
pub fn load_image_from_bytes(bytes: &[u8]) -> Result<DynamicImage> {
    if looks_like_svg(bytes) {
        return rasterize_svg(bytes);
    }
    let img = image::load_from_memory(bytes).map_err(|e| anyhow!("Décodage image : {e}"))?;
    Ok(smart_downscale(img))
}

/// Détection SVG par contenu (le type MIME client n'est pas fiable).
fn looks_like_svg(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(1024)];
    let Ok(text) = std::str::from_utf8(head) else {
        return false;
    };
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    trimmed.starts_with("<svg") || (trimmed.starts_with("<?xml") && text.contains("<svg"))
}

/// Réduit intelligemment si > 4096px sur un côté (RAM protection).
fn smart_downscale(img: DynamicImage) -> DynamicImage {
    const MAX_DIM: u32 = 4096;
    let (w, h) = (img.width(), img.height());
    if w <= MAX_DIM && h <= MAX_DIM {
        return img;
    }
    let scale = MAX_DIM as f32 / w.max(h) as f32;
    let nw = (w as f32 * scale).max(1.0) as u32;
    let nh = (h as f32 * scale).max(1.0) as u32;
    img.resize_exact(nw, nh, image::imageops::FilterType::Lanczos3)
}

// ─── SVG → Bitmap ────────────────────────────────────────────────────────────

fn rasterize_svg(svg_data: &[u8]) -> Result<DynamicImage> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg_data, &options)
        .map_err(|e| anyhow!("SVG parse error : {e}"))?;

    let size = tree.size();

    if size.width() <= 0.0 || size.height() <= 0.0 {
        return Err(anyhow!("SVG invalide : dimensions nulles ou négatives"));
    }

    const MIN_SVG_WIDTH: f32 = 2048.0;
    const MAX_SVG_PIXELS: u32 = 8192;

    let scale = (MIN_SVG_WIDTH / size.width()).max(1.0);
    let raw_w = size.width() * scale;
    let raw_h = size.height() * scale;

    // Cap proportionnel : préserve l'aspect ratio
    let (px_w, px_h) = if raw_w >= raw_h {
        let w = raw_w.min(MAX_SVG_PIXELS as f32) as u32;
        let h = ((raw_h / raw_w) * w as f32).max(1.0) as u32;
        (w, h)
    } else {
        let h = raw_h.min(MAX_SVG_PIXELS as f32) as u32;
        let w = ((raw_w / raw_h) * h as f32).max(1.0) as u32;
        (w, h)
    };

    let mut pixmap = resvg::tiny_skia::Pixmap::new(px_w, px_h)
        .ok_or_else(|| anyhow!("Impossible de créer le Pixmap SVG ({px_w}×{px_h})"))?;

    let transform = resvg::tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny_skia Pixmap est en RGBA prémultiplié — on dé-multiplie pour image crate
    let rgba_data: Vec<u8> = pixmap
        .data()
        .chunks(4)
        .flat_map(|px| {
            let a = px[3] as f32 / 255.0;
            if a > 0.004 {
                [
                    (px[0] as f32 / a).min(255.0) as u8,
                    (px[1] as f32 / a).min(255.0) as u8,
                    (px[2] as f32 / a).min(255.0) as u8,
                    px[3],
                ]
            } else {
                [0, 0, 0, 0]
            }
        })
        .collect();

    let rgba_img = RgbaImage::from_raw(px_w, px_h, rgba_data)
        .ok_or_else(|| anyhow!("Conversion SVG→RgbaImage échouée"))?;

    Ok(smart_downscale(DynamicImage::ImageRgba8(rgba_img)))
}

// ─── Application du masque alpha ─────────────────────────────────────────────

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum BackgroundColor {
    Transparent,
    White,
    Black,
    Color { r: u8, g: u8, b: u8 },
}

/// Applique le masque alpha (avec flou de bords) sur l'image originale.
/// Retourne une RgbaImage avec le fond choisi.
/// Parallélisé avec rayon — gain 3-4x sur images > 2K.
pub fn apply_mask(
    img: &DynamicImage,
    mask: &GrayImage,
    bg: &BackgroundColor,
) -> Result<DynamicImage> {
    let (w, h) = (img.width(), img.height());
    let rgba_src = img.to_rgba8();

    let blurred_mask = blur_mask(mask)?;

    let src_data  = rgba_src.as_raw();
    let mask_data = blurred_mask.as_raw();
    let w_usize   = w as usize;

    let mut out_data = vec![0u8; (w * h * 4) as usize];

    out_data
        .par_chunks_mut(w_usize * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let row_off = y * w_usize;
            for x in 0..w_usize {
                let src_off = (row_off + x) * 4;
                let src     = &src_data[src_off..src_off + 4];
                let alpha   = mask_data[row_off + x];
                let alpha_f = alpha as f32 / 255.0;

                let out: [u8; 4] = match bg {
                    BackgroundColor::Transparent => [src[0], src[1], src[2], alpha],
                    BackgroundColor::White => {
                        let blend = |fg: u8, bg_c: u8| -> u8 {
                            (fg as f32 * alpha_f + bg_c as f32 * (1.0 - alpha_f)) as u8
                        };
                        [blend(src[0], 255), blend(src[1], 255), blend(src[2], 255), 255]
                    }
                    BackgroundColor::Black => {
                        let blend = |fg: u8| -> u8 { (fg as f32 * alpha_f) as u8 };
                        [blend(src[0]), blend(src[1]), blend(src[2]), 255]
                    }
                    BackgroundColor::Color { r, g, b } => {
                        let blend = |fg: u8, bg_c: u8| -> u8 {
                            (fg as f32 * alpha_f + bg_c as f32 * (1.0 - alpha_f)) as u8
                        };
                        [blend(src[0], *r), blend(src[1], *g), blend(src[2], *b), 255]
                    }
                };

                let off = x * 4;
                row[off..off + 4].copy_from_slice(&out);
            }
        });

    let output = RgbaImage::from_raw(w, h, out_data)
        .ok_or_else(|| anyhow!("apply_mask: incohérence buffer {}×{}", w, h))?;
    Ok(DynamicImage::ImageRgba8(output))
}

/// Gaussian blur 3×3 léger sur le masque pour adoucir les contours.
fn blur_mask(mask: &GrayImage) -> Result<GrayImage> {
    let (w, h)   = mask.dimensions();
    let w_usize  = w as usize;
    let h_usize  = h as usize;
    let raw      = mask.as_raw();
    let kernel: [f32; 9] = [
        1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0,
        2.0 / 16.0, 4.0 / 16.0, 2.0 / 16.0,
        1.0 / 16.0, 2.0 / 16.0, 1.0 / 16.0,
    ];

    let mut out_data = vec![0u8; w_usize * h_usize];

    out_data
        .par_chunks_mut(w_usize)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w_usize {
                let mut sum = 0.0f32;
                for ky in 0..3i32 {
                    let py = (y as i32 + ky - 1).clamp(0, h_usize as i32 - 1) as usize;
                    for kx in 0..3i32 {
                        let px = (x as i32 + kx - 1).clamp(0, w_usize as i32 - 1) as usize;
                        sum += raw[py * w_usize + px] as f32
                            * kernel[(ky * 3 + kx) as usize];
                    }
                }
                row[x] = sum as u8;
            }
        });

    GrayImage::from_raw(w, h, out_data)
        .ok_or_else(|| anyhow!("blur_mask: incohérence buffer {}×{}", w, h))
}

// ─── Encodage ────────────────────────────────────────────────────────────────

pub fn encode_png(img: &DynamicImage) -> Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| anyhow!("Encodage PNG : {e}"))?;
    Ok(buf.into_inner())
}
