use crate::{
    maths::{Matrix, Vec2},
    sk::MainThreadToken,
    system::{IAsset, TextAlign},
    tex::{Tex, TexT},
    util::Color32,
    StereoKitError,
};
use std::{
    ffi::{c_char, CStr, CString},
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
/// see also [`stereokit::Sprite`]
#[repr(C)]
#[derive(Debug)]
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
#[repr(C)]
#[derive(Debug)]
pub struct _SpriteT {
    _unused: [u8; 0],
}
pub type SpriteT = *mut _SpriteT;
extern "C" {
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
    /// * type_ - If None has default of Atlased
    /// * atlas_id - If None has default of "default"
    ///
    /// see also [`crate::sprite::sprite_create`]
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
    /// * type_ - If None has default of Atlased
    /// * atlas_id - If None has default of "default"
    ///
    /// see also [`crate::sprite::sprite_create_file`]
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
    ///
    /// see also [`crate::sprite::sprite_find`]
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
    /// see also [`crate::sprite::sprite_find()`]
    pub fn clone_ref(&self) -> Sprite {
        Sprite(
            NonNull::new(unsafe { sprite_find(sprite_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"),
        )
    }

    /// Sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    ///<https://stereokit.net/Pages/StereoKit/Sprite/Id.html>
    ///
    /// see also [`crate::sprite::sprite_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr_id = CString::new(id.as_ref()).unwrap();
        unsafe { sprite_set_id(self.0.as_ptr(), cstr_id.as_ptr()) };
        self
    }

    /// Draws the sprite at the location specified by the transform matrix. A sprite is always sized in model space as 1 x Aspect
    /// meters on the x and y axes respectively, so scale appropriately. The ‘position’ attribute describes what corner of the sprite
    ///  you’re specifying the transform of.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Draw.html>
    /// * color_linear - if None has default value of WHITE
    /// * text_align - indicate how
    ///
    /// see also [`stereokit::StereoKitDraw::sprite_draw`]
    pub fn draw(
        &self,
        _token: &MainThreadToken,
        transform: impl Into<Matrix>,
        anchor_position: TextAlign,
        color_linear: Option<Color32>,
    ) {
        let color_linear = color_linear.unwrap_or(Color32::WHITE);
        unsafe { sprite_draw(self.0.as_ptr(), transform.into(), anchor_position, color_linear) };
    }

    /// The id of this sprite
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Id.html>
    ///
    /// see also [`crate::sprite::sprite_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(sprite_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// The aspect ratio of the sprite! This is width/height. You may also be interested in the NormalizedDimensions property,
    /// which are normalized to the 0-1 range.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Aspect.html>
    ///
    /// see also [`crate::sprite::sprite_get_aspect`]
    pub fn get_aspect(&self) -> f32 {
        unsafe { sprite_get_aspect(self.0.as_ptr()) }
    }

    /// Get the height in pixel
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Height.html>
    ///
    /// see also [`crate::sprite::sprite_get_height`]
    pub fn get_height(&self) -> i32 {
        unsafe { sprite_get_height(self.0.as_ptr()) }
    }

    /// Get the width in pixel
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Width.html>
    ///
    /// see also [`crate::sprite::sprite_get_width`]
    pub fn get_width(&self) -> i32 {
        unsafe { sprite_get_width(self.0.as_ptr()) }
    }

    /// Get the width and height of the sprite, normalized so the maximum value is 1.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/NormalizedDimensions.html>
    ///
    /// see also [`crate::sprite::sprite_get_dimensions_normalized`]
    pub fn get_normalized_dimensions(&self) -> Vec2 {
        unsafe { sprite_get_dimensions_normalized(self.0.as_ptr()) }
    }

    /// This is a 64x64 image of a filled hole. This is common iconography for radio buttons which use an empty hole to
    /// indicate an un-selected radio, and a filled hole for a selected radio. This is used by the UI for radio buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/RadioOn.html>
    pub fn radio_on() -> Self {
        let cstr_id = CString::new("sk/ui/radio_on").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of an empty hole. This is common iconography for radio buttons which use an empty hole to
    /// indicate an un-selected radio, and a filled hole for a selected radio. This is used by the UI for radio buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/RadioOff.html>
    pub fn radio_off() -> Self {
        let cstr_id = CString::new("sk/ui/radio_off").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a filled rounded square. This is common iconography for checkboxes which use an
    /// empty square to indicate an un-selected checkbox, and a filled square for a selected checkbox. This is used
    /// by the UI for toggle buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ToggleOn.html>
    pub fn toggle_on() -> Self {
        let cstr_id = CString::new("sk/ui/toggle_on").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of an empty rounded square. This is common iconography for checkboxes which use an empty
    /// square to indicate an un-selected checkbox, and a filled square for a selected checkbox. This is used by the UI
    /// for toggle buttons!
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ToggleOff.html>
    pub fn toggle_off() -> Self {
        let cstr_id = CString::new("sk/ui/toggle_off").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing left.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowLeft.html>
    pub fn arrow_left() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_left").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing right.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowRight.html>
    pub fn arrow_right() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_right").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing up.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowUp.html>
    pub fn arrow_up() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_up").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a slightly rounded triangle pointing down.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/ArrowDown.html>
    pub fn arrow_down() -> Self {
        let cstr_id = CString::new("sk/ui/arrow_down").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a backspace action button, similar to a backspace button you might find on a mobile
    /// keyboard.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Backspace.html>
    pub fn backspace() -> Self {
        let cstr_id = CString::new("sk/ui/backspace").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of an upward facing rounded arrow. This is a triangular top with a narrow rectangular
    /// base, and is used to indicate a ‘shift’ icon on a keyboard.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Shift.html>
    pub fn shift() -> Self {
        let cstr_id = CString::new("sk/ui/shift").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// This is a 64x64 image of a square aspect X, with rounded edge. It’s used to indicate a ‘close’ icon.
    /// <https://stereokit.net/Pages/StereoKit/Sprite/Close.html>
    pub fn close() -> Self {
        let cstr_id = CString::new("sk/ui/close").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// <https://stereokit.net/Pages/StereoKit/Sprite/List.html>
    pub fn list() -> Self {
        let cstr_id = CString::new("sk/ui/list").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }

    /// <https://stereokit.net/Pages/StereoKit/Sprite/Grid.html>
    pub fn grid() -> Self {
        let cstr_id = CString::new("sk/ui/grid").unwrap();
        Sprite(NonNull::new(unsafe { sprite_find(cstr_id.as_ptr()) }).unwrap())
    }
}
