use crate::{system::IAsset, tex::TexT, StereoKitError};
use std::{
    ffi::{c_char, CStr, CString},
    path::Path,
    ptr::NonNull,
};

/// This class represents a text font asset! On the back-end, this asset is composed of a texture with font characters
/// rendered to it, and a list of data about where, and how large those characters are on the texture.
///
/// This asset is used anywhere that text shows up, like in the UI or Text classes!
/// <https://stereokit.net/Pages/StereoKit/Font.html>
///
/// # Examples
///
/// ```
/// stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
///
/// use stereokit_rust::{ui::Ui, maths::{Vec3, Quat, Pose, Matrix}, font::Font, system::Text, util::named_colors::{RED, GREEN} };
///
/// // Load a font asset
/// let mut emoji_font = Font::from_file("fonts/Noto_Emoji/NotoEmoji-VariableFont_wght.ttf").unwrap_or_default();
/// let emoji_style = Some(Text::make_style(emoji_font, 0.35, RED));
/// let mut text_font =  Font::from_file("fonts/Inter/Inter-VariableFont_opsz,wght.ttf").unwrap_or_default();
/// let text_style = Text::make_style(text_font, 0.15, GREEN);
/// let mut window_pose = Pose::new(Vec3::new(0.00, 0.0, 0.85), Some(Quat::from_angles(0.0, 130.0, 0.0)));
///
/// filename_scr = "screenshots/font.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     Text::add_at(token, "😋 Emojis🤪\n\n  🧐", Matrix::IDENTITY, emoji_style, None, None, None, None, None, None);
///
///     Ui::window_begin("Default Font", &mut window_pose, None, None, None);
///     Ui::push_text_style(text_style);
///     Ui::text("text font", None, None, None, Some(0.90), None, None);
///     Ui::pop_text_style();
///     Ui::window_end();
/// );
/// ```
///
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/font.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug)]
pub struct Font(pub NonNull<_FontT>);
impl Drop for Font {
    fn drop(&mut self) {
        unsafe { font_release(self.0.as_ptr()) };
    }
}
impl AsRef<Font> for Font {
    fn as_ref(&self) -> &Font {
        self
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct _FontT {
    _unused: [u8; 0],
}
pub type FontT = *mut _FontT;

extern "C" {
    pub fn font_find(id: *const c_char) -> FontT;
    pub fn font_create(file_utf8: *const c_char) -> FontT;
    pub fn font_create_files(in_arr_files: *mut *const c_char, file_count: i32) -> FontT;
    pub fn font_create_family(font_family: *const c_char) -> FontT;
    pub fn font_set_id(font: FontT, id: *const c_char);
    pub fn font_get_id(font: FontT) -> *const c_char;
    pub fn font_addref(font: FontT);
    pub fn font_release(font: FontT);
    pub fn font_get_tex(font: FontT) -> TexT;
}

impl IAsset for Font {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }
}

impl Default for Font {
    /// The default font used by StereoKit’s text. This varies from platform to platform, but is typically a sans-serif
    /// general purpose font, such as Segoe UI.
    /// <https://stereokit.net/Pages/StereoKit/Font/Default.html>
    ///
    /// see also [`crate::font::font_find`]
    fn default() -> Self {
        let c_str = CString::new("default/font").unwrap();
        Font(NonNull::new(unsafe { font_find(c_str.as_ptr()) }).unwrap())
    }
}

impl Font {
    /// Loads a font and creates a font asset from it.
    /// <https://stereokit.net/Pages/StereoKit/Font/FromFile.html>
    ///
    /// see also [`crate::font::font_create_file`]
    pub fn from_file(file_utf8: impl AsRef<Path>) -> Result<Font, StereoKitError> {
        let path_buf = file_utf8.as_ref().to_path_buf();
        let c_str = CString::new(
            path_buf
                .clone()
                .to_str()
                .ok_or(StereoKitError::FontFile(path_buf.clone(), "CString conversion".to_string()))?,
        )?;
        Ok(Font(
            NonNull::new(unsafe { font_create(c_str.as_ptr()) })
                .ok_or(StereoKitError::FontFile(path_buf, "font_create failed".to_string()))?,
        ))
    }

    /// Loads a font and creates a font asset from it.
    /// If a glyph is not found, StereoKit will look in the next font file in the list.
    /// <https://stereokit.net/Pages/StereoKit/Font/FromFile.html>
    ///
    /// see also [`crate::font::font_create_file`]
    pub fn from_files<P: AsRef<Path>>(files_utf8: &[P]) -> Result<Font, StereoKitError> {
        let mut c_files = Vec::new();
        for path in files_utf8 {
            let path = path.as_ref();
            let c_str = CString::new(path.to_str().ok_or(StereoKitError::FontFiles(
                path.to_str().unwrap().to_string(),
                "CString conversion".to_string(),
            ))?)?;
            c_files.push(c_str);
        }
        let mut c_files_ptr = Vec::new();
        for str in c_files.iter() {
            c_files_ptr.push(str.as_ptr());
        }
        let in_arr_files_cstr = c_files_ptr.as_mut_slice().as_mut_ptr();

        Ok(Font(NonNull::new(unsafe { font_create_files(in_arr_files_cstr, files_utf8.len() as i32) }).ok_or(
            StereoKitError::FontFiles("many files".to_owned(), "font_create_files failed".to_string()),
        )?))
    }
    /// Loads font from a specified list of font family names.
    /// Returns a font from the given font family names, Most of the OS provide fallback fonts, hence there will always
    /// be a set of fonts.
    /// <https://stereokit.net/Pages/StereoKit/Font/FromFamily.html>
    /// * font_family - List of font family names separated by comma(,) similar to a list of names css allows.
    ///
    /// see also [`crate::font::font_create_family`]
    pub fn from_family(font_family: impl AsRef<str>) -> Result<Font, StereoKitError> {
        let c_str = CString::new(font_family.as_ref()).map_err(|_| {
            StereoKitError::FontFamily(font_family.as_ref().into(), "CString conversion error".to_string())
        })?;
        Ok(Font(NonNull::new(unsafe { font_create_family(c_str.as_ptr()) }).ok_or(
            StereoKitError::FontFamily(font_family.as_ref().into(), "font_create_family failed".to_string()),
        )?))
    }

    /// Searches the asset list for a font with the given Id.
    /// <https://stereokit.net/Pages/StereoKit/Font/Find.html>
    ///
    /// see also [`crate::font::font_find`]
    pub fn find<S: AsRef<str>>(id: S) -> Result<Font, StereoKitError> {
        let c_str = CString::new(id.as_ref())
            .map_err(|_| StereoKitError::FontFind(id.as_ref().into(), "CString conversion error".to_string()))?;
        Ok(Font(
            NonNull::new(unsafe { font_find(c_str.as_ptr()) })
                .ok_or(StereoKitError::FontFind(id.as_ref().into(), "font_find failed".to_string()))?,
        ))
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Font/Find.html>
    ///
    /// see also [`crate::font::font_find()`]
    pub fn clone_ref(&self) -> Font {
        Font(NonNull::new(unsafe { font_find(font_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    /// Gets or sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Font/Id.html>
    ///
    /// see also [`crate::font::font_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { font_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// The id of this font
    /// <https://stereokit.net/Pages/StereoKit/Font/Id.html>
    ///
    /// see also [`crate::font::font_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(font_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }
}
