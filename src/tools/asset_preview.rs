use crate::{
    material::Material,
    maths::{Matrix, Vec2, Vec3},
    mesh::Mesh,
    model::{AnimMode, Model},
    sound::{Sound, SoundInst},
    system::{Assets, Log},
    tex::Tex,
    util::Color32,
};
use std::path::Path;

/// A file loaded as a displayable 3D visualization, the file visualization of the `asset1` demo shared with the
/// other tools (see `examples/demos/asset1.rs` and `examples/demos/documents1.rs`): the [`AssetToShow::model`] field
/// displays the file content, and [`AssetToShow::sound_inst`] plays it when the file is a sound.
///
/// [`AssetToShow::from_file`] dispatches on the file extension:
/// - `Assets::MODEL_FORMATS` (`.gltf`, `.glb`, ...): the model itself, with its first animation played in loop,
/// - `Assets::TEXTURE_FORMATS` (`.png`, `.jpeg`, ...): a 6x6m plane showing the texture through a clipped material,
/// - `.sks`: a 6x6m plane showing the shader of the file, with `textures/open_gltf.jpeg` as diffuse,
/// - `Assets::SOUND_FORMATS` (`.wav`, `.mp3`): a 4m cube with the `textures/sound.jpeg` texture, and the sound
///   playing at the `sound_origin` given to [`AssetToShow::from_file`].
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Vec3, tools::asset_preview::AssetToShow};
/// use std::path::Path;
///
/// // A texture file is displayed on a plane, with no sound:
/// let visual = AssetToShow::from_file(Path::new("textures/open_gltf.jpeg"), Vec3::ZERO);
/// assert!(visual.is_some() && visual.unwrap().sound_inst.is_none());
/// // Unknown extensions are not displayable:
/// assert!(AssetToShow::from_file(Path::new("notes.txt"), Vec3::ZERO).is_none());
/// # sk::Sk::shutdown();
/// ```
#[derive(Debug)]
pub struct AssetToShow {
    /// The 3D model displaying the file content (the file itself, or a plane/cube carrying its texture/material).
    pub model: Model,
    /// The playing instance of the file sound, when the file is a sound file.
    pub sound_inst: Option<SoundInst>,
}

impl AssetToShow {
    /// Loads the file at `file_path` as a displayable 3D visualization (see [`AssetToShow`]), the sound of sound
    /// files playing at `sound_origin`. Returns `None` when the extension is not displayable, or when the loading
    /// fails (logged with [`Log::err`] for the model formats).
    pub fn from_file(file_path: &Path, sound_origin: Vec3) -> Option<AssetToShow> {
        let file_name_str =
            file_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "!!ERROR!!".into());
        if let Some(ext) = file_path.extension() {
            let ext = ".".to_string() + ext.to_str().unwrap_or("!!ERROR!!");
            if Assets::MODEL_FORMATS.contains(&ext.as_str()) {
                if let Ok(model) = Model::from_file(file_path, None, None) {
                    let mut anims = model.get_anims();
                    if anims.get_count() > 0 {
                        anims.play_anim_idx(0, AnimMode::Loop);
                    }
                    Some(AssetToShow { model, sound_inst: None })
                } else {
                    Log::err(format!("Unable to load model {file_name_str:?} !!"));
                    None
                }
            } else if Assets::TEXTURE_FORMATS.contains(&ext.as_str()) {
                let model = Model::new();
                let mesh = Mesh::generate_plane_up(Vec2::ONE * 6.0, None, true);
                let tex = Tex::from_file(file_path, true, None).unwrap_or_default();
                let mut material = Material::pbr_clip().copy();
                material.diffuse_tex(tex).clip_cutoff(0.1);
                model.get_nodes().add("tex_plane", Matrix::IDENTITY, Some(&mesh), Some(&material), true);
                Some(AssetToShow { model, sound_inst: None })
            } else if ext == ".sks" {
                let model = Model::new();
                let mesh = Mesh::generate_plane_up(Vec2::ONE * 6.0, None, true);
                let tex = Tex::from_file("textures/open_gltf.jpeg", true, None).unwrap_or_default();
                if let Ok(mut material) = Material::from_file(file_path, None) {
                    material.diffuse_tex(tex);
                    model.get_nodes().add("tex_plane", Matrix::IDENTITY, Some(&mesh), Some(&material), true);
                    Some(AssetToShow { model, sound_inst: None })
                } else {
                    None
                }
            } else if Assets::SOUND_FORMATS.contains(&ext.as_str()) {
                let model = Model::new();
                let mesh = Mesh::generate_cube(Vec3::ONE * 4.0, None);
                let tex = Tex::from_file("textures/sound.jpeg", true, None).unwrap_or_default();

                if let Ok(sound) = Sound::from_file(file_path) {
                    let sound_inst = sound.play(sound_origin, None);

                    let mut material = Material::default_copy();
                    material.diffuse_tex(tex);
                    model.get_nodes().add("tex_sound", Matrix::IDENTITY, Some(&mesh), Some(&material), true);
                    Some(AssetToShow { model, sound_inst: Some(sound_inst) })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Stops the sound of this visualization, when it is playing one. Call it before replacing or dropping a
    /// playing [`AssetToShow`], otherwise the sound keeps playing.
    pub fn stop_sound(&mut self) {
        if let Some(mut sound_inst) = self.sound_inst.take() {
            sound_inst.stop();
        }
    }
}

/// Reads a raw RGBA bitmap file (see <https://github.com/bzotto/rgba_bitmap>): four bytes of `"RGBA"` magic, then the
/// width and the height as big-endian `u32`s, then the RGBA8888 pixel data. Returns the size and the pixels as
/// [`Color32`]s, ready for `Tex::set_colors32`.
///
/// ### Examples
/// ```
/// use stereokit_rust::tools::asset_preview::read_rgba_bitmap;
/// use std::io::Write;
///
/// let mut path = std::env::temp_dir();
/// path.push("asset_preview_read_rgba_bitmap.rgba");
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
/// use stereokit_rust::tools::asset_preview::{read_rgba_bitmap, write_rgba_bitmap};
///
/// let mut path = std::env::temp_dir();
/// path.push("asset_preview_write_rgba_bitmap.rgba");
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
