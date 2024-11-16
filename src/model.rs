use crate::maths::{Bool32T, Matrix};
use crate::sk::MainThreadToken;
use crate::{
    material::{Cull, Material, MaterialT},
    maths::{Bounds, Ray, Vec3},
    mesh::{Mesh, MeshT},
    shader::{Shader, ShaderT},
    system::{IAsset, Log, RenderLayer},
    util::Color128,
    StereoKitError,
};
use std::{
    ffi::{c_char, c_void, CStr, CString},
    path::Path,
    ptr::{null_mut, NonNull},
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
#[derive(Debug)]
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

#[repr(C)]
#[derive(Debug)]
pub struct _ModelT {
    _unused: [u8; 0],
}
pub type ModelT = *mut _ModelT;

extern "C" {
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
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Create an empty model
    /// <https://stereokit.net/Pages/StereoKit/Model/Model.html>
    ///
    /// see also [`crate::model::model_create`]
    pub fn new() -> Model {
        Model(NonNull::new(unsafe { model_create() }).unwrap())
    }

    /// Creates a single mesh subset Model using the indicated Mesh and Material!
    /// An id will be automatically generated for this asset.
    /// <https://stereokit.net/Pages/StereoKit/Model/FromMesh.html>
    ///
    /// see also [`crate::model::model_create_mesh`]
    pub fn from_mesh<Me: AsRef<Mesh>, Ma: AsRef<Material>>(mesh: Me, material: Ma) -> Model {
        Model(
            NonNull::new(unsafe { model_create_mesh(mesh.as_ref().0.as_ptr(), material.as_ref().0.as_ptr()) }).unwrap(),
        )
    }

    /// Loads a list of mesh and material subsets from a .obj, .stl, .ply (ASCII),
    /// .gltf, or .glb file stored in memory. Note that this function won’t work
    /// well on files that reference other files, such as .gltf files with
    /// references in them.
    /// <https://stereokit.net/Pages/StereoKit/Model/FromMemory.html>
    ///
    /// see also [`crate::model::model_create_mem`]
    pub fn from_memory<S: AsRef<str>>(
        file_name: S,
        memory: &[u8],
        shader: Option<Shader>,
    ) -> Result<Model, StereoKitError> {
        let c_file_name = CString::new(file_name.as_ref())?;
        let shader = shader.map(|shader| shader.0.as_ptr()).unwrap_or(null_mut());
        match NonNull::new(unsafe {
            model_create_mem(c_file_name.as_ptr(), memory.as_ptr() as *const c_void, memory.len(), shader)
        }) {
            Some(model) => Ok(Model(model)),
            None => Err(StereoKitError::ModelFromMem(file_name.as_ref().to_owned(), "file not found!".to_owned())),
        }
    }

    /// Loads a list of mesh and material subsets from a .obj, .stl, .ply (ASCII), .gltf, or .glb file.
    /// <https://stereokit.net/Pages/StereoKit/Model/FromFile.html>
    ///
    /// see also [`crate::model::model_create_file`]
    pub fn from_file(file_utf8: impl AsRef<Path>, shader: Option<Shader>) -> Result<Model, StereoKitError> {
        let path = file_utf8.as_ref();
        let path_buf = path.to_path_buf();
        let c_str = CString::new(path.to_str().unwrap())?;
        let shader = shader.map(|shader| shader.0.as_ptr()).unwrap_or(null_mut());
        match NonNull::new(unsafe { model_create_file(c_str.as_ptr(), shader) }) {
            Some(model) => Ok(Model(model)),
            None => Err(StereoKitError::ModelFromFile(path_buf.to_owned(), "file not found!".to_owned())),
        }
    }
    /// Creates a new Model from an existing one.
    /// <https://stereokit.net/Pages/StereoKit/Model/Copy.html>
    ///
    /// see also [`crate::model::model_copy()`]
    pub fn copy(model: impl AsRef<Model>) -> Model {
        Model(NonNull::new(unsafe { model_copy(model.as_ref().0.as_ptr()) }).unwrap())
    }

    /// Looks for a Model asset that’s already loaded, matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/Model/Find.html>
    ///
    /// see also [`crate::model::model_find`]
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
    /// see also [`crate::model::model_find()`]
    pub fn clone_ref(&self) -> Model {
        Model(NonNull::new(unsafe { model_find(model_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    //-----------------Modify Model :
    /// Set a new id to the model.
    /// <https://stereokit.net/Pages/StereoKit/Model/Id.html>
    ///
    /// see also [`crate::model::model_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { model_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Set the bounds of this model. This is a bounding box that encapsulates the Model and all its subsets! It’s used for collision,
    /// visibility testing, UI layout, and probably other things. While it’s normally calculated from the mesh bounds, you can also override this to suit your needs.
    /// <https://stereokit.net/Pages/StereoKit/Model/Bounds.html>
    ///
    /// see also [`crate::model::model_set_bounds`][`crate::model::model_recalculate_bounds`]
    pub fn bounds(&mut self, bounds: impl AsRef<Bounds>) -> &mut Self {
        unsafe { model_set_bounds(self.0.as_ptr(), bounds.as_ref()) };
        self
    }

    /// Adds the model to the render queue of this frame
    /// <https://stereokit.net/Pages/StereoKit/Model/Draw.html>
    /// * color_linear - if None has default value of WHITE
    /// * layer - if None has default value of Layer0
    ///
    /// see also [`stereokit::StereoKitDraw::model_draw`]
    pub fn draw(
        &self,
        _token: &MainThreadToken,
        transform: impl Into<Matrix>,
        color_linear: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color_linear = match color_linear {
            Some(c) => c,
            None => Color128::WHITE,
        };
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe { model_draw(self.0.as_ptr(), transform.into(), color_linear, layer) };
    }

    /// Adds the model to the render queue of this frame overrided with the given material
    /// <https://stereokit.net/Pages/StereoKit/Model/Draw.html>
    /// * material_override - the material that will override all materials of this model
    /// * color_linear - if None has default value of WHITE
    /// * layer - if None has default value of Layer0
    ///
    /// see also [`stereokit::StereoKitDraw::model_draw`]
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
    /// see also [`crate::model::model_recalculate_bounds`][`crate::model::model_set_bounds`]
    pub fn recalculate_bounds(&self) {
        unsafe { model_recalculate_bounds(self.0.as_ptr()) };
    }

    /// Examines the visuals as they currently are, and rebuilds the bounds based on all the vertices in the model! This leads (in general) to a tighter bound than the
    /// default bound based on bounding boxes. However, computing the exact bound can take much longer!
    /// <https://stereokit.net/Pages/StereoKit/Model/RecalculateBoundsExact.html>
    ///
    /// see also [`crate::model::model_recalculate_bounds_exact`][`crate::model::model_set_bounds`]
    pub fn recalculate_bounds_exact(&self) {
        unsafe { model_recalculate_bounds_exact(self.0.as_ptr()) };
    }

    /// Get the Id
    /// <https://stereokit.net/Pages/StereoKit/Model/Id.html>
    ///
    /// see also [`crate::model::model_get_bounds`][`crate::model::model_set_bounds`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(model_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// Get the bounds
    /// <https://stereokit.net/Pages/StereoKit/Model/Bounds.html>
    ///
    /// see also [`crate::model::model_get_bounds`][`crate::model::model_set_bounds`]
    pub fn get_bounds(&self) -> Bounds {
        unsafe { model_get_bounds(self.0.as_ptr()) }
    }

    /// Get the nodes
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [Nodes]
    pub fn get_nodes(&self) -> Nodes {
        Nodes::from(self)
    }

    /// Get the anims
    /// <https://stereokit.net/Pages/StereoKit/ModelAnimCollection.html>
    ///
    /// see also [Anims]
    pub fn get_anims(&self) -> Anims {
        Anims::from(self)
    }

    /// Checks the intersection point of a ray and the Solid flagged Meshes in the Model’s visual nodes. Ray must
    /// be in model space, intersection point will be in model space too. You can use the inverse of the mesh’s world
    /// transform matrix to bring the ray into model space, see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Model/Intersect.html>
    /// * cull - If None has default value of Cull::Back.
    ///
    /// see also [`stereokit::model_ray_intersect`]
    #[inline]
    pub fn intersect_model(&self, ray: Ray, cull: Option<Cull>) -> Option<Vec3> {
        ray.intersect_model(self, cull)
    }

    /// Checks the intersection point of a ray and the Solid flagged Meshes in the Model’s visual nodes. Ray must
    /// be in model space, intersection point will be in model space too. You can use the inverse of the mesh’s world
    /// transform matrix to bring the ray into model space, see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Model/Intersect.html>
    /// * cull - If None has default value of Cull::Back.
    ///
    /// see also [`stereokit::model_ray_intersect`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn intersect_model_to_ptr(&self, ray: Ray, cull: Option<Cull>, out_ray: *mut Ray) -> bool {
        ray.intersect_model_to_ptr(self, cull, out_ray)
    }
}

/// Animations of a Model
/// <https://stereokit.net/Pages/StereoKit/ModelAnimCollection.html>
///
/// see also [`stereokit::Model`]
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

impl<'a> Iterator for Anims<'a> {
    type Item = Anim;

    fn next(&mut self) -> Option<Self::Item> {
        self.curr += 1;
        if self.curr < self.get_count() {
            return Some(Anim {
                name: match self.get_name_at_index(self.curr) {
                    Some(name) => name.to_string(),
                    None => {
                        Log::err(format!("animation {:?}, is missing", self.curr));
                        "<<error !!>>".to_string()
                    }
                },
                duration: self.get_duration_at_index(self.curr),
            });
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
    pub fn from<M: AsRef<Model>>(model: &'a M) -> Anims<'a> {
        Anims { model: model.as_ref(), curr: -1 }
    }

    /// Get the name of the animation at given index
    fn get_name_at_index(&self, index: i32) -> Option<&str> {
        unsafe { CStr::from_ptr(model_anim_get_name(self.model.0.as_ptr(), index)) }.to_str().ok()
    }

    /// Get the duration of the animation at given index
    fn get_duration_at_index(&self, index: i32) -> f32 {
        unsafe { model_anim_get_duration(self.model.0.as_ptr(), index) }
    }

    /// Calling Draw will automatically step the Model’s animation, but if you don’t draw the Model, or need access to
    /// the animated nodes before drawing,
    /// then you can step the animation early manually via this method. Animation will only ever be stepped once per
    /// frame, so it’s okay to call this multiple times,
    /// or in addition to Draw.
    /// <https://stereokit.net/Pages/StereoKit/Model/StepAnim.html>
    ///
    /// see also [`crate::model::model_step_anim`][`crate::model::model_play_anim`]
    pub fn step_anim(&mut self) -> &mut Self {
        unsafe { model_step_anim(self.model.0.as_ptr()) };
        self
    }

    /// Searches for an animation with the given name, and if it’s found, sets it up as the active animation and begins
    /// laying it with the animation mode.
    /// <https://stereokit.net/Pages/StereoKit/Model/PlayAnim.html>
    ///
    /// see also [`crate::model::model_step_anim`][`crate::model::model_play_anim`]
    pub fn play_anim(&mut self, animation_name: impl AsRef<str>, mode: AnimMode) -> &mut Self {
        let c_str = CString::new(animation_name.as_ref()).unwrap();
        unsafe { model_play_anim(self.model.0.as_ptr(), c_str.as_ptr(), mode) };
        self
    }

    /// Sets it up the animation at index idx as the active animation and begins playing it with the animation mode.
    /// <https://stereokit.net/Pages/StereoKit/Model/PlayAnim.html>
    ///
    /// see also [`crate::model::model_play_anim_idx`][`crate::model::model_play_anim`]
    pub fn play_anim_idx(&mut self, idx: i32, mode: AnimMode) -> &mut Self {
        unsafe { model_play_anim_idx(self.model.0.as_ptr(), idx, mode) };
        self
    }

    /// This is the current time of the active animation in seconds, from the start of the animation. If no animation is
    /// active, this will be zero. This will always be a value between zero and the active animation’s Duration. For a
    /// percentage of completion, see AnimCompletion instead.
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimTime.html>
    ///
    /// see also [`crate::model::model_set_anim_time`][`crate::model::model_play_anim`]
    pub fn anim_time(&mut self, time: f32) -> &mut Self {
        unsafe { model_set_anim_time(self.model.0.as_ptr(), time) };
        self
    }

    /// This is the percentage of completion of the active animation. This will always be a value between 0-1. If no
    /// animation is active, this will be zero.
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimCompletion.html>
    ///
    /// see also [`crate::model::model_set_anim_completion`][`crate::model::model_play_anim`]
    pub fn anim_completion(&mut self, percent: f32) -> &mut Self {
        unsafe { model_set_anim_completion(self.model.0.as_ptr(), percent) };
        self
    }

    /// get anim by name
    /// <https://stereokit.net/Pages/StereoKit/Model/FindAnim.html>
    ///
    /// see also [`crate::model::model_anim_find`][`crate::model::model_play_anim`]
    pub fn find_anim<S: AsRef<str>>(&self, name: S) -> Option<i32> {
        let c_str = match CString::new(name.as_ref()) {
            Ok(c_str) => c_str,
            Err(..) => return None,
        };
        let index = unsafe { model_anim_find(self.model.0.as_ptr(), c_str.as_ptr()) };
        if index < 0 {
            None
        } else {
            Some(index)
        }
    }

    /// Get the number of animations
    /// <https://stereokit.net/Pages/StereoKit/Model/ModelAnimCollection.html>
    ///
    /// see also [`crate::model::model_anim_count`]
    pub fn get_count(&self) -> i32 {
        unsafe { model_anim_count(self.model.0.as_ptr()) }
    }

    /// Get the current animation
    /// <https://stereokit.net/Pages/StereoKit/Model/ActiveAnim.html>
    ///
    /// see also [`crate::model::model_anim_active`]
    pub fn get_active_anim(&self) -> i32 {
        unsafe { model_anim_active(self.model.0.as_ptr()) }
    }

    /// Get the current animation, mode
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimMode.html>
    ///
    /// see also [`crate::model::model_anim_active_mode`]
    pub fn get_anim_mode(&self) -> AnimMode {
        unsafe { model_anim_active_mode(self.model.0.as_ptr()) }
    }

    /// Get the current animation duration
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimTime.html>
    ///
    /// see also [`crate::model::model_anim_time`]
    pub fn get_anim_time(&self) -> f32 {
        unsafe { model_anim_active_time(self.model.0.as_ptr()) }
    }

    /// Get the current animation completion %
    /// <https://stereokit.net/Pages/StereoKit/Model/AnimCompletion.html>
    ///
    /// see also [`crate::model::model_anim_active_completion`]
    pub fn get_anim_completion(&self) -> f32 {
        unsafe { model_anim_active_completion(self.model.0.as_ptr()) }
    }
}

/// Nodes of a Model
/// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
/// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
///
/// see also [`stereokit::ModelNodeId`]
#[derive(Debug, Copy, Clone)]
pub struct Nodes<'a> {
    model: &'a Model,
}

extern "C" {
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
/// see also [Nodes::all][Nodes::visuals]
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
    ///Get an iterator for all node of the given model
    pub fn all_from(model: &'a impl AsRef<Model>) -> NodeIter<'a> {
        NodeIter { index: -1, model: model.as_ref(), visual: false }
    }

    ///Get an iterator for all visual node of the given model
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
    /// you’ll be able to access it via [get_root].
    /// <https://stereokit.net/Pages/StereoKit/Model/AddNode.html>
    ///
    /// see also [ModelNode::add_child] [`crate::model::model_node_add`]
    pub fn add<S: AsRef<str>>(
        &mut self,
        name: S,
        local_transform: impl Into<Matrix>,
        mesh: impl AsRef<Mesh>,
        material: impl AsRef<Material>,
        solid: bool,
    ) -> &mut Self {
        let c_str = CString::new(name.as_ref()).unwrap();
        unsafe {
            model_node_add(
                self.model.0.as_ptr(),
                c_str.as_ptr(),
                local_transform.into(),
                mesh.as_ref().0.as_ptr(),
                material.as_ref().0.as_ptr(),
                solid as Bool32T,
            )
        };
        self
    }

    /// Get an iterator of all the nodes
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [NodeIter::all_from]
    pub fn all(&self) -> NodeIter {
        NodeIter::all_from(self.model)
    }

    /// Get an iterator of all the visual nodes
    /// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
    ///
    /// see also [NodeIter::visuals_from]
    pub fn visuals(&self) -> NodeIter {
        NodeIter::visuals_from(self.model)
    }

    /// get node by name
    /// <https://stereokit.net/Pages/StereoKit/Model/FindNode.html>
    ///
    /// see also [`crate::model::model_node_find`]
    pub fn find<S: AsRef<str>>(&self, name: S) -> Option<ModelNode> {
        let c_str = CString::new(name.as_ref()).unwrap();
        match unsafe { model_node_find(self.model.0.as_ptr(), c_str.as_ptr()) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// Get the number of node
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [NodeIter][`crate::model::model_node_count`]
    pub fn get_count(&self) -> i32 {
        unsafe { model_node_count(self.model.0.as_ptr()) }
    }

    /// Get the number of visual node
    /// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
    ///
    /// see also [NodeIter][`crate::model::model_node_visual_count`]
    pub fn get_visual_count(&self) -> i32 {
        unsafe { model_node_visual_count(self.model.0.as_ptr()) }
    }

    /// Get the node at index
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [NodeIter][`crate::model::model_node_index`]
    pub fn get_index(&self, index: i32) -> Option<ModelNode> {
        match unsafe { model_node_index(self.model.0.as_ptr(), index) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// Get the visual node at index
    /// <https://stereokit.net/Pages/StereoKit/ModelVisualCollection.html>
    ///
    /// see also [NodeIter][`crate::model::model_node_visual_index`]
    pub fn get_visual_index(&self, index: i32) -> Option<ModelNode> {
        match unsafe { model_node_visual_index(self.model.0.as_ptr(), index) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// Get the root node
    /// <https://stereokit.net/Pages/StereoKit/Model/RootNode.html>
    ///
    /// see also [`crate::model::model_node_get_root`]
    pub fn get_root_node(&self) -> ModelNode {
        ModelNode { model: self.model, id: unsafe { model_node_get_root(self.model.0.as_ptr()) } }
    }
}

/// ModelNode
/// <https://stereokit.net/Pages/StereoKit/ModelNode.html>

#[derive(Debug, Copy, Clone)]
pub struct ModelNode<'a> {
    model: &'a Model,
    id: ModelNodeId,
}
pub type ModelNodeId = i32;

impl<'a> ModelNode<'a> {
    /// Set the name of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Name.html>
    ///
    /// see also [`crate::model::model_node_set_name`]
    pub fn name<S: AsRef<str>>(&mut self, name: S) -> &mut Self {
        let c_str = CString::new(name.as_ref()).unwrap();
        unsafe { model_node_set_name(self.model.0.as_ptr(), self.id, c_str.as_ptr()) };
        self
    }

    /// Set the solid of the node. A flag that indicates the Mesh for this node will be used in ray intersection tests.
    /// This flag is ignored if no Mesh is attached.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Solid.html>
    ///
    /// see also [`crate::model::model_node_set_solid`]
    pub fn solid(&mut self, solid: bool) -> &mut Self {
        unsafe { model_node_set_solid(self.model.0.as_ptr(), self.id, solid as Bool32T) };
        self
    }

    /// Set the visibility of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Visible.html>
    ///
    /// see also [`crate::model::model_node_set_visible`]
    pub fn visible(&mut self, visible: bool) -> &mut Self {
        unsafe { model_node_set_visible(self.model.0.as_ptr(), self.id, visible as Bool32T) };
        self
    }

    /// Set the material of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Material.html>
    ///
    /// see also [`crate::model::model_node_set_material`]
    pub fn material<M: AsRef<Material>>(&mut self, material: M) -> &mut Self {
        unsafe { model_node_set_material(self.model.0.as_ptr(), self.id, material.as_ref().0.as_ptr()) };
        self
    }

    /// Set the mesh of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Mesh.html>
    ///
    /// see also [`crate::model::model_node_set_mesh`]
    pub fn mesh<M: AsRef<Mesh>>(&mut self, mesh: M) -> &mut Self {
        unsafe { model_node_set_mesh(self.model.0.as_ptr(), self.id, mesh.as_ref().0.as_ptr()) };
        self
    }

    /// Set the transform model of the node. The transform of this node relative to the Model itself. This incorporates transforms from all parent nodes.
    /// Setting this transform will update the LocalTransform, as well as all Child nodes below this one.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/ModelTransform.html>
    ///
    /// see also [`crate::model::model_node_set_transform_model`]
    pub fn model_transform(&mut self, transform_model_space: impl Into<Matrix>) -> &mut Self {
        unsafe { model_node_set_transform_model(self.model.0.as_ptr(), self.id, transform_model_space.into()) };
        self
    }

    /// Set the local transform  of the node. The transform of this node relative to the Parent node.
    /// Setting this transform will update the ModelTransform, as well as all Child nodes below this one.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/LocalTransform.html>
    ///
    /// see also [`crate::model::model_node_set_transform_local`]
    pub fn local_transform(&mut self, transform_model_space: impl Into<Matrix>) -> &mut Self {
        unsafe { model_node_set_transform_local(self.model.0.as_ptr(), self.id, transform_model_space.into()) };
        self
    }

    /// Adds a Child node below this node, at the end of the child chain! The local transform of the child will have this node as reference
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/AddChild.html>
    ///
    /// see also [Node::add] [`crate::model::model_node_add_child`]
    pub fn add_child<S: AsRef<str>>(
        &mut self,
        name: S,
        local_transform: impl Into<Matrix>,
        mesh: impl AsRef<Mesh>,
        material: impl AsRef<Material>,
        solid: bool,
    ) -> &mut Self {
        let c_str = CString::new(name.as_ref()).unwrap();
        unsafe {
            model_node_add_child(
                self.model.0.as_ptr(),
                self.id,
                c_str.as_ptr(),
                local_transform.into(),
                mesh.as_ref().0.as_ptr(),
                material.as_ref().0.as_ptr(),
                solid as Bool32T,
            )
        };
        self
    }

    /// Get the node Id
    ///
    pub fn get_id(&self) -> &ModelNodeId {
        &self.id
    }

    /// Get the node Name
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Name.html>
    ///
    /// see also [`crate::model::model_node_get_name`]
    pub fn get_name(&self) -> Option<&str> {
        unsafe { CStr::from_ptr(model_node_get_name(self.model.0.as_ptr(), self.id)).to_str().ok() }
    }

    /// Get the solid of the node. A flag that indicates if the Mesh for this node will be used in ray intersection tests.
    /// This flag is ignored if no Mesh is attached.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Solid.html>
    ///
    /// see also [`crate::model::model_node_get_solid`]
    pub fn get_solid(&self) -> bool {
        unsafe { model_node_get_solid(self.model.0.as_ptr(), self.id) != 0 }
    }

    /// Get the visibility of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Visible.html>
    ///
    /// see also [`crate::model::model_node_get_visible`]
    pub fn get_visible(&self) -> bool {
        unsafe { model_node_get_visible(self.model.0.as_ptr(), self.id) != 0 }
    }

    /// Get the material of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Material.html>
    ///
    /// see also [`crate::model::model_node_get_material`]
    pub fn get_material(&self) -> Option<Material> {
        Some(Material(NonNull::new(unsafe { model_node_get_material(self.model.0.as_ptr(), self.id) })?))
    }

    /// Get the mesh of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Mesh.html>
    ///
    /// see also [`crate::model::model_node_get_mesh`]
    pub fn get_mesh(&self) -> Option<Mesh> {
        Some(Mesh(NonNull::new(unsafe { model_node_get_mesh(self.model.0.as_ptr(), self.id) })?))
    }

    /// Get the transform matrix of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/ModelTransform.html>
    ///
    /// see also [`crate::model::model_node_get_transform_model`]
    pub fn get_model_transform(&self) -> Matrix {
        unsafe { model_node_get_transform_model(self.model.0.as_ptr(), self.id) }
    }

    /// Get the local transform matrix of the node
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/LocalTransform.html>
    ///
    /// see also [`crate::model::model_node_get_transform_local`]
    pub fn get_local_transform(&self) -> Matrix {
        unsafe { model_node_get_transform_local(self.model.0.as_ptr(), self.id) }
    }

    /// Iterate to the next node
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeCollection.html>
    ///
    /// see also [`crate::model::model_node_iterate`]
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
    /// see also [`crate::model::model_node_child`]
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
    /// see also [`crate::model::model_node_sibling`]
    pub fn get_sibling(&self) -> Option<ModelNode> {
        match unsafe { model_node_sibling(self.model.0.as_ptr(), self.id) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// The ModelNode above this one (“up”) in the hierarchy tree, or None if this is a root node.
    /// <https://stereokit.net/Pages/StereoKit/ModelNode/Parent.html>
    ///
    /// see also [`crate::model::model_node_parent`]
    pub fn get_parent(&self) -> Option<ModelNode> {
        match unsafe { model_node_parent(self.model.0.as_ptr(), self.id) } {
            -1 => None,
            otherwise => Some(ModelNode { model: self.model, id: otherwise }),
        }
    }

    /// The next ModelNode  in the hierarchy tree, or None if this is the last node.
    /// <https://stereokit.net/Pages/StereoKit/Model/ModelNodeCollection.html>
    ///
    /// see also [`crate::model::model_node_iterate`]
    pub fn get_next(&self) -> Option<ModelNode> {
        self.iterate()
    }

    /// The whole model in which this node belongs
    /// <https://stereokit.net/Pages/StereoKit/Model.html>
    ///
    /// see also [`stereokit::Model`]
    pub fn get_model(&self) -> &Model {
        self.model
    }

    /// Get Info for this node
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection.html>
    ///
    pub fn get_infos(&self) -> Infos {
        Infos::from(self)
    }
}

/// Infos of a ModelNode
/// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection.html>
///
/// see also [`stereokit::ModelNode`]
pub struct Infos<'a> {
    model: &'a Model,
    node_id: ModelNodeId,
    curr: i32,
}

impl<'a> Iterator for Infos<'a> {
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
pub struct Info {
    pub name: String,
    pub value: String,
}

impl<'a> Infos<'a> {
    /// Helper to get the collection struct
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
    /// see also [`crate::model::model_node_info_clear`]
    pub fn clear(&mut self) -> &mut Self {
        unsafe { model_node_info_clear(self.model.0.as_ptr(), self.node_id) };
        self
    }

    /// Remove the first occurence found of an info. An error is logged if the info do not exist
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Remove.html>
    ///
    /// see also [`crate::model::model_node_info_remove`]
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
    /// see also [`crate::model::model_node_info_set`]
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
    /// see also [`crate::model::model_node_info_get`]
    pub fn get_info<S: AsRef<str>>(&self, info_key_utf8: S) -> Option<&str> {
        let c_str = CString::new(info_key_utf8.as_ref()).unwrap();
        match NonNull::new(unsafe { model_node_info_get(self.model.0.as_ptr(), self.node_id, c_str.as_ptr()) }) {
            Some(non_null) => return unsafe { CStr::from_ptr(non_null.as_ref()).to_str().ok() },
            None => None,
        }
    }

    /// Check if there is a node corresponding to the key.
    /// (get_info is more efficient, thanks to rust)
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Contains.html>
    ///
    /// see also [`crate::model::model_node_info_get`]
    pub fn contains<S: AsRef<str>>(&self, info_key_utf8: S) -> bool {
        self.get_info(info_key_utf8).is_some()
    }

    /// Get the number of infos for this node
    /// <https://stereokit.net/Pages/StereoKit/ModelNodeInfoCollection/Count.html>
    ///
    /// see also [`crate::model::model_node_info_count`]
    pub fn get_count(&self) -> i32 {
        unsafe { model_node_info_count(self.model.0.as_ptr(), self.node_id) }
    }
}
