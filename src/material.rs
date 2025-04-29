use crate::StereoKitError;
use crate::maths::{Bool32T, Matrix, Vec2, Vec3, Vec4};
use crate::shader::{Shader, ShaderT};
use crate::system::{IAsset, Log};
use crate::tex::{Tex, TexT};
use crate::ui::IdHashT;
use crate::util::Color128;
use std::ffi::{CStr, CString, c_char, c_void};
use std::marker::PhantomData;
use std::path::Path;
use std::ptr::NonNull;

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
    /// Also known as Alpha To Coverage, this mode uses MSAA samples to create transparency. This works with a z-buffer
    /// and therefore functionally behaves more like an opaque material, but has a quantized number of "transparent
    /// values" it supports rather than a full range of  0-255 or 0-1. For 4x MSAA, this will give only 4 different
    /// transparent values, 8x MSAA only 8, etc. From a performance perspective, MSAA usually is only costly around
    /// triangle edges, but using this mode, MSAA is used for the whole triangle.
    MSAA = 2,
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
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::Matrix, util::Color128, mesh::Mesh, material::{*}};
///
/// let cube = Mesh::cube();
/// // Create a material with default properties
/// let mut material_cube = Material::default();
///
/// // Set some shader properties
/// material_cube.color_tint   (Color128::new(1.0, 0.5, 0.3, 1.0))
///              .transparency (Transparency::MSAA)
///              .depth_test   (DepthTest::LessOrEq)
///              .face_cull    (Cull::Front);
///
/// filename_scr = "screenshots/materials.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     cube.draw(token, &material_cube, Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/materials.jpeg" alt="screenshot" width="200">
#[derive(Debug, PartialEq)]
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
/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _MaterialT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type MaterialT = *mut _MaterialT;

unsafe extern "C" {
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
    /// see also [crate::font::font_find]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::default();
    /// assert_eq!(material.get_id(), "default/material");
    /// ```
    fn default() -> Self {
        let c_str = CString::new("default/material").unwrap();
        Material(NonNull::new(unsafe { material_find(c_str.as_ptr()) }).unwrap())
    }
}

impl Material {
    /// Creates a material from a shader, and uses the shader’s default settings.
    /// <https://stereokit.net/Pages/StereoKit/Material/Material.html>
    /// * `shader` - Any valid shader.
    /// * `id` - If None the id will be set to a default value "auto/asset_???"
    ///
    /// see also [`material_create`] [`material_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Matrix, Vec3}, mesh::Mesh, material::Material, shader::Shader};
    ///
    /// // Create Mesh and its material
    /// let plane = Mesh::generate_plane(Vec2::ONE, Vec3::NEG_Z, Vec3::X, None,  true);
    /// let mut material_plane = Material::new(Shader::unlit(), Some("my_material_plane"));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     plane.draw(token, &material_plane,  Matrix::IDENTITY, None, None);
    /// );
    /// ```
    pub fn new(shader: impl AsRef<Shader>, id: Option<&str>) -> Material {
        let mut mat = Material(NonNull::new(unsafe { material_create(shader.as_ref().0.as_ptr()) }).unwrap());
        if let Some(id) = id {
            mat.id(id);
        }
        mat
    }

    /// Loads a Shader asset and creates a Material using it. If the shader fails to load, an error will be returned,
    /// if so you can use unwrap_or_default() to get the default.
    /// <https://stereokit.net/Pages/StereoKit/Material/Material.html>
    /// * `id` - If None the id will be set to a default value "auto/asset_???"
    /// * `shader_file_name` - The filename of a Shader asset.
    ///
    /// see also [`material_create`] [`material_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Matrix, Vec3}, mesh::Mesh, material::Material};
    ///
    /// // Create Mesh and its material
    /// let circle = Mesh::generate_circle(1.0, Vec3::NEG_Z, Vec3::X, None,  true);
    /// let material_circle =
    ///     Material::from_file("shaders/blinker.hlsl.sks", Some("my_material_circle")).unwrap();
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     circle.draw(token, &material_circle,  Matrix::IDENTITY, None, None);
    /// );
    /// ```
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
    /// see also [`material_copy`]
    pub fn default_copy() -> Material {
        Material::default().copy()
    }

    /// Creates a new Material asset with the same shader and properties! Draw calls with the new Material will not
    /// batch together with this one.
    /// <https://stereokit.net/Pages/StereoKit/Material/Copy.html>
    ///
    /// see also [`material_copy()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, util::named_colors, material::Material};
    ///
    /// let mut material_blue = Material::default_copy();
    /// material_blue.metallic_amount(0.63);
    /// material_blue.color_tint(named_colors::BLUE);
    /// let mut material_red = material_blue.copy();
    /// material_red.id("my_red_material").color_tint(named_colors::RED);
    ///
    /// assert_eq!(&material_blue.get_all_param_info().get_float("metal"),
    ///            &material_red.get_all_param_info().get_float("metal"));
    /// assert_ne!(&material_blue.get_id(), &material_red.get_id());
    /// assert_ne!(&material_blue.get_all_param_info().get_color("color"),
    ///            &material_red.get_all_param_info().get_color("color"));
    /// ```
    pub fn copy(&self) -> Material {
        Material(NonNull::new(unsafe { material_copy(self.0.as_ptr()) }).unwrap())
    }

    /// Creates a new Material asset with the same shader and properties! Draw calls with the new Material will not
    /// batch together with this one.
    /// <https://stereokit.net/Pages/StereoKit/Material/Copy.html>
    /// * `id` - Which material are you looking for?
    ///
    /// Returns a new Material asset with the same shader and properties if the id is found.
    /// see also [`material_copy_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, material::Material, shader::Shader};
    ///
    /// let mut material = Material::new(Shader::pbr(), Some("my_material"));
    /// material.roughness_amount(0.42);
    /// let mut material_red = Material::copy_id("my_material").unwrap();
    /// material_red.id("my_red_material").color_tint(named_colors::RED);
    ///
    /// assert_eq!(&material.get_all_param_info().get_float("roughness"),
    ///            &material_red.get_all_param_info().get_float("roughness"));
    /// assert_ne!(&material.get_all_param_info().get_color("color"),
    ///            &material_red.get_all_param_info().get_color("color"));
    /// assert_ne!(&material.get_id(), &material_red.get_id());
    /// ```
    pub fn copy_id<S: AsRef<str>>(id: S) -> Result<Material, StereoKitError> {
        let c_str = CString::new(id.as_ref())?;
        match NonNull::new(unsafe { material_copy_id(c_str.as_ptr()) }) {
            Some(pt) => Ok(Material(pt)),
            None => Err(StereoKitError::MaterialFind(id.as_ref().to_owned(), "copy_id".to_owned())),
        }
    }

    /// Looks for a Material asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Material/Find.html>
    /// * `id` - Which material are you looking for ?
    ///
    /// see also [`material_find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors,material::Material, shader::Shader};
    ///
    /// let mut material = Material::new(Shader::pbr(), Some("my_material"));
    /// let mut material_red = Material::find("my_material").unwrap();
    /// material_red.id("my_red_material").color_tint(named_colors::RED);
    ///
    /// assert_eq!(&material.get_all_param_info().get_color("color"),
    ///            &material_red.get_all_param_info().get_color("color"));
    /// assert_eq!(&material.get_id(),&"my_red_material");
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<Material, StereoKitError> {
        let c_str = CString::new(id.as_ref())?;
        let material = NonNull::new(unsafe { material_find(c_str.as_ptr()) });
        match material {
            Some(material) => Ok(Material(material)),
            None => Err(StereoKitError::MaterialFind(id.as_ref().to_owned(), "not found".to_owned())),
        }
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Material/Find.html>
    ///
    /// see also [`material_find()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors,material::Material, shader::Shader};
    ///
    /// let mut material = Material::new(Shader::pbr(), Some("my_material"));
    /// let mut material_red = material.clone_ref();
    /// material_red.id("my_red_material").color_tint(named_colors::RED);
    ///
    /// assert_eq!(&material.get_all_param_info().get_color("color"),
    ///            &material_red.get_all_param_info().get_color("color"));
    /// assert_eq!(&material.get_id(),&"my_red_material");
    /// ```
    pub fn clone_ref(&self) -> Material {
        Material(
            NonNull::new(unsafe { material_find(material_get_id(self.0.as_ptr())) })
                .expect("<asset>::clone_ref failed!"),
        )
    }

    /// Non-canonical function of convenience!! Use this for Icons and other Ui Images
    /// Copy a Material and set a Tex image to its diffuse_tex. If the Tex fails to load, an error will be returned,
    /// if so you can use unwrap_or_default() to get the default.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromFile.html>
    /// * `tex_file_name` - The file name of the texture to load.
    /// * `srgb_data` - If true, the texture will be loaded as sRGB data.
    /// * `priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner.
    ///
    /// see also [Material::diffuse_tex] [`material_create`] [`material_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::Material;
    ///
    /// let material1 = Material::unlit().copy();
    /// let material2 = Material::unlit().copy();
    /// let mut material3 = Material::unlit().tex_file_copy("textures/open_gltf.jpeg", true, None)
    ///                    .expect("open_gltf.jpeg should load");
    ///
    /// assert_eq!(&material1.get_all_param_info().get_texture("diffuse").unwrap().get_id(),
    ///            &material2.get_all_param_info().get_texture("diffuse").unwrap().get_id());
    /// assert_ne!(&material2.get_all_param_info().get_texture("diffuse").unwrap().get_id(),
    ///            &material3.get_all_param_info().get_texture("diffuse").unwrap().get_id());
    /// ```
    pub fn tex_file_copy(
        &mut self,
        tex_file_name: impl AsRef<Path>,
        srgb_data: bool,
        priority: Option<i32>,
    ) -> Result<Material, StereoKitError> {
        let tex = Tex::from_file(&tex_file_name, srgb_data, priority);
        match tex {
            Ok(tex) => {
                let mut mat = self.copy();
                mat.diffuse_tex(tex);
                Ok(mat)
            }
            Err(err) => Err(StereoKitError::TexFile(tex_file_name.as_ref().to_path_buf(), err.to_string())),
        }
    }

    /// Non-canonical function of convenience!! Use this for Icons and other Ui Images
    /// Copy a Material and set a Tex image to its diffuse_tex. If the Tex fails to load, an error will be returned,
    /// if so you can use unwrap_or_default() to get the default.
    /// <https://stereokit.net/Pages/StereoKit/Tex/FromFile.html>
    /// * `tex_file_name` - The file name of the texture to load.
    /// * `srgb_data` - If true, the texture will be loaded as sRGB data.
    /// * `priority` - The priority sort order for this asset in the async loading system. Lower values mean loading
    ///   sooner.
    ///
    /// see also [Material::diffuse_tex] [`material_create`] [`material_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material, tex::Tex};
    ///
    /// let material1 = Material::unlit().copy();
    /// let material2 = Material::unlit().copy();
    /// let tex = Tex::from_file("textures/open_gltf.jpeg", true, None)
    ///                    .expect("tex should be created");
    /// let mut material3 = Material::unlit().tex_copy(tex);
    ///
    /// assert_eq!(&material1.get_all_param_info().get_texture("diffuse").unwrap().get_id(),
    ///            &material2.get_all_param_info().get_texture("diffuse").unwrap().get_id());
    /// assert_ne!(&material2.get_all_param_info().get_texture("diffuse").unwrap().get_id(),
    ///            &material3.get_all_param_info().get_texture("diffuse").unwrap().get_id());
    /// ```
    pub fn tex_copy(&mut self, tex: impl AsRef<Tex>) -> Material {
        let mut mat = self.copy();
        mat.diffuse_tex(tex);
        mat
    }

    /// Set a new id to the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Id.html>
    ///
    /// see also [`material_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material, shader::Shader};
    ///
    /// let mut material = Material::new(Shader::pbr(), Some("my_material"));
    /// assert_eq!(material.get_id(), "my_material");
    ///
    /// material.id("my_new_material");
    /// assert_eq!(material.get_id(), "my_new_material");
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { material_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Overrides the Shader this material uses.
    /// <https://stereokit.net/Pages/StereoKit/Material/Shader.html>
    ///
    /// see also [`material_set_shader`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to init sk !!!!
    /// use stereokit_rust::{material::Material, shader::Shader};
    ///
    /// let mut material = Material::new(Shader::pbr(), Some("my_material"));
    /// assert_eq!(material.get_shader().get_id(), Shader::pbr().get_id());
    ///
    /// material.shader(Shader::unlit());
    /// assert_eq!(material.get_shader().get_id(), Shader::unlit().get_id());
    /// ```
    pub fn shader(&mut self, shader: impl AsRef<Shader>) -> &mut Self {
        unsafe { material_set_shader(self.0.as_ptr(), shader.as_ref().0.as_ptr()) };
        self
    }

    /// Non canonical shader parameter to indicate a border size if the shader have one (especially for [`Material::ui_box`])
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::Material;
    ///
    /// let mut material = Material::ui_box();
    /// material.border_size(0.0428);
    ///
    /// assert_eq!(material.get_all_param_info().get_float("border_size"), 0.0428);
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("border_size"), 12300782195362451721);
    /// ```
    pub fn border_size(&mut self, time: f32) -> &mut Self {
        let ptr: *const f32 = &time;
        unsafe {
            material_set_param_id(self.0.as_ptr(), 12300782195362451721, MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// In clip shaders, this is the cutoff value below which pixels are discarded.
    /// Typically, the diffuse/albedo’s alpha component is sampled for comparison here. This represents the float param ‘cutoff’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to init sk !!!!
    /// use stereokit_rust::{material::Material};
    ///
    /// let mut material = Material::pbr_clip().copy();
    /// material.clip_cutoff(0.53);
    /// assert_eq!(material.get_all_param_info().get_float("cutoff"), 0.53);
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("cutoff"), 9874215895386126464);
    /// ```
    pub fn clip_cutoff(&mut self, cutoff: f32) -> &mut Self {
        let ptr: *const f32 = &cutoff;
        unsafe {
            material_set_param_id(self.0.as_ptr(), 9874215895386126464, MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// A per-material color tint, behavior could vary from shader to shader, but often this is just multiplied against
    /// the diffuse texture right at the start. This represents the Color param ‘color’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to init sk !!!!
    /// use stereokit_rust::{material::Material, util::named_colors};
    ///
    /// let mut material = Material::unlit().copy();
    /// material.color_tint(named_colors::RED);
    /// assert_eq!(material.get_all_param_info().get_color("color"), named_colors::RED.into());
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("color"), 8644027876048135736);
    /// ```    
    pub fn color_tint(&mut self, color: impl Into<Color128>) -> &mut Self {
        let ptr: *const Color128 = &color.into();
        unsafe {
            material_set_param_id(self.0.as_ptr(), 8644027876048135736, MaterialParam::Color128, ptr as *const c_void);
        }
        self
    }

    /// The primary color texture for the shader! Diffuse, Albedo, ‘The Texture’, or whatever you want to call it, this
    /// is usually the base color that the shader works with. This represents the texture param ‘diffuse’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`Material::tex_file_copy`]
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material, tex::Tex};
    ///
    /// let mut material = Material::unlit().copy();
    /// let default_tex = material.get_all_param_info().get_texture("diffuse").unwrap();
    ///
    /// let tex = Tex::from_file("textures/open_gltf.jpeg", true, None)
    ///                    .expect("tex should be created");
    /// material.diffuse_tex(&tex);
    ///
    /// assert_eq!(&material.get_all_param_info().get_texture("diffuse").unwrap().get_id(),
    ///            &tex.get_id());
    /// assert_ne!(&default_tex.get_id(),
    ///            &tex.get_id());
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("diffuse"), 17401384459118377917);
    /// ```
    pub fn diffuse_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            material_set_param_id(
                self.0.as_ptr(),
                17401384459118377917,
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
    /// see also [`Material::emission_tex`]
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to init sk !!!!
    /// use stereokit_rust::{material::Material, util::named_colors};
    ///
    /// let mut material = Material::pbr().copy();
    /// material.emission_factor(named_colors::RED);
    /// assert_eq!(material.get_all_param_info().get_color("emission_factor"), named_colors::RED.into());
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("emission_factor"), 5248711978018327020);
    /// ```  
    pub fn emission_factor(&mut self, color: impl Into<Color128>) -> &mut Self {
        let ptr: *const Color128 = &color.into();
        unsafe {
            material_set_param_id(self.0.as_ptr(), 5248711978018327020, MaterialParam::Color128, ptr as *const c_void);
        }
        self
    }

    /// This texture is unaffected by lighting, and is frequently just added in on top of the material’s final color!
    /// Tends to look really glowy. This represents the texture param ‘emission’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`Material::emission_factor`]
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::named_colors, material::Material, tex::Tex};
    ///
    /// let mut material = Material::pbr_clip().tex_file_copy("textures/water/bump_large.ktx2", true, Some(0)).unwrap();
    ///
    /// let tex = Tex::from_file("textures/water/bump_large_inverse.ktx2", true, None)
    ///                    .expect("tex should be created");
    /// material.emission_tex(&tex).emission_factor(named_colors::RED);
    ///
    /// assert_eq!(&material.get_all_param_info().get_texture("emission").unwrap().get_id(),
    ///            &tex.get_id());
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("emission"), 17756472659261185998);
    /// ```
    pub fn emission_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            material_set_param_id(
                self.0.as_ptr(),
                17756472659261185998,
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
    /// see also [`Material::metal_tex`]
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to init sk !!!!
    /// use stereokit_rust::{material::Material, util::named_colors};
    ///
    /// let mut material = Material::pbr().copy();
    /// material.metallic_amount(0.68);
    /// assert_eq!(material.get_all_param_info().get_float("metallic"), 0.68);
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("metallic"), 16113330016842241480);
    /// ```  
    pub fn metallic_amount(&mut self, amount: f32) -> &mut Self {
        let ptr: *const f32 = &amount;
        unsafe {
            material_set_param_id(self.0.as_ptr(), 16113330016842241480, MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// For physically based shaders, metal is a texture that encodes metallic and roughness data into the ‘B’ and ‘G’
    /// channels, respectively. This represents the texture param ‘metal’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`Material::metallic_amount`]
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material, tex::Tex};
    ///
    /// let mut material = Material::pbr_clip().tex_file_copy("textures/parquet2/parquet2.ktx2", true, Some(0)).unwrap();
    ///
    /// let tex = Tex::from_file("textures/parquet2/parquet2metal.ktx2", true, None)
    ///                    .expect("tex should be created");
    /// material.metal_tex(&tex).metallic_amount(0.68);
    ///
    /// assert_eq!(&material.get_all_param_info().get_texture("metal").unwrap().get_id(),
    ///            &tex.get_id());
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("metal"), 4582786214424138428);
    /// ```
    pub fn metal_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            material_set_param_id(
                self.0.as_ptr(),
                4582786214424138428,
                MaterialParam::Texture,
                texture.as_ref().0.as_ptr() as *const c_void,
            );
        }
        self
    }

    /// The ‘normal map’ texture for the material! This texture contains information about the direction of the
    /// material’s surface, which is used to calculate lighting, and make surfaces look like they have more detail than
    /// they actually do. Normals are in Tangent Coordinate Space, and the RGB values map to XYZ values. This represents
    /// the texture param ‘normal’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material, tex::Tex};
    ///
    /// let mut material = Material::from_file("shaders/water_pbr2.hlsl.sks", None).unwrap();
    ///
    /// let tex = Tex::from_file("textures/water/bump_large.ktx2", true, None)
    ///                    .expect("tex should be created");
    /// material.normal_tex(&tex);
    ///
    /// assert_eq!(&material.get_all_param_info().get_texture("normal").unwrap().get_id(),
    ///            &tex.get_id());
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("normal"), 6991063326977151602);
    /// ```
    pub fn normal_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            material_set_param_id(
                self.0.as_ptr(),
                6991063326977151602,
                MaterialParam::Texture,
                texture.as_ref().0.as_ptr() as *const c_void,
            );
        }
        self
    }

    /// Used by physically based shaders, this can be used for baked ambient occlusion lighting, or to remove specular
    /// reflections from areas that are surrounded by geometry that would likely block reflections. This represents the
    /// texture param ‘occlusion’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material, tex::Tex};
    ///
    /// let mut material = Material::pbr().tex_file_copy("textures/parquet2/parquet2.ktx2", true, None).unwrap();
    ///
    /// let tex = Tex::from_file("textures/parquet2/parquet2ao.ktx2", true, None)
    ///                    .expect("tex should be created");
    /// material.occlusion_tex(&tex);
    ///
    /// assert_eq!(&material.get_all_param_info().get_texture("occlusion").unwrap().get_id(),
    ///            &tex.get_id());
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("occlusion"), 10274420935108893154);
    /// ```
    pub fn occlusion_tex(&mut self, texture: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            material_set_param_id(
                self.0.as_ptr(),
                10274420935108893154,
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
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::Material;
    ///
    /// let mut material = Material::pbr().copy();
    /// material.roughness_amount(0.78);
    ///
    /// assert_eq!(material.get_all_param_info().get_float("roughness"), 0.78);
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("roughness"), 14293098357166276437);
    /// ```
    pub fn roughness_amount(&mut self, amount: f32) -> &mut Self {
        let ptr: *const f32 = &amount;
        unsafe {
            material_set_param_id(self.0.as_ptr(), 14293098357166276437, MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// Not necessarily present in all shaders, this multiplies the UV coordinates of the mesh, so that the texture will repeat.
    /// This is great for tiling textures! This represents the float param ‘tex_scale’.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`material_set_param`]
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
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::Material;
    ///
    /// let mut material = Material::from_file("shaders/water_pbr2.hlsl.sks", None).unwrap();
    /// material.time(0.38);
    ///
    /// assert_eq!(material.get_all_param_info().get_float("time"), 0.38);
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("time"), 2185518981507421060);
    /// ```
    pub fn time(&mut self, time: f32) -> &mut Self {
        let ptr: *const f32 = &time;
        unsafe {
            material_set_param_id(self.0.as_ptr(), 2185518981507421060, MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    /// Not necessarily present in all shaders, this transforms the UV coordinates of the mesh, so that the texture can
    /// repeat and scroll. XY components are offset, and ZW components are scale.
    ///  
    /// This represents the float param 'tex_trans'.
    /// <https://stereokit.net/Pages/StereoKit/MatParamName.html>
    ///
    /// see also [`material_set_param_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec4, material::Material};
    ///
    /// let mut material = Material::unlit().copy();
    /// material.tex_transform(Vec4::ONE * 5.5);
    ///
    /// assert_eq!(material.get_all_param_info().get_vector4("tex_trans"), Vec4::ONE * 5.5);
    /// # use stereokit_rust::util::Hash;
    /// # assert_eq!(Hash::string("tex_trans"), 11548192078170871263);
    /// ```
    pub fn tex_transform(&mut self, transform: impl Into<Vec4>) -> &mut Self {
        let ptr: *const Vec4 = &transform.into();
        unsafe {
            material_set_param_id(self.0.as_ptr(), 11548192078170871263, MaterialParam::Float, ptr as *const c_void);
        }
        self
    }

    //--- Others parameters

    /// What type of transparency does this Material use? Default is None. Transparency has an impact on performance,
    /// and draw order.
    /// Check the [`Transparency`] enum for details.
    /// <https://stereokit.net/Pages/StereoKit/Material/Transparency.html>
    ///
    /// see also [`material_set_transparency`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Quat}, util::{named_colors,Color32},
    ///                      mesh::Mesh, material::{Material, Transparency}};
    ///
    /// // Creating Meshes and their materials
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
    /// let sphere = Mesh::generate_sphere(1.4, None);
    ///
    /// let mut material_sphere = Material::pbr().copy();
    /// material_sphere.color_tint(named_colors::BLUE).transparency(Transparency::Add);
    ///
    /// let material_cube = Material::pbr().copy();
    /// let cube_transform = Matrix::r(Quat::from_angles(40.0, 50.0, 20.0));
    ///
    /// assert_eq!(material_sphere.get_transparency(), Transparency::Add);
    /// assert_eq!(material_cube.get_transparency(), Transparency::None);
    /// filename_scr = "screenshots/material_transparency.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     cube.draw(token, &material_cube, cube_transform, None, None);
    ///     sphere.draw(token, &material_sphere, Matrix::IDENTITY, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/material_transparency.jpeg" alt="screenshot" width="200">
    pub fn transparency(&mut self, mode: Transparency) -> &mut Self {
        unsafe { material_set_transparency(self.0.as_ptr(), mode) };
        self
    }

    /// How should this material cull faces?
    /// <https://stereokit.net/Pages/StereoKit/Material/FaceCull.html>
    ///
    /// see also [`material_set_cull`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, util::{named_colors,Color32},
    ///                      mesh::Mesh, material::{Material, Cull}};
    ///
    /// // Creating Meshes and their materials
    /// let cube1 = Mesh::generate_cube(Vec3::ONE * 1.0, None);
    /// let cube2 = Mesh::generate_cube(Vec3::ONE * 0.4, None);
    ///
    /// let mut material_cube1 = Material::pbr().copy();
    /// material_cube1.face_cull(Cull::Front).color_tint(named_colors::RED);
    ///
    /// let mut material_cube2 = Material::pbr().copy();
    /// assert_eq!(material_cube2.get_face_cull(), Cull::Back);
    /// material_cube2.face_cull(Cull::None).color_tint(named_colors::GREEN);
    ///
    /// assert_eq!(material_cube1.get_face_cull(), Cull::Front);
    /// assert_eq!(material_cube2.get_face_cull(), Cull::None);
    ///
    /// filename_scr = "screenshots/material_face_cull.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     cube1.draw(token, &material_cube1,  Matrix::IDENTITY, None, None);
    ///     cube2.draw(token, &material_cube2,  Matrix::IDENTITY, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/material_face_cull.jpeg" alt="screenshot" width="200">
    pub fn face_cull(&mut self, mode: Cull) -> &mut Self {
        unsafe { material_set_cull(self.0.as_ptr(), mode) };
        self
    }

    /// Should this material draw only the edges/wires of the mesh? This can be useful for debugging, and even some
    /// kinds of visualization work.
    ///
    /// Note that this may not work on some mobile OpenGL systems like Quest.
    /// <https://stereokit.net/Pages/StereoKit/Material/Wireframe.html>
    ///
    /// see also [`material_set_wireframe`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors,Color32},material::Material};
    ///
    /// let mut material_cube = Material::pbr().copy();
    /// assert_eq!(material_cube.get_wireframe(), false);
    /// material_cube.wireframe(true).color_tint(named_colors::CYAN);
    /// assert_eq!(material_cube.get_wireframe(), true);
    /// ```
    pub fn wireframe(&mut self, wireframe: bool) -> &mut Self {
        unsafe { material_set_wireframe(self.0.as_ptr(), wireframe as Bool32T) };
        self
    }

    /// How does this material interact with the ZBuffer? Generally [DepthTest::Less] would be normal behavior: don’t draw
    /// objects that are occluded. But this can also be used to achieve some interesting effects, like you could use
    /// [DepthTest::Greater] to draw a glow that
    /// indicates an object is behind something.
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthTest.html>
    ///
    /// see also [`material_set_depth_test`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ util::{named_colors,Color32}, material::{Material, DepthTest}};
    ///
    /// let mut material_cube = Material::pbr().copy();
    /// assert_eq!(material_cube.get_depth_test(), DepthTest::Less);
    /// material_cube.depth_test(DepthTest::Greater).color_tint(named_colors::CYAN);
    /// assert_eq!(material_cube.get_depth_test(), DepthTest::Greater);
    /// ```
    pub fn depth_test(&mut self, depth_test_mode: DepthTest) -> &mut Self {
        unsafe { material_set_depth_test(self.0.as_ptr(), depth_test_mode) };
        self
    }

    /// Should this material write to the ZBuffer? For opaque objects, this generally should be true. But transparent
    /// objects writing to the ZBuffer can be problematic and cause draw order issues. Note that turning this off can
    /// mean that this material won’t get properly accounted for when the MR system is performing late stage
    /// reprojection. Not writing to the buffer can also be faster! :)
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthWrite.html>
    ///
    /// see also [`material_set_depth_write`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors,Color32},material::Material};
    ///
    /// let mut material_cube = Material::pbr().copy();
    /// assert_eq!(material_cube.get_depth_write(), true);
    /// material_cube.depth_write(false).color_tint(named_colors::CYAN);
    /// assert_eq!(material_cube.get_depth_write(), false);
    /// ```
    pub fn depth_write(&mut self, write_enabled: bool) -> &mut Self {
        unsafe { material_set_depth_write(self.0.as_ptr(), write_enabled as Bool32T) };
        self
    }

    /// This property will force this material to draw earlier or later in the draw queue. Positive values make it draw
    /// later, negative makes it earlier. This can be helpful for tweaking performance! If you know an object is always
    /// going to be close to the user and likely to obscure lots of objects (like hands), drawing it earlier can mean
    /// objects behind it get discarded much faster! Similarly, objects that are far away (skybox!) can be pushed
    /// towards the back of the queue, so they’re more likely to be discarded early.
    /// <https://stereokit.net/Pages/StereoKit/Material/QueueOffset.html>
    ///
    /// see also [`material_set_queue_offset`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{util::{named_colors,Color32},material::Material};
    ///
    /// let mut material_cube = Material::pbr().copy();
    /// assert_eq!(material_cube.get_queue_offset(), 0);
    /// material_cube.queue_offset(8).color_tint(named_colors::CYAN);
    /// assert_eq!(material_cube.get_queue_offset(), 8);
    /// ```
    pub fn queue_offset(&mut self, offset: i32) -> &mut Self {
        unsafe { material_set_queue_offset(self.0.as_ptr(), offset) };
        self
    }

    /// Allows you to chain Materials together in a form of multi-pass rendering! Any time the Material is used, the
    /// chained Materials will also be used to draw the same item.
    /// <https://stereokit.net/Pages/StereoKit/Material/Chain.html>
    ///
    /// see also [`material_set_chain`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::Material;
    ///
    /// let mut material_cube = Material::pbr().copy();
    /// assert!(material_cube.get_chain().is_none());
    ///
    /// let mut material_to_chain = Material::ui_quadrant().copy();
    /// material_to_chain.id("material_to_chain");
    /// material_cube.chain(&material_to_chain);
    /// assert_eq!(material_cube.get_chain().unwrap().get_id(), material_to_chain.get_id());
    /// ```
    pub fn chain(&mut self, chained_material: &Material) -> &mut Self {
        unsafe { material_set_chain(self.0.as_ptr(), chained_material.0.as_ptr()) };
        self
    }

    /// Get the [`Material::id`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Id.html>
    ///
    /// see also [`material_get_id`]
    ///
    /// see example in [`Material::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(material_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// Get the [`Material::shader`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Shader.html>
    ///
    /// see also [`material_get_shader`]
    ///
    /// see example in [`Material::shader`]
    pub fn get_shader(&self) -> Shader {
        unsafe { Shader(NonNull::new(material_get_shader(self.0.as_ptr())).unwrap()) }
    }

    /// Get the [`Material::transparency`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Transparency.html>
    ///
    /// see also [`material_get_transparency`]
    ///
    /// see example in [`Material::transparency`]
    pub fn get_transparency(&self) -> Transparency {
        unsafe { material_get_transparency(self.0.as_ptr()) }
    }

    /// Get the [`Material::face_cull`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/FaceCull.html>
    ///
    /// see also [`material_get_cull`]
    ///
    /// see example in [`Material::face_cull`]
    pub fn get_face_cull(&self) -> Cull {
        unsafe { material_get_cull(self.0.as_ptr()) }
    }

    /// Get the [`Material::wireframe`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Wireframe.html>
    ///
    /// see also [`material_get_wireframe`]
    ///
    /// see example in [`Material::wireframe`]
    pub fn get_wireframe(&self) -> bool {
        unsafe { material_get_wireframe(self.0.as_ptr()) != 0 }
    }

    /// Get the [`Material::depth_test`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthTest.html>
    ///
    /// see also [`material_get_depth_test`]
    ///
    /// see example in [`Material::depth_test`]
    pub fn get_depth_test(&self) -> DepthTest {
        unsafe { material_get_depth_test(self.0.as_ptr()) }
    }

    /// Get the [`Material::depth_write`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/DepthWrite.html>
    ///
    /// see also [`material_get_depth_write`]
    ///
    /// see example in [`Material::depth_write`]
    pub fn get_depth_write(&self) -> bool {
        unsafe { material_get_depth_write(self.0.as_ptr()) != 0 }
    }

    /// Get the [`Material::queue_offset`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/QueueOffset.html>
    ///
    /// see also [`material_get_queue_offset`]
    ///
    /// see example in [`Material::queue_offset`]
    pub fn get_queue_offset(&self) -> i32 {
        unsafe { material_get_queue_offset(self.0.as_ptr()) }
    }

    /// Get the [`Material::chain`] of the material.
    /// <https://stereokit.net/Pages/StereoKit/Material/Chain.html>
    ///
    /// see also [`material_get_chain`]
    ///
    /// see example in [`Material::chain`]
    pub fn get_chain(&self) -> Option<Material> {
        unsafe { NonNull::new(material_get_chain(self.0.as_ptr())).map(Material) }
    }

    /// Get All param infos.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetAllParamInfo.html>
    ///
    /// see also [`ParamInfos`] [`ParamInfo`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::{Material, MaterialParam}, shader::Shader};
    ///
    /// let material = Material::new(Shader::pbr(), Some("my_material"));
    /// let param_infos = material.get_all_param_info();
    /// assert_ne!(param_infos.get_count(), 0);
    /// for param in param_infos {
    ///    match param.name.as_str()  {
    ///        "diffuse" | "emission" | "metal" | "occlusion"  
    ///             => assert_eq!(param.type_info, MaterialParam::Texture),
    ///        "color" | "emission_factor"  
    ///             => assert_eq!(param.type_info, MaterialParam::Color128),
    ///        "metallic" | "roughness"  
    ///             => assert_eq!(param.type_info, MaterialParam::Float),
    ///        "tex_trans"  
    ///             => assert_eq!(param.type_info, MaterialParam::Vec4),
    ///        _ => {}
    ///    }
    /// }
    /// ```
    pub fn get_all_param_info(&self) -> ParamInfos<'_> {
        ParamInfos::from(self)
    }

    /// The default Physically Based Rendering material! This is used by StereoKit anytime a mesh or model has metallic
    /// or roughness properties, or needs to look more realistic. Its shader may change based on system performance
    /// characteristics, so it can be great to copy this one when creating your own materials! Or if you want to
    /// override StereoKit’s default PBR behavior, here’s where you do it! Note that the shader used by default here is
    /// much more costly than Default.Material.
    ///  <https://stereokit.net/Pages/StereoKit/Material/PBR.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::pbr();
    /// assert_eq!(material.get_id(), "default/material_pbr");
    /// ```
    pub fn pbr() -> Self {
        Self::find("default/material_pbr").unwrap()
    }

    /// Same as MaterialPBR, but it uses a discard clip for transparency.
    /// <https://stereokit.net/Pages/StereoKit/Material/PBRClip.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::pbr_clip();
    /// assert_eq!(material.get_id(), "default/material_pbr_clip");
    /// ```
    pub fn pbr_clip() -> Self {
        Self::find("default/material_pbr_clip").unwrap()
    }

    /// The default unlit material! This is used by StereoKit any time a mesh or model needs to be rendered with an
    /// unlit surface. Its shader may change based on system performance characteristics, so it can be great to copy
    /// this one when creating your own materials! Or if you want to override StereoKit’s default unlit behavior,
    /// here’s where you do it!
    /// <https://stereokit.net/Pages/StereoKit/Material/Unlit.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::unlit();
    /// assert_eq!(material.get_id(), "default/material_unlit");
    /// ```
    pub fn unlit() -> Self {
        Self::find("default/material_unlit").unwrap()
    }

    /// The default unlit material with alpha clipping! This is used by StereoKit for unlit content with transparency,
    /// where completely transparent pixels are discarded. This means less alpha blending, and fewer visible alpha
    /// blending issues! In particular, this is how Sprites are drawn. Its shader may change based on system
    /// performance characteristics, so it can be great to copy this one when creating your own materials!
    /// Or if you want to override StereoKit’s default unlit clipped behavior, here’s where you do it!
    /// <https://stereokit.net/Pages/StereoKit/Material/UnlitClip.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::unlit_clip();
    /// assert_eq!(material.get_id(), "default/material_unlit_clip");
    /// ```
    pub fn unlit_clip() -> Self {
        Self::find("default/material_unlit_clip").unwrap()
    }

    /// The material used by cubemap
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::equirect();
    /// assert_eq!(material.get_id(), "default/equirect_convert");
    /// ```
    pub fn equirect() -> Self {
        Self::find("default/equirect_convert").unwrap()
    }

    /// The material used by font
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::font();
    /// assert_eq!(material.get_id(), "default/material_font");
    /// ```
    pub fn font() -> Self {
        Self::find("default/material_font").unwrap()
    }

    /// The material used for hands
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::hand();
    /// assert_eq!(material.get_id(), "default/material_hand");
    /// ```
    pub fn hand() -> Self {
        Self::find("default/material_hand").unwrap()
    }

    /// The material used by the UI! By default, it uses a shader that creates a ‘finger shadow’ that shows how close
    /// the finger
    /// is to the UI.
    /// <https://stereokit.net/Pages/StereoKit/Material/UI.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::ui();
    /// assert_eq!(material.get_id(), "default/material_ui");
    /// ```
    pub fn ui() -> Self {
        Self::find("default/material_ui").unwrap()
    }

    /// A material for indicating interaction volumes! It renders a border around the edges of the UV coordinates that
    /// will ‘grow’ on proximity to the user’s finger. It will discard pixels outside of that border, but will also
    /// show the finger shadow. This is meant to be an opaque material, so it works well for depth LSR. This material
    /// works best on cube-like meshes where each face has UV coordinates from 0-1.
    /// Shader Parameters: color - color border_size - meters border_size_grow - meters border_affect_radius - meters
    /// <https://stereokit.net/Pages/StereoKit/Material/UIBox.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::ui_box();
    /// assert_eq!(material.get_id(), "default/material_ui_box");
    /// ```
    pub fn ui_box() -> Self {
        Self::find("default/material_ui_box").unwrap()
    }

    /// The material used by the UI for Quadrant Sized UI elements. See UI.QuadrantSizeMesh for additional details.
    /// By default, it uses a shader that creates a ‘finger shadow’ that shows how close the finger is to the UI.
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::ui_quadrant();
    /// assert_eq!(material.get_id(), "default/material_ui_quadrant");
    /// ```
    pub fn ui_quadrant() -> Self {
        Self::find("default/material_ui_quadrant").unwrap()
    }

    /// The material used by the UI for Aura, an extra space and visual element that goes around Window elements to make
    /// them easier to grab
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material};
    /// let mut material = Material::ui_aura();
    /// assert_eq!(material.get_id(), "default/material_ui_aura");
    /// ```
    pub fn ui_aura() -> Self {
        Self::find("default/material_ui_aura").unwrap()
    }
}

/// Infos of a Material.  This includes all global shader variables and textures.
/// Warning, you have to be cautious when settings some infos
/// <https://stereokit.net/Pages/StereoKit/Material/GetAllParamInfo.html>
///
/// see also [`Material::get_all_param_info`] [ParamInfo]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{material::{Material, MaterialParam, Cull},
///                      mesh::Mesh, maths::{Vec3, Vec4, Matrix}};
///
/// let cube = Mesh::cube();
/// let mut material_cube = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
/// material_cube.face_cull(Cull::Front).tex_transform(Vec4::new(0.0, 0.0, 0.04, 0.04));
/// let mut param_infos = material_cube.get_all_param_info();
/// assert!(param_infos.has_param("line_color", MaterialParam::Vec3), "line_color is missing");
/// assert_eq!(param_infos.get_float("edge_pos"), 1.5);
///
/// // Change of unusual values that are not listed in Material
/// param_infos .set_float("edge_pos", 0.5)
///             .set_vector3("line_color", Vec3::new(0.54, 0.54, 0.20));
///
/// filename_scr = "screenshots/param_infos.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     cube.draw(token, &material_cube, Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/param_infos.jpeg" alt="screenshot" width="200">
pub struct ParamInfos<'a> {
    material: &'a Material,
    index: i32,
}

unsafe extern "C" {
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
    /// An i32, or 1 component array composed of i32 values
    Int = 8,
    /// A 2 component array composed of i32 values
    Int2 = 9,
    /// A 3 component array composed of i32 values
    Int3 = 10,
    /// A 4 component array composed of i32 values
    Int4 = 11,
    /// A u32, or 1 component array composed of u32 values
    UInt = 12,
    /// A 2 component array composed of u32 values
    UInt2 = 13,
    /// A 3 component array composed of u32 values
    UInt3 = 14,
    /// A 4 component array composed of u32 values
    UInt4 = 15,
}

impl Iterator for ParamInfos<'_> {
    type Item = ParamInfo;

    /// get all the param info
    ///
    /// see also [`material_get_param_info`] [`material_get_param_count`] [`material_get_param`]
    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        let count = unsafe { material_get_param_count(self.material.0.as_ptr()) };
        if self.index < count {
            let res = self.material_get_param_info(self.index);
            match res {
                Some((name, type_info)) => Some(ParamInfo::new(name, type_info)),
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
    /// <https://stereokit.net/Pages/StereoKit/Material/GetAllParamInfo.html>
    /// * `material` - the material to get the param info from.
    ///
    /// see also [`Material::get_all_param_info`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam, ParamInfos};
    ///
    /// let mut material_cube = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = ParamInfos::from(&material_cube);
    /// assert!(param_infos.has_param("line_color", MaterialParam::Vec3), "line_color is missing");
    /// assert_eq!(param_infos.get_float("edge_pos"), 1.5);
    /// ```
    pub fn from(material: &'a Material) -> ParamInfos<'a> {
        ParamInfos { material, index: -1 }
    }

    /// Only way to see if a shader has a given parameter if you do not iterate over parameters.
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// * `name`: The name of the parameter to check for.
    ///
    /// see also [`material_has_param`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam, ParamInfos};
    ///
    /// let mut material_cube = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = ParamInfos::from(&material_cube);
    /// assert!(param_infos.has_param("line_color", MaterialParam::Vec3), "line_color is missing");
    /// assert!(param_infos.has_param("edge_pos", MaterialParam::Float),   "edge_pos is missing");
    /// ```
    pub fn has_param<S: AsRef<str>>(&self, name: S, type_: MaterialParam) -> bool {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_has_param(self.material.0.as_ptr(), cstr.as_ptr(), type_) != 0
        }
    }

    /// This allows you to set more complex shader data types such as structs. Note the SK doesn’t guard against setting
    /// data of the wrong size here, so pay extra attention to the size of your data here, and ensure it matched up with
    /// the shader! Consider using [`ParamInfos::set_data_with_id`] if you have to change the data type often (i.e. in
    /// the main loop).
    /// <https://stereokit.net/Pages/StereoKit/Material/SetData.html>
    /// * `name` - The name of the parameter to set
    /// * `type_info` - The type of the data being set.
    /// * `value` - A pointer to the data being set.
    ///
    /// see also [`material_set_param`] [`ParamInfos::set_data_with_id`]
    ///    # Safety
    ///    Be sure of the data you want to modify this way.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// // an odd way to do : material.color_tint(...);
    /// let mut new_color: std::vec::Vec<f32>  = vec![1.0, 0.5, 0.2, 1.0];
    /// unsafe{
    ///     param_infos.set_data("color", MaterialParam::Color128,
    ///                          new_color.as_ptr() as *mut std::ffi::c_void);
    /// }
    /// assert_eq!( param_infos.get_color("color"),
    ///             util::Color128::new(1.0, 0.5, 0.2, 1.0));
    /// ```
    pub unsafe fn set_data<S: AsRef<str>>(
        &mut self,
        name: S,
        type_info: MaterialParam,
        value: *mut c_void,
    ) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_param(self.material.0.as_ptr(), cstr.as_ptr(), type_info, value)
        };
        self
    }

    /// Add an info value (identified with an id) to the shader of this material. Be sure of using a pointer 'value'
    /// corresponding to the right type 'type_info'
    /// <https://stereokit.net/Pages/StereoKit/Material/SetData.html>
    /// * `id` - the hash_fnv64_string value of the name of the parameter.
    /// * `type_info` - the type of the parameter you want to set.
    /// * `value` - a pointer to the data you want to set.
    ///
    /// see also [`material_set_param_id`] [`ParamInfos::set_data`]
    ///    # Safety
    ///    Be sure of the data you want to modify this way.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::{Material, MaterialParam, Cull},
    ///                      mesh::Mesh, maths::{Vec3, Vec4, Matrix}, util::Hash};
    ///
    /// let cube = Mesh::cube();
    /// let mut material_cube = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// material_cube.face_cull(Cull::Front).tex_transform(Vec4::new(0.0, 0.0, 0.04, 0.04));
    /// let mut param_infos = material_cube.get_all_param_info();
    /// // an odd way to do : material.color_tint(...);
    /// let mut new_color: std::vec::Vec<f32>  = vec![0.2, 0.2, 0.9, 1.0];
    /// let hash_color = Hash::string("color");
    ///
    /// filename_scr = "screenshots/param_infos_with_id.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     unsafe{
    ///         param_infos.set_data_with_id(hash_color, MaterialParam::Color128,
    ///                          new_color.as_ptr() as *mut std::ffi::c_void);
    ///     }
    ///     cube.draw(token, &material_cube, Matrix::IDENTITY, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/param_infos_with_id.jpeg" alt="screenshot" width="200">
    pub unsafe fn set_data_with_id(&mut self, id: IdHashT, type_info: MaterialParam, value: *mut c_void) -> &mut Self {
        unsafe { material_set_param_id(self.material.0.as_ptr(), id, type_info, value) };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetBool.html>
    /// * `name` - Name of the shader parameter.
    /// * `value` - The new value to set.
    ///
    /// see also [`material_set_bool`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// param_infos.set_bool("use_occlusion", true);
    ///
    /// assert_eq!( param_infos.get_bool("use_occlusion"),true);
    /// ```
    pub fn set_bool<S: AsRef<str>>(&mut self, name: S, value: bool) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_bool(self.material.0.as_ptr(), cstr.as_ptr(), value as Bool32T)
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetColor.html>
    /// * `name` - Name of the shader parameter.
    /// * `color_gamma` - The gamma space color for the shader to use.
    ///
    /// see also [`material_set_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::{Material, MaterialParam},util::Color128};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// let new_color = Color128::new(1.0, 0.5, 0.2, 1.0);
    /// // same as Material::color_tint(new_color);
    /// param_infos.set_color("color", new_color);  
    ///
    /// assert_eq!( param_infos.get_color("color"),new_color.to_linear() );
    /// ```
    pub fn set_color<S: AsRef<str>>(&mut self, name: S, color_gamma: impl Into<Color128>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_color(self.material.0.as_ptr(), cstr.as_ptr(), color_gamma.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetFloat.html>
    /// * `name` - The name of the parameter to set.
    /// * `value` - The value to set for the parameter.
    ///
    /// see also [`material_set_float`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// param_infos.set_float("edge_pos", 0.18);
    ///
    /// assert_eq!( param_infos.get_float("edge_pos"), 0.18);
    /// ```
    pub fn set_float<S: AsRef<str>>(&mut self, name: S, value: f32) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_float(self.material.0.as_ptr(), cstr.as_ptr(), value)
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetInt.html>
    /// * `name` - the name of the parameter to set
    /// * `value` - up to 4 integer values
    ///
    /// see also [`material_set_int`]
    /// see also [`material_set_int2`]
    /// see also [`material_set_int3`]
    /// see also [`material_set_int4`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// let new_factors = vec![302,50,20,10];
    /// param_infos.set_int("size_factors", new_factors.as_slice());
    ///
    /// assert_eq!( param_infos.get_int_vector("size_factors", MaterialParam::Int4).unwrap(), new_factors);
    /// ```
    pub fn set_int<S: AsRef<str>>(&mut self, name: S, values: &[i32]) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
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
    /// and the value is not set! Warning, this may work on Int values as you can see in the examples.
    /// <https://stereokit.net/Pages/StereoKit/Material/SetUInt.html>
    /// * `name` - the name of the parameter to set
    /// * `value` : up to 4 unsigned integer values
    ///
    /// see also [`material_set_uint`]
    /// see also [`material_set_uint2`]
    /// see also [`material_set_uint3`]
    /// see also [`material_set_uint4`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// assert_eq!( param_infos.get_uint_vector("size_factors", MaterialParam::UInt4).unwrap(), vec![300, 4294967196, 50, 25]);
    /// let new_factors = vec![303,502,201,100];
    /// param_infos.set_uint("size_factors", new_factors.as_slice());
    ///
    /// assert!( param_infos.has_param("size_factors", MaterialParam::UInt4),"size_factors should be here");
    /// assert_eq!( param_infos.get_uint_vector("size_factors", MaterialParam::UInt4).unwrap(), new_factors);
    /// ```
    pub fn set_uint<S: AsRef<str>>(&mut self, name: S, values: &[u32]) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
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
    /// * `name` - The name of the parameter to set.
    /// * `value` - The [Matrix] to set for the parameter.
    ///
    /// see also [`material_set_matrix`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::{Material, MaterialParam}, maths::{Vec3, Matrix}};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// assert_eq!( param_infos.get_matrix("useless"), Matrix::NULL );
    /// let new_matrix = Matrix::t( Vec3::new(1.0, 2.0, 3.0));
    /// param_infos.set_matrix("useless", new_matrix);
    ///
    /// assert!( param_infos.has_param("useless", MaterialParam::Matrix),"size_factors should be here");
    /// assert_eq!( param_infos.get_matrix("useless"), new_matrix);
    /// ```
    pub fn set_matrix<S: AsRef<str>>(&mut self, name: S, value: impl Into<Matrix>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_matrix(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetTexture.html>
    /// * `name` - the name of the parameter to set
    /// * `value` - the [Tex] to set for the parameter
    ///
    /// see also [`material_set_texture`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::{Material, MaterialParam}, tex::{Tex}};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// assert_ne!( param_infos.get_texture("metal").unwrap(), Tex::default() );
    /// let metal_tex = Tex::from_file("textures/open_gltf.jpeg", true, None)
    ///                    .expect("tex should be created");
    /// param_infos.set_texture("metal", &metal_tex);
    /// assert_eq!( param_infos.get_texture("metal").unwrap(), metal_tex );
    /// ```
    pub fn set_texture<S: AsRef<str>>(&mut self, name: S, value: impl AsRef<Tex>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_texture(self.material.0.as_ptr(), cstr.as_ptr(), value.as_ref().0.as_ptr())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetVector.html>
    /// * `name` - the name of the parameter to set
    /// * `value` - the [Vec2] to set for the parameter
    ///
    /// see also [`material_set_vector2`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::{Material, MaterialParam}, maths::Vec2};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// assert_eq!( param_infos.get_vector2("edge_limit"), Vec2::new(0.1, 0.9) );
    /// let new_vec2 = Vec2::new(0.15, 0.85);
    /// param_infos.set_vector2("edge_limit", new_vec2);
    /// assert_eq!( param_infos.get_vector2("edge_limit"), new_vec2);
    /// ```
    pub fn set_vector2<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec2>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_vector2(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetVector.html>
    /// * `name` - the name of the parameter to set
    /// * `value` - the [Vec3] to set for the parameter
    ///
    /// see also [`material_set_vector2`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::{Material, MaterialParam}, maths::Vec3};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// assert_eq!( param_infos.get_vector3("line_color"), Vec3::new(0.84, 0.84, 0.84) );
    /// let new_vec3 = Vec3::new(0.75, 0.75, 0.75);
    /// param_infos.set_vector3("line_color", new_vec3);
    /// assert_eq!( param_infos.get_vector3("line_color"), new_vec3);
    /// ```
    pub fn set_vector3<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec3>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_vector3(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Sets a shader parameter with the given name to the provided value. If no parameter is found, nothing happens,
    /// and the value is not set!
    /// <https://stereokit.net/Pages/StereoKit/Material/SetVector.html>
    /// * `name`- The name of the parameter to set.
    /// * `value` - the [Vec4] to set for the parameter
    ///
    /// see also [`material_set_vector4`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{material::Material, maths::Vec4};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// assert_eq!( param_infos.get_vector4("tex_trans"), Vec4::new(0.0, 0.0, 0.1,0.1) );
    ///
    /// let new_vec4 = Vec4::new(0.0, 0.0, 0.75, 0.75);
    /// // same as material.tex_transform(new_vec4)
    /// param_infos.set_vector4("tex_trans", new_vec4);
    /// assert_eq!( param_infos.get_vector4("tex_trans"), new_vec4);
    /// ```
    pub fn set_vector4<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec4>) -> &mut Self {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_set_vector4(self.material.0.as_ptr(), cstr.as_ptr(), value.into())
        };
        self
    }

    /// Get the name and type of the info at given index
    fn material_get_param_info(&self, index: i32) -> Option<(&str, MaterialParam)> {
        let name_info = CString::new("H").unwrap().into_raw() as *mut *mut c_char;
        let mut type_info = MaterialParam::Unknown;
        unsafe { material_get_param_info(self.material.0.as_ptr(), index, name_info, &mut type_info) }
        let name_info = unsafe { CStr::from_ptr(*name_info).to_str().unwrap() };
        Some((name_info, type_info))
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of ‘false’
    /// will be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetBool.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_bool`]
    /// see example in [`ParamInfos::set_bool`]
    pub fn get_bool<S: AsRef<str>>(&self, name: S) -> bool {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_bool(self.material.0.as_ptr(), cstr.as_ptr()) != 0
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of ‘0’ will
    /// be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetFloat.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_float`]
    /// see example in [`ParamInfos::set_float`]
    pub fn get_float<S: AsRef<str>>(&self, name: S) -> f32 {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_float(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of
    /// Vec2::ZERO will be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetVector2.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_vector2`]
    /// see example in [`ParamInfos::set_vector2`]
    pub fn get_vector2<S: AsRef<str>>(&self, name: S) -> Vec2 {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_vector2(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of
    /// Vec3::ZERO will be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetVector3.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_vector3`]
    /// see example in [`ParamInfos::set_vector3`]
    pub fn get_vector3<S: AsRef<str>>(&self, name: S) -> Vec3 {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_vector3(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of
    /// Vec4::ZERO will be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetVector4.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_vector4`]
    /// see example in [`ParamInfos::set_vector4`]
    pub fn get_vector4<S: AsRef<str>>(&self, name: S) -> Vec4 {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_vector4(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of
    /// Color128::WHITE will be returned. Warning: This function returns a gamma color.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetColor.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_color`]
    /// see example in [`ParamInfos::set_color`]
    pub fn get_color<S: AsRef<str>>(&self, name: S) -> Color128 {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_color(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of ‘0’ will
    /// be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetInt.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_int]
    /// see example in [`ParamInfos::set_int`]
    pub fn get_int<S: AsRef<str>>(&self, name: S) -> i32 {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_int(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Get int vector using unsafe material_get_param function
    /// * `name` - The name of the shader parameter to get.
    /// * `type_info` - The type of the shader parameter to get: Int, Int2, Int3 or Int4
    ///
    /// see example in [`ParamInfos::set_int`]
    pub fn get_int_vector<S: AsRef<str>>(&self, name: S, type_info: MaterialParam) -> Option<Vec<i32>> {
        if let Some(out_value) = self.get_data(name, type_info) {
            match type_info {
                MaterialParam::Int => Some(unsafe { std::ptr::read(out_value as *const [i32; 1]).to_vec() }),
                MaterialParam::Int2 => Some(unsafe { std::ptr::read(out_value as *const [i32; 2]).to_vec() }),
                MaterialParam::Int3 => Some(unsafe { std::ptr::read(out_value as *const [i32; 3]).to_vec() }),
                MaterialParam::Int4 => Some(unsafe { std::ptr::read(out_value as *const [i32; 4]).to_vec() }),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of ‘0’ will
    /// be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetUInt.html>
    /// * `name` - The name of the shader parameter to get.
    ///
    /// see also [`material_get_uint`]
    /// see example in [`ParamInfos::set_uint`]
    pub fn get_uint<S: AsRef<str>>(&self, name: S) -> u32 {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_uint(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Get uint vector using unsafe material_get_param function
    /// * `name` - The name of the shader parameter to get.
    /// * type_info - The type of the shader parameter to get: UInt, UInt2, UInt3, UInt4.
    ///
    /// see example in [`ParamInfos::set_uint`]
    pub fn get_uint_vector<S: AsRef<str>>(&self, name: S, type_info: MaterialParam) -> Option<Vec<u32>> {
        if let Some(out_value) = self.get_data(name, type_info) {
            match type_info {
                MaterialParam::UInt => Some(unsafe { std::ptr::read(out_value as *const [u32; 1]).to_vec() }),
                MaterialParam::UInt2 => Some(unsafe { std::ptr::read(out_value as *const [u32; 2]).to_vec() }),
                MaterialParam::UInt3 => Some(unsafe { std::ptr::read(out_value as *const [u32; 3]).to_vec() }),
                MaterialParam::UInt4 => Some(unsafe { std::ptr::read(out_value as *const [u32; 4]).to_vec() }),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found, a default value of
    /// Matrix.Identity will be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetMatrix.html>
    /// * `name` - The name of the parameter to get.
    ///
    /// see also [`material_get_matrix`]
    /// see example in [`ParamInfos::set_matrix`]
    pub fn get_matrix<S: AsRef<str>>(&self, name: S) -> Matrix {
        unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_matrix(self.material.0.as_ptr(), cstr.as_ptr())
        }
    }

    /// Gets the value of a shader parameter with the given name. If no parameter is found,None will be returned.
    /// <https://stereokit.net/Pages/StereoKit/Material/GetTexture.html>
    /// * `name` - The name of the parameter to get.
    ///
    /// see also [`material_get_texture`]
    /// see example in [`ParamInfos::set_texture`]
    pub fn get_texture<S: AsRef<str>>(&self, name: S) -> Option<Tex> {
        NonNull::new(unsafe {
            let cstr = &CString::new(name.as_ref()).unwrap_or_default();
            material_get_texture(self.material.0.as_ptr(), cstr.as_ptr())
        })
        .map(Tex)
    }

    /// Get an info value of the shader of this material
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// * `name` - The name of the parameter to get.
    /// * `type_info` - The type of the parameter to get.
    ///
    /// see also [`ParamInfo`] [`material_get_param`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::{Material, MaterialParam};
    ///
    /// let mut material = Material::from_file("shaders/brick_pbr.hlsl.sks", None).unwrap();
    /// let mut param_infos = material.get_all_param_info();
    /// if let Some(out_value) = param_infos.get_data("size_factors", MaterialParam::Int4) {
    ///     let vec4 = unsafe { std::ptr::read(out_value as *const [i32; 4]).to_vec() };
    ///     assert_eq!( vec4, vec![300,-100,50,25] );
    /// } else { panic!("Failed to size_factors Int4");}
    /// ```
    /// see [`ParamInfos::set_data`]
    pub fn get_data<S: AsRef<str>>(&self, name: S, type_info: MaterialParam) -> Option<*mut c_void> {
        let out_value = CString::new("H").unwrap().into_raw() as *mut c_void;
        let cstr = &CString::new(name.as_ref()).unwrap();
        if unsafe { material_get_param(self.material.0.as_ptr(), cstr.as_ptr(), type_info, out_value) } != 0 {
            Some(out_value)
        } else {
            None
        }
    }

    /// Get an info value (identified with an id) of the shader of this material
    /// <https://stereokit.net/Pages/StereoKit/Material.html>
    /// * `id` - the [`crate::util::Hash::string`] value of the name of the parameter.
    /// * `type_info` - the type of the parameter.
    ///
    /// Returns a pointer to the value that will be filled in if the parameter is found.
    /// see also [`ParamInfo`] [`material_get_param_id`] [`ParamInfos::set_data_with_id`]
    pub fn get_data_with_id(&self, id: IdHashT, type_info: MaterialParam) -> Option<*mut c_void> {
        let out_value = CString::new("H").unwrap().into_raw() as *mut c_void;
        if unsafe { material_get_param_id(self.material.0.as_ptr(), id, type_info, out_value) } != 0 {
            Some(out_value)
        } else {
            None
        }
    }

    /// Get the number of infos for this node
    /// <https://stereokit.net/Pages/StereoKit/Material/ParamCount.html>
    ///
    /// see also [`material_get_param_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::Material;
    ///
    /// let mut material = Material::unlit();
    /// let mut param_infos = material.get_all_param_info();
    /// assert_eq!( param_infos.get_count(), 3);
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe { material_get_param_count(self.material.0.as_ptr()) }
    }

    /// Get the string value of the given ParamInfo
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::material::Material;
    ///
    /// let mut material = Material::unlit();
    /// let mut param_infos_iter = material.get_all_param_info();
    /// let mut param_infos = material.get_all_param_info();
    /// for param in param_infos_iter {
    ///     match (param.get_name()) {
    ///         "color" =>
    ///             assert_eq!(param_infos.string_of(&param), "r:1, g:1, b:1, a:1"),
    ///         "tex_trans" =>
    ///             assert_eq!(param_infos.string_of(&param), "[x:0, y:0, z:1, w:1]"),
    ///         "diffuse" =>
    ///             assert_eq!(param_infos.string_of(&param), "Texture data..."),
    ///        otherwise =>
    ///             panic!("Unknown param type: {}", otherwise)
    ///     }
    /// }
    /// ```
    pub fn string_of(&self, info: &ParamInfo) -> String {
        match info.get_type() {
            MaterialParam::Unknown => "Unknown".into(),
            MaterialParam::Float => self.get_float(info.get_name()).to_string(),
            MaterialParam::Color128 => self.get_color(info.get_name()).to_string(),
            MaterialParam::Vec2 => self.get_vector2(info.get_name()).to_string(),
            MaterialParam::Vec3 => self.get_vector3(info.get_name()).to_string(),
            MaterialParam::Vec4 => self.get_vector4(info.get_name()).to_string(),
            MaterialParam::Matrix => self.get_matrix(info.get_name()).to_string(),
            MaterialParam::Texture => "Texture data...".to_string(),
            MaterialParam::Int => self.get_int(info.get_name()).to_string(),
            MaterialParam::Int2 => format!("{:?}", self.get_int_vector(info.get_name(), MaterialParam::Int2)),
            MaterialParam::Int3 => format!("{:?}", self.get_int_vector(info.get_name(), MaterialParam::Int3)),
            MaterialParam::Int4 => format!("{:?}", self.get_int_vector(info.get_name(), MaterialParam::Int4)),
            MaterialParam::UInt => self.get_color(info.get_name()).to_string(),
            MaterialParam::UInt2 => format!("{:?}", self.get_int_vector(info.get_name(), MaterialParam::UInt2)),
            MaterialParam::UInt3 => format!("{:?}", self.get_int_vector(info.get_name(), MaterialParam::UInt3)),
            MaterialParam::UInt4 => format!("{:?}", self.get_int_vector(info.get_name(), MaterialParam::UInt4)),
        }
    }
}

/// One Info of a Material. This is only useful for [`Material::get_all_param_info`] iterator.
/// <https://stereokit.net/Pages/StereoKit/Material/GetAllParamInfo.html>
///
/// see also [ParamInfos] [`Material::get_all_param_info`]
pub struct ParamInfo {
    pub name: String,
    pub type_info: MaterialParam,
}

impl ParamInfo {
    /// Create a new ParamInfo with the given name and type info. There is no reason to use this method as you can
    /// get values from [`Material`] get_???? methods
    /// [`Material::get_all_param_info`] iterator
    pub fn new<S: AsRef<str>>(name: S, type_info: MaterialParam) -> ParamInfo {
        ParamInfo { name: name.as_ref().to_string(), type_info }
    }

    /// Get the name of the shader parameter
    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    /// Get the type of the shader parameter
    pub fn get_type(&self) -> MaterialParam {
        self.type_info
    }
}

unsafe extern "C" {
    pub fn material_buffer_create(register_slot: i32, size: i32) -> MaterialBufferT;
    pub fn material_buffer_set_data(buffer: MaterialBufferT, buffer_data: *const c_void);
    pub fn material_buffer_release(buffer: MaterialBufferT);
}

/// This is a chunk of memory that will get bound to all shaders at a particular register slot. StereoKit uses this to
/// provide engine values to the shader, and you can also use this to drive graphical shader systems of your own!
///
/// For example, if your application has a custom lighting system, fog, wind, or some other system that multiple shaders
/// might need to refer to, this is the perfect tool to use.
///
/// The type ‘T’ for this buffer must be a struct that uses the #[repr(C)] attribute for
/// proper copying. It should also match the layout of your equivalent cbuffer in the shader file. Note that shaders
/// often have specific byte alignment requirements!
/// <https://stereokit.net/Pages/StereoKit/MaterialBuffer.html>
pub struct MaterialBuffer<T> {
    _material_buffer: MaterialBufferT,
    phantom: PhantomData<T>,
}
/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _MaterialBufferT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type MaterialBufferT = *mut _MaterialBufferT;

impl<T> Drop for MaterialBuffer<T> {
    fn drop(&mut self) {
        unsafe { material_buffer_release(self._material_buffer) }
    }
}
impl<T> AsRef<MaterialBuffer<T>> for MaterialBuffer<T> {
    fn as_ref(&self) -> &MaterialBuffer<T> {
        self
    }
}

impl<T> MaterialBuffer<T> {
    /// Create a new global MaterialBuffer bound to the register slot id. All shaders will have access to the data
    /// provided via this instance’s Set.
    /// <https://stereokit.net/Pages/StereoKit/MaterialBuffer/MaterialBuffer.html>
    /// * `register_slot` - Valid values are 3-16. This is the register id that this data will be bound to. In HLSL,
    ///   you’ll see the slot id for ‘3’ indicated like this : register(b3)
    ///
    /// see also [`material_buffer_create`]
    pub fn new(register_slot: i32) -> MaterialBuffer<T> {
        let size = std::mem::size_of::<T>();
        let mat_buffer = unsafe { material_buffer_create(register_slot, size as i32) };
        MaterialBuffer { _material_buffer: mat_buffer, phantom: PhantomData }
    }

    /// This will upload your data to the GPU for shaders to use.
    /// <https://stereokit.net/Pages/StereoKit/MaterialBuffer/Set.html>
    ///
    /// see also [`material_buffer_set_data`]
    pub fn set(&self, in_data: *mut T) {
        unsafe { material_buffer_set_data(self._material_buffer, in_data as *const c_void) };
    }
}
