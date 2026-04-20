use crate::{
    StereoKitError,
    material::{MaterialBuffer, MaterialBufferT, MaterialParam, ParamInfo},
    maths::{Bool32T, Matrix, Vec2, Vec3, Vec4},
    shader::{Shader, ShaderT},
    system::IAsset,
    tex::{Tex, TexT},
    util::Color128,
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    mem::size_of,
    path::Path,
    ptr::NonNull,
};

/// Compute shaders allow you to run code on the GPU in a massively parallel way! This is great for
/// accelerating complex work, or simply for working inline with the graphics pipeline with easy
/// access to GPU memory.
/// <https://stereokit.net/Pages/StereoKit/Compute.html>
///
/// see also [`ComputeBuffer`] [`crate::shader::Shader`]
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{compute::Compute, material::Material,
///                      maths::{Matrix, Vec2, Vec3, Vec4},mesh::Mesh,
///                      tex::{Tex, TexFormat, TexType},util::Color128};
///
/// const SIZE:   usize = 512;
/// const GROUPS: u32   = SIZE as u32 / 8;
///
/// # {
/// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
///     .expect("shader should be compiled — run `cargo compile_sks`");
///
/// let mut out_tex = Tex::new(TexType::ImageNomips | TexType::Compute,
///                            TexFormat::Rgba128, None);
/// out_tex.set_size(SIZE, SIZE, None, None);
///
/// // Set cbuffer parameters and bind the output texture
/// let mut params = compute.get_all_param_info();
/// params.set_float  ("ring_freq",    1.0)
///       .set_int    ("arm_count",    5)
///       .set_uint   ("tex_size",     SIZE as u32)
///       .set_bool   ("center_glow",  false)
///       .set_vector2("uv_offset",    Vec2::ZERO)
///       .set_vector3("spiral_twist", Vec3::new(2.0, 0.0, 0.0))
///       .set_vector4("highlight",    Vec4::new(1.0, 0.6, 0.1, 1.0))
///       .set_color  ("base_color",   Color128::new(0.05, 0.15, 0.7, 1.0))
///       .set_matrix ("brightness",   Matrix::IDENTITY);
/// assert!(params.set_texture("out_tex", &out_tex));
/// drop(params);
///
/// // Display the texture on a cube with Cull::Front (inside-out rendering)
/// let cube = Mesh::cube();
/// let mut mat = Material::unlit().copy();
/// mat.diffuse_tex(&out_tex);
///
/// filename_scr = "screenshots/compute.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     compute.dispatch(GROUPS, GROUPS, 1);
///     cube.draw(token, &mat, Matrix::IDENTITY, None, None);
/// );
/// # } sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/compute.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct Compute(pub NonNull<_ComputeT>);
impl Drop for Compute {
    fn drop(&mut self) {
        unsafe { compute_release(self.0.as_ptr()) };
    }
}
impl AsRef<Compute> for Compute {
    fn as_ref(&self) -> &Compute {
        self
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _ComputeT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type ComputeT = *mut _ComputeT;

unsafe extern "C" {
    pub fn compute_create(shader: ShaderT) -> ComputeT;
    pub fn compute_find(id: *const c_char) -> ComputeT;
    pub fn compute_set_id(compute: ComputeT, id: *const c_char);
    pub fn compute_get_id(compute: ComputeT) -> *const c_char;
    pub fn compute_get_shader(compute: ComputeT) -> ShaderT;
    pub fn compute_set_float(compute: ComputeT, name: *const c_char, value: f32);
    pub fn compute_set_int(compute: ComputeT, name: *const c_char, value: i32);
    pub fn compute_set_uint(compute: ComputeT, name: *const c_char, value: u32);
    pub fn compute_set_vector2(compute: ComputeT, name: *const c_char, value: Vec2);
    pub fn compute_set_vector3(compute: ComputeT, name: *const c_char, value: Vec3);
    pub fn compute_set_vector4(compute: ComputeT, name: *const c_char, value: Vec4);
    pub fn compute_set_color(compute: ComputeT, name: *const c_char, color_gamma: Color128);
    pub fn compute_set_bool(compute: ComputeT, name: *const c_char, value: Bool32T);
    pub fn compute_set_matrix(compute: ComputeT, name: *const c_char, value: Matrix);
    pub fn compute_get_float(compute: ComputeT, name: *const c_char) -> f32;
    pub fn compute_get_int(compute: ComputeT, name: *const c_char) -> i32;
    pub fn compute_get_uint(compute: ComputeT, name: *const c_char) -> u32;
    pub fn compute_get_vector2(compute: ComputeT, name: *const c_char) -> Vec2;
    pub fn compute_get_vector3(compute: ComputeT, name: *const c_char) -> Vec3;
    pub fn compute_get_vector4(compute: ComputeT, name: *const c_char) -> Vec4;
    pub fn compute_get_bool(compute: ComputeT, name: *const c_char) -> Bool32T;
    pub fn compute_get_color(compute: ComputeT, name: *const c_char) -> Color128;
    pub fn compute_get_matrix(compute: ComputeT, name: *const c_char) -> Matrix;
    pub fn compute_set_texture(compute: ComputeT, name: *const c_char, texture: TexT) -> Bool32T;
    pub fn compute_set_storage(compute: ComputeT, name: *const c_char, buffer: ComputeBufferT) -> Bool32T;
    pub fn compute_set_constant(compute: ComputeT, name: *const c_char, buffer: MaterialBufferT) -> Bool32T;
    pub fn compute_dispatch(compute: ComputeT, group_count_x: u32, group_count_y: u32, group_count_z: u32);
    pub fn compute_get_param_count(compute: ComputeT) -> i32;
    pub fn compute_get_param_info(
        compute: ComputeT,
        index: i32,
        out_name: *mut *mut c_char,
        out_type: *mut MaterialParam,
    );
    pub fn compute_addref(compute: ComputeT);
    pub fn compute_release(compute: ComputeT);
}

impl IAsset for Compute {
    fn get_id(&self) -> &str {
        self.get_id()
    }
}

impl Compute {
    /// Creates a Compute dispatch from an existing [`Shader`].
    /// <https://stereokit.net/Pages/StereoKit/Compute/Compute.html>
    ///
    /// see also [`compute_create`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::shader::Shader;
    ///
    /// # {
    /// let shader = Shader::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let compute = Compute::new(&shader)
    ///     .expect("Failed to create Compute from shader");
    /// assert_eq!(compute.get_id()[..13], *"auto/compute_");
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn new(shader: &Shader) -> Result<Self, StereoKitError> {
        Ok(Compute(
            NonNull::new(unsafe { compute_create(shader.0.as_ptr()) })
                .ok_or(StereoKitError::ComputeCreate("compute_create failed".to_string()))?,
        ))
    }

    /// Creates a Compute dispatch directly from a compiled `.sks` shader file that contains a
    /// compute stage.
    /// <https://stereokit.net/Pages/StereoKit/Compute/Compute.html>
    ///
    /// see also [`compute_create`] [`crate::shader::Shader::from_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    ///
    /// assert_eq!(compute.get_id()[..13], *"auto/compute_");
    /// # } sk::Sk::shutdown();
    /// ```
    /// see example in [`Compute`]
    pub fn from_file(file_utf8: impl AsRef<Path>) -> Result<Self, StereoKitError> {
        let shader = Shader::from_file(file_utf8)?;
        Self::new(&shader)
    }

    /// Looks for a Compute object that has already been created with a matching id!
    /// <https://stereokit.net/Pages/StereoKit/Compute/Find.html>
    ///
    /// see also [`compute_find`] [`Compute::clone_ref`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let mut compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// compute.id("find_example");
    ///
    /// // Retrieve the already-loaded compute object by its id.
    /// let found = Compute::find("find_example").expect("should find by id");
    /// assert_eq!(found.get_id(), "find_example");
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<Self, StereoKitError> {
        let c_str = CString::new(id.as_ref())
            .map_err(|_| StereoKitError::ComputeFind(id.as_ref().into(), "CString conversion".to_string()))?;
        Ok(Compute(
            NonNull::new(unsafe { compute_find(c_str.as_ptr()) })
                .ok_or(StereoKitError::ComputeFind(id.as_ref().into(), "compute_find failed".to_string()))?,
        ))
    }

    /// Creates a clone of the same reference. Basically the new variable is the same asset.
    /// <https://stereokit.net/Pages/StereoKit/Compute/Find.html>
    ///
    /// see also [`compute_find`] [`Compute::find`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let mut compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// compute.id("clone_example");
    ///
    /// // clone_ref: another handle to the same underlying GPU object.
    /// let clone = compute.clone_ref();
    /// assert_eq!(clone.get_id(), "clone_example");
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn clone_ref(&self) -> Compute {
        Compute(
            NonNull::new(unsafe { compute_find(compute_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"),
        )
    }

    /// Gets or sets the unique identifier of this asset resource.
    /// <https://stereokit.net/Pages/StereoKit/Compute/Id.html>
    ///
    /// see also [`compute_set_id`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let mut compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    ///
    /// assert_eq!(compute.get_id()[..13], *"auto/compute_");
    /// compute.id("clone_example");
    ///
    /// assert_eq!(compute.get_id(), "clone_example");
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap_or_default();
        unsafe { compute_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Fire off the compute shader on the GPU! The parameters here are the number of thread
    /// *groups*, not individual threads. The total thread count will be
    /// `group_count * numthreads` (as defined in your HLSL). So if your shader declares
    /// `[numthreads(8,8,1)]` and you dispatch `(64,64,1)`, you'll get 512×512 total threads.
    /// <https://stereokit.net/Pages/StereoKit/Compute/Dispatch.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::maths::{Matrix, Vec2, Vec3, Vec4};
    /// use stereokit_rust::tex::{Tex, TexType, TexFormat};
    /// use stereokit_rust::util::Color128;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    ///
    /// const SIZE: u32 = 64;
    /// let mut tex = Tex::new(TexType::ImageNomips | TexType::Compute, TexFormat::Rgba128, None);
    /// tex.set_size(SIZE as usize, SIZE as usize, None, None);
    ///
    /// // Initialise fixed parameters once before the loop.
    /// let mut p = compute.get_all_param_info();
    /// p.set_uint  ("tex_size",     SIZE)
    ///  .set_int   ("arm_count",    5)
    ///  .set_bool  ("center_glow",  false)
    ///  .set_vector2("uv_offset",   Vec2::ZERO)
    ///  .set_vector3("spiral_twist",Vec3::new(2.0, 0.0, 0.0))
    ///  .set_vector4("highlight",   Vec4::new(1.0, 0.6, 0.1, 1.0))
    ///  .set_color ("base_color",   Color128::new(0.05, 0.15, 0.7, 1.0))
    ///  .set_matrix("brightness",   Matrix::IDENTITY);
    /// p.set_texture("out_tex", &tex);
    /// drop(p);
    ///
    /// // Each frame update ring_freq and re-dispatch the shader.
    /// number_of_steps = 4;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     let freq = 0.5 + iter as f32 * 0.25; // animate: 0.5 → 0.75 → 1.0 → 1.25
    ///     compute.get_all_param_info().set_float("ring_freq", freq);
    ///     compute.dispatch(SIZE / 8, SIZE / 8, 1);
    /// );
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn dispatch(&self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        unsafe { compute_dispatch(self.0.as_ptr(), group_count_x, group_count_y, group_count_z) };
    }

    /// The id of this compute object.
    /// <https://stereokit.net/Pages/StereoKit/Compute/Id.html>
    ///
    /// see also [`compute_get_id`]
    /// see example in [`Compute::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(compute_get_id(self.0.as_ptr())) }.to_str().unwrap_or_default()
    }

    /// Gets the shader this Compute was built from.
    /// <https://stereokit.net/Pages/StereoKit/Compute/Shader.html>
    ///
    /// see also [`compute_get_shader`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// // The named param must exist in the shader cbuffer to be read back.
    ///
    /// let shader = compute.get_shader();
    /// assert_eq!(shader.get_id(), "shaders/compute_test.hlsl.sks");
    /// assert_eq!(shader.get_name(), "app/compute_test");
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn get_shader(&self) -> Shader {
        Shader(
            NonNull::new(unsafe { compute_get_shader(self.0.as_ptr()) })
                .expect("compute_get_shader shouldn't return null"),
        )
    }

    /// Returns all shader parameters as an iterable [`ComputeParamInfos`].
    /// Parameters include cbuffer variables, textures and buffers.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetAllParamInfo.html>
    ///
    /// see also [`ComputeParamInfos`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::{Compute};
    ///
    /// # {
    /// // Create a Compute dispatch from a compiled shader file.
    /// let mut compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// compute.id("my_compute");
    ///
    /// assert_eq!(compute.get_id(), "my_compute");
    ///
    /// // Set and round-trip scalar parameters declared in the shader.
    /// let mut param_infos = compute.get_all_param_info();
    /// param_infos.set_float("ring_freq",  1.5)
    ///            .set_int("arm_count",    6)
    ///            .set_uint("tex_size", 512);
    ///
    /// assert_eq!(param_infos.get_float("ring_freq"), 1.5);
    /// assert_eq!(param_infos.get_int("arm_count"),   6);
    /// assert_eq!(param_infos.get_uint("tex_size"),   512);
    ///
    /// // Inspect all parameters exposed by the shader.
    /// let param_infos = compute.get_all_param_info();
    /// assert!(param_infos.get_count() > 0);
    /// for param in param_infos{
    ///     println!("  param: {} ({:?})", param.get_name(), param.get_type());
    /// }
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn get_all_param_info(&self) -> ComputeParamInfos<'_> {
        ComputeParamInfos::from(self)
    }
}

/// Infos of a [`Compute`] shader's parameters — an iterable collection returned by
/// [`Compute::get_all_param_info`]. Each iteration yields a [`crate::material::ParamInfo`].
/// <https://stereokit.net/Pages/StereoKit/Compute/GetAllParamInfo.html>
///
/// see also [`Compute::get_all_param_info`]
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::compute::Compute;
/// use stereokit_rust::maths::{Vec2};
///
/// # {
/// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
///     .expect("shader should be compiled — run `cargo compile_sks`");
///
/// // Set some params and read them back via ComputeParamInfos.
/// let mut param_infos = compute.get_all_param_info();
/// param_infos.set_float("ring_freq", 1.5)
///            .set_vector2("uv_offset", Vec2::new(1.0, 2.0));
///
/// assert_eq!(param_infos.get_float("ring_freq"), 1.5);
/// assert_eq!(param_infos.get_vector2("uv_offset"), Vec2::new(1.0, 2.0));
///
/// // Iterate all params.
/// for param in compute.get_all_param_info() {
///     println!("  param: {} ({:?})", param.get_name(), param.get_type());
/// }
/// # } sk::Sk::shutdown();
/// ```
pub struct ComputeParamInfos<'a> {
    compute: &'a Compute,
    index: i32,
}

impl Iterator for ComputeParamInfos<'_> {
    type Item = ParamInfo;

    /// Yields the next [`ParamInfo`] for this compute shader, or `None` when exhausted.
    ///
    /// see also [`compute_get_param_count`] [`compute_get_param_info`]
    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        let count = unsafe { compute_get_param_count(self.compute.0.as_ptr()) };
        if self.index < count {
            let (name, type_info) = self.get_param_info_impl(self.index);
            Some(ParamInfo::new(name, type_info))
        } else {
            None
        }
    }
}

impl<'a> ComputeParamInfos<'a> {
    /// Creates a [`ComputeParamInfos`] for the given compute shader.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetAllParamInfo.html>
    /// * `compute` - The compute shader to inspect.
    ///
    /// see also [`Compute::get_all_param_info`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::{Compute, ComputeParamInfos};
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let param_infos = ComputeParamInfos::from(&compute);
    /// assert!(param_infos.get_count() > 0);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn from(compute: &'a Compute) -> ComputeParamInfos<'a> {
        ComputeParamInfos { compute, index: -1 }
    }

    fn get_param_info_impl(&self, index: i32) -> (String, MaterialParam) {
        let mut name_ptr: *mut c_char = std::ptr::null_mut();
        let mut type_ = MaterialParam::Unknown;
        unsafe { compute_get_param_info(self.compute.0.as_ptr(), index, &mut name_ptr, &mut type_) };
        let name = if name_ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(name_ptr) }.to_str().unwrap_or("").to_string()
        };
        (name, type_)
    }

    /// Bind a texture to this compute shader by parameter name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetTexture.html>
    ///
    /// see also [`compute_set_texture`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::tex::{Tex, TexType, TexFormat};
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // Output texture written by the shader (must be Compute-typed).
    /// let mut tex = Tex::new(TexType::ImageNomips | TexType::Compute, TexFormat::Rgba128, None);
    /// tex.set_size(8, 8, None, None);
    /// assert!(param_infos.set_texture("out_tex", &tex));
    ///
    /// // Non-compute texture — binding will fail.
    /// let mut tex2 = Tex::new(TexType::ImageNomips, TexFormat::Rgba128, None);
    /// tex2.set_size(8, 8, None, None);
    /// assert!(!param_infos.set_texture("not_valid_tex", &tex2));
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_texture<S: AsRef<str>>(&mut self, name: S, texture: impl AsRef<Tex>) -> bool {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_texture(self.compute.0.as_ptr(), c.as_ptr(), texture.as_ref().0.as_ptr()) != 0 }
    }

    /// Sets a RW/StructuredBuffer or ByteAddressBuffer on the shader. This is used to provide BIG arrays of data to
    /// the GPU, for both reading and writing! These perform very similarly to textures, and can be thought of as big
    /// textures of just data!
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetStorage.html>
    /// * `name` - the name of the shader parameter in the HLSL
    /// * `buffer` - the [`ComputeBuffer`] to bind (an array of `<T>` elements)
    /// * `<T>` - The element type of the cells of buffer.
    ///
    /// see also [`compute_set_storage`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::{Compute, ComputeBuffer, ComputeBufferType};
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // compute_test has no structured buffers, so all set_storage calls return false.
    /// let cells = vec![[1.0_f32, 0.0_f32]; 8 * 8];
    /// let buf_a = ComputeBuffer::with_data(ComputeBufferType::ReadWrite, &cells)
    ///                .expect("should create compute buffer with data");
    /// let buf_b = ComputeBuffer::with_data(ComputeBufferType::ReadWrite, &cells)
    ///                .expect("should create compute buffer with data");
    /// assert!(!param_infos.set_storage("input",        &buf_a));
    /// assert!(!param_infos.set_storage("do_not_exist", &buf_b));
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_storage<T, S: AsRef<str>>(&mut self, name: S, buffer: &ComputeBuffer<T>) -> bool {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_storage(self.compute.0.as_ptr(), c.as_ptr(), buffer.as_ptr()) != 0 }
    }

    /// Sets a constant/uniform buffer (cbuffer) on the shader. This is for smaller chunks of data (16kb max) that can
    /// be read from faster than textures or StructuredBuffers.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetConstant.html>
    /// * `name` - the name of the shader parameter in the HLSL
    /// * `buffer` - the [`MaterialBuffer`] to bind
    /// * `<T>` - The element type of the buffer.
    ///
    /// see also [`compute_set_constant`]
    pub fn set_constant<S: AsRef<str>, T>(&mut self, name: S, buffer: &MaterialBuffer<T>) -> bool {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_constant(self.compute.0.as_ptr(), c.as_ptr(), buffer.as_ptr()) != 0 }
    }

    /// Set a float shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetFloat.html>
    ///
    /// see also [`compute_set_float`] [`ComputeParamInfos::get_float`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // 'ring_freq' is the float parameter of compute_test.
    /// param_infos.set_float("ring_freq",    0.75)
    ///            .set_float("do_not_exist", 0.999);
    ///
    /// assert_eq!(param_infos.get_float("ring_freq"),    0.75);
    /// assert_eq!(param_infos.get_float("do_not_exist"), 0.0);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_float<S: AsRef<str>>(&mut self, name: S, value: f32) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_float(self.compute.0.as_ptr(), c.as_ptr(), value) };
        self
    }

    /// Set an i32 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetInt.html>
    ///
    /// see also [`compute_set_int`] [`ComputeParamInfos::get_int`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    ///
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // The named param must exist in the shader cbuffer to be read back.
    /// // Undeclared params are ignored (get_int returns 0).
    /// param_infos.set_int("arm_count", 123)
    ///            .set_int("do_not_exist", 999);
    ///
    /// assert_eq!(param_infos.get_int("arm_count"), 123);
    /// assert_eq!(param_infos.get_int("do_not_exist"), 0);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_int<S: AsRef<str>>(&mut self, name: S, value: i32) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_int(self.compute.0.as_ptr(), c.as_ptr(), value) };
        self
    }

    /// Set a u32 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetUInt.html>
    ///
    /// see also [`compute_set_uint`] [`ComputeParamInfos::get_uint`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // 'tex_size' is the uint parameter of compute_test.
    /// param_infos.set_uint("tex_size", 512);
    /// param_infos.set_uint("do_not_exist", 999);
    ///
    /// assert_eq!(param_infos.get_uint("tex_size"),     512);
    /// assert_eq!(param_infos.get_uint("do_not_exist"), 0);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_uint<S: AsRef<str>>(&mut self, name: S, value: u32) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_uint(self.compute.0.as_ptr(), c.as_ptr(), value) };
        self
    }

    /// Set a Vec2 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetVector2.html>
    ///
    /// see also [`compute_set_vector2`] [`ComputeParamInfos::get_vector2`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::maths::Vec2;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // The named params must exist in the shader cbuffer to be read back.
    /// param_infos.set_vector2("uv_offset", Vec2::new(1.0, 2.0));
    /// param_infos.set_vector2("do_not_exist", Vec2::new(9.0, 9.0));
    ///
    /// assert_eq!(param_infos.get_vector2("uv_offset"), Vec2::new(1.0, 2.0));
    /// assert_eq!(param_infos.get_vector2("do_not_exist"), Vec2::ZERO);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_vector2<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec2>) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_vector2(self.compute.0.as_ptr(), c.as_ptr(), value.into()) };
        self
    }

    /// Set a Vec3 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetVector3.html>
    ///
    /// see also [`compute_set_vector3`] [`ComputeParamInfos::get_vector3`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::maths::Vec3;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // The named params must exist in the shader cbuffer to be read back.
    /// param_infos.set_vector3("spiral_twist", Vec3::new(1.0, 2.0, 3.0));
    /// param_infos.set_vector3("do_not_exist", Vec3::new(9.0, 9.0, 9.0));
    ///
    /// assert_eq!(param_infos.get_vector3("spiral_twist"), Vec3::new(1.0, 2.0, 3.0));
    /// assert_eq!(param_infos.get_vector3("do_not_exist"), Vec3::ZERO);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_vector3<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec3>) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_vector3(self.compute.0.as_ptr(), c.as_ptr(), value.into()) };
        self
    }

    /// Set a Vec4 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetVector4.html>
    ///
    /// see also [`compute_set_vector4`] [`ComputeParamInfos::get_vector4`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::maths::Vec4;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // The named params must exist in the shader cbuffer to be read back.
    /// param_infos.set_vector4("highlight", Vec4::new(1.0, 2.0, 3.0, 4.0));
    /// param_infos.set_vector4("do_not_exist", Vec4::new(9.0, 9.0, 9.0, 9.0));
    ///
    /// assert_eq!(param_infos.get_vector4("highlight"), Vec4::new(1.0, 2.0, 3.0, 4.0));
    /// assert_eq!(param_infos.get_vector4("do_not_exist"), Vec4::ZERO);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_vector4<S: AsRef<str>>(&mut self, name: S, value: impl Into<Vec4>) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_vector4(self.compute.0.as_ptr(), c.as_ptr(), value.into()) };
        self
    }

    /// Set a Color128 shader parameter by name (gamma-space).
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetColor.html>
    ///
    /// see also [`compute_set_color`] [`ComputeParamInfos::get_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::util::Color128;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    ///
    /// // The named param must exist in the shader cbuffer to be read back.
    /// param_infos.set_color("base_color", Color128::new(1.0, 0.5, 0.25, 1.0).to_gamma());
    /// param_infos.set_color("do_not_exist", Color128::new(0.5, 0.5, 0.5, 0.5).to_gamma());
    ///
    /// assert_eq!(param_infos.get_color("base_color"), Color128::new(1.0, 0.5, 0.25, 1.0));
    /// assert_eq!(param_infos.get_color("do_not_exist"), Color128::WHITE);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_color<S: AsRef<str>>(&mut self, name: S, color_gamma: impl Into<Color128>) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_color(self.compute.0.as_ptr(), c.as_ptr(), color_gamma.into()) };
        self
    }

    /// Set a bool shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetBool.html>
    ///
    /// see also [`compute_set_bool`] [`ComputeParamInfos::get_bool`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    /// // The named param must exist in the shader cbuffer to be read back.
    /// assert_eq!(param_infos.get_bool("center_glow"), false);
    /// param_infos.set_bool("center_glow", true);
    /// param_infos.set_bool("do_not_exist", true);
    ///
    /// assert_eq!(param_infos.get_bool("center_glow"), true);
    /// assert_eq!(param_infos.get_bool("do_not_exist"), false);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_bool<S: AsRef<str>>(&mut self, name: S, value: bool) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_bool(self.compute.0.as_ptr(), c.as_ptr(), value as Bool32T) };
        self
    }

    /// Set a Matrix shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/SetMatrix.html>
    ///
    /// see also [`compute_set_matrix`] [`ComputeParamInfos::get_matrix`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    /// use stereokit_rust::maths::Matrix;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let mut param_infos = compute.get_all_param_info();
    /// // The named param must exist in the shader cbuffer to be read back.
    /// param_infos.set_matrix("brightness", Matrix::Y_180);
    /// param_infos.set_matrix("do_not_exist", Matrix::X_180);
    ///
    /// assert_eq!(param_infos.get_matrix("brightness"), Matrix::Y_180);
    /// assert_eq!(param_infos.get_matrix("do_not_exist"), Matrix::IDENTITY);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_matrix<S: AsRef<str>>(&mut self, name: S, value: impl Into<Matrix>) -> &mut Self {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_set_matrix(self.compute.0.as_ptr(), c.as_ptr(), value.into()) };
        self
    }

    /// Get the total number of shader parameters for this compute shader.
    /// <https://stereokit.net/Pages/StereoKit/Compute/ParamCount.html>
    ///
    /// see also [`compute_get_param_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::Compute;
    ///
    /// # {
    /// let compute = Compute::from_file("shaders/compute_test.hlsl.sks")
    ///     .expect("shader should be compiled — run `cargo compile_sks`");
    /// let count = compute.get_all_param_info().get_count();
    /// assert!(count > 0);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe { compute_get_param_count(self.compute.0.as_ptr()) }
    }

    /// Get a float shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetFloat.html>
    ///
    /// see also [`compute_get_float`]
    /// see example in [`ComputeParamInfos::set_float`]
    pub fn get_float<S: AsRef<str>>(&self, name: S) -> f32 {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_float(self.compute.0.as_ptr(), c.as_ptr()) }
    }

    /// Get an i32 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetInt.html>
    ///
    /// see also [`compute_get_int`]
    /// see example in [`ComputeParamInfos::set_int`]
    pub fn get_int<S: AsRef<str>>(&self, name: S) -> i32 {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_int(self.compute.0.as_ptr(), c.as_ptr()) }
    }

    /// Get a u32 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetUInt.html>
    ///
    /// see also [`compute_get_uint`]
    /// see example in [`ComputeParamInfos::set_uint`]
    pub fn get_uint<S: AsRef<str>>(&self, name: S) -> u32 {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_uint(self.compute.0.as_ptr(), c.as_ptr()) }
    }

    /// Get a Vec2 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetVector2.html>
    ///
    /// see also [`compute_get_vector2`]
    /// see example in [`ComputeParamInfos::set_vector2`]
    pub fn get_vector2<S: AsRef<str>>(&self, name: S) -> Vec2 {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_vector2(self.compute.0.as_ptr(), c.as_ptr()) }
    }

    /// Get a Vec3 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetVector3.html>
    ///
    /// see also [`compute_get_vector3`]
    /// see example in [`ComputeParamInfos::set_vector3`]
    pub fn get_vector3<S: AsRef<str>>(&self, name: S) -> Vec3 {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_vector3(self.compute.0.as_ptr(), c.as_ptr()) }
    }

    /// Get a Vec4 shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetVector4.html>
    ///
    /// see also [`compute_get_vector4`]
    /// see example in [`ComputeParamInfos::set_vector4`]
    pub fn get_vector4<S: AsRef<str>>(&self, name: S) -> Vec4 {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_vector4(self.compute.0.as_ptr(), c.as_ptr()) }
    }

    /// Get a Color128 shader parameter by name (gamma-space).
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetColor.html>
    ///
    /// see also [`compute_get_color`]
    /// see example in [`ComputeParamInfos::set_color`]
    pub fn get_color<S: AsRef<str>>(&self, name: S) -> Color128 {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_color(self.compute.0.as_ptr(), c.as_ptr()) }
    }

    /// Get a bool shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetBool.html>
    ///
    /// see also [`compute_get_bool`]
    /// see example in [`ComputeParamInfos::set_bool`]
    pub fn get_bool<S: AsRef<str>>(&self, name: S) -> bool {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_bool(self.compute.0.as_ptr(), c.as_ptr()) != 0 }
    }

    /// Get a Matrix shader parameter by name.
    /// <https://stereokit.net/Pages/StereoKit/Compute/GetMatrix.html>
    ///
    /// see also [`compute_get_matrix`]
    /// see example in [`ComputeParamInfos::set_matrix`]
    pub fn get_matrix<S: AsRef<str>>(&self, name: S) -> Matrix {
        let c = CString::new(name.as_ref()).unwrap_or_default();
        unsafe { compute_get_matrix(self.compute.0.as_ptr(), c.as_ptr()) }
    }
}

/// Determines read/write access mode for a [`ComputeBuffer`] from compute shaders.
/// <https://stereokit.net/Pages/StereoKit/ComputeBufferType.html>
///
/// see also [`ComputeBuffer`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum ComputeBufferType {
    /// Read-only from compute shaders. Maps to `StructuredBuffer<T>` in HLSL.
    Read = 1,
    /// Read-write from compute shaders. Maps to `RWStructuredBuffer<T>` in HLSL.
    ReadWrite = 2,
}

/// A GPU storage buffer for shuttling data to and from compute shaders! In HLSL, this maps to
/// `StructuredBuffer<T>` or `RWStructuredBuffer<T>` depending on the [`ComputeBufferType`].
/// <https://stereokit.net/Pages/StereoKit/ComputeBuffer.html>
///
/// see also [`Compute`]
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::compute::{ComputeBuffer, ComputeBufferType};
/// use std::mem::size_of;
///
/// // The element type must be #[repr(C)] to guarantee a predictable memory layout for the GPU.
/// #[repr(C)]
/// #[derive(Clone, Copy, Debug, Default, PartialEq)]
/// struct Particle { x: f32, y: f32, vx: f32, vy: f32 }
///
/// # {
/// // Create an empty ReadWrite buffer
/// let mut buf: ComputeBuffer<Particle> = ComputeBuffer::new(
///     ComputeBufferType::ReadWrite,
///     4,                              // 4 elements
///     size_of::<Particle>() as i32,   // 16 bytes per element
/// ).expect("Failed to create ComputeBuffer");
/// buf.id("my_particles");
///
/// assert_eq!(buf.get_id(),     "my_particles");
/// assert_eq!(buf.get_count(),  4);
/// assert_eq!(buf.get_stride(), 16);
///
/// // Upload data to the GPU
/// let src = [
///     Particle { x: 0.0, y: 1.0, vx:  1.0, vy:  0.0 },
///     Particle { x: 1.0, y: 0.0, vx:  0.0, vy:  1.0 },
/// ];
/// buf.set_data(&src);
///
/// // Read back from the GPU
/// let dst = buf.get_data();
/// assert_eq!(dst.len(), 4);
/// assert_eq!(dst[0].x, 0.0);
/// assert_eq!(dst[1].y, 0.0);
///
/// // clone_ref shares the same underlying GPU allocation
/// let clone = buf.clone_ref();
/// assert_eq!(clone.get_id(),    "my_particles");
/// assert_eq!(clone.get_count(), 4);
/// # } sk::Sk::shutdown();
/// ```
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct ComputeBuffer<T> {
    _compute_buffer: ComputeBufferT,
    phantom: PhantomData<T>,
}
impl<T> Drop for ComputeBuffer<T> {
    fn drop(&mut self) {
        unsafe { compute_buffer_release(self._compute_buffer) };
    }
}
impl<T> AsRef<ComputeBuffer<T>> for ComputeBuffer<T> {
    fn as_ref(&self) -> &ComputeBuffer<T> {
        self
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _ComputeBufferT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type ComputeBufferT = *mut _ComputeBufferT;

unsafe extern "C" {
    pub fn compute_buffer_create(
        type_: ComputeBufferType,
        element_count: i32,
        element_size: i32,
        opt_initial_data: *const c_void,
    ) -> ComputeBufferT;
    pub fn compute_buffer_set_id(buffer: ComputeBufferT, id: *const c_char);
    pub fn compute_buffer_get_id(buffer: ComputeBufferT) -> *const c_char;
    pub fn compute_buffer_set_data(buffer: ComputeBufferT, data: *const c_void, element_count: i32);
    pub fn compute_buffer_get_data(buffer: ComputeBufferT, out_data: *mut c_void, element_count: i32);
    pub fn compute_buffer_get_count(buffer: ComputeBufferT) -> i32;
    pub fn compute_buffer_get_stride(buffer: ComputeBufferT) -> i32;
    pub fn compute_buffer_addref(buffer: ComputeBufferT);
    pub fn compute_buffer_release(buffer: ComputeBufferT);
}

impl<T> IAsset for ComputeBuffer<T> {
    fn get_id(&self) -> &str {
        self.get_id()
    }
}

impl<T> ComputeBuffer<T> {
    /// Creates an empty GPU storage buffer with the given element count and element stride.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/ComputeBuffer.html>
    ///
    /// see also [`compute_buffer_create`]
    /// see example in [`ComputeBuffer`]
    pub fn new(type_: ComputeBufferType, element_count: i32, element_size: i32) -> Result<Self, StereoKitError> {
        let ptr = unsafe { compute_buffer_create(type_, element_count, element_size, std::ptr::null()) };
        if ptr.is_null() {
            Err(StereoKitError::ComputeBufferCreate("compute_buffer_create failed".to_string()))
        } else {
            Ok(ComputeBuffer { _compute_buffer: ptr, phantom: PhantomData })
        }
    }

    /// Gets or sets the unique identifier of this asset resource.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/Id.html>
    ///
    /// see also [`compute_buffer_set_id`]
    /// see example in [`ComputeBuffer`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap_or_default();
        unsafe { compute_buffer_set_id(self._compute_buffer, c_str.as_ptr()) };
        self
    }

    /// Creates a GPU storage buffer and immediately uploads `data` to it. The buffer capacity is
    /// set to `data.len()` elements.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/ComputeBuffer.html>
    ///
    /// see also [`compute_buffer_create`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::{ComputeBuffer, ComputeBufferType};
    ///
    /// # {
    /// // Upload an initial Vec<f32> at creation time.
    /// let initial: Vec<f32> = (0..8).map(|i| i as f32).collect();
    /// let buf = ComputeBuffer::with_data(ComputeBufferType::ReadWrite, &initial)
    ///                         .expect("Failed to create ComputeBuffer");
    ///
    /// assert_eq!(buf.get_count(),  8);
    /// assert_eq!(buf.get_stride(), 4); // f32 = 4 bytes
    ///
    /// let readback = buf.get_data();
    /// assert_eq!(readback[3], 3.0_f32);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn with_data(type_: ComputeBufferType, data: &[T]) -> Result<Self, StereoKitError>
    where
        T: Sized,
    {
        let element_size = size_of::<T>() as i32;
        let element_count = data.len() as i32;
        let ptr = unsafe { compute_buffer_create(type_, element_count, element_size, data.as_ptr() as *const c_void) };
        if ptr.is_null() {
            Err(StereoKitError::ComputeBufferCreate("compute_buffer_create failed".to_string()))
        } else {
            Ok(ComputeBuffer { _compute_buffer: ptr, phantom: PhantomData })
        }
    }

    /// Upload new data to the GPU buffer. Uploads at most `data.len()` or the buffer capacity,
    /// whichever is smaller.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/SetData.html>
    ///
    /// see also [`compute_buffer_set_data`] [`ComputeBuffer::get_data`] [`ComputeBuffer::get_data_into`]
    /// see example in [`ComputeBuffer`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way
    /// use stereokit_rust::compute::{ComputeBuffer, ComputeBufferType};
    /// use std::mem::size_of;
    ///
    /// #[repr(C)]
    /// #[derive(Clone, Copy, Debug, Default, PartialEq)]
    /// struct Particle { x: f32, y: f32, vx: f32, vy: f32 }
    ///
    /// # {
    /// let mut buf: ComputeBuffer<Particle> = ComputeBuffer::new(
    ///     ComputeBufferType::ReadWrite,
    ///     4,                              // 4 elements
    ///     size_of::<Particle>() as i32,   // 16 bytes per element
    /// ).expect("Failed to create ComputeBuffer");
    /// let src = [
    ///     Particle { x: 5.0, y: 1.0, vx:  1.0, vy:  0.0 },
    ///     Particle { x: 1.0, y: 2.0, vx:  0.0, vy:  1.0 },
    /// ];
    /// buf.set_data(&src);
    /// let readback = buf.get_data();
    /// assert_eq!(readback[0].x, 5.0);
    /// assert_eq!(readback[1].y, 2.0);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn set_data(&mut self, data: &[T]) {
        unsafe {
            compute_buffer_set_data(self._compute_buffer, data.as_ptr() as *const c_void, data.len() as i32);
        }
    }

    /// Read the full buffer back from the GPU into a freshly allocated `Vec`. Blocks until ready. For per-frame
    /// readbacks, prefer the get_data_into version to avoid allocations.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/GetData.html>
    ///
    /// see also [`compute_buffer_get_data`] [`ComputeBuffer::get_data_into`]
    /// see example in [`ComputeBuffer`] [`ComputeBuffer::with_data`] [`ComputeBuffer::set_data`]
    pub fn get_data(&self) -> Vec<T>
    where
        T: Sized + Default + Clone,
    {
        let count = self.get_count() as usize;
        let mut data = vec![T::default(); count];
        unsafe {
            compute_buffer_get_data(self._compute_buffer, data.as_mut_ptr() as *mut c_void, count as i32);
        }
        data
    }

    /// Read GPU data into a pre-allocated slice.  This is the allocation-free version of GetData, great for calling
    /// every frame without creating new allocations. Reads `min(out.len(), capacity)` elements.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/GetData.html>
    ///
    /// see also [`compute_buffer_get_data`] [`ComputeBuffer::get_data`] [`ComputeBuffer::set_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::compute::{ComputeBuffer, ComputeBufferType};
    ///
    /// # {
    /// // Initialise a buffer with 4 f32 values.
    /// let src: Vec<f32> = (0..4).map(|i| i as f32 * 10.0).collect(); // [0, 10, 20, 30]
    /// let buf = ComputeBuffer::with_data(ComputeBufferType::ReadWrite, &src)
    ///                         .expect("Failed to create ComputeBuffer");
    ///
    /// // Read only the first 2 elements into a pre-allocated array.
    /// let mut out = [0.0_f32; 2];
    /// buf.get_data_into(&mut out);
    ///
    /// assert_eq!(out[0], 0.0);
    /// assert_eq!(out[1], 10.0);
    /// # } sk::Sk::shutdown();
    /// ```
    pub fn get_data_into(&self, out: &mut [T])
    where
        T: Sized,
    {
        unsafe {
            compute_buffer_get_data(self._compute_buffer, out.as_mut_ptr() as *mut c_void, out.len() as i32);
        }
    }

    /// Number of elements this buffer can hold.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/Count.html>
    ///
    /// see also [`compute_buffer_get_count`]
    /// see example in [`ComputeBuffer`] [`ComputeBuffer::with_data`]
    pub fn get_count(&self) -> i32 {
        unsafe { compute_buffer_get_count(self._compute_buffer) }
    }

    /// Size in bytes of a single element.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/Stride.html>
    ///
    /// see also [`compute_buffer_get_stride`]
    /// see example in [`ComputeBuffer`] [`ComputeBuffer::with_data`]
    pub fn get_stride(&self) -> i32 {
        unsafe { compute_buffer_get_stride(self._compute_buffer) }
    }

    /// The id of this compute buffer.
    /// <https://stereokit.net/Pages/StereoKit/ComputeBuffer/Id.html>
    ///
    /// see also [`compute_buffer_get_id`]
    /// see example in [`ComputeBuffer`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(compute_buffer_get_id(self._compute_buffer)) }.to_str().unwrap_or_default()
    }

    /// Creates a clone of the same reference. Basically the new variable is the same asset.
    ///
    /// see also [`compute_buffer_addref`]
    /// see example in [`ComputeBuffer`]
    pub fn clone_ref(&self) -> ComputeBuffer<T> {
        unsafe { compute_buffer_addref(self._compute_buffer) };
        ComputeBuffer { _compute_buffer: self._compute_buffer, phantom: PhantomData }
    }

    /// Returns the raw internal FFI pointer.
    pub fn as_ptr(&self) -> ComputeBufferT {
        self._compute_buffer
    }
}

impl ComputeBuffer<()> {
    /// Wraps a raw FFI pointer without incrementing the refcount. For internal use (Assets iterator).
    pub(crate) fn from_raw(ptr: ComputeBufferT) -> Self {
        ComputeBuffer { _compute_buffer: ptr, phantom: PhantomData }
    }
}
