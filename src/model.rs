use crate::maths::{Bool32T, Matrix};
use crate::sk::MainThreadToken;
use crate::{
    StereoKitError,
    material::{Cull, Material, MaterialT},
    maths::{Bounds, Ray, Vec3},
    mesh::{Mesh, MeshT},
    shader::{Shader, ShaderT},
    system::{IAsset, Log, RenderLayer},
    util::Color128,
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    path::Path,
    ptr::{NonNull, null_mut},
};

/// A Model is a collection of meshes, materials, and transforms that make up a visual element! This is a great way to
/// group together complex objects that have multiple parts in them, and in fact, most model formats are composed this
/// way already!
///
/// This class contains a number of methods for creation. If you pass in a .obj, .stl, , .ply (ASCII), .gltf, or .glb,
/// StereoKit will load that model from file, and assemble materials and transforms from the file information. But you
/// can also assemble a model from procedurally generated meshes!
///
/// Because models include an offset transform for each mesh element, this does have the overhead of an extra matrix
/// multiplication in order to execute a render command. So if you need speed, and only have a single mesh with a
/// precalculated transform matrix, it can be faster to render a Mesh instead of a Model!
/// <https://stereokit.net/Pages/StereoKit/Model.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh, material::Material};
///
/// let sphere =       Mesh::generate_sphere(0.6, None);
/// let rounded_cube = Mesh::generate_rounded_cube(Vec3::ONE * 0.6, 0.2, None);
/// let cylinder =     Mesh::generate_cylinder(0.25, 0.6, Vec3::Z, None);
///
/// let transform1 = Matrix::t([-0.7,-0.5, 0.0]);
/// let transform2 = Matrix::t([ 0.0, 0.0, 0.0]);
/// let transform3 = Matrix::t([ 0.7, 0.5, 0.0]);
///
/// let material = Material::pbr();
///
/// let model = Model::new();
/// let mut nodes = model.get_nodes();
/// nodes.add("sphere",   transform1 , Some(&sphere),       Some(&material), true)
///      .add("cube",     transform2 , Some(&rounded_cube), Some(&material), true)
///      .add("cylinder", transform3 , Some(&cylinder),     Some(&material), true);
///
/// filename_scr = "screenshots/model.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     model.draw(token, Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model.jpeg" alt="screenshot" width="200">
#[derive(Debug, PartialEq)]
pub struct Model(pub NonNull<_ModelT>);
impl Drop for Model {
    fn drop(&mut self) {
        unsafe { model_release(self.0.as_ptr()) }
    }
}
/// AsRef
impl AsRef<Model> for Model {
    fn as_ref(&self) -> &Model {
        self
    }
}
/// From / Into
impl From<Model> for ModelT {
    fn from(val: Model) -> Self {
        val.0.as_ptr()
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _ModelT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type ModelT = *mut _ModelT;

unsafe extern "C" {
    pub fn model_find(id: *const c_char) -> ModelT;
    pub fn model_copy(model: ModelT) -> ModelT;
    pub fn model_create() -> ModelT;
    pub fn model_create_mesh(mesh: MeshT, material: MaterialT) -> ModelT;
    pub fn model_create_mem(
        filename_utf8: *const c_char,
        data: *const c_void,
        data_size: usize,
        shader: ShaderT,
    ) -> ModelT;
    pub fn model_create_file(filename_utf8: *const c_char, shader: ShaderT) -> ModelT;
    pub fn model_set_id(model: ModelT, id: *const c_char);
    pub fn model_get_id(model: ModelT) -> *const c_char;
    pub fn model_addref(model: ModelT);
    pub fn model_release(model: ModelT);
    pub fn model_draw(model: ModelT, transform: Matrix, color_linear: Color128, layer: RenderLayer);
    pub fn model_draw_mat(
        model: ModelT,
        material_override: MaterialT,
        transform: Matrix,
        color_linear: Color128,
        layer: RenderLayer,
    );
    pub fn model_recalculate_bounds(model: ModelT);
    pub fn model_recalculate_bounds_exact(model: ModelT);
    pub fn model_set_bounds(model: ModelT, bounds: *const Bounds);
    pub fn model_get_bounds(model: ModelT) -> Bounds;
    pub fn model_ray_intersect(model: ModelT, model_space_ray: Ray, cull_mode: Cull, out_pt: *mut Ray) -> Bool32T;
    pub fn model_ray_intersect_bvh(model: ModelT, model_space_ray: Ray, cull_mode: Cull, out_pt: *mut Ray) -> Bool32T;
    pub fn model_ray_intersect_bvh_detailed(
        model: ModelT,
        model_space_ray: Ray,
        cull_mode: Cull,
        out_pt: *mut Ray,
        out_mesh: *mut MeshT,
        out_matrix: *mut Matrix,
        out_start_inds: *mut u32,
    ) -> Bool32T;
    pub fn model_step_anim(model: ModelT);
    pub fn model_play_anim(model: ModelT, animation_name: *const c_char, mode: AnimMode) -> Bool32T;
    pub fn model_play_anim_idx(model: ModelT, index: i32, mode: AnimMode);
    pub fn model_set_anim_time(model: ModelT, time: f32);
    pub fn model_set_anim_completion(model: ModelT, percent: f32);
    pub fn model_anim_find(model: ModelT, animation_name: *const c_char) -> i32;
    pub fn model_anim_count(model: ModelT) -> i32;
    pub fn model_anim_active(model: ModelT) -> i32;
    pub fn model_anim_active_mode(model: ModelT) -> AnimMode;
    pub fn model_anim_active_time(model: ModelT) -> f32;
    pub fn model_anim_active_completion(model: ModelT) -> f32;
    pub fn model_anim_get_name(model: ModelT, index: i32) -> *const c_char;
    pub fn model_anim_get_duration(model: ModelT, index: i32) -> f32;
    // Deprecated :pub fn model_get_name(model: ModelT, subset: i32) -> *const c_char;
    // Deprecated :pub fn model_get_material(model: ModelT, subset: i32) -> MaterialT;
    // Deprecated :pub fn model_get_mesh(model: ModelT, subset: i32) -> MeshT;
    // Deprecated :pub fn model_get_transform(model: ModelT, subset: i32) -> Matrix;
    // Deprecated :pub fn model_set_material(model: ModelT, subset: i32, material: MaterialT);
    // Deprecated :pub fn model_set_mesh(model: ModelT, subset: i32, mesh: MeshT);
    // Deprecated :pub fn model_set_transform(model: ModelT, subset: i32, transform: *const Matrix);
    // Deprecated :pub fn model_remove_subset(model: ModelT, subset: i32);
    // Deprecated :pub fn model_add_named_subset(
    //     model: ModelT,
    //     name: *const c_char,
    //     mesh: MeshT,
    //     material: MaterialT,
    //     transform: *const Matrix,
    // ) -> i32;
    // Deprecated :pub fn model_add_subset(model: ModelT, mesh: MeshT, material: MaterialT, transform: *const Matrix) -> i32;
}

impl IAsset for Model {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }
}

impl Default for Model {
    /// Create an empty model
    /// <https://stereokit.net/Pages/StereoKit/Model/Model.html>
    ///
    /// see also [`Model::new`]
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Create an empty model
    /// <https://stereokit.net/Pages/StereoKit/Model/Model.html>
    ///
    /// see also [`model_create`] [`Model::default`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh, material::Material};
    ///
    /// // Let's create a model with two identical spheres.
    /// let sphere =       Mesh::generate_sphere(0.6, None);
    ///
    /// let transform_mesh1  = Matrix::t([-0.7,-0.5, 0.0]);
    /// let transform_mesh2  = Matrix::t([ 0.7, 0.5, 0.0]);
    ///
    /// let transform_model  = Matrix::r([ 0.0, 180.0, 0.0]);
    ///
    /// let material = Material::pbr();
    ///
    /// let model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// assert_eq!(nodes.get_count(), 0);
    ///
    /// nodes.add("sphere1", transform_mesh1, Some(&sphere), Some(&material), true);
    /// nodes.add("sphere2", transform_mesh2, Some(&sphere), Some(&material), true);
    /// assert_eq!(nodes.get_count(), 2);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, transform_model, None, None);
    /// );
    /// ```
    pub fn new() -> Model {
        Model(NonNull::new(unsafe { model_create() }).unwrap())
    }

    /// Creates a single mesh subset Model using the indicated Mesh and Material!
    /// An id will be automatically generated for this asset.
    /// <https://stereokit.net/Pages/StereoKit/Model/FromMesh.html>
    /// * `mesh` - The mesh to use for this model.
    /// * `material` - The material to use for this mesh.
    ///
    /// see also [`model_create_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh, material::Material};
    ///
    /// // Let's create a model with two identical spheres.
    /// let sphere =       Mesh::generate_sphere(0.8, None);
    ///
    /// let transform_mesh1  = Matrix::t([-0.5,-0.5, 0.0]);
    /// let transform_mesh2  = Matrix::t([ 0.5, 0.5, 0.0]);
    ///
    /// let transform_model  = Matrix::r([ 0.0, 150.0, 0.0]);
    ///
    /// let material = Material::pbr();
    ///
    /// let model = Model::from_mesh(&sphere, &material);
    /// let mut nodes = model.get_nodes();
    /// assert_eq!(nodes.get_count(), 1);
    ///
    /// nodes.add("sphere2", transform_mesh1, Some(&sphere), Some(&material), true);
    /// nodes.add("sphere3", transform_mesh2, Some(&sphere), Some(&material), true);
    /// assert_eq!(nodes.get_count(), 3);
    ///
    /// filename_scr = "screenshots/model_from_mesh.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, transform_model, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_mesh.jpeg" alt="screenshot" width="200">
    pub fn from_mesh<Me: AsRef<Mesh>, Ma: AsRef<Material>>(mesh: Me, material: Ma) -> Model {
        Model(
            NonNull::new(unsafe { model_create_mesh(mesh.as_ref().0.as_ptr(), material.as_ref().0.as_ptr()) }).unwrap(),
        )
    }

    /// Loads a list of mesh and material subsets from a .obj, .stl, .ply (ASCII), .gltf, or .glb file stored in memory.
    /// Note that this function won’t work well on files that reference other files, such as .gltf files with
    /// references in them.
    /// <https://stereokit.net/Pages/StereoKit/Model/FromMemory.html>
    /// * `file_name` - StereoKit still uses the filename of the data for format discovery, but not asset Id creation.
    ///   If you don’t have a real filename for the data, just pass in an extension with a leading ‘.’ character here,
    ///   like “.glb”.
    /// * `data` - The binary data of a model file, this is NOT a raw array of vertex and index data!
    /// * `shader` - The shader to use for the model's materials!, if None, this will automatically determine the best
    ///   available shader to use.
    ///
    /// see also [`model_create_mem`] [`Model::from_file`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, util::named_colors};
    ///
    /// let my_bytes = std::include_bytes!("../assets/plane.glb");
    ///
    /// let model = Model::from_memory("my_bytes_center.glb", my_bytes, None).unwrap().copy();
    /// let transform = Matrix::t_r_s(Vec3::Y * 0.10, [0.0, 110.0, 0.0], Vec3::ONE * 0.09);
    ///
    /// filename_scr = "screenshots/model_from_memory.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, transform, Some(named_colors::GREEN.into()), None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_memory.jpeg" alt="screenshot" width="200">    
    pub fn from_memory<S: AsRef<str>>(
        file_name: S,
        data: &[u8],
        shader: Option<Shader>,
    ) -> Result<Model, StereoKitError> {
        let c_file_name = CString::new(file_name.as_ref())?;
        let shader = shader.map(|shader| shader.0.as_ptr()).unwrap_or(null_mut());
        match NonNull::new(unsafe {
            model_create_mem(c_file_name.as_ptr(), data.as_ptr() as *const c_void, data.len(), shader)
        }) {
            Some(model) => Ok(Model(model)),
            None => Err(StereoKitError::ModelFromMem(file_name.as_ref().to_owned(), "file not found!".to_owned())),
        }
    }

    /// Loads a list of mesh and material subsets from a .obj, .stl, .ply (ASCII), .gltf, or .glb file.
    ///
    /// **Important**: The model is loaded only once. If you open the same file a second time, it will return the model
    /// loaded the first time and all its modifications afterwards. If you want two different instances, remember to
    /// copy the model while not forgetting that the assets that the copies contain when loaded are the same.
    /// <https://stereokit.net/Pages/StereoKit/Model/FromFile.html>
    /// * `file` - Name of the file to load! This gets prefixed with the StereoKit asset folder if no drive letter
    ///   is specified in the path.
    /// * `shader` - The shader to use for the model’s materials! If None, this will automatically determine the best
    ///   shader available to use.
    ///
    /// see also [`model_create_file`] [`Model::from_memory`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                              .expect("Could not load model").copy();
    /// let transform = Matrix::t_r_s(Vec3::NEG_Y * 0.40, [0.0, 190.0, 0.0], Vec3::ONE * 0.25);
    ///
    /// filename_scr = "screenshots/model_from_file.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, transform, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_file.jpeg" alt="screenshot" width="200">
    pub fn from_file(file: impl AsRef<Path>, shader: Option<Shader>) -> Result<Model, StereoKitError> {
        let path = file.as_ref();
        let path_buf = path.to_path_buf();
        let c_str = CString::new(path.to_str().unwrap())?;
        let shader = shader.map(|shader| shader.0.as_ptr()).unwrap_or(null_mut());
        match NonNull::new(unsafe { model_create_file(c_str.as_ptr(), shader) }) {
            Some(model) => Ok(Model(model)),
            None => Err(StereoKitError::ModelFromFile(path_buf.to_owned(), "file not found!".to_owned())),
        }
    }
    /// Creates a shallow copy of a Model asset! Meshes and Materials referenced by this Model will be referenced, not
    /// copied.
    /// <https://stereokit.net/Pages/StereoKit/Model/Copy.html>
    ///
    /// see also [`model_copy()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model};
    ///
    /// let model = Model::from_file("center.glb", None).expect("Model should load");
    /// let model_copy = model.copy();
    ///
    /// assert_ne!(model, model_copy);
    /// assert_eq!(model.get_nodes().get_count(), model_copy.get_nodes().get_count());
    ///
    /// // The nodes contains the same meshes as these are not copied.
    /// assert_eq!(model.get_nodes().get_index(0).expect("Node should exist").get_mesh(),
    ///            model_copy.get_nodes().get_index(0).expect("Node should exist").get_mesh());
    ///
    /// assert_eq!(model.get_nodes().get_index(2).expect("Node should exist").get_mesh(),
    ///            model_copy.get_nodes().get_index(2).expect("Node should exist").get_mesh());
    /// ```
    pub fn copy(&self) -> Model {
        Model(NonNull::new(unsafe { model_copy(self.0.as_ptr()) }).unwrap())
    }

    /// Looks for a Model asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Model/Find.html>
    /// * `id` - Which Model are you looking for?
    ///
    /// see also [`model_find`] [`Model::clone_ref`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model};
    ///
    /// let mut model = Model::from_file("center.glb", None).expect("Model should load");
    /// model.id("my_model_id");
    ///
    /// let same_model = Model::find("my_model_id").expect("Model should be found");
    ///
    /// assert_eq!(model, same_model);
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<Model, StereoKitError> {
        let c_str = CString::new(id.as_ref())?;
        match NonNull::new(unsafe { model_find(c_str.as_ptr()) }) {
            Some(model) => Ok(Model(model)),
            None => Err(StereoKitError::ModelFind(id.as_ref().to_owned())),
        }
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Model/Find.html>
    ///
    /// see also [`model_find`] [`Model::find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model};
    ///
    /// let mut model = Model::from_file("center.glb", None).expect("Model should load");
    ///
    /// let same_model = model.clone_ref();
    ///
    /// assert_eq!(model, same_model);
    /// ```
    pub fn clone_ref(&self) -> Model {
        Model(NonNull::new(unsafe { model_find(model_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    //-----------------Modify Model :
    /// Set a new id to the model.
    /// <https://stereokit.net/Pages/StereoKit/Model/Id.html>
    ///
    /// see also [`model_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model};
    ///
    /// let mut model = Model::new();
    /// assert!(model.get_id().starts_with("auto/model_"));
    ///
    /// model.id("my_model_id");
    /// assert_eq!(model.get_id(), "my_model_id");
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { model_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Set the bounds of this model. This is a bounding box that encapsulates the Model and all its subsets! It’s used for collision,
    /// visibility testing, UI layout, and probably other things. While it’s normally calculated from the mesh bounds, you can also override this to suit your needs.
    /// <https://stereokit.net/Pages/StereoKit/Model/Bounds.html>
    ///
    /// see also [`model_set_bounds`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Bounds}, model::Model, mesh::Mesh,
    ///                      material::Material, util::named_colors};
    ///
    /// let cube_bounds  = Mesh::cube();
    ///
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.3, None);
    ///
    /// let transform1 = Matrix::t([-0.30,-0.30,-0.30]);
    /// let transform2 = Matrix::t([ 0.30, 0.30, 0.30]);
    ///
    /// let material = Material::pbr();
    /// let mut model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("cube1", transform1, Some(&cube), Some(&material), true)
    ///      .add("cube2", transform2, Some(&cube), Some(&material), true);
    ///
    /// let mut material_before = Material::ui_box();
    /// material_before .color_tint(named_colors::GOLD)
    ///                 .get_all_param_info().set_float("border_size", 0.025);
    ///
    /// let mut material_after = material_before.copy();
    /// material_after.color_tint(named_colors::RED);
    ///
    /// let bounds = model.get_bounds();
    /// let transform_before = Matrix::t_s( bounds.center, bounds.dimensions);
    /// assert_eq!(bounds.center, Vec3::ZERO);
    /// assert_eq!(bounds.dimensions, Vec3::ONE * 0.9);
    ///
    /// // let's reduce the bounds to the upper cube only
    /// model.bounds( Bounds::new([0.30, 0.30, 0.30].into(), Vec3::ONE * 0.301));
    /// let new_bounds = model.get_bounds();
    /// let transform_after = Matrix::t_s( new_bounds.center, new_bounds.dimensions);
    ///
    /// filename_scr = "screenshots/model_bounds.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, Matrix::IDENTITY, None, None);
    ///     cube_bounds.draw(  token, &material_before, transform_before, None, None);
    ///     cube_bounds.draw(  token, &material_after,  transform_after,  None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_bounds.jpeg" alt="screenshot" width="200">
    pub fn bounds(&mut self, bounds: impl AsRef<Bounds>) -> &mut Self {
        unsafe { model_set_bounds(self.0.as_ptr(), bounds.as_ref()) };
        self
    }

    /// Adds this Model to the render queue for this frame! If the Hierarchy has a transform on it, that transform is
    /// combined with the Matrix provided here.
    /// <https://stereokit.net/Pages/StereoKit/Model/Draw.html>
    /// * `token` - To be sure we are in the right thread, once per frame.
    /// * `transform` - A Matrix that will transform the Model from Model Space into the current Hierarchy Space.
    /// * `color_linear` - A per-instance linear space color value to pass into the shader! Normally this gets used like
    ///   a material tint. If you’re adventurous and don’t need per-instance colors, this is a great spot to pack in
    ///   extra per-instance data for the shader! If None has default value of WHITE.
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can
    ///   be useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the
    ///   user’s head from a 3rd person perspective, but filtering it out from the 1st person perspective. If None has
    ///   default value of Layer0.
    ///
    /// see also [`model_draw`] [`Model::draw_with_material`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, util::named_colors,
    ///                      system::RenderLayer};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                        .expect("Could not load model").copy();
    /// let transform1 = Matrix::t_r_s([-0.70, -0.70, 0.0], [0.0, 190.0, 0.0], [0.25, 0.25, 0.25]);
    /// let transform2 = transform1 * Matrix::t(Vec3::X * 0.70);
    /// let transform3 = transform2 * Matrix::t(Vec3::X * 0.70);
    ///
    /// filename_scr = "screenshots/model_draw.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, transform1, None, None);
    ///     model.draw(token, transform2, Some(named_colors::YELLOW.into()), None);
    ///     model.draw(token, transform3, Some(named_colors::BLACK.into()),
    ///                Some(RenderLayer::FirstPerson));
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_draw.jpeg" alt="screenshot" width="200">
    pub fn draw(
        &self,
        _token: &MainThreadToken,
        transform: impl Into<Matrix>,
        color_linear: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color_linear = color_linear.unwrap_or(Color128::WHITE);
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe { model_draw(self.0.as_ptr(), transform.into(), color_linear, layer) };
    }

    /// Adds the model to the render queue of this frame overrided with the given material
    /// <https://stereokit.net/Pages/StereoKit/Model/Draw.html>
    /// * `token` - To be sure we are in the right thread, once per frame.
    /// * `material_override` - the material that will override all materials of this model
    /// * `transform` - A Matrix that will transform the Model from Model Space into the current Hierarchy Space.
    /// * `color_linear` - A per-instance linear space color value to pass into the shader! Normally this gets used like
    ///   a material tint. If you’re adventurous and don’t need per-instance colors, this is a great spot to pack in
    ///   extra per-instance data for the shader! If None has default value of WHITE.
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can
    ///   be useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the
    ///   user’s head from a 3rd person perspective, but filtering it out from the 1st person perspective. If None has
    ///   default value of Layer0.
    ///
    /// see also [`model_draw`] [`Model::draw`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, util::named_colors,
    ///                      material::Material, system::RenderLayer};
    ///
    /// let model = Model::from_file("cuve.glb", None)
    ///                        .expect("Could not load model").copy();
    /// let transform1 = Matrix::t_r_s([-0.50, -0.10, 0.0], [45.0, 0.0, 0.0], [0.07, 0.07, 0.07]);
    /// let transform2 = transform1 * Matrix::t(Vec3::X * 0.70);
    /// let transform3 = transform2 * Matrix::t(Vec3::X * 0.70);
    ///
    /// let mut material_ui = Material::ui_box();
    /// material_ui.color_tint(named_colors::GOLD)
    ///            .get_all_param_info().set_float("border_size", 0.01);
    ///
    /// let material_brick =Material::from_file("shaders/brick_pbr.hlsl.sks",
    ///                                         Some("my_material_brick")).unwrap();
    ///
    /// filename_scr = "screenshots/model_draw_with_material.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw_with_material(token, &material_ui,    transform1, None, None);
    ///     model.draw_with_material(token, &material_brick, transform2, None, None);
    ///     model.draw_with_material(token, &material_ui,    transform3, Some(named_colors::RED.into()),
    ///                Some(RenderLayer::FirstPerson));
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_draw_with_material.jpeg" alt="screenshot" width="200">
    pub fn draw_with_material<M: AsRef<Material>>(
        &self,
        _token: &MainThreadToken,
        material_override: M,
        transform: impl Into<Matrix>,
        color_linear: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color_linear = match color_linear {
            Some(c) => c,
            None => Color128::WHITE,
        };
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe {
            model_draw_mat(
                self.0.as_ptr(),
                material_override.as_ref().0.as_ptr(),
                transform.into(),
                color_linear,
                layer,
            )
        };
    }

    /// Examines the visuals as they currently are, and rebuilds the bounds based on that! This is normally done automatically,
    /// but if you modify a Mesh that this Model is using, the Model can’t see it, and you should call this manually.
    /// <https://stereokit.net/Pages/StereoKit/Model/RecalculateBounds.html>
    ///
    /// see also [`model_recalculate_bounds`] [`Model::bounds`] [`Model::recalculate_bounds_exact`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Vec3, Matrix, Bounds}, mesh::{Mesh, Vertex},
    ///                      model::Model, material::Material, util::named_colors};
    ///
    /// let material = Material::pbr();
    /// let mut square = Mesh::new();
    /// square.set_verts(&[
    ///     Vertex::new([-1.0, -1.0, 0.0].into(), Vec3::UP, None,            None),
    ///     Vertex::new([ 1.0, -1.0, 0.0].into(), Vec3::UP, Some(Vec2::X),   None),
    ///     Vertex::new([-1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::Y),   None),
    ///     ], true)
    ///    .set_inds(&[0, 1, 2]);
    ///
    /// let model = Model::from_mesh(&square, &material);
    ///
    /// assert_eq!(model.get_bounds(), Bounds::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 0.0)));
    ///
    /// // We add a second vertex to our mesh adding the dimension-Z.
    /// let mut vertices = square.get_verts_copy();
    /// vertices.push(
    ///     Vertex::new([ 1.0,  1.0, 1.0].into(), Vec3::UP, Some(Vec2::ONE), None));
    ///
    /// square.set_verts(&vertices, true)
    ///       .set_inds(&[0, 1, 2, 2, 1, 3]);
    ///
    /// assert_eq!(model.get_bounds(), Bounds::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 0.0)));
    ///
    /// model.recalculate_bounds();
    ///
    /// assert_eq!(model.get_bounds(), Bounds::new([0.0, 0.0, 0.5], [2.0, 2.0, 1.0]));
    /// ```
    pub fn recalculate_bounds(&self) {
        unsafe { model_recalculate_bounds(self.0.as_ptr()) };
    }

    /// Examines the visuals as they currently are, and rebuilds the bounds based on all the vertices in the model!
    /// This leads (in general) to a tighter bound than the default bound based on bounding boxes. However, computing
    /// the exact bound can take much longer!
    /// <https://stereokit.net/Pages/StereoKit/Model/RecalculateBoundsExact.html>
    ///
    /// see also [`model_recalculate_bounds_exact`] [`Model::bounds`] [`Model::recalculate_bounds`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Vec3, Matrix, Bounds}, mesh::{Mesh, Vertex},
    ///                      model::Model, material::Material, util::named_colors};
    ///
    /// let material = Material::pbr();
    /// let mut square = Mesh::new();
    /// square.set_verts(&[
    ///     Vertex::new([-1.0, -1.0, 0.0].into(), Vec3::UP, None,            None),
    ///     Vertex::new([ 1.0, -1.0, 0.0].into(), Vec3::UP, Some(Vec2::X),   None),
    ///     Vertex::new([-1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::Y),   None),
    ///     ], true)
    ///    .set_inds(&[0, 1, 2]);
    ///
    /// let model = Model::from_mesh(&square, &material);
    ///
    /// assert_eq!(model.get_bounds(), Bounds::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 0.0)));
    ///
    /// // We add a second vertex to our mesh adding the dimension-Z.
    /// let mut vertices = square.get_verts_copy();
    /// vertices.push(
    ///     Vertex::new([ 1.0,  1.0, 1.0].into(), Vec3::UP, Some(Vec2::ONE), None));
    ///
    /// square.set_verts(&vertices, true)
    ///       .set_inds(&[0, 1, 2, 2, 1, 3]);
    ///
    /// assert_eq!(model.get_bounds(), Bounds::new(Vec3::ZERO, Vec3::new(2.0, 2.0, 0.0)));
    ///
    /// model.recalculate_bounds_exact();
    ///
    /// assert_eq!(model.get_bounds(), Bounds::new([0.0, 0.0, 0.5], [2.0, 2.0, 1.0]));
    /// ```
    pub fn recalculate_bounds_exact(&self) {
        unsafe { model_recalculate_bounds_exact(self.0.as_ptr()) };
    }

    /// Get the Id
    /// <https://stereokit.net/Pages/StereoKit/Model/Id.html>
    ///
    /// see also [`model_get_id`] [`model_set_id`]
    /// see example in [`Model::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(model_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// Get the bounds
    /// <https://stereokit.net/Pages/StereoKit/Model/Bounds.html>
    ///
    /// see also [`model_get_bounds`] [`model_set_bounds`]
    /// see example in [`Model::bounds`]
    pub fn get_bounds(&self) -> Bounds {
        unsafe { model_get_bounds(self.0.as_ptr()) }
    }

    /// Get the nodes
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [Nodes]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Bounds}, model::Model, mesh::Mesh,
    ///                      material::Material, util::named_colors};
    ///
    /// let cube_bounds  = Mesh::cube();
    ///
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.3, None);
    ///
    /// let transform1 = Matrix::t([-0.30,-0.30,-0.30]);
    /// let transform2 = Matrix::t([ 0.30, 0.30, 0.30]);
    /// let transform3 = Matrix::t([ 1.30, 1.30, 1.30]);
    ///
    /// let material = Material::pbr();
    /// let mut model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("cube1", transform1, Some(&cube), Some(&material), true)
    ///      .add("cube2", transform2, Some(&cube), Some(&material), true)
    ///      .add("not_a_mesh", transform3, None, None, true);
    ///
    /// for (iter, node) in nodes.all().enumerate() {
    ///    match iter {
    ///        0 => assert_eq!(node.get_name(), Some("cube1")),
    ///        1 => assert_eq!(node.get_name(), Some("cube2")),
    ///        _ => assert_eq!(node.get_name(), Some("not_a_mesh")),
    ///    }
    /// }
    /// ```
    pub fn get_nodes(&self) -> Nodes {
        Nodes::from(self)
    }

    /// Get the anims
    /// <https://stereokit.net/Pages/StereoKit/ModelAnimCollection.html>
    ///
    /// see also [Anims]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, Anims, AnimMode};
    ///
    /// let model = Model::from_file("mobiles.gltf", None)
    ///                              .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_count(), 3);
    ///
    /// for (iter, anim) in anims.enumerate() {
    ///     match iter {
    ///         0 => assert_eq!(anim.name, "rotate"),
    ///         1 => assert_eq!(anim.name, "flyRotate"),
    ///         _ => assert_eq!(anim.name, "fly"),
    ///     }
    /// }
    ///
    /// model.get_anims().play_anim_idx(0, AnimMode::Loop);
    /// ```
    pub fn get_anims(&self) -> Anims {
        Anims::from(self)
    }

    /// Checks the intersection point of a ray and the Solid flagged Meshes in the Model’s visual nodes. Ray must
    /// be in model space, intersection point will be in model space too. You can use the inverse of the mesh’s world
    /// transform matrix to bring the ray into model space, see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Model/Intersect.html>
    /// * `ray` - Ray must be in model space, the intersection point will be in model space too. You can use the inverse
    ///   of the mesh’s world transform matrix to bring the ray into model space, see the example in the docs!
    /// * `cull` - How should intersection work with respect to the direction the triangles are facing? Should we skip
    ///   triangles that are facing away from the ray, or don’t skip anything? If None has default value of Cull::Back.
    ///
    /// see also [`model_ray_intersect`] [`Model::intersect_to_ptr`] same as [`Ray::intersect_model`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Bounds, Ray}, model::Model, mesh::Mesh,
    ///                      material::{Material, Cull}, util::named_colors, system::Lines};
    ///
    /// let cube_bounds  = Mesh::cube();
    ///
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.4, None);
    ///
    /// let transform1 = Matrix::t([-0.30,-0.30,-0.30]);
    /// let transform2 = Matrix::t([ 0.30, 0.30, 0.30]);
    ///
    /// let material = Material::pbr();
    /// let mut model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("cube1", transform1, Some(&cube), Some(&material), true)
    ///      .add("cube2", transform2, Some(&cube), Some(&material), true);
    /// let transform_model = Matrix::r([0.0, 15.0, 0.0]);
    /// let inv = transform_model.get_inverse();
    ///
    /// let ray = Ray::from_to([-0.80, -2.8, -1.0],[0.35, 2.5, 1.0]);
    /// let inv_ray = inv.transform_ray(ray);
    ///
    /// let contact_model = model.intersect(inv_ray, Some(Cull::Front))
    ///           .expect("Intersection should be found");
    ///
    /// let transform_contact_model = Matrix::t(transform_model.transform_point(contact_model));
    /// let point = Mesh::generate_sphere(0.1, Some(2));
    /// let material = Material::pbr();
    ///
    /// let mut material_bounds = Material::ui_box();
    /// material_bounds .color_tint(named_colors::GOLD)
    ///                 .get_all_param_info().set_float("border_size", 0.025);
    ///
    /// let bounds = model.get_bounds();
    /// let transform_before = transform_model * Matrix::t_s( bounds.center, bounds.dimensions);
    ///
    /// filename_scr = "screenshots/model_intersect.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     model.draw(token, transform_model, None, None);
    ///     cube_bounds.draw( token, &material_bounds, transform_before, None, None);
    ///     Lines::add_ray(token, ray, 7.2, named_colors::BLUE, Some(named_colors::RED.into()), 0.02);
    ///     point.draw(token, &material, transform_contact_model,
    ///                Some(named_colors::RED.into()), None );
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_intersect.jpeg" alt="screenshot" width="200">
    #[inline]
    pub fn intersect(&self, ray: Ray, cull: Option<Cull>) -> Option<Vec3> {
        ray.intersect_model(self, cull)
    }

    /// Checks the intersection point of a ray and the Solid flagged Meshes in the Model’s visual nodes. Ray must
    /// be in model space, intersection point will be in model space too. You can use the inverse of the mesh’s world
    /// transform matrix to bring the ray into model space, see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Model/Intersect.html>
    /// * `ray` - Ray must be in model space, the intersection point will be in model space too. You can use the inverse
    ///   of the mesh’s world transform matrix to bring the ray into model space, see the example in the docs!
    /// * `cull` - How should intersection work with respect to the direction the triangles are facing? Should we skip
    ///   triangles that are facing away from the ray, or don’t skip anything? If None has default value of Cull::Back.
    /// * `out_ray` - The intersection point and surface direction of the ray and the mesh, if an intersection occurs.
    ///   This is in model space, and must be transformed back into world space later. Direction is not guaranteed to be
    ///   normalized, especially if your own model->world transform contains scale/skew in it.
    ///
    /// see also [`model_ray_intersect`] [`Model::intersect`] same as [`Ray::intersect_model_to_ptr`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Bounds, Ray}, model::Model, mesh::Mesh,
    ///                      material::{Material, Cull}, util::named_colors, system::Lines};
    ///
    /// let cube_bounds  = Mesh::cube();
    ///
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.4, None);
    ///
    /// let transform1 = Matrix::t([-0.30,-0.30,-0.30]);
    /// let transform2 = Matrix::t([ 0.30, 0.30, 0.30]);
    ///
    /// let material = Material::pbr();
    /// let mut model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("cube1", transform1, Some(&cube), Some(&material), true)
    ///      .add("cube2", transform2, Some(&cube), Some(&material), true);
    /// let transform_model = Matrix::r([0.0, 15.0, 0.0]);
    /// let inv = transform_model.get_inverse();
    ///
    /// let ray = Ray::from_to([-0.80, -2.8, -1.0],[0.35, 2.5, 1.0]);
    /// let inv_ray = inv.transform_ray(ray);
    ///
    /// let mut inv_contact_model_ray = Ray::default();
    /// assert!( model.intersect_to_ptr(inv_ray, Some(Cull::Front), &mut inv_contact_model_ray)
    ///     ,"Ray should touch model");
    ///
    /// let contact_model_ray = transform_model.transform_ray(inv_contact_model_ray);
    /// assert_eq!(contact_model_ray,
    ///     Ray { position:  Vec3 {  x: -0.24654332, y: -0.24928647, z: -0.037466552 },
    ///           direction: Vec3 {  x:  0.25881907, y:  0.0,        z:  0.9659258   } });
    /// ```
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn intersect_to_ptr(&self, ray: Ray, cull: Option<Cull>, out_ray: *mut Ray) -> bool {
        ray.intersect_model_to_ptr(self, cull, out_ray)
    }
}

/// Animations of a Model
/// <https://stereokit.net/Pages/StereoKit/ModelAnimCollection.html>
///
/// see also [`Model::get_anims`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, model::{Model, Anims, AnimMode}};
///
/// let model = Model::from_file("center.glb", None)
///                              .expect("Could not load model").copy();
/// let transform = Matrix::t_r_s(Vec3::NEG_Y * 0.40, [0.0, 190.0, 0.0], Vec3::ONE * 0.25);
///
/// let mut anims = model.get_anims();
/// assert_eq!(anims.get_count(), 1);
/// anims.play_anim("SuzanneAction", AnimMode::Manual).anim_completion(0.80);
///
/// for (iter, anim) in anims.enumerate() {
///     match iter {
///         0 => assert_eq!(anim.name, "SuzanneAction"),
///         _ => panic!("Unexpected animation name"),
///     }
/// }
///
/// let mut anims = model.get_anims();
/// filename_scr = "screenshots/anims.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     model.draw(token, transform, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_file.jpeg" alt="screenshot" width="200">
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/anims.jpeg" alt="screenshot" width="200">
pub struct Anims<'a> {
    model: &'a Model,
    curr: i32,
}

/// Describes how an animation is played back, and what to do when the animation hits the end.
/// <https://stereokit.net/Pages/StereoKit/AnimMode.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum AnimMode {
    /// If the animation reaches the end, it will always loop back around to the start again.
    Loop = 0,
    /// When the animation reaches the end, it will freeze in-place.
    Once = 1,
    /// The animation will not progress on its own, and instead must be driven by providing information to the model’s
    /// AnimTime or AnimCompletion properties.
    Manual = 2,
}

impl Iterator for Anims<'_> {
    type Item = Anim;

    fn next(&mut self) -> Option<Self::Item> {
        self.curr += 1;
        if self.curr < self.get_count() {
            Some(Anim {
                name: match self.get_name_at_index(self.curr) {
                    Some(name) => name.to_string(),
                    None => {
                        Log::err(format!("animation {:?}, is missing", self.curr));
                        "<<error !!>>".to_string()
                    }
                },
                duration: self.get_duration_at_index(self.curr),
            })
        } else {
            None
        }
    }
}

/// A link to a Model’s animation! You can use this to get some basic information about the animation, or store it for
/// reference. This maintains a link to the Model asset, and will keep it alive as long as this object lives.
/// <https://stereokit.net/Pages/StereoKit/Anim.html>
#[derive(Debug, Clone, PartialEq)]
pub struct Anim {
    pub name: String,
    pub duration: f32,
}

impl<'a> Anims<'a> {
    /// Same as [`Model::get_anims`]
    pub fn from<M: AsRef<Model>>(model: &'a M) -> Anims<'a> {
        Anims { model: model.as_ref(), curr: -1 }
    }

    /// Get the name of the animation at given index
    ///
    /// see also [`model_anim_get_name`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{model::{Model, Anims, AnimMode}, system::Assets};
    ///
    /// let model = Model::from_file("mobiles.gltf", None)
    ///                              .expect("Could not load model").copy();
    /// Assets::block_for_priority(i32::MAX);
    ///
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_count(), 3);
    /// assert_eq!(anims.get_name_at_index(0), Some("rotate"));
    /// assert_eq!(anims.get_name_at_index(1), Some("flyRotate"));
    /// assert_eq!(anims.get_name_at_index(2), Some("fly"));
    /// assert_eq!(anims.get_name_at_index(3), None);
    /// ```
    pub fn get_name_at_index(&self, index: i32) -> Option<&str> {
        unsafe {
            if model_anim_count(self.model.0.as_ptr()) > index {
                CStr::from_ptr(model_anim_get_name(self.model.0.as_ptr(), index)).to_str().ok()
            } else {
                None
            }
        }
    }

    /// Get the duration of the animation at given index
    ///
    /// Returns `-0.01` if the index is out of bounds.
    /// see also [`model_anim_get_duration`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, Anims, AnimMode};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                              .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_count(), 1);
    /// assert_eq!(anims.get_duration_at_index(0), 2.5);
    /// assert_eq!(anims.get_duration_at_index(1), -0.01);
    /// ```
    pub fn get_duration_at_index(&self, index: i32) -> f32 {
        unsafe {
            if model_anim_count(self.model.0.as_ptr()) > index {
                model_anim_get_duration(self.model.0.as_ptr(), index)
            } else {
                -0.01
            }
        }
    }

    /// Calling Draw will automatically step the Model’s animation, but if you don’t draw the Model, or need access to
    /// the animated nodes before drawing,
    /// then you can step the animation early manually via this method. Animation will only ever be stepped once per
    /// frame, so it’s okay to call this multiple times,
    /// or in addition to Draw.
    /// <https://stereokit.net/Pages/StereoKit/Model/StepAnim.html>
    ///
    /// see also [`model_step_anim`][`model_play_anim`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::{Model, Anims, AnimMode}};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                              .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_count(), 1);
    /// anims.play_anim("SuzanneAction", AnimMode::Loop);
    ///
    /// number_of_steps = 20;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter % 10 < 5 {
    ///         model.draw(token, Matrix::IDENTITY, None, None);
    ///     } else {
    ///         anims.step_anim();
    ///     }
    /// );
    /// ```
    pub fn step_anim(&mut self) -> &mut Self {
        unsafe { model_step_anim(self.model.0.as_ptr()) };
        self
    }

    /// Searches for an animation with the given name, and if it’s found, sets it up as the active animation and begins
    /// laying it with the animation mode.
    /// <https://stereokit.net/Pages/StereoKit/Model/PlayAnim.html>
    /// * `name` - The name of the animation to play. Case sensitive.
    /// * `mode` - The animation mode to use.
    ///
    /// see also [`model_play_anim`] [`Anims::play_anim_idx`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, Anims, AnimMode};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                              .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    ///
    /// anims.play_anim("SuzanneAction", AnimMode::Loop);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Loop);
    ///
    /// anims.play_anim("SuzanneAction", AnimMode::Once);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Once);
    ///
    /// anims.play_anim("SuzanneAction", AnimMode::Manual);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Manual);
    ///
    /// // If anim does not exist:
    /// anims.play_anim("Not exist", AnimMode::Manual);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Manual);
    /// ```
    pub fn play_anim(&mut self, animation_name: impl AsRef<str>, mode: AnimMode) -> &mut Self {
        let c_str = CString::new(animation_name.as_ref()).unwrap();
        unsafe { model_play_anim(self.model.0.as_ptr(), c_str.as_ptr(), mode) };
        self
    }

    /// Sets it up the animation at index idx as the active animation and begins playing it with the animation mode.
    /// <https://stereokit.net/Pages/StereoKit/Model/PlayAnim.html>
    /// * `idx` - index of the animation to play
    /// * `mode` - animation mode to play the animation with
    ///
    /// see also [`model_play_anim_idx`] [`Anims::play_anim`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, Anims, AnimMode};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                              .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    ///
    /// anims.play_anim_idx(0, AnimMode::Loop);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Loop);
    ///
    /// anims.play_anim_idx(0, AnimMode::Once);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Once);
    ///
    /// // If index does not exist:
    /// anims.play_anim_idx(102, AnimMode::Manual);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Once);
    /// ```
    pub fn play_anim_idx(&mut self, idx: i32, mode: AnimMode) -> &mut Self {
        unsafe { model_play_anim_idx(self.model.0.as_ptr(), idx, mode) };
        self
    }

    /// Set the current time of the active animation in seconds, from the start of the animation. This may be a value
    /// superior to the animation’s Duration if the animation is a loop. For a percentage of completion,
    /// see anim_completion instead.
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimTime.html>
    ///
    /// see also [`model_set_anim_time`] [`Anims::anim_completion`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, Anims, AnimMode};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                              .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    /// anims.play_anim_idx(0, AnimMode::Manual);
    /// assert_eq!(anims.get_duration_at_index(0), 2.5);
    ///
    /// anims.anim_time(1.0);
    /// assert_eq!(anims.get_anim_completion(), 0.4);
    ///
    /// anims.anim_time(2.0);
    /// assert_eq!(anims.get_anim_completion(), 0.8);
    ///
    /// // if the asking for animation longer than the duration (AnimMode::Manual):
    /// anims.anim_time(4.0);
    /// assert_eq!(anims.get_anim_completion(), 1.0);
    ///
    /// anims.play_anim_idx(0, AnimMode::Loop);
    /// // if the asking for animation longer than the duration (AnimMode::Loop):
    /// anims.anim_time(4.0);
    /// assert_eq!(anims.get_anim_completion(), 0.6);
    /// ```
    pub fn anim_time(&mut self, time: f32) -> &mut Self {
        unsafe { model_set_anim_time(self.model.0.as_ptr(), time) };
        self
    }

    /// This set the percentage of completion of the active animation. This may be a value superior to 1.0 if the
    /// animation is a loop.
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimCompletion.html>
    ///
    /// see also [`model_set_anim_completion`] [`Anims::anim_time`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, Anims, AnimMode};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                              .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    /// anims.play_anim_idx(0, AnimMode::Manual);
    /// assert_eq!(anims.get_duration_at_index(0), 2.5);
    ///
    /// anims.anim_completion(0.4);
    /// assert_eq!(anims.get_anim_time(), 1.0);
    ///
    /// anims.anim_completion(0.8);
    /// assert_eq!(anims.get_anim_time(), 2.0);
    ///
    /// // If asking for a completion over 100% (AnimMode::Manual):
    /// anims.anim_completion(1.8);
    /// assert_eq!(anims.get_anim_time(), 2.5);
    ///
    /// anims.play_anim_idx(0, AnimMode::Loop);
    /// // if the asking for a completion over 100% (AnimMode::Loop):
    /// anims.anim_completion(1.8);
    /// assert_eq!(anims.get_anim_time(), 2.0);
    /// ```
    pub fn anim_completion(&mut self, percent: f32) -> &mut Self {
        unsafe { model_set_anim_completion(self.model.0.as_ptr(), percent) };
        self
    }

    /// get anim by name
    /// <https://stereokit.net/Pages/StereoKit/Model/FindAnim.html>
    ///
    /// see also [`model_anim_find`] [`Anims::play_anim`] [`Anims::play_anim_idx]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, Anims};
    /// let model = Model::from_file("center.glb", None)
    ///     .expect("Could not load model")
    ///     .copy();
    ///
    /// let anims = model.get_anims();
    ///
    /// assert_eq!(anims.find_anim("SuzanneAction"), Some(0));
    /// assert_eq!(anims.find_anim("Not exist"), None);
    /// ```
    pub fn find_anim<S: AsRef<str>>(&self, name: S) -> Option<i32> {
        let c_str = match CString::new(name.as_ref()) {
            Ok(c_str) => c_str,
            Err(..) => return None,
        };
        let index = unsafe { model_anim_find(self.model.0.as_ptr(), c_str.as_ptr()) };
        if index < 0 { None } else { Some(index) }
    }

    /// Get the number of animations
    /// <https://stereokit.net/Pages/StereoKit/Model/ModelAnimCollection.html>
    ///
    /// see also [`model_anim_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model};
    ///
    /// let model = Model::from_file("mobiles.gltf", None)
    ///                             .expect("Could not load model").copy();
    ///
    /// let count = model.get_anims().get_count();
    ///
    /// assert_eq!(count, 3);
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe { model_anim_count(self.model.0.as_ptr()) }
    }

    /// Get the current animation
    /// <https://stereokit.net/Pages/StereoKit/Model/ActiveAnim.html>
    ///
    /// see also [`model_anim_active`] [`Anims::play_anim`] [`Anims::play_anim_idx`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, AnimMode};
    ///
    /// let model = Model::from_file("mobiles.gltf", None)
    ///                            .expect("Could not load model").copy();
    ///
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_active_anim(), -1);
    ///
    /// anims.play_anim("flyRotate", AnimMode::Loop);
    /// assert_eq!(anims.get_active_anim(), 1);
    /// ```
    pub fn get_active_anim(&self) -> i32 {
        unsafe { model_anim_active(self.model.0.as_ptr()) }
    }

    /// Get the current animation, mode
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimMode.html>
    ///
    /// see also [`model_anim_active_mode`] [`Anims::play_anim`] [`Anims::play_anim_idx`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, AnimMode};
    ///
    /// let model = Model::from_file("mobiles.gltf", None)
    ///                           .expect("Could not load model").copy();
    ///
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Loop);
    ///
    /// anims.play_anim("flyRotate", AnimMode::Once);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Once);
    ///
    /// anims.play_anim("fly", AnimMode::Manual);
    /// assert_eq!(anims.get_anim_mode(), AnimMode::Manual);
    /// ```
    pub fn get_anim_mode(&self) -> AnimMode {
        unsafe { model_anim_active_mode(self.model.0.as_ptr()) }
    }
    /// Get the current animation duration
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimTime.html>
    ///
    /// see also [`model_anim_active_time`] [`Anims::anim_time`] [`Anims::anim_completion`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, AnimMode};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                           .expect("Could not load model").copy();
    ///
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_anim_time(), 0.0);
    ///
    /// anims.play_anim("SuzanneAction", AnimMode::Loop ).anim_completion(0.5);
    /// assert_eq!(anims.get_anim_time(), 1.25);
    /// ```
    pub fn get_anim_time(&self) -> f32 {
        unsafe { model_anim_active_time(self.model.0.as_ptr()) }
    }

    /// Get the current animation completion %
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimCompletion.html>
    ///
    /// see also [`model_anim_active_completion`] [`Anims::anim_time`] [`Anims::anim_completion`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::model::{Model, AnimMode};
    ///
    /// let model = Model::from_file("center.glb", None)
    ///                          .expect("Could not load model").copy();
    /// let mut anims = model.get_anims();
    /// assert_eq!(anims.get_anim_completion(), 0.0);
    ///
    /// anims.play_anim("SuzanneAction", AnimMode::Loop);
    /// anims.anim_time(0.5);
    /// assert_eq!(anims.get_anim_completion(), 0.2);
    /// ```
    pub fn get_anim_completion(&self) -> f32 {
        unsafe { model_anim_active_completion(self.model.0.as_ptr()) }
    }
}

/// Nodes of a Model
/// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
/// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
///
/// see also [`Model::get_nodes`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix} ,model::Model, util::Color128};
///
/// let model = Model::from_file("center.glb", None)
///                           .expect("Could not load model").copy();
/// let transform = Matrix::t_r_s(Vec3::NEG_Y * 0.40, [0.0, 190.0, 0.0], Vec3::ONE * 0.25);
///
/// let mut nodes = model.get_nodes();
///
/// // Duplicate Suzanne ModelNode 5 times at different positions.
/// let original_node = nodes.find("Suzanne").expect("Could not find Suzanne");
/// let mesh = original_node.get_mesh().expect("Could not get Suzanne's mesh");
/// let material_original = original_node.get_material().expect("Could not get Suzanne's material");
///
/// for i in -1..4 {
///     let coord = i as f32 * 1.25;
///     let color_idx = ((i+5) * 13694856) as u32;
///     let position = Matrix::t([coord, coord, coord]);
///     let name = format!("Suzanne_{}", i);
///     let mut material = material_original.copy();
///     material.color_tint(Color128::hex(color_idx));
///
///     nodes.add(name, position, Some(&mesh), Some(&material), true);
/// }
///
/// filename_scr = "screenshots/model_nodes.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     model.draw(token, transform, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_nodes.jpeg" alt="screenshot" width="200">
#[derive(Debug, Copy, Clone)]
pub struct Nodes<'a> {
    model: &'a Model,
}

unsafe extern "C" {
    pub fn model_subset_count(model: ModelT) -> i32;
    pub fn model_node_add(
        model: ModelT,
        name: *const c_char,
        model_transform: Matrix,
        mesh: MeshT,
        material: MaterialT,
        solid: Bool32T,
    ) -> ModelNodeId;
    pub fn model_node_add_child(
        model: ModelT,
        parent: ModelNodeId,
        name: *const c_char,
        local_transform: Matrix,
        mesh: MeshT,
        material: MaterialT,
        solid: Bool32T,
    ) -> ModelNodeId;
    pub fn model_node_find(model: ModelT, name: *const c_char) -> ModelNodeId;
    pub fn model_node_sibling(model: ModelT, node: ModelNodeId) -> ModelNodeId;
    pub fn model_node_parent(model: ModelT, node: ModelNodeId) -> ModelNodeId;
    pub fn model_node_child(model: ModelT, node: ModelNodeId) -> ModelNodeId;
    pub fn model_node_count(model: ModelT) -> i32;
    pub fn model_node_index(model: ModelT, index: i32) -> ModelNodeId;
    pub fn model_node_visual_count(model: ModelT) -> i32;
    pub fn model_node_visual_index(model: ModelT, index: i32) -> ModelNodeId;
    pub fn model_node_iterate(model: ModelT, node: ModelNodeId) -> ModelNodeId;
    pub fn model_node_get_root(model: ModelT) -> ModelNodeId;
    pub fn model_node_get_name(model: ModelT, node: ModelNodeId) -> *const c_char;
    pub fn model_node_get_solid(model: ModelT, node: ModelNodeId) -> Bool32T;
    pub fn model_node_get_visible(model: ModelT, node: ModelNodeId) -> Bool32T;
    pub fn model_node_get_material(model: ModelT, node: ModelNodeId) -> MaterialT;
    pub fn model_node_get_mesh(model: ModelT, node: ModelNodeId) -> MeshT;
    pub fn model_node_get_transform_model(model: ModelT, node: ModelNodeId) -> Matrix;
    pub fn model_node_get_transform_local(model: ModelT, node: ModelNodeId) -> Matrix;
    pub fn model_node_set_name(model: ModelT, node: ModelNodeId, name: *const c_char);
    pub fn model_node_set_solid(model: ModelT, node: ModelNodeId, solid: Bool32T);
    pub fn model_node_set_visible(model: ModelT, node: ModelNodeId, visible: Bool32T);
    pub fn model_node_set_material(model: ModelT, node: ModelNodeId, material: MaterialT);
    pub fn model_node_set_mesh(model: ModelT, node: ModelNodeId, mesh: MeshT);
    pub fn model_node_set_transform_model(model: ModelT, node: ModelNodeId, transform_model_space: Matrix);
    pub fn model_node_set_transform_local(model: ModelT, node: ModelNodeId, transform_local_space: Matrix);
    pub fn model_node_info_get(model: ModelT, node: ModelNodeId, info_key_utf8: *const c_char) -> *mut c_char;
    pub fn model_node_info_set(
        model: ModelT,
        node: ModelNodeId,
        info_key_utf8: *const c_char,
        info_value_utf8: *const c_char,
    );
    pub fn model_node_info_remove(model: ModelT, node: ModelNodeId, info_key_utf8: *const c_char) -> Bool32T;
    pub fn model_node_info_clear(model: ModelT, node: ModelNodeId);
    pub fn model_node_info_count(model: ModelT, node: ModelNodeId) -> i32;
    pub fn model_node_info_iterate(
        model: ModelT,
        node: ModelNodeId,
        ref_iterator: *mut i32,
        out_key_utf8: *mut *const c_char,
        out_value_utf8: *mut *const c_char,
    ) -> Bool32T;

}

/// Iterator of the nodes of a model. Can be instanciate from [Model]
///
/// see also [Nodes::all] [Nodes::visuals]
#[derive(Debug, Copy, Clone)]
pub struct NodeIter<'a> {
    model: &'a Model,
    index: i32,
    visual: bool,
}

impl<'a> Iterator for NodeIter<'a> {
    type Item = ModelNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        if self.visual {
            let count = unsafe { model_node_visual_count(self.model.0.as_ptr()) };
            if self.index < count {
                match unsafe { model_node_visual_index(self.model.0.as_ptr(), self.index) } {
                    -1 => {
                        Log::err(format!(
                            "node at index {:?}, is missing when {:?} visual nodes are expected for Model {:?}",
                            self.index,
                            count,
                            self.model.get_id()
                        ));
                        None
                    }
                    otherwise => Some(ModelNode { model: self.model, id: otherwise }),
                }
            } else {
                None
            }
        } else {
            let count = unsafe { model_node_count(self.model.0.as_ptr()) };
            if self.index < count {
                match unsafe { model_node_index(self.model.0.as_ptr(), self.index) } {
                    -1 => {
                        Log::err(format!(
                            "node at index {:?}, is missing when {:?} visual nodes are expected for Model {:?}",
                            self.index,
                            count,
                            self.model.get_id()
                        ));
                        None
                    }
                    otherwise => Some(ModelNode { model: self.model, id: otherwise }),
                }
            } else {
                None
            }
        }
    }
}

impl<'a> NodeIter<'a> {
    /// Get an iterator for all node of the given model
    /// see also [Nodes::all]
    pub fn all_from(model: &'a impl AsRef<Model>) -> NodeIter<'a> {
        NodeIter { index: -1, model: model.as_ref(), visual: false }
    }

    ///Get an iterator for all visual node of the given model
    /// see also [Nodes::visuals]
    pub fn visuals_from(model: &'a impl AsRef<Model>) -> NodeIter<'a> {
        NodeIter { index: -1, model: model.as_ref(), visual: true }
    }
}

impl<'a> Nodes<'a> {
    pub fn from(model: &'a impl AsRef<Model>) -> Nodes<'a> {
        Nodes { model: model.as_ref() }
    }

    /// This adds a root node to the Model’s node hierarchy! If There is already an initial root node,
    /// this node will still be a root node, but will be a Sibling of the Model’s RootNode. If this is the first root node added,
    /// you’ll be able to access it via [Nodes::get_root_node].
    /// <https://stereokit.net/Pages/StereoKit/Model/AddNode.html>
    ///
    /// see also [ModelNode::add_child] [`model_node_add`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh, material::Material};
    ///
    /// let sphere =       Mesh::generate_sphere(0.6, None);
    /// let cylinder =     Mesh::generate_cylinder(0.25, 0.6, Vec3::Z, None);
    ///
    /// let transform1 = Matrix::t([-0.7,-0.5, 0.0]);
    /// let transform2 = Matrix::t([ 0.0, 0.0, 0.0]);
    ///
    /// let material = Material::pbr();
    ///
    /// let model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("sphere",   transform1 , Some(&sphere),       Some(&material), true)
    ///      .add("cylinder", transform2 , Some(&cylinder),     Some(&material), true)
    ///      .add("A matrix", Matrix::IDENTITY, None, None, false);
    ///
    /// assert_eq!(nodes.get_count(), 3);
    /// assert_eq!(nodes.get_root_node().unwrap().get_name(), Some("sphere"));
    /// ```
    pub fn add<S: AsRef<str>>(
        &mut self,
        name: S,
        local_transform: impl Into<Matrix>,
        mesh: Option<&Mesh>,
        material: Option<&Material>,
        solid: bool,
    ) -> &mut Self {
        let c_str = CString::new(name.as_ref()).unwrap();
        let mesh = match mesh {
            Some(mesh) => mesh.0.as_ptr(),
            None => null_mut(),
        };
        let material = match material {
            Some(material) => material.0.as_ptr(),
            None => null_mut(),
        };
        unsafe {
            model_node_add(
                self.model.0.as_ptr(),
                c_str.as_ptr(),
                local_transform.into(),
                mesh,
                material,
                solid as Bool32T,
            )
        };
        self
    }

    /// Get an iterator of all the nodes
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [NodeIter::all_from]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh, material::Material};
    ///
    /// let sphere =       Mesh::generate_sphere(0.6, None);
    /// let cylinder =     Mesh::generate_cylinder(0.25, 0.6, Vec3::Z, None);
    ///
    /// let transform1 = Matrix::t([-0.7,-0.5, 0.0]);
    /// let transform2 = Matrix::t([ 0.0, 0.0, 0.0]);
    ///
    /// let material = Material::pbr();
    ///
    /// let model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("sphere",   transform1 , Some(&sphere),       Some(&material), true)
    ///      .add("cylinder", transform2 , Some(&cylinder),     Some(&material), true)
    ///      .add("A matrix", Matrix::IDENTITY, None, None, false);
    ///
    /// assert_eq!(nodes.get_count(), 3);
    /// assert_eq!(nodes.all().count(), 3);
    ///
    /// for (iter, node) in nodes.all().enumerate() {
    ///     match iter {
    ///         0 => assert_eq!(node.get_name(), Some("sphere")),
    ///         1 => assert_eq!(node.get_name(), Some("cylinder")),
    ///         _ => assert_eq!(node.get_name(), Some("A matrix")),
    ///     }
    /// }
    /// ```
    pub fn all(&self) -> NodeIter {
        NodeIter::all_from(self.model)
    }

    /// Get an iterator of all the visual nodes
    /// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
    ///
    /// see also [NodeIter::visuals_from]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh, material::Material};
    ///
    /// let sphere =       Mesh::generate_sphere(0.6, None);
    /// let cylinder =     Mesh::generate_cylinder(0.25, 0.6, Vec3::Z, None);
    ///
    /// let transform1 = Matrix::t([-0.7,-0.5, 0.0]);
    /// let transform2 = Matrix::t([ 0.0, 0.0, 0.0]);
    ///
    /// let material = Material::pbr();
    ///
    /// let model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("sphere",   transform1 , Some(&sphere),       Some(&material), true)
    ///      .add("cylinder", transform2 , Some(&cylinder),     Some(&material), true)
    ///      .add("A matrix", Matrix::IDENTITY, None, None, false);
    ///
    /// assert_eq!(nodes.get_count(), 3);
    /// assert_eq!(nodes.get_visual_count(), 2);
    /// assert_eq!(nodes.visuals().count(), 2);
    ///
    /// for (iter, node) in nodes.visuals().enumerate() {
    ///     match iter {
    ///         0 => assert_eq!(node.get_name(), Some("sphere")),
    ///         _ => assert_eq!(node.get_name(), Some("cylinder")),
    ///     }
    /// }
    /// ```
    pub fn visuals(&self) -> NodeIter {
        NodeIter::visuals_from(self.model)
    }

    /// get node by name
    /// <https://stereokit.net/Pages/StereoKit/Model/FindNode.html>
    /// * `name` - Exact name to match against. ASCII only for now.
    ///
    /// see also [`model_node_find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh, material::Material};
    ///
    /// let sphere =      Mesh::generate_sphere(0.6, None);
    /// let cylinder =    Mesh::generate_cylinder(0.25, 0.6, Vec3::Z, None);
    ///
    /// let transform1 = Matrix::t([-0.7,-0.5, 0.0]);
    /// let transform2 = Matrix::t([ 0.0, 0.0, 0.0]);
    ///
    /// let material = Material::pbr();
    ///
    /// let model = Model::new();
    /// let mut nodes = model.get_nodes();
    /// nodes.add("sphere",   transform1 , Some(&sphere),      Some(&material), true)
    ///      .add("cylinder", transform2 , Some(&cylinder),    Some(&material), true)
    ///      .add("A matrix", Matrix::IDENTITY, None, None, false);
    ///
    /// let found_sphere = nodes.find("sphere");
    /// assert!(found_sphere.is_some());
    /// assert_eq!(found_sphere.unwrap().get_name(), Some("sphere"));
    ///
    /// let found_non_existent = nodes.find("non_existent");
    /// assert!(found_non_existent.is_none());
    /// ```
    pub fn find<S: AsRef<str>>(&self, name: S) -> Option<ModelNode> {
        let c_str = CString::new(name.as_ref()).unwrap();
        match unsafe { model_node_find(self.model.0.as_ptr(), c_str.as_ptr()) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// Get the number of node.
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [NodeIter] [`model_node_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// assert_eq!(nodes.get_count(), 0);
    ///
    /// nodes.add("root", Matrix::IDENTITY, None, None, false);
    /// assert_eq!(nodes.get_count(), 1);
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe { model_node_count(self.model.0.as_ptr()) }
    }

    /// Get the number of visual node
    /// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
    ///
    /// see also [NodeIter] [`model_node_visual_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// assert_eq!(nodes.get_count(), 0);
    /// assert_eq!(nodes.get_visual_count(), 0);
    ///
    /// nodes.add("root", Matrix::IDENTITY, None, None, false);
    /// assert_eq!(nodes.get_count(), 1);
    /// assert_eq!(nodes.get_visual_count(), 0);
    /// ```
    pub fn get_visual_count(&self) -> i32 {
        unsafe { model_node_visual_count(self.model.0.as_ptr()) }
    }

    /// Get the node at index
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [NodeIter] [`model_node_index`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// let node = nodes.get_index(0);
    /// assert!(node.is_none());
    ///
    /// nodes.add("root", Matrix::IDENTITY, None, None, false);
    /// let node = nodes.get_index(0).expect("Node should exist");
    /// assert_eq!(node.get_name(), Some("root"));
    /// ```
    pub fn get_index(&self, index: i32) -> Option<ModelNode> {
        if unsafe { model_node_count(self.model.0.as_ptr()) } <= index {
            return None;
        }
        match unsafe { model_node_index(self.model.0.as_ptr(), index) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// Get the visual node at index
    /// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
    ///
    /// see also [NodeIter] [`model_node_visual_index`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// let node = nodes.get_visual_index(0);
    /// assert!(node.is_none());
    ///
    /// nodes.add("root", Matrix::IDENTITY, None, None, false);
    /// let node = nodes.get_visual_index(0);
    /// assert!(node.is_none());
    /// ```
    pub fn get_visual_index(&self, index: i32) -> Option<ModelNode> {
        if unsafe { model_node_visual_count(self.model.0.as_ptr()) } <= index {
            return None;
        }
        match unsafe { model_node_visual_index(self.model.0.as_ptr(), index) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// Get the root node
    /// <https://stereokit.net/Pages/StereoKit/Model/RootNode.html>
    ///
    /// see also [`model_node_get_root`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// let node = nodes.get_root_node();
    /// assert!(node.is_none());
    ///
    /// nodes.add("root", Matrix::IDENTITY, None, None, false);
    /// let node = nodes.get_root_node().expect("Node should exist");
    /// assert_eq!(node.get_name(), Some("root"));
    /// ```
    pub fn get_root_node(&self) -> Option<ModelNode> {
        let id = unsafe { model_node_get_root(self.model.0.as_ptr()) };
        if id == -1 { None } else { Some(ModelNode { model: self.model, id }) }
    }
}

/// This class is a link to a node in a Model’s internal hierarchy tree. It’s composed of node information, and links to
/// the directly adjacent tree nodes.
/// <https://stereokit.net/Pages/StereoKit/ModelNode.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, model::Model, mesh::Mesh,
///                      material::Material, util::named_colors};
///
/// let sphere =       Mesh::generate_sphere(0.6, None);
/// let rounded_cube = Mesh::generate_rounded_cube(Vec3::ONE * 0.6, 0.2, None);
/// let cylinder =     Mesh::generate_cylinder(0.25, 0.6, Vec3::Z, None);
///
/// let transform1 = Matrix::t( [-0.7,-0.5, 0.0]);
/// let transform2 = Matrix::t( [ 0.0, 0.0, 0.0]);
/// let transform3 = Matrix::t( [ 0.7, 0.5, 0.0]);
/// let trans_mini = Matrix::t_s([ 0.0, 0.0, 0.5].into(), Vec3::ONE * 0.15);
///
/// let material = Material::pbr();
///
/// let model = Model::new();
/// let mut nodes = model.get_nodes();
/// nodes.add("sphere",   transform1 , Some(&sphere),       Some(&material), true)
///      .add("cube",     transform2 , Some(&rounded_cube), Some(&material), true)
///      .add("cylinder", transform3 , Some(&cylinder),     Some(&material), true)
///      .add("mini",     trans_mini, None, None, true);
///
/// let mut material = material.copy();
/// material.color_tint(named_colors::RED);
/// let mut mini = nodes.find("mini").expect("mini node should exist!");
/// mini.add_child("sphere",   transform1 , Some(&sphere),       Some(&material), true)
///     .add_child("cube",     transform2 , Some(&rounded_cube), Some(&material), true)
///     .add_child("cylinder", transform3 , Some(&cylinder),     Some(&material), true);
///
/// assert_eq!(nodes.get_visual_count(), 6);
/// assert_eq!(nodes.get_count(), 7);
///
/// filename_scr = "screenshots/model_node.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     model.draw(token, Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_node.jpeg" alt="screenshot" width="200">
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ModelNode<'a> {
    model: &'a Model,
    id: ModelNodeId,
}
pub type ModelNodeId = i32;

impl ModelNode<'_> {
    /// Set the name of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Name.html>
    ///
    /// see also [`model_node_set_name`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("root", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.find("root").expect("A node should exist!");
    /// assert_eq!(node.get_name(), Some("root"));
    ///
    /// node.name("my_root_node");
    /// assert_eq!(node.get_name(), Some("my_root_node"));
    ///
    /// let node = nodes.find("my_root_node").expect("A node should exist!");
    /// assert!(nodes.find("root").is_none());
    /// ```
    pub fn name<S: AsRef<str>>(&mut self, name: S) -> &mut Self {
        let c_str = CString::new(name.as_ref()).unwrap();
        unsafe { model_node_set_name(self.model.0.as_ptr(), self.id, c_str.as_ptr()) };
        self
    }

    /// Set the solid of the node. A flag that indicates the Mesh for this node will be used in ray intersection tests.
    /// This flag is ignored if no Mesh is attached.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Solid.html>
    ///
    /// see also [`model_node_set_solid`] [`Nodes::add`] [`ModelNode::add_child`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("cube", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), false);
    ///
    /// let mut node = nodes.find("cube").expect("A node should exist!");
    /// assert_eq!(node.get_solid(), false);
    ///
    /// node.solid(true);
    /// assert_eq!(node.get_solid(), true);
    /// ```
    pub fn solid(&mut self, solid: bool) -> &mut Self {
        unsafe { model_node_set_solid(self.model.0.as_ptr(), self.id, solid as Bool32T) };
        self
    }

    /// Is this node flagged as visible? By default, this is true for all nodes with visual elements attached. These
    /// nodes will not be drawn or skinned if you set this flag to false. If a ModelNode has no visual elements attached
    /// to it, it will always return false, and setting this value will have no effect.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Visible.html>
    ///
    /// see also [`model_node_set_visible`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("cube", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let mut node = nodes.find("cube").expect("A node should exist!");
    /// assert_eq!(node.get_visible(), true);
    ///
    /// node.visible(false);
    /// assert_eq!(node.get_visible(), false);
    /// ```
    pub fn visible(&mut self, visible: bool) -> &mut Self {
        unsafe { model_node_set_visible(self.model.0.as_ptr(), self.id, visible as Bool32T) };
        self
    }

    /// Set the material of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Material.html>
    ///
    /// see also [`model_node_set_material`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("cube", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let mut node = nodes.find("cube").expect("A node should exist!");
    /// assert_eq!(node.get_material(), Some(Material::pbr()));
    ///
    /// node.material(Material::unlit());
    /// assert_eq!(node.get_material(), Some(Material::unlit()));
    /// ```
    pub fn material<M: AsRef<Material>>(&mut self, material: M) -> &mut Self {
        unsafe { model_node_set_material(self.model.0.as_ptr(), self.id, material.as_ref().0.as_ptr()) };
        self
    }

    /// Set the mesh of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Mesh.html>
    ///
    /// see also [`model_node_set_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let mut node = nodes.find("mesh").expect("A node should exist!");
    /// assert_eq!(node.get_mesh(), Some(Mesh::cube()));
    ///
    /// node.mesh(Mesh::sphere());
    /// assert_eq!(node.get_mesh(), Some(Mesh::sphere()));
    /// ```
    pub fn mesh<M: AsRef<Mesh>>(&mut self, mesh: M) -> &mut Self {
        unsafe { model_node_set_mesh(self.model.0.as_ptr(), self.id, mesh.as_ref().0.as_ptr()) };
        self
    }

    /// Set the transform model of the node. The transform of this node relative to the Model itself. This incorporates
    /// transforms from all parent nodes.
    /// Setting this transform will update the LocalTransform, as well as all Child nodes below this one.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/ModelTransform.html>
    ///
    /// see also [`model_node_set_transform_model`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("root_mesh", Matrix::t([1.0, 1.0, 1.0]), Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// assert_eq!(model.get_bounds().center, [1.0, 1.0, 1.0].into());
    /// assert_eq!(model.get_bounds().dimensions, [1.0, 1.0, 1.0].into());
    ///
    /// let mut node = nodes.find("root_mesh").expect("A node should exist!");
    /// node.add_child("child_mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), false);
    /// assert_eq!(model.get_bounds().center, [1.0, 1.0, 1.0].into());
    /// assert_eq!(model.get_bounds().dimensions, [1.0, 1.0, 1.0].into());
    ///
    /// // Model_transform!!!
    /// let mut node_child = nodes.find("child_mesh").expect("A node should exist!");
    /// node_child.model_transform(Matrix::t([-2.0, -2.0, -2.0]));
    /// assert_eq!(model.get_bounds().center, [-0.5, -0.5, -0.5].into());
    /// assert_eq!(model.get_bounds().dimensions, [4.0, 4.0, 4.0].into());
    ///
    /// // Local_transform!!!
    /// let mut node_child = nodes.find("child_mesh").expect("A node should exist!");
    /// node_child.local_transform(Matrix::t([-2.0, -2.0, -2.0]));
    /// assert_eq!(model.get_bounds().center, [0.0, 0.0, 0.0].into());
    /// assert_eq!(model.get_bounds().dimensions, [3.0, 3.0, 3.0].into());
    /// ```
    pub fn model_transform(&mut self, transform_model_space: impl Into<Matrix>) -> &mut Self {
        unsafe { model_node_set_transform_model(self.model.0.as_ptr(), self.id, transform_model_space.into()) };
        self
    }

    /// Set the local transform  of the node. The transform of this node relative to the Parent node.
    /// Setting this transform will update the ModelTransform, as well as all Child nodes below this one.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/LocalTransform.html>
    ///
    /// see also [`model_node_set_transform_local`]
    /// see example [ModelNode::model_transform]
    pub fn local_transform(&mut self, transform_model_space: impl Into<Matrix>) -> &mut Self {
        unsafe { model_node_set_transform_local(self.model.0.as_ptr(), self.id, transform_model_space.into()) };
        self
    }

    /// Adds a Child node below this node, at the end of the child chain! The local transform of the child will have
    /// this node as reference
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/AddChild.html>
    /// * `name` - A text name to identify the node.
    /// * `local_transform` - A Matrix describing this node’s transform in local space relative to the currently selected
    ///   node.
    /// * `mesh` - The Mesh to attach to this Node’s visual. If None, the material must also be None.
    /// * `material` - The Material to attach to this Node’s visual. If None, the mesh must also be None.
    /// * `solid` - A flag that indicates the Mesh for this node will be used in ray intersection tests. This flag
    ///   is ignored if no Mesh is attached.
    ///
    /// see also [Nodes::add] [`model_node_add_child`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    /// let cube = Mesh::generate_cube([0.1, 0.1, 0.1], None);
    /// let sphere = Mesh::generate_sphere(0.15, None);
    /// let material = Material::pbr();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("root_mesh", Matrix::IDENTITY, Some(&cube), Some(&material), true);
    /// let mut root_node = nodes.get_root_node().expect("A node should exist!");
    ///
    /// root_node
    ///     .add_child("child_mesh2", Matrix::IDENTITY, Some(&sphere), Some(&material), true)
    ///     .add_child("child_no_mesh", Matrix::IDENTITY, None, None, false);
    /// ```
    pub fn add_child<S: AsRef<str>>(
        &mut self,
        name: S,
        local_transform: impl Into<Matrix>,
        mesh: Option<&Mesh>,
        material: Option<&Material>,
        solid: bool,
    ) -> &mut Self {
        let c_str = CString::new(name.as_ref()).unwrap();
        let mesh = match mesh {
            Some(mesh) => mesh.0.as_ptr(),
            None => null_mut(),
        };
        let material = match material {
            Some(material) => material.0.as_ptr(),
            None => null_mut(),
        };
        unsafe {
            model_node_add_child(
                self.model.0.as_ptr(),
                self.id,
                c_str.as_ptr(),
                local_transform.into(),
                mesh,
                material,
                solid as Bool32T,
            )
        };
        self
    }

    /// Get the node Id
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("mosh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mush", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let node = nodes.find("mesh").expect("Node mesh should exist");
    /// assert_eq!(node.get_id(), 1);
    ///
    /// let node = nodes.find("mush").expect("Node mesh should exist");
    /// assert_eq!(node.get_id(), 2);
    /// ```
    pub fn get_id(&self) -> ModelNodeId {
        self.id
    }

    /// Get the node Name
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Name.html>
    ///
    /// see also [`model_node_get_name`]
    /// see example [`ModelNode::name`]
    pub fn get_name(&self) -> Option<&str> {
        unsafe { CStr::from_ptr(model_node_get_name(self.model.0.as_ptr(), self.id)).to_str().ok() }
    }

    /// Get the solid of the node. A flag that indicates if the Mesh for this node will be used in ray intersection tests.
    /// This flag is ignored if no Mesh is attached.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Solid.html>
    ///
    /// see also [`model_node_get_solid`]
    /// see example [`ModelNode::solid`]
    pub fn get_solid(&self) -> bool {
        unsafe { model_node_get_solid(self.model.0.as_ptr(), self.id) != 0 }
    }

    /// Get the visibility of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Visible.html>
    ///
    /// see also [`model_node_get_visible`]
    /// see example [`ModelNode::visible`]
    pub fn get_visible(&self) -> bool {
        unsafe { model_node_get_visible(self.model.0.as_ptr(), self.id) != 0 }
    }

    /// Get the material of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Material.html>
    ///
    /// see also [`model_node_get_material`]
    /// see example [`ModelNode::material`]
    pub fn get_material(&self) -> Option<Material> {
        NonNull::new(unsafe { model_node_get_material(self.model.0.as_ptr(), self.id) }).map(Material)
    }

    /// Get the mesh of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Mesh.html>
    ///
    /// see also [`model_node_get_mesh`]
    /// see example [`ModelNode::mesh`]
    pub fn get_mesh(&self) -> Option<Mesh> {
        NonNull::new(unsafe { model_node_get_mesh(self.model.0.as_ptr(), self.id) }).map(Mesh)
    }

    /// Get the transform matrix of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/ModelTransform.html>
    ///
    /// see also [`model_node_get_transform_model`]
    /// see example [`ModelNode::model_transform`]
    pub fn get_model_transform(&self) -> Matrix {
        unsafe { model_node_get_transform_model(self.model.0.as_ptr(), self.id) }
    }

    /// Get the local transform matrix of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/LocalTransform.html>
    ///
    /// see also [`model_node_get_transform_local`]
    /// see example [`ModelNode::model_transform`]
    pub fn get_local_transform(&self) -> Matrix {
        unsafe { model_node_get_transform_local(self.model.0.as_ptr(), self.id) }
    }

    /// Iterate to the next node
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [`model_node_iterate`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("mosh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mush", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let mut node = nodes.find("mosh").expect("Node mosh should exist");
    /// let mut next_node = node.iterate().expect("Node should have a follower");
    /// assert_eq!(next_node.get_name(), Some("mesh"));
    ///
    /// next_node.add_child("mesh child", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// let next_node = next_node.iterate().expect("Node should have a follower");
    /// assert_eq!(next_node.get_name(), Some("mesh child"));
    ///
    /// let next_node = next_node.iterate().expect("Node should have a follower");
    /// assert_eq!(next_node.get_name(), Some("mush"));
    ///
    /// let next_node = next_node.iterate();
    /// assert!(next_node.is_none());
    /// ```
    pub fn iterate(&self) -> Option<ModelNode> {
        match unsafe { model_node_iterate(self.model.0.as_ptr(), self.id) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// Get the The first child node “below” on the hierarchy tree, or null if there are none. To see all children,
    /// get the Child and then iterate through its Siblings.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Child.html>
    ///
    /// see also [`model_node_child`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("mosh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mush", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let mut node = nodes.find("mesh").expect("Node mosh should exist");
    /// node.add_child("mesh child1", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// node.add_child("mesh child2", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let child_node = node.get_child().expect("Node should have a child");
    /// assert_eq!(child_node.get_name(), Some("mesh child1"));
    /// ```
    pub fn get_child(&self) -> Option<ModelNode> {
        match unsafe { model_node_child(self.model.0.as_ptr(), self.id) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// The next ModelNode in the hierarchy, at the same level as this one. To the “right” on a hierarchy tree.
    /// None if there are no more ModelNodes in the tree there.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Sibling.html>
    ///
    /// see also [`model_node_sibling`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("mosh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mush", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let mut node = nodes.find("mesh").expect("Node mosh should exist");
    /// node.add_child("mesh child1", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// node.add_child("mesh child2", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let child_node = node.get_child().expect("Node should have a child");
    /// assert_eq!(child_node.get_name(), Some("mesh child1"));
    ///
    /// let child_node = child_node.get_sibling().expect("Child_node should have a sibling");
    /// assert_eq!(child_node.get_name(), Some("mesh child2"));
    ///
    /// let sibling_node = node.get_sibling().expect("Node should have a sibling");
    /// assert_eq!(sibling_node.get_name(), Some("mush"));
    /// ```
    pub fn get_sibling(&self) -> Option<ModelNode> {
        match unsafe { model_node_sibling(self.model.0.as_ptr(), self.id) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// The ModelNode above this one (“up”) in the hierarchy tree, or None if this is a root node.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Parent.html>
    ///
    /// see also [`model_node_parent`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// nodes.add("mush", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let mut node = nodes.find("mesh").expect("Node mosh should exist");
    /// node.add_child("mesh child1", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// node.add_child("mesh child2", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    ///
    /// let child_node = node.get_child().expect("Node should have a child");
    /// assert_eq!(child_node.get_name(), Some("mesh child1"));
    /// assert_eq!(child_node.get_parent().unwrap().get_name(), Some("mesh"));
    ///
    /// let child_node = child_node.get_sibling().expect("Child_node should have a sibling");
    /// assert_eq!(child_node.get_name(), Some("mesh child2"));
    /// assert_eq!(child_node.get_parent().unwrap().get_name(), Some("mesh"));
    ///
    /// // Mesh is it's own parent.
    /// assert_eq!(child_node.get_parent().unwrap().get_name(), Some("mesh"));
    /// ```
    pub fn get_parent(&self) -> Option<ModelNode> {
        match unsafe { model_node_parent(self.model.0.as_ptr(), self.id) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// The next ModelNode  in the hierarchy tree, or None if this is the last node.
    /// <https://stereokit.net/Pages/StereoKit/Model/ModelNodeCollection.html>
    ///
    /// see also [`model_node_iterate`]
    /// same as [`ModelNode::iterate`]
    pub fn get_next(&self) -> Option<ModelNode> {
        self.iterate()
    }

    /// The whole model in which this node belongs
    /// <https://stereokit.net/Pages/StereoKit/Model.html>
    ///
    /// see also [`Model`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::Model, mesh::Mesh, material::Material};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("mesh", Matrix::IDENTITY, Some(&Mesh::cube()), Some(&Material::pbr()), true);
    /// let node = nodes.get_root_node().expect("We should have a root node");
    /// assert_eq!(node.get_name(), Some("mesh"));
    /// assert_eq!(node.get_model(), &model);
    /// ```
    pub fn get_model(&self) -> &Model {
        self.model
    }

    /// Get Info for this node
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::{Model, Info}};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("some_info", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.get_root_node().expect("We should have a root node");
    /// let mut infos = node.get_infos();
    /// infos.set_info( "name1", "value1");
    /// infos.set_info( "name2", "value2");
    ///
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// for (item, info) in infos.enumerate() {
    ///    match item {
    ///        0 => assert_eq!(info, Info { name: "name2".to_string(), value: "value2".to_string() }),
    ///        _ => assert_eq!(info, Info { name: "name1".to_string(), value: "value1".to_string() }),
    ///    }
    /// }
    /// ```
    pub fn get_infos(&self) -> Infos {
        Infos::from(self)
    }
}

/// Infos of a ModelNode
/// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection.html>
///
/// see also [`ModelNode`]
pub struct Infos<'a> {
    model: &'a Model,
    node_id: ModelNodeId,
    curr: i32,
}

impl Iterator for Infos<'_> {
    type Item = Info;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(res) = Infos::info_iterate(self.model, self.curr, self.node_id) {
            self.curr = res.2;
            Some(Info { name: res.0.to_string(), value: res.1.to_string() })
        } else {
            None
        }
    }
}

/// One Info of a ModelNode
/// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection.html>
#[derive(Debug, Clone, PartialEq)]
pub struct Info {
    pub name: String,
    pub value: String,
}

impl<'a> Infos<'a> {
    /// Helper to get the collection struct same as [`ModelNode::get_infos`]
    pub fn from(node: &'a ModelNode) -> Infos<'a> {
        Infos { model: node.model, node_id: node.id, curr: 0 }
    }

    /// iterator of the node infos
    fn info_iterate(model: &Model, mut iterator: i32, node: ModelNodeId) -> Option<(&str, &str, i32)> {
        let out_key_utf8 = CString::new("H").unwrap().into_raw() as *mut *const c_char;
        let out_value_utf8 = CString::new("H").unwrap().into_raw() as *mut *const c_char;

        let ref_iterator = &mut iterator as *mut i32;

        unsafe {
            let res = model_node_info_iterate(model.0.as_ptr(), node, ref_iterator, out_key_utf8, out_value_utf8);
            if res != 0 {
                let key = CStr::from_ptr(*out_key_utf8);
                let value = CStr::from_ptr(*out_value_utf8);
                Some((key.to_str().unwrap(), value.to_str().unwrap(), *ref_iterator))
            } else {
                None
            }
        }
    }

    /// Clear all infos
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Clear.html>
    ///
    /// see also [`model_node_info_clear`] [`ModelNode::get_infos`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::{Model, Info}};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("some_info", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.get_root_node().expect("We should have a root node");
    /// let mut infos = node.get_infos();
    /// infos.set_info( "name1", "value1");
    /// infos.set_info( "name2", "value2");
    ///
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// infos.clear();
    /// assert_eq!(infos.get_count(), 0);
    /// ```
    pub fn clear(&mut self) -> &mut Self {
        unsafe { model_node_info_clear(self.model.0.as_ptr(), self.node_id) };
        self
    }

    /// Remove the first occurence found of an info. An error is logged if the info do not exist
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Remove.html>
    ///
    /// see also [`model_node_info_remove`] [`ModelNode::get_infos`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::{Model, Info}};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("some_info", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.get_root_node().expect("We should have a root node");
    /// let mut infos = node.get_infos();
    /// infos.set_info( "name1", "value1");
    /// infos.set_info( "name2", "value2");
    ///
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// infos.remove_info("name1000");
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// infos.remove_info("name1");
    /// assert_eq!(infos.get_count(), 1);
    ///
    /// assert_eq!(infos.get_info("name1"), None);
    /// assert_eq!(infos.get_info("name2"), Some("value2"));
    /// ```
    pub fn remove_info<S: AsRef<str>>(&mut self, info_key_utf8: S) -> &mut Self {
        let c_str = CString::new(info_key_utf8.as_ref()).unwrap();
        unsafe {
            if model_node_info_remove(self.model.0.as_ptr(), self.node_id, c_str.as_ptr()) == 0 {
                Log::err(format!("Info {:?} was not found during remove", info_key_utf8.as_ref()));
            }
        }
        self
    }

    /// Set an info value to this node (key is unique). The last added key (if multiple) will be the first found.
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Add.html>
    ///
    /// see also [`model_node_info_set`] [`ModelNode::get_infos`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::{Model, Info}};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("some_info", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.get_root_node().expect("We should have a root node");
    /// let mut infos = node.get_infos();
    /// infos.set_info( "name1", "value111");
    /// infos.set_info( "name1", "value1");
    /// infos.set_info( "name2", "value2");
    ///
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// assert_eq!(infos.get_info("name1"), Some("value1"));
    /// assert_eq!(infos.get_info("name2"), Some("value2"));
    /// ```
    pub fn set_info<S: AsRef<str>>(&mut self, info_key_utf8: S, info_value_utf8: S) -> &mut Self {
        let c_str = CString::new(info_key_utf8.as_ref()).unwrap();
        let c_value = CString::new(info_value_utf8.as_ref()).unwrap();
        unsafe { model_node_info_set(self.model.0.as_ptr(), self.node_id, c_str.as_ptr(), c_value.as_ptr()) };
        self
    }

    /// Get the first info value of this node corresponding to the key.
    /// Return None if the key doesn't exist
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Get.html>
    ///
    /// see also [`model_node_info_get`] [`ModelNode::get_infos`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::{Model, Info}};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("some_info", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.get_root_node().expect("We should have a root node");
    /// let mut infos = node.get_infos();
    /// infos.set_info( "name1", "value1");
    /// infos.set_info( "name2", "value2");
    ///
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// assert_eq!(infos.get_info("name1"), Some("value1"));
    /// assert_eq!(infos.get_info("name2"), Some("value2"));
    /// assert_eq!(infos.get_info("name3333"), None);
    /// ```
    pub fn get_info<S: AsRef<str>>(&self, info_key_utf8: S) -> Option<&str> {
        let c_str = CString::new(info_key_utf8.as_ref()).unwrap();
        match NonNull::new(unsafe { model_node_info_get(self.model.0.as_ptr(), self.node_id, c_str.as_ptr()) }) {
            Some(non_null) => unsafe { CStr::from_ptr(non_null.as_ref()).to_str().ok() },
            None => None,
        }
    }

    /// Check if there is a node corresponding to the key.
    /// (get_info is more efficient, thanks to rust)
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Contains.html>
    ///
    /// see also [`model_node_info_get`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::{Model, Info}};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("some_info", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.get_root_node().expect("We should have a root node");
    /// let mut infos = node.get_infos();
    /// infos.set_info( "name1", "value111");
    /// infos.set_info( "name1", "value1");
    /// infos.set_info( "name2", "value2");
    ///
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// assert_eq!(infos.contains("name1"), true);
    /// assert_eq!(infos.contains("name2"), true);
    /// assert_eq!(infos.contains("name333"), false);
    /// ```
    pub fn contains<S: AsRef<str>>(&self, info_key_utf8: S) -> bool {
        self.get_info(info_key_utf8).is_some()
    }

    /// Get the number of infos for this node
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Count.html>
    ///
    /// see also [`model_node_info_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, model::{Model, Info}};
    ///
    /// let model = Model::new();
    ///
    /// let mut nodes = model.get_nodes();
    /// nodes.add("some_info", Matrix::IDENTITY, None, None, false);
    ///
    /// let mut node = nodes.get_root_node().expect("We should have a root node");
    /// let mut infos = node.get_infos();
    /// infos.set_info( "name0", "value0");
    /// assert_eq!(infos.get_count(), 1);
    ///
    /// infos.set_info( "name1", "value1");
    /// assert_eq!(infos.get_count(), 2);
    ///
    /// infos.set_info( "name2", "value2");
    /// assert_eq!(infos.get_count(), 3);
    ///
    /// infos.clear();
    /// assert_eq!(infos.get_count(), 0);
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe { model_node_info_count(self.model.0.as_ptr(), self.node_id) }
    }
}
