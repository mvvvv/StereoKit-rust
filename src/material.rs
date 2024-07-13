use std::ffi::{c_char, c_void, CStr, CString};
use std::marker::PhantomData;
use std::path::Path;
use std::ptr::NonNull;

use crate::maths::{Bool32T, Matrix, Vec2, Vec3, Vec4};
use crate::shader::{Shader, ShaderT};
use crate::system::{IAsset, Log};
use crate::tex::{Tex, TexT};
use crate::util::Color128;
use crate::StereoKitError;

/// Also known as ‘alpha’ for those in the know. But there’s actually more than one type of transparency in rendering!
/// The horrors. We’re keepin’ it fairly simple for now, so you get three options!
/// <https://stereokit.net/Pages/StereoKit/Transparency.html>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum Transparency {
    /// Not actually transparent! This is opaque! Solid! It’s the default option, and it’s the fastest option! Opaque
    /// objects write to the z-buffer, the occlude pixels behind them, and they can be used as input to important
    /// Mixed Reality features like Late Stage Reprojection that’ll make your view more stable!
    None = 1,
    /// This will blend with the pixels behind it. This is transparent! You may not want to write to the z-buffer, and
    /// it’s slower than opaque materials.
    Blend = 3,
    /// This will straight up add the pixel color to the color buffer! This usually looks -really- glowy, so it makes
    /// for good particles or lighting effects.
    Add = 4,
}

/// Depth test describes how this material looks at and responds to depth information in the zbuffer! The default is
/// Less, which means if the material pixel’s depth is Less than the existing depth data, (basically, is this in front
/// of some other object) it will draw that pixel. Similarly, Greater would only draw  the material if it’s ‘behind’
/// the depth buffer. Always would just draw all the time, and not read from the depth buffer at all.
/// <https://stereokit.net/Pages/StereoKit/DepthTest.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DepthTest {
    /// Default behavior, pixels behind the depth buffer will be discarded, and pixels in front of it will be drawn.
    Less = 0,
    /// Pixels behind the depth buffer will be discarded, and pixels in front of, or at the depth buffer’s value it
    /// will be drawn. This could be great for things that might be sitting exactly on a floor or wall.
    LessOrEq = 1,
    /// Pixels in front of the zbuffer will be discarded! This is opposite of how things normally work. Great for
    /// drawing indicators that something is occluded by a wall or other geometry.
    Greater = 2,
    /// Pixels in front of (or exactly at) the zbuffer will be discarded! This is opposite of how things normally
    /// work. Great for drawing indicators that something is occluded by a wall or other geometry.
    GreaterOrEq = 3,
    /// Only draw pixels if they’re at exactly the same depth as the zbuffer!
    Equal = 4,
    /// Draw any pixel that’s not exactly at the value in the zbuffer.
    NotEqual = 5,
    /// Don’t look at the zbuffer at all, just draw everything, always, all the time! At this point, the order at
    /// which the mesh gets drawn will be super important, so don’t forget about Material.QueueOffset!
    Always = 6,
    /// Never draw a pixel, regardless of what’s in the zbuffer. I can think of better ways to do this, but uhh,
    /// this is here for completeness! Maybe you can find a use for it.
    Never = 7,
}

/// Culling is discarding an object from the render pipeline! This enum describes how mesh faces get discarded on the
/// graphics card. With culling set to none, you can double the number of pixels the GPU ends up drawing, which can
/// have a big impact on performance. None can be appropriate in cases where the mesh is designed to be ‘double sided’.
/// Front can also be helpful when you want to flip a mesh ‘inside-out’!
/// <https://stereokit.net/Pages/StereoKit/Cull.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Cull {
    /// Discard if the back of the triangle face is pointing towards the camera. This is the default behavior.
    Back = 0,
    /// Discard if the front of the triangle face is pointing towards the camera. This is opposite the default behavior.
    Front = 1,
    /// No culling at all! Draw the triangle regardless of which way it’s pointing.
    None = 2,
}

/// A Material describes the surface of anything drawn on the graphics card! It is typically composed of a Shader, and
/// shader properties like colors, textures, transparency info, etc.
///
/// Items drawn with the same Material can be batched together into a single, fast operation on the graphics card, so
/// re-using materials can be extremely beneficial for performance!
/// <https://stereokit.net/Pages/StereoKit/Material.html>
#[derive(Debug)]
pub struct Material(pub NonNull<_MaterialT>);
impl Drop for Material {
    fn drop(&mut self) {
        unsafe { material_release(self.0.as_ptr()) }
    }
}
impl AsRef<Material> for Material {
    fn as_ref(&self) -> &Material {
        self
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct _MaterialT {
    _unused: [u8; 0],
}
pub type MaterialT = *mut _MaterialT;

extern "C" {
    pub fn material_find(id: *const c_char) -> MaterialT;
    pub fn material_create(shader: ShaderT) -> MaterialT;
    pub fn material_copy(material: MaterialT) -> MaterialT;
    pub fn material_copy_id(id: *const c_char) -> MaterialT;
    pub fn material_set_id(material: MaterialT, id: *const c_char);
    pub fn material_get_id(material: MaterialT) -> *const c_char;
    pub fn material_addref(material: MaterialT);
    pub fn material_release(material: MaterialT);
    pub fn material_set_transparency(material: MaterialT, mode: Transparency);
    pub fn material_set_cull(material: MaterialT, mode: Cull);
    pub fn material_set_wireframe(material: MaterialT, wireframe: Bool32T);
    pub fn material_set_depth_test(material: MaterialT, depth_test_mode: DepthTest);
    pub fn material_set_depth_write(material: MaterialT, write_enabled: Bool32T);
    pub fn material_set_queue_offset(material: MaterialT, offset: i32);
    pub fn material_set_chain(material: MaterialT, chain_material: MaterialT);
    pub fn material_get_transparency(material: MaterialT) -> Transparency;
    pub fn material_get_cull(material: MaterialT) -> Cull;
    pub fn material_get_wireframe(material: MaterialT) -> Bool32T;
    pub fn material_get_depth_test(material: MaterialT) -> DepthTest;
    pub fn material_get_depth_write(material: MaterialT) -> Bool32T;
    pub fn material_get_queue_offset(material: MaterialT) -> i32;
    pub fn material_get_chain(material: MaterialT) -> MaterialT;
    pub fn material_set_shader(material: MaterialT, shader: ShaderT);
    pub fn material_get_shader(material: MaterialT) -> ShaderT;
}

impl IAsset for Material {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }
}

impl Default for Material {
    /// The default material! This is used by many models and meshes rendered from within StereoKit. Its shader is
    /// tuned for high performance, and may change based on system performance characteristics, so it can be great
    /// to copy this one when creating your own materials! Or if you want to override StereoKit’s default material,
    /// here’s where you do it!
    /// <https://stereokit.net/Pages/StereoKit/Material/Default.html>
    ///
    /// see also [`crate::font::font_find`]
    fn default() -> Self {
        let c_str = CString::new("default/material").unwrap();
        Material(NonNull::new(unsafe { material_find(c_str.as_ptr()) }).unwrap())
    }
}

impl Material {
    /// Creates a material from a shader, and uses the shader’s default settings.
    /// <https://stereokit.net/Pages/StereoKit/Material/Material.html>
    /// * id - If None the id will be set to a default value "auto/asset_???"
    ///
    /// see also [`crate::material::material_create`][`crate::material::material_set_id`]
    pub fn new(shader: impl AsRef<Shader>, id: Option<&str>) -> Material {
        let mut mat = Material(NonNull::new(unsafe { material_create(shader.as_ref().0.as_ptr()) }).unwrap());
        if let Some(id) = id {
            mat.id(id);
        }
        mat
    }

    /// Loads a Shader asset and creates a Material using it. If the shader fails to load, a warning will be added to the log,
    /// and this Material will default to using an Unlit shader.
    /// <https://stereokit.net/Pages/StereoKit/Material/Material.html>
    /// * id - If None the id will be set to a default value "auto/asset_???"
    ///
    /// see also [`crate::material::material_create`][`crate::material::material_set_id`]
    pub fn from_file(shader_file_name: impl AsRef<Path>, id: Option<&str>) -> Result<Material, StereoKitError> {
        let shader = Shader::from_file(&shader_file_name);
        match shader {
            Ok(shader) => {
                let mut mat = Material(NonNull::new(unsafe { material_create(shader.as_ref().0.as_ptr()) }).unwrap());
                if let Some(id) = id {
                    mat.id(id);
                }
                Ok(mat)
            }
            Err(err) => Err(StereoKitError::ShaderFile(shader_file_name.as_ref().to_path_buf(), err.to_string())),
        }
    }

    /// Creates a new Material asset with the default Material and its properties!
    /// <https://stereokit.net/Pages/StereoKit/Material/Copy.html>
    ///
    /// see also [`crate::material::material_copy`]
    pub fn default_copy() -> Material {
        Material::default().copy()
    }

    /// Creates a new Material asset with the same shader and properties! Draw calls with the new Material will not
    /// batch together with this one.
    /// <https://stereokit.net/Pages/StereoKit/Material/Copy.html>
    ///
    /// see also [`crate::material::material_copy()`]
    pub fn copy(&self) -> Material {
        Material(NonNull::new(unsafe { material_copy(self.0.as_ptr()) }).unwrap())
    }

    /// Creates a new Material asset with the same shader and properties! Draw calls with the new Material will not
    /// batch together with this one.
    /// <https://stereokit.net/Pages/StereoKit/Material/Copy.html>
    ///
    /// see also [`crate::material::material_copy_id`]
    pub fn copy_id<S: AsRef<str>>(id: S) -> Result<Material, StereoKitError> {
        let c_str = CString::new(id.as_ref())?;
        match NonNull::new(unsafe { material_copy_id(c_str.as_ptr()) }) {
            Some(pt) => Ok(Material(pt)),
            None => Err(StereoKitError::MaterialFind(id.as_ref().to_owned(), "copy_id".to_owned())),
        }
    }

    /// Looks for a Material asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Material/Find.html>
    ///
    /// see also [`crate::material::material_find`]
    pub fn find<S: AsRef<str>>(id: S) -> Result<Material, StereoKitError> {
        let c_str = CString::new(id.as_ref())?;
        let material = NonNull::new(unsafe { material_find(c_str.as_ptr()) });
        match material {
            Some(material) => Ok(Material(material)),
            None => Err(StereoKitError::MaterialFind(id.as_ref().to_owned(), "not found".to_owned())),
        }
    }

    /// Overrides the Shader this material uses.
    /// <https://stereokit.net/Pages/StereoKit/Material/Shader.html>
    ///
    /// see also [`crate::material::material_set_shader`]
    pub fn shader(&mut self, shader: impl AsRef<Shader>) -> &mut Self {
        unsafe { material_set_shader(self.0.as_ptr(), shader.as_ref().0.as_ptr()) };
        self
    }

    /// In clip shaders, this is the cutoff value below which pixels are discarded.
    /// Typically, the diffuse/albedo’s alpha component is sampled for comparison here. This represents the float param ‘cutoff’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn clip_cutoff(&mut self, cutoff: f32) -> &mut Self {
        let ptr: *const f32 = &cutoff;
        unsafe {
            let name = CString::new("cutoff").unwrap();
            material_set_param(self.0.as_ptr(), name.as_ptr(), MaterialParam::Float, ptr as *const c_void);
        }
        self
    }
    /// A per-material color tint, behavior could vary from shader to shader, but often this is just multiplied against
    /// the diffuse texture right at the start. This represents the Color param ‘color’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn color_tint(&mut self, color: impl Into<Color128>) -> &mut Self {
        let ptr: *const Color128 = &color.into();
        unsafe {
            let cstr = &CString::new("color").unwrap();
            material_set_param(self.0.as_ptr(), cstr.as_ptr(), MaterialParam::Color128, ptr as *const c_void);
        }
        self
    }

    /// The primary color texture for the shader! Diffuse, Albedo, ‘The Texture’, or whatever you want to call it, this
    /// is usually the base color that the shader works with. This represents the texture param ‘diffuse’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn diffuse_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            let cstr = &CString::new("diffuse").unwrap();
            material_set_param(
                self.0.as_ptr(),
                cstr.as_ptr(),
                MaterialParam::Texture,
                texture.as_ref().0.as_ptr() as *const c_void,
            );
        }
        self
    }

    /// A multiplier for emission values sampled from the emission texture. The default emission texture in SK shaders is
    /// white, and the default value for this parameter is 0,0,0,0. This represents the Color param ‘emission_factor’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn emission_factor(&mut self, color: Color128) -> &mut Self {
        let ptr: *const Color128 = &color;
        unsafe {
            let cstr = &CString::new("emission_factor").unwrap();
            material_set_param(self.0.as_ptr(), cstr.as_ptr(), MaterialParam::Color128, ptr as *const c_void);
        }
        self
    }

    /// This texture is unaffected by lighting, and is frequently just added in on top of the material’s final color!
    /// Tends to look really glowy. This represents the texture param ‘emission’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn emission_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            let cstr = &CString::new("emission").unwrap();
            material_set_param(
                self.0.as_ptr(),
                cstr.as_ptr(),
                MaterialParam::Texture,
                texture.as_ref().0.as_ptr() as *const c_void,
            );
        }
        self
    }

    /// For physically based shader, this is a multiplier to scale the metallic properties of the material.
    /// This represents the float param ‘metallic’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn metallic_amount(&mut self, amount: f32) -> &mut Self {
        let ptr: *const f32 = &amount;
        unsafe {
            let cstr = &CString::new("metallic").unwrap();
            material_set_param(self.0.as_ptr(), cstr.as_ptr(), MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// For physically based shaders, metal is a texture that encodes metallic and roughness data into the ‘B’ and ‘G’ channels,
    /// respectively. This represents the texture param ‘metal’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn metal_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            let cstr = &CString::new("metal").unwrap();
            material_set_param(
                self.0.as_ptr(),
                cstr.as_ptr(),
                MaterialParam::Texture,
                texture.as_ref().0.as_ptr() as *const c_void,
            );
        }
        self
    }

    /// The ‘normal map’ texture for the material! This texture contains information about the direction of the material’s surface,
    /// which is used to calculate lighting, and make surfaces look like they have more detail than they actually do. Normals are in
    /// Tangent Coordinate Space, and the RGB values map to XYZ values. This represents the texture param ‘normal’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn normal_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            let cstr = &CString::new("normal").unwrap();
            material_set_param(
                self.0.as_ptr(),
                cstr.as_ptr(),
                MaterialParam::Texture,
                texture.as_ref().0.as_ptr() as *const c_void,
            );
        }
        self
    }

    /// Used by physically based shaders, this can be used for baked ambient occlusion lighting, or to remove specular reflections from
    /// areas that are surrounded by geometry that would likely block reflections. This represents the texture param ‘occlusion’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn occlusion_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            let cstr = &CString::new("occlusion").unwrap();
            material_set_param(
                self.0.as_ptr(),
                cstr.as_ptr(),
                MaterialParam::Texture,
                texture.as_ref().0.as_ptr() as *const c_void,
            );
        }
        self
    }

    /// For physically based shader, this is a multiplier to scale the roughness properties of the material.
    /// This represents the float param ‘roughness’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn roughness_amount(&mut self, amount: f32) -> &mut Self {
        let ptr: *const f32 = &amount;
        unsafe {
            let cstr = &CString::new("roughness").unwrap();
            material_set_param(self.0.as_ptr(), cstr.as_ptr(), MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// Not necessarily present in all shaders, this multiplies the UV coordinates of the mesh, so that the texture will repeat.
    /// This is great for tiling textures! This represents the float param ‘tex_scale’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    #[deprecated(since = "0.40.0", note = "please use `tex_transform` instead")]
    pub fn tex_scale(&mut self, scale: f32) -> &mut Self {
        let ptr: *const f32 = &scale;
        unsafe {
            let cstr = &CString::new("tex_scale").unwrap();
            material_set_param(self.0.as_ptr(), cstr.as_ptr(), MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// Non canonical shader parameter to indicate a time multiplier if the shader have one (water, blinker ...)
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn time(&mut self, time: f32) -> &mut Self {
        let ptr: *const f32 = &time;
        unsafe {
            let cstr = &CString::new("time").unwrap();
            material_set_param(self.0.as_ptr(), cstr.as_ptr(), MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// Not necessarily present in all shaders, this transforms the UV coordinates of the mesh, so that the texture can
    /// repeat and scroll. XY components are offset, and ZW components are scale.
    ///  
    /// This represents the float param 'tex_trans'.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`crate::material::material_set_param`]
    pub fn tex_transform(&mut self, transform: impl Into<Vec4>) -> &mut Self {
        let ptr: *const Vec4 = &transform.into();
        unsafe {
            let cstr = &CString::new("tex_trans").unwrap();
            material_set_param(self.0.as_ptr(), cstr.as_ptr(), MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    //--- Others parameters
    /// What type of transparency does this Material use? Default is None. Transparency has an impact on performance, and draw order.
    /// Check the [stereokit::Transparency] enum for details.
    /// <https://stereokit.net/Pages/StereoKit/Material/Transparency.html>
    ///
    /// see also [`crate::material::material_set_transparency`]
    pub fn transparency(&mut self, mode: Transparency) -> &mut Self {
        unsafe { material_set_transparency(self.0.as_ptr(), mode) };
        self
    }

    /// How should this material cull faces?
    /// <https://stereokit.net/Pages/StereoKit/Material/FaceCull.html>
    ///
    /// see also [`crate::material::material_set_cull`]
    pub fn face_cull(&mut self, mode: Cull) -> &mut Self {
        unsafe { material_set_cull(self.0.as_ptr(), mode) };
        self
    }

    /// Should this material draw only the edges/wires of the mesh? This can be useful for debugging, and even some kinds of visualization work.
    /// Note that this may not work on some mobile OpenGL systems like Quest.
    /// <https://stereokit.net/Pages/StereoKit/Material/Wireframe.html>
    ///
    /// see also [`crate::material::material_set_wireframe`]
    pub fn wireframe(&mut self, wireframe: bool) -> &mut Self {
        unsafe { material_set_wireframe(self.0.as_ptr(), wireframe as Bool32T) };
        self
    }

    /// How does this material interact with the ZBuffer? Generally DepthTest.Less would be normal behavior: don’t draw objects that are
    /// occluded. But this can also be used to achieve some interesting effects, like you could use DepthTest.Greater to draw a glow that
    /// indicates an object is behind something.
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthTest.html>
    ///
    /// see also [`crate::material::material_set_depth_test`]
    pub fn depth_test(&mut self, depth_test_mode: DepthTest) -> &mut Self {
        unsafe { material_set_depth_test(self.0.as_ptr(), depth_test_mode) };
        self
    }

    /// Should this material write to the ZBuffer? For opaque objects, this generally should be true. But transparent objects writing to the
    /// ZBuffer can be problematic and cause draw order issues. Note that turning this off can mean that this material won’t get properly
    /// accounted for when the MR system is performing late stage reprojection. Not writing to the buffer can also be faster! :)
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthWrite.html>
    ///
    /// see also [`crate::material::material_set_depth_write`]
    pub fn depth_write(&mut self, write_enabled: bool) -> &mut Self {
        unsafe { material_set_depth_write(self.0.as_ptr(), write_enabled as Bool32T) };
        self
    }

    /// This property will force this material to draw earlier or later in the draw queue. Positive values make it draw later, negative makes
    ///  it earlier. This can be helpful for tweaking performance! If you know an object is always going to be close to the user and likely
    ///  to obscure lots of objects (like hands), drawing it earlier can mean objects behind it get discarded much faster! Similarly, objects
    ///  that are far away (skybox!) can be pushed towards the back of the queue, so they’re more likely to be discarded early.
    /// <https://stereokit.net/Pages/StereoKit/Material/QueueOffset.html>
    ///
    /// see also [`crate::material::material_set_queue_offset`]
    pub fn queue_offset(&mut self, offset: i32) -> &mut Self {
        unsafe { material_set_queue_offset(self.0.as_ptr(), offset) };
        self
    }

    /// Allows you to chain Materials together in a form of multi-pass rendering! Any time the Material is used, the chained Materials will
    /// also be used to draw the same item.
    /// <https://stereokit.net/Pages/StereoKit/Material/Chain.html>
    ///
    /// see also [`crate::material::material_set_chain`]
    pub fn chain(&mut self, chained_material: &Material) -> &mut Self {
        unsafe { material_set_chain(self.0.as_ptr(), chained_material.0.as_ptr()) };
        self
    }

    /// Set a new id to the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Id.html>
    ///
    /// see also [`crate::material::material_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { material_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Get the id of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Id.html>
    ///
    /// see also [`crate::material::material_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(material_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// Get the [`MaterialParam::shader`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Shader.html>
    ///
    /// see also [`crate::material::material_get_shader`]
    pub fn get_shader(&self) -> Shader {
        unsafe { Shader(NonNull::new(material_get_shader(self.0.as_ptr())).unwrap()) }
    }

    /// Get the [`MaterialParam::transparency`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Transparency.html>
    ///
    /// see also [`crate::material::material_get_transparency`]
    pub fn get_transparency(&self) -> Transparency {
        unsafe { material_get_transparency(self.0.as_ptr()) }
    }

    /// Get the [`MaterialParam::face_cull`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/FaceCull.html>
    ///
    /// see also [`crate::material::material_get_cull`]
    pub fn get_face_cull(&self) -> Cull {
        unsafe { material_get_cull(self.0.as_ptr()) }
    }

    /// Get the [`MaterialParam::wireframe`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Wireframe.html>
    ///
    /// see also [`crate::material::material_get_wireframe`]
    pub fn get_wireframe(&self) -> bool {
        unsafe { material_get_wireframe(self.0.as_ptr()) != 0 }
    }

    /// Get the [`MaterialParam::depth_test`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthTest.html>
    ///
    /// see also [`crate::material::material_get_depth_test`]
    pub fn get_depth_test(&self) -> DepthTest {
        unsafe { material_get_depth_test(self.0.as_ptr()) }
    }

    /// Get the [`MaterialParam::depth_write`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthWrite.html>
    ///
    /// see also [`crate::material::material_get_depth_write`]
    pub fn get_depth_write(&self) -> bool {
        unsafe { material_get_depth_write(self.0.as_ptr()) != 0 }
    }

    /// Get the [`MaterialParam::queue_offset`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/QueueOffset.html>
    ///
    /// see also [`crate::material::material_get_queue_offset`]
    pub fn get_queue_offset(&self) -> i32 {
        unsafe { material_get_queue_offset(self.0.as_ptr()) }
    }

    /// Get the [`MaterialParam::chain`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Chain.html>
    ///
    /// see also [`crate::material::material_get_chain`]
    pub fn get_chain(&self) -> Option<Material> {
        unsafe { NonNull::new(material_get_chain(self.0.as_ptr())).map(Material) }
    }

    /// Get All param infos.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetAllParamInfo.html>
    ///
    /// see also [`ParamInfos`][`stereokit::Material`]
    pub fn get_all_param_info(&self) -> ParamInfos<'_> {
        ParamInfos::from(self)
    }

    /// The default Physically Based Rendering material! This is used by StereoKit anytime a mesh or model has metallic
    /// or roughness properties, or needs to look more realistic. Its shader may change based on system performance
    /// characteristics, so it can be great to copy this one when creating your own materials! Or if you want to
    /// override StereoKit’s default PBR behavior, here’s where you do it! Note that the shader used by default here is
    /// much more costly than Default.Material.
    ///  <https://stereokit.net/Pages/StereoKit/Material/PBR.html>
    pub fn pbr() -> Self {
        Self::find("default/material_pbr").unwrap()
    }

    /// Same as MaterialPBR, but it uses a discard clip for transparency.
    /// <https://stereokit.net/Pages/StereoKit/Material/PBRClip.html>
    pub fn pbr_clip() -> Self {
        Self::find("default/material_pbr_clip").unwrap()
    }

    /// The default unlit material! This is used by StereoKit any time a mesh or model needs to be rendered with an
    /// unlit surface. Its shader may change based on system performance characteristics, so it can be great to copy
    /// this one when creating your own materials! Or if you want to override StereoKit’s default unlit behavior,
    /// here’s where you do it!
    /// <https://stereokit.net/Pages/StereoKit/Material/Unlit.html>
    pub fn unlit() -> Self {
        Self::find("default/material_unlit").unwrap()
    }

    /// The default unlit material with alpha clipping! This is used by StereoKit for unlit content with transparency,
    /// where completely transparent pixels are discarded. This means less alpha blending, and fewer visible alpha
    /// blending issues! In particular, this is how Sprites are drawn. Its shader may change based on system
    /// performance characteristics, so it can be great to copy this one when creating your own materials!
    /// Or if you want to override StereoKit’s default unlit clipped behavior, here’s where you do it!
    /// <https://stereokit.net/Pages/StereoKit/Material/UnlitClip.html>
    pub fn unlit_clip() -> Self {
        Self::find("default/material_unlit_clip").unwrap()
    }

    /// The material used by the UI! By default, it uses a shader that creates a ‘finger shadow’ that shows how close
    /// the finger
    /// is to the UI.
    /// <https://stereokit.net/Pages/StereoKit/Material/UI.html>
    pub fn ui() -> Self {
        Self::find("default/material_ui").unwrap()
    }

    /// A material for indicating interaction volumes! It renders a border around the edges of the UV coordinates that
    /// will ‘grow’ on proximity to the user’s finger. It will discard pixels outside of that border, but will also
    /// show the finger shadow. This is meant to be an opaque material, so it works well for depth LSR. This material
    /// works best on cube-like meshes where each face has UV coordinates from 0-1.
    /// Shader Parameters: color - color border_size - meters border_size_grow - meters border_affect_radius - meters
    /// <https://stereokit.net/Pages/StereoKit/Material/UIBox.html>
    pub fn ui_box() -> Self {
        Self::find("default/material_ui_box").unwrap()
    }

    /// The material used by the UI for Quadrant Sized UI elements. See UI.QuadrantSizeMesh for additional details.
    /// By default, it uses a shader that creates a ‘finger shadow’ that shows how close the finger is to the UI.
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    pub fn ui_quadrant() -> Self {
        Self::find("default/material_ui_quadrant").unwrap()
    }
}

/// Infos of a Material.  This includes all global shader variables and textures.
/// Warning, you have to be cautious when settings some infos
/// <https://stereokit.net/Pages/StereoKit/Material/GetAllParamInfo.html>
///
/// see also [`stereokit::Material`]
pub struct ParamInfos<'a> {
    material: &'a Material,
    index: i32,
}

extern "C" {
    pub fn material_set_float(material: MaterialT, name: *const c_char, value: f32);
    pub fn material_set_vector2(material: MaterialT, name: *const c_char, value: Vec2);
    pub fn material_set_vector3(material: MaterialT, name: *const c_char, value: Vec3);
    pub fn material_set_color(material: MaterialT, name: *const c_char, color_gamma: Color128);
    pub fn material_set_vector4(material: MaterialT, name: *const c_char, value: Vec4);
    // Deprecated: pub fn material_set_vector(material: MaterialT, name: *const c_char, value: Vec4);
    pub fn material_set_int(material: MaterialT, name: *const c_char, value: i32);
    pub fn material_set_int2(material: MaterialT, name: *const c_char, value1: i32, value2: i32);
    pub fn material_set_int3(material: MaterialT, name: *const c_char, value1: i32, value2: i32, value3: i32);
    pub fn material_set_int4(
        material: MaterialT,
        name: *const c_char,
        value1: i32,
        value2: i32,
        value3: i32,
        value4: i32,
    );
    pub fn material_set_bool(material: MaterialT, name: *const c_char, value: Bool32T);
    pub fn material_set_uint(material: MaterialT, name: *const c_char, value: u32);
    pub fn material_set_uint2(material: MaterialT, name: *const c_char, value1: u32, value2: u32);
    pub fn material_set_uint3(material: MaterialT, name: *const c_char, value1: u32, value2: u32, value3: u32);
    pub fn material_set_uint4(
        material: MaterialT,
        name: *const c_char,
        value1: u32,
        value2: u32,
        value3: u32,
        value4: u32,
    );
    pub fn material_set_matrix(material: MaterialT, name: *const c_char, value: Matrix);
    pub fn material_set_texture(material: MaterialT, name: *const c_char, value: TexT) -> Bool32T;
    pub fn material_set_texture_id(material: MaterialT, id: u64, value: TexT) -> Bool32T;
    pub fn material_get_float(material: MaterialT, name: *const c_char) -> f32;
    pub fn material_get_vector2(material: MaterialT, name: *const c_char) -> Vec2;
    pub fn material_get_vector3(material: MaterialT, name: *const c_char) -> Vec3;
    pub fn material_get_color(material: MaterialT, name: *const c_char) -> Color128;
    pub fn material_get_vector4(material: MaterialT, name: *const c_char) -> Vec4;
    pub fn material_get_int(material: MaterialT, name: *const c_char) -> i32;
    pub fn material_get_bool(material: MaterialT, name: *const c_char) -> Bool32T;
    pub fn material_get_uint(material: MaterialT, name: *const c_char) -> u32;
    pub fn material_get_matrix(material: MaterialT, name: *const c_char) -> Matrix;
    pub fn material_get_texture(material: MaterialT, name: *const c_char) -> TexT;
    pub fn material_has_param(material: MaterialT, name: *const c_char, type_: MaterialParam) -> Bool32T;
    pub fn material_set_param(material: MaterialT, name: *const c_char, type_: MaterialParam, value: *const c_void);
    pub fn material_set_param_id(material: MaterialT, id: u64, type_: MaterialParam, value: *const c_void);
    pub fn material_get_param(
        material: MaterialT,
        name: *const c_char,
        type_: MaterialParam,
        out_value: *mut c_void,
    ) -> Bool32T;
    pub fn material_get_param_id(material: MaterialT, id: u64, type_: MaterialParam, out_value: *mut c_void)
        -> Bool32T;
    pub fn material_get_param_info(
        material: MaterialT,
        index: i32,
        out_name: *mut *mut c_char,
        out_type: *mut MaterialParam,
    );
    pub fn material_get_param_count(material: MaterialT) -> i32;

}

/// TODO: v0.4 This may need significant revision? What type of data does this material parameter need?
/// This is used to tell the shader how large the data is, and where to attach it to on the shader.
/// <https://stereokit.net/Pages/StereoKit/MaterialParam.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum MaterialParam {
    /// This data type is not currently recognized. Please report your case on GitHub Issues!
    Unknown = 0,
    /// A single 32 bit float value
    Float = 1,
    /// A color value described by 4 floating point values. Memory-wise this is
    /// the same as a Vector4, but in the shader this variable has a ':color'
    /// tag applied to it using StereoKits's shader info syntax, indicating it's
    /// a color value. Color values for shaders should be in linear space, not
    /// gamma.
    Color128 = 2,
    /// A 2 component vector composed of floating point values
    Vec2 = 3,
    /// A 3 component vector composed of floating point values
    Vec3 = 4,
    /// A 4 component vector composed of floating point values
    Vec4 = 5,
    /// A 4x4 matrix of floats.
    Matrix = 6,
    /// Texture information!
    Texture = 7,
    Int = 8,
    Int2 = 9,
    Int3 = 10,
    Int4 = 11,
    UInt = 12,
    UInt2 = 13,
    UInt3 = 14,
    UInt4 = 15,
}

impl<'a> Iterator for ParamInfos<'a> {
    type Item = ParamInfo;

    /// get all the param info
    ///
    /// see also [`crate::material::material_get_param_info`][`crate::material::material_get_param_count`][`crate::material::material_get_param`]
    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        let count = unsafe { material_get_param_count(self.material.0.as_ptr()) };
        if self.index < count {
            let i = self.index;
            let res = self.material_get_param_info(i);
            match res {
                Some((name, type_info)) => {
                    //self.get_data_with_id(self.index - 1,  type_info)
                    self.get_data(name, type_info)
                }
                None => {
                    //error
                    Log::err(format!(
                        "Unable to get info {:?}/{:?} for material {:?}",
                        self.index,
                        count,
                        self.material.get_id()
                    ));
                    None
                }
            }
        } else {
            None
        }
    }
}

impl<'a> ParamInfos<'a> {
    /// helper to get the infos with only a material
    pub fn from(material: &'a Material) -> ParamInfos<'a> {
        ParamInfos { material, index: -1 }
    }

    /// This allows you to set more complex shader data types such as structs. Note the SK doesn’t guard against setting
    /// data of the wrong size here, so pay extra attention to the size of your data here, and ensure it matched up with
    /// the shader!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetData.html>
    ///
    /// see also [`crate::material::material_set_param`]
    ///    # Safety
    ///    Be sure of the data you want to modify this way.
    pub unsafe fn set_data<S: AsRef<str>>(
        &mut self,
        name: S,
        type_info: MaterialParam,
        value: *mut c_void,
    ) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_param(self.material.0.as_ptr(), cstr.as_ptr(), type_info, value)
        };
        self
    }

    /// Add an info value (identified with an id) to the shader of this material. Be sure of using a pointer 'value'
    /// corresponding to the right type 'type_info'
    /// <https://stereokit.net/Pages/StereoKit/Material/SetData.html>
    ///
    /// see also [`ParamInfo`][`crate::material::material_set_param_id`]
    ///    # Safety
    ///    Be sure of the data you want to modify this way.
    pub unsafe fn set_data_with_id<S: AsRef<str>>(
        &mut self,
        id: u64,
        type_info: MaterialParam,
        value: *mut c_void,
    ) -> &mut Self {
        unsafe { material_set_param_id(self.material.0.as_ptr(), id, type_info, value) };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetBool.html>
    ///
    /// see also [`crate::material::material_set_bool`]
    pub fn set_bool<S: AsRef<str>>(&mut self, name: S, value: bool) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_bool(self.material.0.as_ptr(), cstr.as_ptr(), value as Bool32T)
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetColor.html>
    ///
    /// see also [`crate::material::material_set_color`]
    pub fn set_color<S: AsRef<str>>(&mut self, name: S, value: impl Into<Color128>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_color(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetFloat.html>
    ///
    /// see also [`crate::material::material_set_float`]
    pub fn set_float<S: AsRef<str>>(&mut self, name: S, value: f32) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_float(self.material.0.as_ptr(), cstr.as_ptr(), value)
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetInt.html>
    /// * value : up to 4 integer values
    ///
    /// see also [`crate::material::material_set_int`]
    /// see also [`crate::material::material_set_int2`]
    /// see also [`crate::material::material_set_int3`]
    /// see also [`crate::material::material_set_int4`]
    pub fn set_int<S: AsRef<str>>(&mut self, name: S, values: &[i32]) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            match values.len() {
                1 => material_set_int(self.material.0.as_ptr(), cstr.as_ptr(), values[0]),
                2 => material_set_int2(self.material.0.as_ptr(), cstr.as_ptr(), values[0], values[1]),
                3 => material_set_int3(self.material.0.as_ptr(), cstr.as_ptr(), values[0], values[1], values[2]),
                4 => material_set_int4(
                    self.material.0.as_ptr(),
                    cstr.as_ptr(),
                    values[0],
                    values[1],
                    values[2],
                    values[3],
                ),
                _ => {}
            }
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetUInt.html>
    /// * value : up to 4 integer values
    ///
    /// see also [`crate::material::material_set_uint`]
    /// see also [`crate::material::material_set_uint2`]
    /// see also [`crate::material::material_set_uint3`]
    /// see also [`crate::material::material_set_uint4`]
    pub fn set_uint<S: AsRef<str>>(&mut self, name: S, values: &[u32]) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            match values.len() {
                1 => material_set_uint(self.material.0.as_ptr(), cstr.as_ptr(), values[0]),
                2 => material_set_uint2(self.material.0.as_ptr(), cstr.as_ptr(), values[0], values[1]),
                3 => material_set_uint3(self.material.0.as_ptr(), cstr.as_ptr(), values[0], values[1], values[2]),
                4 => material_set_uint4(
                    self.material.0.as_ptr(),
                    cstr.as_ptr(),
                    values[0],
                    values[1],
                    values[2],
                    values[3],
                ),
                _ => {}
            }
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetMatrix.html>
    ///
    /// see also [`crate::material::material_set_matrix`]
    pub fn set_matrix<S: AsRef<str>>(&mut self, name: S, value: impl Into<Matrix>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_matrix(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetTexture.html>
    ///
    /// see also [`crate::material::material_set_texture`]
    pub fn set_texture<S: AsRef<str>>(&mut self, name: S, value: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_texture(self.material.0.as_ptr(), cstr.as_ptr(), value.as_ref().0.as_ptr())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetVector.html>
    ///
    /// see also [`crate::material::material_set_vector2`]
    pub fn set_vec2<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec2>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_vector2(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetVector.html>
    ///
    /// see also [`crate::material::material_set_vector3`]
    pub fn set_vec3<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec3>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_vector3(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetVector.html>
    ///
    /// see also [`crate::material::material_set_vector4`]
    pub fn set_vec4<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec4>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_set_vector4(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    //Get the name and type of the info at given index
    fn material_get_param_info(&self, index: i32) -> Option<(&str, MaterialParam)> {
        let name_info = CString::new("H").unwrap().into_raw() as *mut *mut c_char;
        let mut type_info = MaterialParam::Unknown;
        unsafe { material_get_param_info(self.material.0.as_ptr(), index, name_info, &mut type_info) }
        let name_info = unsafe { CStr::from_ptr(*name_info).to_str().unwrap() };
        Some((name_info, type_info))
    }

    /// Get an info value of the shader of this material
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    ///
    /// see also [`ParamInfo`][`crate::material::material_get_param`]
    pub fn get_data<S: AsRef<str>>(&self, name: S, type_info: MaterialParam) -> Option<ParamInfo> {
        let value = CString::new("H").unwrap().into_raw() as *mut c_void;
        if unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap();
            material_get_param(self.material.0.as_ptr(), cstr.as_ptr(), type_info, value)
        } != 0
        {
            Some(ParamInfo::new(name, value, type_info))
        } else {
            None
        }
    }

    /// Get an info value (identified with an id) of the shader of this material
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    ///
    /// see also [`ParamInfo`][`crate::material::material_get_param_id`]
    pub fn get_data_with_id<S: AsRef<str>>(&self, id: u64, type_info: MaterialParam) -> Option<ParamInfo> {
        let value = CString::new("H").unwrap().into_raw() as *mut c_void;
        if unsafe { material_get_param_id(self.material.0.as_ptr(), id, type_info, value) } != 0 {
            Some(ParamInfo::new(id.to_string(), value, type_info))
        } else {
            None
        }
    }

    /// Get the number of infos for this node
    /// <https://stereokit.net/Pages/StereoKit/Material/ParamCount.html>
    ///
    /// see also [`crate::material::material_get_param_count`]
    pub fn get_count(&self) -> i32 {
        unsafe { material_get_param_count(self.material.0.as_ptr()) }
    }
}

/// One Info of a Material. This is only used for read
/// <https://stereokit.net/Pages/StereoKit/Material/GetAllParamInfo.html>
///
/// see also [`stereokit::Material`]
pub struct ParamInfo {
    pub name: String,
    pub value: *mut c_void,
    pub type_info: MaterialParam,
}

impl ParamInfo {
    pub fn new<S: AsRef<str>>(name: S, value: *mut c_void, type_info: MaterialParam) -> ParamInfo {
        ParamInfo { name: name.as_ref().to_string(), value, type_info }
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_type(&self) -> MaterialParam {
        self.type_info
    }

    pub fn get_raw_ptr(&self) -> *mut c_void {
        self.value
    }

    pub fn get_float(&self) -> Option<&f32> {
        if self.value.is_null() || self.type_info != MaterialParam::Float {
            None
        } else {
            Some(unsafe { &*(self.value as *const f32) })
        }
    }

    pub fn get_vector2(&self) -> Option<&Vec2> {
        if self.value.is_null() || self.type_info != MaterialParam::Vec2 {
            None
        } else {
            Some(unsafe { &*(self.value as *const Vec2) })
        }
    }

    pub fn get_vector3(&self) -> Option<&Vec3> {
        if self.value.is_null() || self.type_info != MaterialParam::Vec2 {
            None
        } else {
            Some(unsafe { &*(self.value as *const Vec3) })
        }
    }

    pub fn get_vector4(&self) -> Option<&Vec4> {
        if self.value.is_null() || self.type_info != MaterialParam::Vec4 {
            None
        } else {
            Some(unsafe { &*(self.value as *const Vec4) })
        }
    }

    pub fn get_color(&self) -> Option<&Color128> {
        if self.value.is_null() || self.type_info != MaterialParam::Color128 {
            None
        } else {
            Some(unsafe { &*(self.value as *const Color128) })
        }
    }

    pub fn get_int(&self) -> Option<&i32> {
        if self.value.is_null()
            || self.type_info != MaterialParam::Int
            || self.type_info != MaterialParam::Int2
            || self.type_info != MaterialParam::Int3
            || self.type_info != MaterialParam::Int4
        {
            None
        } else {
            Some(unsafe { &*(self.value as *const i32) })
        }
    }

    pub fn get_uint(&self) -> Option<&u32> {
        if self.value.is_null()
            || self.type_info != MaterialParam::UInt
            || self.type_info != MaterialParam::UInt2
            || self.type_info != MaterialParam::UInt3
            || self.type_info != MaterialParam::UInt4
        {
            None
        } else {
            Some(unsafe { &*(self.value as *const u32) })
        }
    }

    pub fn get_matrix(&self) -> Option<&Matrix> {
        if self.value.is_null() || self.type_info != MaterialParam::Matrix {
            None
        } else {
            Some(unsafe { &*(self.value as *const Matrix) })
        }
    }

    pub fn get_texture(&self) -> Option<&mut Tex> {
        if self.value.is_null() || self.type_info != MaterialParam::Texture {
            None
        } else {
            Some(unsafe { &mut *(self.value as *mut Tex) })
        }
    }

    pub fn to_string(&self) -> Option<String> {
        match self.type_info {
            MaterialParam::Unknown => Some("<data>".to_string()),
            MaterialParam::Float => self.get_float().map(|v| v.to_string()),
            MaterialParam::Color128 => {
                self.get_color().map(|v| format!("RGBA : {:?}/{:?}/{:?} : {:?}", v.r, v.g, v.b, v.a))
            }
            MaterialParam::Vec2 => self.get_vector2().map(|v| v.to_string()),
            MaterialParam::Vec3 => self.get_vector3().map(|v| v.to_string()),
            MaterialParam::Vec4 => self.get_vector4().map(|v| v.to_string()),
            MaterialParam::Matrix => self.get_matrix().map(|v| v.to_string()),
            MaterialParam::Texture => self.get_texture().map(|v| v.get_id().to_string()),
            MaterialParam::Int => self.get_int().map(|v| v.to_string()),
            MaterialParam::Int2 => self.get_int().map(|v| v.to_string()),
            MaterialParam::Int3 => self.get_int().map(|v| v.to_string()),
            MaterialParam::Int4 => self.get_int().map(|v| v.to_string()),
            MaterialParam::UInt => self.get_int().map(|v| v.to_string()),
            MaterialParam::UInt2 => self.get_int().map(|v| v.to_string()),
            MaterialParam::UInt3 => self.get_int().map(|v| v.to_string()),
            MaterialParam::UInt4 => self.get_int().map(|v| v.to_string()),
        }
    }
}

extern "C" {
    pub fn material_buffer_create(register_slot: i32, size: i32) -> MaterialBufferT;
    pub fn material_buffer_set_data(buffer: MaterialBufferT, buffer_data: *const c_void);
    pub fn material_buffer_release(buffer: MaterialBufferT);
}

pub struct MaterialBuffer_<T> {
    material_buffer: MaterialBufferT,
    phantom: PhantomData<T>,
}
#[repr(C)]
#[derive(Debug)]
pub struct _MaterialBufferT {
    _unused: [u8; 0],
}
pub type MaterialBufferT = *mut _MaterialBufferT;

impl<T> Drop for MaterialBuffer_<T> {
    fn drop(&mut self) {
        unsafe { material_buffer_release(self.material_buffer) }
    }
}
impl<T> AsRef<MaterialBuffer_<T>> for MaterialBuffer_<T> {
    fn as_ref(&self) -> &MaterialBuffer_<T> {
        self
    }
}

impl<T> MaterialBuffer_<T> {
    /// Create a new global MaterialBuffer bound to the register slot id. All shaders will have access to the data
    /// provided via this instance’s Set.
    /// <https://stereokit.net/Pages/StereoKit/MaterialBuffer/MaterialBuffer.html>
    ///
    /// see also [`crate::material::material_buffer_create`]
    pub fn material_buffer(register_slot: i32) -> MaterialBuffer_<T> {
        let size = std::mem::size_of::<T>();
        let mat_buffer = unsafe { material_buffer_create(register_slot, size as i32) };
        MaterialBuffer_ { material_buffer: mat_buffer, phantom: PhantomData }
    }

    /// This will upload your data to the GPU for shaders to use.
    /// <https://stereokit.net/Pages/StereoKit/MaterialBuffer/Set.html>
    ///
    /// see also [`crate::material::material_buffer_set_data`]
    pub fn set(&self, in_data: *mut T) {
        unsafe { material_buffer_set_data(self.material_buffer, in_data as *const c_void) };
    }
}
