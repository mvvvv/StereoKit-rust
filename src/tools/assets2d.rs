use crate::util::Color32;
use std::path::Path;

/// Reads a raw RGBA bitmap file (see <https://github.com/bzotto/rgba_bitmap>): four bytes of `"RGBA"` magic, then the
/// width and the height as big-endian `u32`s, then the RGBA8888 pixel data. Returns the size and the pixels as
/// [`Color32`]s, ready for `Tex::set_colors32`.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::assets2d::read_rgba_bitmap;
/// use std::io::Write;
///
/// let mut path = std::env::temp_dir();
/// path.push("assets2d_read_rgba_bitmap.rgba");
/// {
///     let mut file = std::fs::File::create(&path).expect("cannot create the temp file");
///     file.write_all(b"RGBA").unwrap();
///     file.write_all(&2u32.to_be_bytes()).unwrap(); // width
///     file.write_all(&1u32.to_be_bytes()).unwrap(); // height
///     file.write_all(&[10, 20, 30, 255, 40, 50, 60, 128]).unwrap();
/// }
/// let (width, height, pixels) = read_rgba_bitmap(&path).unwrap();
/// assert_eq!((width, height), (2, 1));
/// assert_eq!(pixels.len(), 2);
/// assert_eq!((pixels[0].r, pixels[0].g, pixels[0].b, pixels[0].a), (10, 20, 30, 255));
/// std::fs::remove_file(&path).unwrap();
/// ```
pub fn read_rgba_bitmap(path: &Path) -> Result<(usize, usize, Vec<Color32>), std::io::Error> {
    use std::io::Read;

    let mut header = [0u8; 12];
    let mut file = std::fs::File::open(path)?;

    file.read_exact(&mut header)?;
    if &header[0..4] != b"RGBA" {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid magic"));
    }
    let width = u32::from_be_bytes([header[4], header[5], header[6], header[7]]) as usize;
    let height = u32::from_be_bytes([header[8], header[9], header[10], header[11]]) as usize;
    if width == 0 || height == 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid dimensions"));
    }

    let mut data = Vec::new();
    file.read_to_end(&mut data)?;
    if data.len() != width * height * 4 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Pixel data size mismatch"));
    }
    // The length check above guarantees `data.len()` is a multiple of 4, so `as_chunks` leaves no remainder.
    let pixels = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|px| Color32 { r: px[0], g: px[1], b: px[2], a: px[3] })
        .collect();
    Ok((width, height, pixels))
}

/// Writes a raw RGBA bitmap file (see <https://github.com/bzotto/rgba_bitmap>), the counterpart of
/// [`read_rgba_bitmap`]: four bytes of `"RGBA"` magic, then the width and the height as big-endian `u32`s, then the
/// RGBA8888 pixel data `pixels` (exactly `width * height * 4` bytes, row-major).
///
/// Returns an error when `pixels.len() != width * height * 4` (an [`std::io::ErrorKind::InvalidData`]) or when the
/// file cannot be created / written.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::assets2d::{read_rgba_bitmap, write_rgba_bitmap};
///
/// let mut path = std::env::temp_dir();
/// path.push("assets2d_write_rgba_bitmap.rgba");
/// let (width, height) = (2usize, 1usize);
/// let pixels: Vec<u8> = (0..width * height * 4).map(|i| i as u8).collect();
/// write_rgba_bitmap(&path, width, height, &pixels).unwrap();
/// let (w, h, read_pixels) = read_rgba_bitmap(&path).unwrap();
/// assert_eq!((w, h), (width, height));
/// assert_eq!(read_pixels.len(), pixels.len() / 4);
/// assert_eq!((read_pixels[0].r, read_pixels[0].g, read_pixels[0].b, read_pixels[0].a), (0, 1, 2, 3));
/// std::fs::remove_file(&path).unwrap();
/// ```
pub fn write_rgba_bitmap(path: &Path, width: usize, height: usize, pixels: &[u8]) -> Result<(), std::io::Error> {
    use std::io::Write;

    if pixels.len() != width * height * 4 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Pixel data size mismatch: {} bytes for a {width}x{height} bitmap", pixels.len()),
        ));
    }

    let mut file = std::fs::File::create(path)?;
    file.write_all(b"RGBA")?;
    file.write_all(&(width as u32).to_be_bytes())?;
    file.write_all(&(height as u32).to_be_bytes())?;
    file.write_all(pixels)?;
    Ok(())
}
