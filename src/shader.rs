use crate::{StereoKitError, system::IAsset};
use std::{
    ffi::{CStr, CString, c_void},
    path::Path,
    ptr::NonNull,
};

/// fluent syntax for Shader
/// <https://stereokit.net/Pages/StereoKit/Shader.html>
#[repr(C)]
#[derive(Debug)]
pub struct Shader(pub NonNull<_ShaderT>);
impl Drop for Shader {
    fn drop(&mut self) {
        unsafe { shader_release(self.0.as_ptr()) };
    }
}
impl AsRef<Shader> for Shader {
    fn as_ref(&self) -> &Shader {
        self
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct _ShaderT {
    _unused: [u8; 0],
}
pub type ShaderT = *mut _ShaderT;
unsafe extern "C" {
    pub fn shader_find(id: *const ::std::os::raw::c_char) -> ShaderT;
    pub fn shader_create_file(filename_utf8: *const ::std::os::raw::c_char) -> ShaderT;
    pub fn shader_create_mem(data: *mut ::std::os::raw::c_void, data_size: usize) -> ShaderT;
    pub fn shader_set_id(shader: ShaderT, id: *const ::std::os::raw::c_char);
    pub fn shader_get_id(shader: ShaderT) -> *const ::std::os::raw::c_char;
    pub fn shader_get_name(shader: ShaderT) -> *const ::std::os::raw::c_char;
    pub fn shader_addref(shader: ShaderT);
    pub fn shader_release(shader: ShaderT);
}

impl IAsset for Shader {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }
}

/// This is a fast, general purpose shader. It uses a texture for ‘diffuse’, a ‘color’ property for tinting the
/// material, and a ‘tex_scale’ for scaling the UV coordinates. For lighting, it just uses a lookup from the current
/// cubemap.
/// <https://stereokit.net/Pages/StereoKit/Shader/Default.html>
impl Default for Shader {
    fn default() -> Self {
        Self::find("default/shader").unwrap()
    }
}
impl Shader {
    /// Loads an image file stored in memory directly into a texture! Supported formats are: jpg, png, tga, bmp, psd,
    /// gif, hdr, pic.
    /// Asset Id will be the same as the filename.
    /// <https://stereokit.net/Pages/StereoKit/Shader/FromMemory.html>
    ///
    /// see also [`crate::shader::shader_create_mem`]
    pub fn from_memory(data: &[u8]) -> Result<Shader, StereoKitError> {
        Ok(Shader(
            NonNull::new(unsafe { shader_create_mem(data.as_ptr() as *mut c_void, data.len()) })
                .ok_or(StereoKitError::ShaderMem)?,
        ))
    }

    /// Loads a shader from a precompiled StereoKit Shader (.sks) file! HLSL files can be compiled using the skshaderc
    /// tool called with `cargo compile_sks` or `cargo build_sk_rs`.
    /// <https://stereokit.net/Pages/StereoKit/Shader/FromFile.html>
    ///
    /// see also [`crate::shader::shader_create_file`]
    pub fn from_file(file_utf8: impl AsRef<Path>) -> Result<Shader, StereoKitError> {
        let path_buf = file_utf8.as_ref().to_path_buf();
        let c_str = CString::new(
            path_buf
                .clone()
                .to_str()
                .ok_or(StereoKitError::ShaderFile(path_buf.clone(), "CString conversion".to_string()))?,
        )?;
        Ok(Shader(
            NonNull::new(unsafe { shader_create_file(c_str.as_ptr()) })
                .ok_or(StereoKitError::ShaderFile(path_buf.clone(), "shader_create_file failed".to_string()))?,
        ))
    }

    /// Looks for a shader asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Shader/Find.html>
    ///
    /// see also [`crate::shader::shader_find`]
    pub fn find<S: AsRef<str>>(id: S) -> Result<Shader, StereoKitError> {
        let c_str = CString::new(id.as_ref())
            .map_err(|_| StereoKitError::ShaderFind(id.as_ref().into(), "CString conversion".to_string()))?;
        Ok(Shader(
            NonNull::new(unsafe { shader_find(c_str.as_ptr()) })
                .ok_or(StereoKitError::ShaderFind(id.as_ref().into(), "shader_find failed".to_string()))?,
        ))
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Shader/Find.html>
    ///
    /// see also [`crate::shader::shader_find()`]
    pub fn clone_ref(&self) -> Shader {
        Shader(
            NonNull::new(unsafe { shader_find(shader_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"),
        )
    }

    /// Gets or sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Shader/Id.html>
    ///
    /// see also [`crate::shader::shader_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { shader_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// The id of this shader
    /// <https://stereokit.net/Pages/StereoKit/Shader/Id.html>
    ///
    /// see also [`crate::shader::shader_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(shader_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// The name of the shader, provided in the shader file itself. Not the filename or id.
    /// <https://stereokit.net/Pages/StereoKit/Shader/Name.html>
    ///
    /// see also [`crate::shader::shader_get_name`]
    pub fn get_name(&self) -> &str {
        unsafe { CStr::from_ptr(shader_get_name(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Shader/Blit.html>
    pub fn blit() -> Self {
        Self::find("default/shader_blit").unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Shader/LightMap.html>
    pub fn light_map() -> Self {
        Self::find("default/shader_lightmap").unwrap()
    }

    /// Sometimes lighting just gets in the way! This is an extremely simple and fast shader that uses a ‘diffuse’
    /// texture and a ‘color’ tint property to draw a model without any lighting at all!
    /// <https://stereokit.net/Pages/StereoKit/Shader/Unlit.html>
    pub fn unlit() -> Self {
        Self::find("default/shader_unlit").unwrap()
    }

    /// Sometimes lighting just gets in the way! This is an extremely simple and fast shader that uses a ‘diffuse’
    /// texture and a ‘color’ tint property to
    /// draw a model without any lighting at all! This shader will also discard pixels with an alpha of zero.
    /// <https://stereokit.net/Pages/StereoKit/Shader/UnlitClip.html>
    pub fn unlit_clip() -> Self {
        Self::find("default/shader_unlit_clip").unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Shader.html>
    pub fn font() -> Self {
        Self::find("default/shader_font").unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Shader.html>
    pub fn equirect() -> Self {
        Self::find("default/shader_equirect").unwrap()
    }

    /// A shader for UI or interactable elements, this’ll be the same as the Shader, but with an additional finger
    /// ‘shadow’ and distance circle effect that helps indicate finger distance from the surface of the object.
    /// <https://stereokit.net/Pages/StereoKit/Shader/UI.html>
    pub fn ui() -> Self {
        Self::find("default/shader_ui").unwrap()
    }

    /// A shader for indicating interaction volumes! It renders a border around the edges of the UV coordinates that
    /// will ‘grow’ on proximity to the user’s finger. It will discard pixels outside of that border, but will also show
    /// the finger shadow. This is meant to be an opaque shader, so it works well for depth LSR. This shader works best
    /// on cube-like meshes where each face has UV coordinates from 0-1.
    /// Shader Parameters: color - color border_size - meters border_size_grow - meters border_affect_radius - meters
    /// <https://stereokit.net/Pages/StereoKit/Shader/UIBox.html>
    pub fn ui_box() -> Self {
        Self::find("default/shader_ui_box").unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Shader.html>
    pub fn ui_quadrant() -> Self {
        Self::find("default/shader_ui_quadrant").unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Shader.html>
    pub fn sky() -> Self {
        Self::find("default/shader_sky").unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Shader.html>
    pub fn line() -> Self {
        Self::find("default/shader_line").unwrap()
    }

    /// A physically based shader.
    /// <https://stereokit.net/Pages/StereoKit/Shader/PBR.html>
    pub fn pbr() -> Self {
        Self::find("default/shader_pbr").unwrap()
    }

    /// Same as ShaderPBR, but with a discard clip for transparency.
    /// <https://stereokit.net/Pages/StereoKit/Shader/PBRClip.html>
    pub fn pbr_clip() -> Self {
        Self::find("default/shader_pbr_clip").unwrap()
    }
}
