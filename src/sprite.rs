use crate::{
    StereoKitError,
    maths::{Matrix, Vec2},
    sk::MainThreadToken,
    system::{IAsset, TextAlign},
    tex::{Tex, TexT},
    util::Color32,
};
use std::{
    ffi::{CStr, CString, c_char},
    path::Path,
    ptr::NonNull,
};

/// The way the Sprite is stored on the backend! Does it get batched and atlased for draw efficiency, or is it a single image?
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum SpriteType {
    /// The sprite will be batched onto an atlas texture so all sprites can be drawn in a single pass. This is excellent for performance! The only thing to watch out for here, adding a sprite to an atlas will rebuild the atlas texture! This can be a bit expensive, so it’s recommended to add all sprites to an atlas at start, rather than during runtime. Also, if an image is too large, it may take up too much space on the atlas, and may be better as a Single sprite type.
    Atlased = 0,
    /// This sprite is on its own texture. This is best for large images, items that get loaded and unloaded during runtime, or for sprites that may have edge artifacts or severe ‘bleed’ from adjacent atlased images.
    Single = 1,
}

/// A Sprite is an image that’s set up for direct 2D rendering, without using a mesh or model! This is technically a
/// wrapper over a texture, but it also includes atlasing functionality, which can be pretty important to performance!
/// This is used a lot in UI, for image rendering.
///
/// Atlasing is not currently implemented, it’ll swap to Single for now. But here’s how it works!
/// StereoKit will batch your sprites into an atlas if you ask it to! This puts all the images on a single texture to
/// significantly reduce draw calls when many images are present. Any time you add a sprite to an atlas, it’ll be marked
/// as dirty and rebuilt at the end of the frame. So it can be a good idea to add all your images to the atlas on
/// initialize rather than during execution!
///
/// Since rendering is atlas based, you also have only one material per atlas. So this is why you might wish to put a
/// sprite in one atlas or another, so you can apply different
/// <https://stereokit.net/Pages/StereoKit/Sprite.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, sprite::{Sprite, SpriteType},
///                      tex::Tex, util::{Gradient, Color128, named_colors}, system::TextAlign};
/// let mut gradient = Gradient::new(None);
/// gradient
///     .add(Color128::BLACK_TRANSPARENT, 0.0)
///     .add(named_colors::YELLOW, 0.1)
///     .add(named_colors::LIGHT_BLUE, 0.4)
///     .add(named_colors::BLUE, 0.5)
///     .add(Color128::BLACK, 0.7);
/// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
/// let tex_particule2 = Tex::gen_particle(128, 128, 0.2, None);
///
/// let sprite1 = Sprite::from_tex(&tex_particule1, None, None)
///                   .expect("Should be able to create sprite");
/// let sprite2 = Sprite::from_file("icons/fly_over.png", Some(SpriteType::Single), Some("MY_ID"))
///                   .expect("open_gltf.jpeg should be able to create sprite");
/// let sprite3 = sprite1.clone_ref();
/// let sprite4 = Sprite::from_tex(&tex_particule2, None, None)
///                   .expect("Should be able to create sprite");
///
/// let transform1 = Matrix::t([-0.7, 0.4, 0.0]) * Matrix::Y_180;
/// let transform2 = Matrix::t([ 0.7, 0.4, 0.0]) * Matrix::Y_180;
/// let transform3 = Matrix::t([-0.7,-0.4, 0.0]) * Matrix::Y_180;
/// let transform4 = Matrix::t([ 0.7,-0.4, 0.0]) * Matrix::Y_180;
///
/// filename_scr = "screenshots/sprite.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     sprite1.draw(token, transform1, TextAlign::Center, None);
///     sprite2.draw(token, transform2, TextAlign::XLeft, Some(named_colors::AZURE.into()));
///     sprite3.draw(token, transform3, TextAlign::TopRight, Some(named_colors::LIME.into()));
///     sprite4.draw(token, transform4, TextAlign::YCenter, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Sprite(pub NonNull<_SpriteT>);

impl Drop for Sprite {
    fn drop(&mut self) {
        unsafe { sprite_release(self.0.as_ptr()) };
    }
}

impl AsRef<Sprite> for Sprite {
    fn as_ref(&self) -> &Sprite {
        self
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _SpriteT {
    _unused: [u8; 0],
}

/// StereoKit ffi type.
pub type SpriteT = *mut _SpriteT;

unsafe extern "C" {
    pub fn sprite_find(id: *const c_char) -> SpriteT;
    pub fn sprite_create(sprite: TexT, type_: SpriteType, atlas_id: *const c_char) -> SpriteT;
    pub fn sprite_create_file(filename_utf8: *const c_char, type_: SpriteType, atlas_id: *const c_char) -> SpriteT;

    pub fn sprite_set_id(sprite: SpriteT, id: *const c_char);

    pub fn sprite_get_id(sprite: SpriteT) -> *const c_char;
    pub fn sprite_addref(sprite: SpriteT);
    pub fn sprite_release(sprite: SpriteT);
    pub fn sprite_get_aspect(sprite: SpriteT) -> f32;
    pub fn sprite_get_width(sprite: SpriteT) -> i32;
    pub fn sprite_get_height(sprite: SpriteT) -> i32;
    pub fn sprite_get_dimensions_normalized(sprite: SpriteT) -> Vec2;
    pub fn sprite_draw(sprite: SpriteT, transform: Matrix, anchor_position: TextAlign, color: Color32);
}

impl IAsset for Sprite {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }
}

/// A Default sprite is asked when a Sprite creation or find returned an error. (close is the default sprite)
impl Default for Sprite {
    fn default() -> Self {
        Self::close()
    }
}

impl Sprite {
    /// Create a sprite from a texture.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/FromTex.html>
    /// * `tex` - The texture to build a sprite from. Must be a valid, 2D image!
    /// * `sprite_type` - Should this sprite be atlased, or an individual image? Adding this as an atlased image is better for
    ///   performance, but will cause the atlas to be rebuilt! Images that take up too much space on the atlas, or might
    ///   be loaded or unloaded during runtime may be better as Single rather than Atlased!
    ///   If None has default of Atlased.
    /// * `atlas_id` - The name of which atlas the sprite should belong to, this is only relevant if the SpriteType is
    ///   Atlased. If None has default of "default".
    ///
    /// Returns a  Sprite asset, or null if the image failed when adding to the sprite system!
    /// see also [`sprite_create`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{ maths::Matrix, sprite::{Sprite, SpriteType}, tex::Tex, system::TextAlign,
    ///                      util::{Gradient, Color128, named_colors}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(named_colors::YELLOW, 0.1)
    ///     .add(named_colors::LIGHT_BLUE, 0.4)
    ///     .add(named_colors::BLUE, 0.5)
    ///     .add(Color128::BLACK, 0.7);
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    /// let sprite = Sprite::from_tex(&tex_particule1, Some(SpriteType::Atlased), Some("my_sprite".to_string()))
    ///                   .expect("Should be able to create sprite");
    ///
    /// assert!(sprite.get_id().starts_with("auto/tex_"));
    /// assert_eq!(sprite.get_height(), 128);
    /// assert_eq!(sprite.get_width(), 128);
    /// assert_eq!(sprite.get_normalized_dimensions(), [1.0, 1.0].into());
    /// assert_eq!(sprite.get_aspect(), 1.0);
    ///
    /// filename_scr = "screenshots/sprite_from_tex.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::XRight,  None);
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::XLeft,   Some(named_colors::BLUE.into()));
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::YTop,    Some(named_colors::RED.into()));
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::YBottom, Some(named_colors::GREEN.into()));
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_from_tex.jpeg" alt="screenshot" width="200">
    pub fn from_tex(
        sprite_tex: impl AsRef<Tex>,
        sprite_type: Option<SpriteType>,
        atlas_id: Option<String>,
    ) -> Result<Sprite, StereoKitError> {
        let sprite_type = sprite_type.unwrap_or(SpriteType::Atlased);
        let atlas_id = match atlas_id {
            Some(s) => s,
            None => "default".to_owned(),
        };
        let c_atlas_id = CString::new(atlas_id)?;
        Ok(Sprite(
            NonNull::new(unsafe { sprite_create(sprite_tex.as_ref().0.as_ptr(), sprite_type, c_atlas_id.as_ptr()) })
                .ok_or(StereoKitError::SpriteCreate)?,
        ))
    }

    /// Create a sprite from an image file! This loads a Texture from file, and then uses that Texture as the source for the Sprite.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/FromFile.html>
    /// * `file_utf8` - The filename of the image, an absolute filename, or a filename relative to the assets folder.
    ///   Supports jpg, png, tga, bmp, psd, gif, hdr, pic.
    /// * `sprite_type` - Should this sprite be atlased, or an individual image? Adding this as an atlased image is
    ///   better for performance, but will cause the atlas to be rebuilt! Images that take up too much space on the
    ///   atlas, or might be loaded or unloaded during runtime may be better as Single rather than Atlased!
    ///   If None has default of Atlased
    /// * `atlas_id` - The name of which atlas the sprite should belong to, this is only relevant if the SpriteType is
    ///   Atlased. If None has default of "default".
    ///
    /// see also [`sprite_create_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{ maths::Matrix, sprite::{Sprite, SpriteType}, system::TextAlign,
    ///                      util::named_colors};
    ///
    /// let sprite = Sprite::from_file("icons/log_viewer.png", Some(SpriteType::Single), Some("MY_ID"))
    ///                   .expect("open_gltf.jpeg should be able to create sprite");
    ///
    /// assert_eq!(sprite.get_id(), "icons/log_viewer.png/sprite");
    /// assert_eq!(sprite.get_height(), 128);
    /// assert_eq!(sprite.get_width(), 128);
    /// assert_eq!(sprite.get_normalized_dimensions().x, 1.0);
    /// assert_eq!(sprite.get_normalized_dimensions().y, 1.0);
    ///
    /// filename_scr = "screenshots/sprite_from_file.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::XRight,  None);
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::XLeft,   Some(named_colors::BLUE.into()));
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::YTop,    Some(named_colors::RED.into()));
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::YBottom, Some(named_colors::GREEN.into()));
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_from_file.jpeg" alt="screenshot" width="200">
    pub fn from_file(
        file_utf8: impl AsRef<Path>,
        sprite_type: Option<SpriteType>,
        atlas_id: Option<&str>,
    ) -> Result<Sprite, StereoKitError> {
        let sprite_type = sprite_type.unwrap_or(SpriteType::Atlased);
        let atlas_id = match atlas_id {
            Some(s) => s.to_owned(),
            None => "default".to_owned(),
        };
        let c_atlas_id = CString::new(atlas_id)?;

        let path_buf = file_utf8.as_ref().to_path_buf();
        let c_str = CString::new(path_buf.clone().to_str().ok_or(StereoKitError::SpriteFile(path_buf.clone()))?)?;

        Ok(Sprite(
            NonNull::new(unsafe { sprite_create_file(c_str.as_ptr(), sprite_type, c_atlas_id.as_ptr()) })
                .ok_or(StereoKitError::SpriteFile(path_buf))?,
        ))
    }

    /// Finds a sprite that matches the given id! Check out the DefaultIds static class for some built-in ids. Sprites
    /// will auto-id themselves using this pattern if single sprites: {Tex.Id}/sprite, and this pattern if atlased
    /// sprites: {Tex.Id}/sprite/atlas/{atlasId}.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Find.html>
    /// * `id` - The id of the sprite to find.
    ///
    /// see also [`sprite_find`] [`Sprite::clone_ref`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::{Sprite, SpriteType}, tex::Tex,
    ///                      util::{Gradient, Color128, named_colors}};
    ///
    /// let mut gradient = Gradient::new(None);
    /// gradient
    ///     .add(Color128::BLACK_TRANSPARENT, 0.0)
    ///     .add(Color128::BLACK, 0.7);
    /// let tex_particule1 = Tex::gen_particle(128, 128, 0.2, Some(gradient));
    /// let mut sprite = Sprite::from_tex(&tex_particule1, None, None)
    ///                   .expect("Should be able to create sprite");
    /// assert!(sprite.get_id().starts_with( "auto/tex_"));
    ///
    /// sprite.id("My_sprite_ID");
    ///
    /// let same_sprite = Sprite::find("My_sprite_ID")
    ///                        .expect("Should be able to find sprite");
    ///
    /// assert_eq!(same_sprite, sprite);
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<Sprite, StereoKitError> {
        let cstr_id = CString::new(id.as_ref())?;
        Ok(Sprite(
            NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) })
                .ok_or(StereoKitError::SpriteFind(id.as_ref().to_string(), "sprite_find failed".to_string()))?,
        ))
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Find.html>
    ///
    /// see also [`sprite_find()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::{Sprite, SpriteType}, tex::Tex};
    ///
    /// let tex = Tex::rough();
    /// let mut sprite = Sprite::from_tex(&tex, None, None)
    ///                   .expect("Should be able to create sprite");
    /// assert_eq!(sprite.get_id(), "default/tex_rough/sprite");
    ///
    /// sprite.id("My_sprite_ID");
    ///
    /// let same_sprite = sprite.clone_ref();
    ///
    /// assert_eq!(same_sprite, sprite);
    /// ```
    pub fn clone_ref(&self) -> Sprite {
        Sprite(
            NonNull::new(unsafe { sprite_find(sprite_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"),
        )
    }

    /// Sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    ///<https://stereokit.net/Pages/StereoKit/Sprite/Id.html>
    ///
    /// see also [`sprite_get_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::{Sprite, SpriteType}, tex::Tex};
    ///
    /// let tex = Tex::rough();
    /// let mut sprite = Sprite::from_tex(&tex, None, None)
    ///                   .expect("Should be able to create sprite");
    /// assert_eq!(sprite.get_id(), "default/tex_rough/sprite");
    ///
    /// sprite.id("My_sprite_ID");
    ///
    /// assert_eq!(sprite.get_id(), "My_sprite_ID");
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr_id = CString::new(id.as_ref()).unwrap();
        unsafe { sprite_set_id(self.0.as_ptr(), cstr_id.as_ptr()) };
        self
    }

    /// Draws the sprite at the location specified by the transform matrix. A sprite is always sized in model space as 1 x Aspect
    /// meters on the x and y axes respectively, so scale appropriately. The ‘position’ attribute describes what corner of the sprite
    ///  you’re specifying the transform of.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Draw.html>
    /// * `token` - The token to ensure the sprite is drawn in the correct frame.
    /// * `transform` - A Matrix describing a transform from model space to world space. A sprite is always sized in
    ///   model space as 1 x Aspect meters on the x and y axes respectively, so scale appropriately and remember that
    ///   your anchor position may affect the transform as well.
    /// * `anchor_position` - Describes what corner of the sprite you’re specifying the transform of. The ‘Anchor’ point
    ///   or ‘Origin’ of the Sprite.
    /// * `linear_color` - Per-instance color data for this render item. It is unmodified by StereoKit, and is generally
    ///   interpreted as linear. If None has default value of WHITE.
    ///
    /// see also [`sprite_draw`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{ maths::Matrix, sprite::{Sprite, SpriteType}, tex::Tex, system::TextAlign,
    ///                      util::{Gradient, Color128, named_colors}};
    ///
    /// let sprite = Sprite::close();
    ///
    /// filename_scr = "screenshots/sprite_draw.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::TopLeft,     None);
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::TopRight,    Some(named_colors::BLUE.into()));
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::BottomLeft,  Some(named_colors::RED.into()));
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::BottomRight, Some(named_colors::GREEN.into()));
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_draw.jpeg" alt="screenshot" width="200">
    pub fn draw(
        &self,
        _token: &MainThreadToken,
        transform: impl Into<Matrix>,
        anchor_position: TextAlign,
        linear_color: Option<Color32>,
    ) {
        let color_linear = linear_color.unwrap_or(Color32::WHITE);
        unsafe { sprite_draw(self.0.as_ptr(), transform.into(), anchor_position, color_linear) };
    }

    /// The id of this sprite
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Id.html>
    ///
    /// see also [`sprite_get_id`]
    /// see example in [`Sprite::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(sprite_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// The aspect ratio of the sprite! This is width/height. You may also be interested in the NormalizedDimensions property,
    /// which are normalized to the 0-1 range.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Aspect.html>
    ///
    /// see also [`sprite_get_aspect`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::sprite::{Sprite, SpriteType};
    ///
    /// let sprite = Sprite::from_file("icons/log_viewer.png", Some(SpriteType::Single), Some("MY_ID"))
    ///                   .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_aspect(), 1.0);
    ///
    /// let sprite = Sprite::from_file("hdri/sky_dawn.hdr", None, None)
    ///                  .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_aspect(), 2.0);
    /// ```
    pub fn get_aspect(&self) -> f32 {
        unsafe { sprite_get_aspect(self.0.as_ptr()) }
    }

    /// Get the height in pixel
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Height.html>
    ///
    /// see also [`sprite_get_height`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::sprite::{Sprite, SpriteType};
    ///
    /// let sprite = Sprite::from_file("icons/log_viewer.png", Some(SpriteType::Single), Some("MY_ID"))
    ///                   .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_height(), 128);
    ///
    /// let sprite = Sprite::from_file("hdri/sky_dawn.hdr", None, None)
    ///                  .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_height(), 2048);
    /// ```
    pub fn get_height(&self) -> i32 {
        unsafe { sprite_get_height(self.0.as_ptr()) }
    }

    /// Get the width in pixel
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Width.html>
    ///
    /// see also [`sprite_get_width`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::sprite::{Sprite, SpriteType};
    ///
    /// let sprite = Sprite::from_file("icons/log_viewer.png", Some(SpriteType::Single), Some("MY_ID"))
    ///                   .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_width(), 128);
    ///
    /// let sprite = Sprite::from_file("hdri/sky_dawn.hdr", None, None)
    ///                  .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_width(), 4096);
    /// ```
    pub fn get_width(&self) -> i32 {
        unsafe { sprite_get_width(self.0.as_ptr()) }
    }

    /// Get the width and height of the sprite, normalized so the maximum value is 1.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/NormalizedDimensions.html>
    ///
    /// see also [`sprite_get_dimensions_normalized`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::{Sprite, SpriteType}, maths::Vec2};
    ///
    /// let sprite = Sprite::from_file("icons/log_viewer.png", Some(SpriteType::Single), Some("MY_ID"))
    ///                   .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_normalized_dimensions(), Vec2::ONE);
    ///
    /// let sprite = Sprite::from_file("hdri/sky_dawn.hdr", None, None)
    ///                  .expect("open_gltf.jpeg should be able to create sprite");
    /// assert_eq!(sprite.get_normalized_dimensions(), Vec2::new(1.0, 0.5));
    /// ```
    pub fn get_normalized_dimensions(&self) -> Vec2 {
        unsafe { sprite_get_dimensions_normalized(self.0.as_ptr()) }
    }

    /// This is a 64x64 image of a filled hole. This is common iconography for radio buttons which use an empty hole to
    /// indicate an un-selected radio, and a filled hole for a selected radio. This is used by the UI for radio buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/RadioOn.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::radio_on();
    /// assert_eq!(sprite.get_id(), "sk/ui/radio_on");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_radio_on.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_radio_on.jpeg" alt="screenshot" width="48">
    pub fn radio_on() -> Self {
        let cstr_id = CString::new("sk/ui/radio_on").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of an empty hole. This is common iconography for radio buttons which use an empty hole to
    /// indicate an un-selected radio, and a filled hole for a selected radio. This is used by the UI for radio buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/RadioOff.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::radio_off();
    /// assert_eq!(sprite.get_id(), "sk/ui/radio_off");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_radio_off.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_radio_off.jpeg" alt="screenshot" width="48">
    pub fn radio_off() -> Self {
        let cstr_id = CString::new("sk/ui/radio_off").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a filled rounded square. This is common iconography for checkboxes which use an
    /// empty square to indicate an un-selected checkbox, and a filled square for a selected checkbox. This is used
    /// by the UI for toggle buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ToggleOn.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::toggle_on();
    /// assert_eq!(sprite.get_id(), "sk/ui/toggle_on");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_toggle_on.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_toggle_on.jpeg" alt="screenshot" width="48">
    pub fn toggle_on() -> Self {
        let cstr_id = CString::new("sk/ui/toggle_on").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of an empty rounded square. This is common iconography for checkboxes which use an empty
    /// square to indicate an un-selected checkbox, and a filled square for a selected checkbox. This is used by the UI
    /// for toggle buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ToggleOff.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::toggle_off();
    /// assert_eq!(sprite.get_id(), "sk/ui/toggle_off");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_toggle_off.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_toggle_off.jpeg" alt="screenshot" width="48">
    pub fn toggle_off() -> Self {
        let cstr_id = CString::new("sk/ui/toggle_off").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing left.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowLeft.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::{Vec3, Matrix}, system::TextAlign};
    ///
    /// let sprite = Sprite::arrow_left();
    /// assert_eq!(sprite.get_id(), "sk/ui/arrow_left");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_arrow_left.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_left.jpeg" alt="screenshot" width="48">
    pub fn arrow_left() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_left").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing right.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowRight.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::{Vec3, Matrix}, system::TextAlign};
    ///
    /// let sprite = Sprite::arrow_right();
    /// assert_eq!(sprite.get_id(), "sk/ui/arrow_right");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_arrow_right.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_right.jpeg" alt="screenshot" width="48">
    pub fn arrow_right() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_right").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing up.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowUp.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::arrow_up();
    /// assert_eq!(sprite.get_id(), "sk/ui/arrow_up");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_arrow_up.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_up.jpeg" alt="screenshot" width="48">
    pub fn arrow_up() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_up").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing down.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowDown.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::arrow_down();
    /// assert_eq!(sprite.get_id(), "sk/ui/arrow_down");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_arrow_down.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_down.jpeg" alt="screenshot" width="48">
    pub fn arrow_down() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_down").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a backspace action button, similar to a backspace button you might find on a mobile
    /// keyboard.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Backspace.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::backspace();
    /// assert_eq!(sprite.get_id(), "sk/ui/backspace");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_backspace.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_backspace.jpeg" alt="screenshot" width="48">
    pub fn backspace() -> Self {
        let cstr_id = CString::new("sk/ui/backspace").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of an upward facing rounded arrow. This is a triangular top with a narrow rectangular
    /// base, and is used to indicate a ‘shift’ icon on a keyboard.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Shift.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::shift();
    /// assert_eq!(sprite.get_id(), "sk/ui/shift");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_shift.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_shift.jpeg" alt="screenshot" width="48">
    pub fn shift() -> Self {
        let cstr_id = CString::new("sk/ui/shift").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a square aspect X, with rounded edge. It’s used to indicate a ‘close’ icon.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Close.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::close();
    /// assert_eq!(sprite.get_id(), "sk/ui/close");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_close.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_close.jpeg" alt="screenshot" width="48">
    pub fn close() -> Self {
        let cstr_id = CString::new("sk/ui/close").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// <https://stereokit.net/Pages/StereoKit/Sprite/List.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::list();
    /// assert_eq!(sprite.get_id(), "sk/ui/list");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_list.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_list.jpeg" alt="screenshot" width="48">
    pub fn list() -> Self {
        let cstr_id = CString::new("sk/ui/list").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// <https://stereokit.net/Pages/StereoKit/Sprite/Grid.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!();
    /// use stereokit_rust::{sprite::Sprite, maths::Matrix, system::TextAlign};
    ///
    /// let sprite = Sprite::grid();
    /// assert_eq!(sprite.get_id(), "sk/ui/grid");
    ///
    /// width_scr = 48; height_scr = 48; fov_scr = 65.0;
    /// filename_scr = "screenshots/sprite_grid.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sprite.draw(token, Matrix::Y_180, TextAlign::Center, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_grid.jpeg" alt="screenshot" width="48">
    pub fn grid() -> Self {
        let cstr_id = CString::new("sk/ui/grid").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }
}
