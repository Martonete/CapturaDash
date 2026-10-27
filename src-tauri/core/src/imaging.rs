//! Decodificación de datos del portapapeles, hash de contenido y miniaturas.


use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ExtendedColorType, ImageEncoder, ImageFormat, RgbaImage};

use crate::error::{CoreError, Result};

/// Lado máximo de una miniatura. Cubre el panel (≈360 px lógicos) a 150 % de
/// escala y el pizarrón con zoom bajo; por encima se usa el original.
pub const THUMB_MAX_W: u32 = 640;
pub const THUMB_MAX_H: u32 = 1280;
const JPEG_QUALITY: u8 = 86;

/// Datos de imagen tal como llegan del sistema.
#[derive(Debug, Clone)]
pub enum ImagePayload {
    /// Formato registrado "PNG" del portapapeles (Recortes, navegadores, Office).
    Png(Vec<u8>),
    /// CF_DIB / CF_DIBV5: encabezado BITMAPINFO + píxeles, sin BITMAPFILEHEADER.
    Dib(Vec<u8>),
    /// Archivo PNG o JPEG completo (por ejemplo, leído de disco).
    Encoded(Vec<u8>),
}

pub struct DecodedImage {
    pub rgba: RgbaImage,
    /// Bytes a guardar como original y su tipo. Si la fuente ya era un PNG/JPEG
    /// se conservan tal cual (calidad y metadatos originales).
    pub original: Vec<u8>,
    pub mime: &'static str,
    pub opaque: bool,
}

pub struct Thumbnail {
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub ext: &'static str,
}

pub fn decode(payload: ImagePayload) -> Result<DecodedImage> {
    let (rgba, keep): (RgbaImage, Option<(Vec<u8>, &'static str)>) = match payload {
        ImagePayload::Png(bytes) => {
            let img = image::load_from_memory_with_format(&bytes, ImageFormat::Png)?;
            (img.to_rgba8(), Some((bytes, "image/png")))
        }
        ImagePayload::Encoded(bytes) => {
            let (fmt, mime) = sniff(&bytes)
                .ok_or_else(|| CoreError::Invalid("formato de imagen no soportado".into()))?;
            let img = image::load_from_memory_with_format(&bytes, fmt)?;
            (img.to_rgba8(), Some((bytes, mime)))
        }
        ImagePayload::Dib(dib) => {
            let bmp = dib_to_bmp(&dib)?;
            let img = image::load_from_memory_with_format(&bmp, ImageFormat::Bmp)?;
            let mut rgba = img.to_rgba8();
            fix_zero_alpha(&mut rgba);
            (rgba, None)
        }
    };

    if rgba.width() == 0 || rgba.height() == 0 {
        return Err(CoreError::Invalid("imagen vacía".into()));
    }
    let opaque = rgba.pixels().all(|p| p.0[3] == 255);
    let (original, mime) = match keep {
        Some(k) => k,
        None => (encode_png(&rgba, opaque)?, "image/png"),
    };
    Ok(DecodedImage { rgba, original, mime, opaque })
}

/// Hash del contenido visual: dos copias de la misma imagen en formatos
/// distintos (PNG vs DIB) producen el mismo hash.
pub fn content_hash(img: &RgbaImage) -> String {
    let mut h = blake3::Hasher::new();
    h.update(&img.width().to_le_bytes());
    h.update(&img.height().to_le_bytes());
    h.update(img.as_raw());
    h.finalize().to_hex().to_string()
}

pub fn make_thumbnail(img: &RgbaImage, opaque: bool) -> Result<Thumbnail> {
    let (w, h) = fit(img.width(), img.height(), THUMB_MAX_W, THUMB_MAX_H);
    // `thumbnail` promedia áreas: rápido y nítido para reducciones grandes.
    let small = if (w, h) == (img.width(), img.height()) {
        img.clone()
    } else {
        image::imageops::thumbnail(img, w, h)
    };
    if opaque {
        let rgb = DynamicImage::ImageRgba8(small).to_rgb8();
        let mut out = Vec::new();
        JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY).write_image(
            rgb.as_raw(),
            w,
            h,
            ExtendedColorType::Rgb8,
        )?;
        Ok(Thumbnail { bytes: out, width: w, height: h, ext: "jpg" })
    } else {
        Ok(Thumbnail { bytes: encode_png(&small, false)?, width: w, height: h, ext: "png" })
    }
}

pub fn encode_png(img: &RgbaImage, opaque: bool) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    let enc = PngEncoder::new_with_quality(&mut out, CompressionType::Fast, FilterType::Adaptive);
    if opaque {
        // Sin canal alfa el archivo pesa ~25 % menos.
        let rgb = DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        enc.write_image(rgb.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8)?;
    } else {
        enc.write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgba8)?;
    }
    Ok(out)
}

/// Decodifica un original guardado (PNG/JPEG) a RGBA.
pub fn load_rgba(bytes: &[u8]) -> Result<RgbaImage> {
    let (fmt, _) = sniff(bytes).ok_or_else(|| CoreError::Invalid("formato desconocido".into()))?;
    Ok(image::load_from_memory_with_format(bytes, fmt)?.to_rgba8())
}

pub fn sniff(bytes: &[u8]) -> Option<(ImageFormat, &'static str)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some((ImageFormat::Png, "image/png"))
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some((ImageFormat::Jpeg, "image/jpeg"))
    } else {
        None
    }
}

pub fn ext_for_mime(mime: &str) -> &'static str {
    if mime == "image/jpeg" {
        "jpg"
    } else {
        "png"
    }
}

/// Encaja (w, h) dentro de (max_w, max_h) conservando proporción; nunca amplía.
pub fn fit(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    let scale = (max_w as f64 / w as f64).min(max_h as f64 / h as f64).min(1.0);
    (((w as f64 * scale).round() as u32).max(1), ((h as f64 * scale).round() as u32).max(1))
}

/// Antepone un BITMAPFILEHEADER a un DIB del portapapeles para poder
/// decodificarlo como un .bmp normal.
pub fn dib_to_bmp(dib: &[u8]) -> Result<Vec<u8>> {
    const BI_BITFIELDS: u32 = 3;
    const BI_ALPHABITFIELDS: u32 = 6;
    let bad = || CoreError::Invalid("DIB inválido".into());
    if dib.len() < 40 {
        return Err(bad());
    }
    let u32_at = |o: usize| u32::from_le_bytes(dib[o..o + 4].try_into().unwrap());
    let u16_at = |o: usize| u16::from_le_bytes(dib[o..o + 2].try_into().unwrap());

    let header_size = u32_at(0) as usize;
    if header_size < 40 || header_size > dib.len() {
        return Err(bad());
    }
    let bit_count = u16_at(14) as u32;
    let compression = u32_at(16);
    let clr_used = u32_at(32);

    // Con BITMAPINFOHEADER (40 bytes) las máscaras de color van después del encabezado.
    let masks = match (header_size, compression) {
        (40, BI_BITFIELDS) => 12,
        (40, BI_ALPHABITFIELDS) => 16,
        _ => 0,
    };
    let palette_entries = if bit_count <= 8 {
        if clr_used == 0 { 1u32 << bit_count } else { clr_used }
    } else {
        clr_used
    };
    let offset = 14 + header_size + masks + palette_entries as usize * 4;
    let file_size = 14 + dib.len();
    if offset > file_size {
        return Err(bad());
    }

    let mut out = Vec::with_capacity(file_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&[0, 0, 0, 0]);
    out.extend_from_slice(&(offset as u32).to_le_bytes());
    out.extend_from_slice(dib);
    Ok(out)
}

/// Muchas aplicaciones escriben DIB de 32 bits con alfa = 0 en todos los
/// píxeles. Interpretarlo literalmente daría una imagen invisible.
fn fix_zero_alpha(img: &mut RgbaImage) {
    if img.pixels().all(|p| p.0[3] == 0) {
        for p in img.pixels_mut() {
            p.0[3] = 255;
        }
    }
}

/// Convierte RGBA a un CF_DIB de 32 bits (BITMAPINFOHEADER, BI_RGB, de abajo
/// hacia arriba, BGRA). Windows sintetiza CF_DIBV5 y CF_BITMAP a partir de él.
pub fn rgba_to_dib(img: &RgbaImage) -> Vec<u8> {
    let (w, h) = (img.width(), img.height());
    let row = w as usize * 4;
    let mut out = Vec::with_capacity(40 + row * h as usize);
    out.extend_from_slice(&40u32.to_le_bytes()); // biSize
    out.extend_from_slice(&(w as i32).to_le_bytes()); // biWidth
    out.extend_from_slice(&(h as i32).to_le_bytes()); // biHeight (>0 = de abajo hacia arriba)
    out.extend_from_slice(&1u16.to_le_bytes()); // biPlanes
    out.extend_from_slice(&32u16.to_le_bytes()); // biBitCount
    out.extend_from_slice(&0u32.to_le_bytes()); // biCompression = BI_RGB
    out.extend_from_slice(&((row * h as usize) as u32).to_le_bytes()); // biSizeImage
    out.extend_from_slice(&2835i32.to_le_bytes()); // 72 DPI
    out.extend_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // biClrUsed
    out.extend_from_slice(&0u32.to_le_bytes()); // biClrImportant
    let raw = img.as_raw();
    for y in (0..h as usize).rev() {
        for px in raw[y * row..(y + 1) * row].chunks_exact(4) {
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use image::Rgba;

    pub fn sample(w: u32, h: u32, seed: u8) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| {
            Rgba([(x as u8).wrapping_add(seed), (y as u8).wrapping_mul(3), seed, 255])
        })
    }

    pub fn sample_png(w: u32, h: u32, seed: u8) -> Vec<u8> {
        encode_png(&sample(w, h, seed), false).unwrap()
    }

    #[test]
    fn png_and_dib_of_same_pixels_hash_equal() {
        let img = sample(37, 21, 9);
        let png = decode(ImagePayload::Png(encode_png(&img, true).unwrap())).unwrap();
        let dib = decode(ImagePayload::Dib(rgba_to_dib(&img))).unwrap();
        assert_eq!(content_hash(&png.rgba), content_hash(&img));
        assert_eq!(content_hash(&dib.rgba), content_hash(&img));
        assert_eq!(dib.mime, "image/png");
    }

    #[test]
    fn png_original_bytes_are_preserved() {
        let bytes = sample_png(10, 10, 1);
        let d = decode(ImagePayload::Png(bytes.clone())).unwrap();
        assert_eq!(d.original, bytes);
    }

    #[test]
    fn zero_alpha_dib_becomes_opaque() {
        let mut img = sample(4, 4, 2);
        for p in img.pixels_mut() {
            p.0[3] = 0;
        }
        // Un DIB 32 bpp BI_RGB "sin alfa": el decodificador ignora el 4.º byte
        // o lo lee como 0; en ambos casos el resultado debe ser opaco.
        let d = decode(ImagePayload::Dib(rgba_to_dib(&img))).unwrap();
        assert!(d.opaque);
    }

    #[test]
    fn dib_with_bitfields_masks() {
        // BITMAPINFOHEADER + BI_BITFIELDS: 12 bytes de máscaras tras el encabezado.
        let img = sample(3, 2, 5);
        let plain = rgba_to_dib(&img);
        let mut dib = plain[..40].to_vec();
        dib[16..20].copy_from_slice(&3u32.to_le_bytes());
        dib.extend_from_slice(&0x00FF_0000u32.to_le_bytes());
        dib.extend_from_slice(&0x0000_FF00u32.to_le_bytes());
        dib.extend_from_slice(&0x0000_00FFu32.to_le_bytes());
        dib.extend_from_slice(&plain[40..]);
        let d = decode(ImagePayload::Dib(dib)).unwrap();
        assert_eq!(content_hash(&d.rgba), content_hash(&img));
    }

    #[test]
    fn thumbnail_fits_and_keeps_ratio() {
        let img = sample(3840, 2160, 0);
        let t = make_thumbnail(&img, true).unwrap();
        assert_eq!((t.width, t.height), (640, 360));
        assert_eq!(t.ext, "jpg");
        let small = sample(100, 50, 0);
        let t = make_thumbnail(&small, false).unwrap();
        assert_eq!((t.width, t.height), (100, 50));
        assert_eq!(t.ext, "png");
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode(ImagePayload::Dib(vec![1, 2, 3])).is_err());
        assert!(decode(ImagePayload::Encoded(b"hola".to_vec())).is_err());
        assert!(decode(ImagePayload::Png(b"\x89PNG\r\n\x1a\nbroken".to_vec())).is_err());
    }
}
