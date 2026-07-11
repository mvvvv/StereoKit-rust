use crate::{
    StereoKitError,
    material::{Cull, Material, MaterialT},
    maths::{Bool32T, Bounds, Matrix, Ray, Vec2, Vec3, Vec4},
    render::RenderLayer,
    system::{AssetState, IAsset},
    util::{Color32, Color128},
};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    ptr::{NonNull, slice_from_raw_parts_mut},
};

/// This represents a single vertex in a Mesh, all StereoKit Meshes currently use this exact layout!
/// It’s good to fill out all values of a Vertex explicitly, as default values for the normal (0,0,0) and color
/// (0,0,0,0) will cause your mesh to appear completely black, or even transparent in most shaders!
/// <https://stereokit.net/Pages/StereoKit/Vertex.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Vec2, Matrix}, util::Color32,
///                      mesh::{Mesh, Vertex}, material::Material};
///
/// // Creating vertices with all fields specified
/// let vertices = [
///     Vertex::new(Vec3::ZERO,Vec3::UP,None,         Some(Color32::rgb(0, 0, 255))),
///     Vertex::new(Vec3::X,   Vec3::UP,Some(Vec2::X),Some(Color32::rgb(255, 0, 0))),
///     Vertex::new(Vec3::Y,   Vec3::UP,Some(Vec2::Y),Some(Color32::rgb(0,255, 0))),
/// ];
/// let indices = [0, 1, 2, 2, 1, 0];
/// let mut mesh = Mesh::new();
/// mesh.id("most_basic_mesh").keep_data(true).set_data(&vertices, &indices, None, None);
/// let material = Material::pbr();
///
/// filename_scr = "screenshots/basic_mesh.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     mesh.draw(&material, Matrix::IDENTITY, None, None);
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/basic_mesh.jpeg" alt="screenshot" width="200">
#[derive(Default, Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Vertex {
    /// Position of the vertex, in model space coordinates.
    pub pos: Vec3,
    /// The normal of this vertex, or the direction the vertex is facing. Preferably normalized.
    pub norm: Vec3,
    /// The texture coordinates at this vertex.
    pub uv: Vec2,
    /// The color of the vertex. If you aren’t using it, set it to white.
    pub col: Color32,
}

impl Vertex {
    /// Create a new Vertex.
    /// <https://stereokit.net/Pages/StereoKit/Vertex/Vertex.html>
    /// * `position` - Location of the vertex, this is typically meters in model space.
    /// * `normal` - The direction the Vertex is facing. Never leave this as zero, or your lighting may turn out black!
    ///   A good default value if you don’t know what to put here is (0,1,0), but a Mesh composed entirely of this value
    ///   will have flat lighting.
    /// * `texture_coordinate` - If None, set the value to Vec2::ZERO
    /// * `color` - If None, set the value to Color32::WHITE
    ///
    /// ### Examples
    /// ```
    /// use stereokit_rust::{maths::{Vec3, Vec2}, mesh::Vertex, util::Color32};
    ///
    /// let vertex = Vertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], None, None);
    /// let vertex_bis = Vertex{
    ///         pos: Vec3::new(0.0, 0.0, 0.0),
    ///         norm: Vec3::new(0.0, 1.0, 0.0),
    ///         uv: Vec2::ZERO,
    ///         col: Color32::WHITE};
    /// assert_eq!(vertex, vertex_bis);
    ///
    /// let vertex = Vertex::new([0.0, 0.0, 0.0], [0.0, 0.0, 0.0],
    ///                          Some(Vec2::ZERO), Some(Color32::BLACK_TRANSPARENT) );
    /// let vertex_default = Vertex::default();
    /// assert_eq!(vertex, vertex_default);
    /// ```
    pub fn new<V: Into<Vec3>>(
        position: V,
        normal: V,
        texture_coordinate: Option<Vec2>,
        color: Option<Color32>,
    ) -> Self {
        let texture_coordinate = texture_coordinate.unwrap_or(Vec2::ZERO);
        let color = color.unwrap_or(Color32::WHITE);
        Self { pos: position.into(), norm: normal.into(), uv: texture_coordinate, col: color }
    }
}

/// Mesh index data
/// <https://stereokit.net/Pages/StereoKit/Mesh.html>
pub type Inds = u32;

/// The data format of a single element of a vertex component. Normalized formats map their integer range onto 0-1
/// (unsigned) or -1-1 (signed) when read by the GPU, other integer formats arrive as integers.
/// <https://stereokit.net/Pages/StereoKit/VertFmt.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum VertFmt {
    /// Invalid format, this is not a valid value for a component.
    None = 0,
    /// 32 bit float.
    F32,
    /// 16 bit half float.
    F16,
    /// 32 bit signed integer.
    I32,
    /// 16 bit signed integer.
    I16,
    /// 8 bit signed integer.
    I8,
    /// 16 bit signed integer, normalized to -1-1 on the GPU.
    I16Normalized,
    /// 8 bit signed integer, normalized to -1-1 on the GPU.
    I8Normalized,
    /// 32 bit unsigned integer.
    U32,
    /// 16 bit unsigned integer.
    U16,
    /// 8 bit unsigned integer.
    U8,
    /// 16 bit unsigned integer, normalized to 0-1 on the GPU.
    U16Normalized,
    /// 8 bit unsigned integer, normalized to 0-1 on the GPU. A color32 is 4 of these.
    U8Normalized,
}

impl VertFmt {
    /// Returns the size in bytes of a single element of this format.
    pub const fn size(self) -> usize {
        match self {
            VertFmt::F32 | VertFmt::I32 | VertFmt::U32 => 4,
            VertFmt::F16 | VertFmt::I16 | VertFmt::U16 | VertFmt::I16Normalized | VertFmt::U16Normalized => 2,
            VertFmt::I8 | VertFmt::U8 | VertFmt::I8Normalized | VertFmt::U8Normalized => 1,
            VertFmt::None => 0,
        }
    }
}

/// What a vertex component means! This is matched against the semantics the shader's vertex inputs declare, so
/// component order in a format doesn't need to match the shader's input order.
/// <https://stereokit.net/Pages/StereoKit/VertSemantic.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum VertSemantic {
    /// Invalid semantic, this is not a valid value for a component.
    None = 0,
    /// Vertex position, in model space coordinates.
    Position,
    /// Direction the vertex is facing.
    Normal,
    /// Texture coordinates.
    Texcoord,
    /// Vertex color.
    Color,
    /// Tangent direction for normal mapping.
    Tangent,
    /// Binormal/bitangent direction for normal mapping.
    Binormal,
    /// Bone weights for skinning.
    Blendweight,
    /// Bone indices for skinning.
    Blendindices,
    /// Point size for point rendering.
    Psize,
}

/// A single component of a custom vertex layout, such as a position or a UV coordinate. A vertex format is described
/// by an array of these, in the same order the components appear in the vertex data. Data is always tightly packed,
/// aligned to nothing, so the format fully describes the vertex layout.
///
/// This maps to a compact 4 byte native representation, the properties here disguise that byte packing.
/// <https://stereokit.net/Pages/StereoKit/VertComponent.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct VertComponent {
    pub format: VertFmt,
    /// How many format elements this component has, 1-4. A float3 position would be 3.
    pub count: u8,
    /// What this component means, this is matched with the shader's vertex input semantics.
    pub semantic: VertSemantic,
    /// The data format of a single element of this component.
    /// Distinguishes multiple components with the same semantic, like TEXCOORD0 vs TEXCOORD1. Usually 0.
    pub semantic_slot: u8,
}

impl VertComponent {
    /// Describes a single vertex component.
    /// <https://stereokit.net/Pages/StereoKit/VertComponent/VertComponent.html>
    /// * `semantic` - What this component means, this is matched with the shader's vertex input semantics.
    /// * `format` - The data format of a single element of this component.
    /// * `count` - How many format elements this component has, 1-4. A float3 position would be 3.
    /// * `semantic_slot` - Distinguishes multiple components with the same semantic, like TEXCOORD0 vs TEXCOORD1.
    ///   Usually 0.
    pub const fn new(semantic: VertSemantic, format: VertFmt, count: u8, semantic_slot: u8) -> Self {
        Self { semantic, format, count, semantic_slot }
    }

    /// The size in bytes of this component, that is, the format size multiplied by the element count.
    pub const fn size(&self) -> usize {
        self.format.size() * self.count as usize
    }

    /// Calculates the stride (size in bytes) of a single vertex described by the given component array. This sums the
    /// size of each component, that is, the format size multiplied by the element count.
    ///
    /// see also [`mesh_fmt_stride`] [`VertComponent::size`] [`VertFmt::size`]
    pub fn fmt_stride(components: &[VertComponent]) -> i32 {
        unsafe { mesh_fmt_stride(components.as_ptr(), components.len() as i32) }
    }
}

/// Derives the vertex format of a vertex struct, in the same order the fields appear in memory. This is the Rust
/// equivalent of C#'s `[VertComponent]` attribute and `VertLayout<T>` reflection: implement this trait for a custom
/// `#[repr(C, packed)]` vertex struct, returning one [`VertComponent`] per field in declaration order.
///
/// The components must fully describe the struct's memory layout with no padding, and the total size they describe
/// must exactly equal `size_of::<Self>()`.
pub trait VertexLayout: Copy {
    /// The components that describe this vertex struct, in the order the fields appear in memory.
    const COMPONENTS: &'static [VertComponent];
}

impl VertexLayout for Vertex {
    const COMPONENTS: &'static [VertComponent] = &[
        VertComponent::new(VertSemantic::Position, VertFmt::F32, 3, 1),
        VertComponent::new(VertSemantic::Normal, VertFmt::F32, 3, 0),
        VertComponent::new(VertSemantic::Texcoord, VertFmt::F32, 2, 0),
        VertComponent::new(VertSemantic::Color, VertFmt::U8Normalized, 4, 0),
    ];
}

/// For performance sensitive areas, or places dealing with large chunks of memory, it can be faster to get a reference
/// to that memory rather than copying it! However, if this isn’t explicitly stated, it isn’t necessarily clear what’s
/// happening. So this enum allows us to visibly specify what type of memory reference is occurring.
/// <https://stereokit.net/Pages/StereoKit/Memory.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Memory {
    /// The chunk of memory involved here is a reference that is still managed or used by StereoKit! You should not free
    /// it, and be extremely cautious about modifying it.
    Reference = 0,
    /// This memory is now yours and you must free it yourself! Memory has been allocated, and the data has been copied
    /// over to it. Pricey! But safe.
    Copy = 1,
}

bitflags::bitflags! {
    /// Flags that control how mesh data is set. These can be combined to enable multiple behaviors at once.
    /// <https://stereokit.net/Pages/StereoKit/MeshData.html>
    ///
    /// see also [`Mesh::set_data`] [`Mesh::from_data`]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct MeshData: u32 {
        /// No options set, data upload is synchronous and bounds will not be recalculated.
        const None        = 0;
        /// Recalculate the mesh's bounds after uploading the data.
        const CalcBounds = 1 << 0;
        /// Upload the mesh data asynchronously on a background thread. The mesh will be skipped during rendering until
        /// the upload completes.
        const Async       = 1 << 1;
    }
}

/// A Mesh is a single collection of triangular faces with extra surface information to enhance rendering! StereoKit
/// meshes are composed of a list of vertices, and a list of indices to connect the vertices into faces. Nothing more
/// than that is stored here, so typically meshes are combined with Materials, or added to Models in order to draw them.
///
/// Mesh vertices are composed of a position, a normal (direction of the vert), a uv coordinate (for mapping a texture
/// to the mesh’s surface), and a 32 bit color containing red, green, blue, and alpha (transparency).
///
/// Mesh indices are stored as unsigned ints, so you can have a mesh with a fudgeton of verts! 4 billion or so :)
/// <https://stereokit.net/Pages/StereoKit/Mesh.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix}, util::named_colors,
///                      mesh::Mesh, material::Material};
///
/// // Create Meshes
/// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
/// let sphere = Mesh::generate_sphere(1.0, None);
///
/// let material_cube = Material::pbr().copy();
/// let mut material_sphere = Material::pbr().copy();
/// material_sphere.color_tint(named_colors::GREEN);
/// let cube_transform = Matrix::r([40.0, 50.0, 20.0]);
///
/// filename_scr = "screenshots/meshes.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     cube.draw(&material_cube, cube_transform, None, None);
///     sphere.draw(&material_sphere, Matrix::IDENTITY, None, None);
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/meshes.jpeg" alt="screenshot" width="200">
#[derive(Debug, PartialEq)]
pub struct Mesh(pub NonNull<_MeshT>);
impl Drop for Mesh {
    fn drop(&mut self) {
        self.on_load_remove();
        unsafe { mesh_release(self.0.as_ptr()) }
    }
}
impl AsRef<Mesh> for Mesh {
    fn as_ref(&self) -> &Mesh {
        self
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _MeshT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type MeshT = *mut _MeshT;

/// StereoKit ffi type.
pub type VindT = u32;

unsafe extern "C" {
    pub fn mesh_find(name: *const c_char) -> MeshT;
    pub fn mesh_create() -> MeshT;
    pub fn mesh_copy(mesh: MeshT) -> MeshT;
    pub fn mesh_set_id(mesh: MeshT, id: *const c_char);
    pub fn mesh_get_id(mesh: MeshT) -> *const c_char;
    pub fn mesh_addref(mesh: MeshT);
    pub fn mesh_release(mesh: MeshT);
    pub fn mesh_draw(mesh: MeshT, material: MaterialT, transform: Matrix, color_linear: Color128, layer: RenderLayer);
    pub fn mesh_set_keep_data(mesh: MeshT, keep_data: Bool32T);
    pub fn mesh_get_keep_data(mesh: MeshT) -> Bool32T;
    pub fn mesh_set_data(
        mesh: MeshT,
        in_arr_vertices: *const Vertex,
        vertex_count: i32,
        in_arr_indices: *const VindT,
        index_count: i32,
        flags: MeshData,
        priority: i32,
    );
    pub fn mesh_set_data_fmt(
        mesh: MeshT,
        in_arr_format: *const VertComponent,
        component_count: i32,
        vertex_data: *const c_void,
        vertex_count: i32,
        in_arr_indices: *const VindT,
        index_count: i32,
        flags: MeshData,
        priority: i32,
    );
    pub fn mesh_set_verts(mesh: MeshT, in_arr_vertices: *const Vertex, vertex_count: i32, calculate_bounds: Bool32T);
    pub fn mesh_get_verts(
        mesh: MeshT,
        out_arr_vertices: *mut *mut Vertex,
        out_vertex_count: *mut i32,
        reference_mode: Memory,
    );
    pub fn mesh_set_verts_fmt(
        mesh: MeshT,
        in_arr_format: *const VertComponent,
        component_count: i32,
        vertex_data: *const c_void,
        vertex_count: i32,
        calculate_bounds: Bool32T,
    );
    pub fn mesh_get_verts_fmt(
        mesh: MeshT,
        out_arr_format: *mut *mut VertComponent,
        out_component_count: *mut i32,
        out_vertex_data: *mut *mut c_void,
        out_vertex_count: *mut i32,
        reference_mode: Memory,
    );
    pub fn mesh_fmt_stride(in_arr_format: *const VertComponent, component_count: i32) -> i32;
    pub fn mesh_get_vert_count(mesh: MeshT) -> i32;
    pub fn mesh_set_inds(mesh: MeshT, in_arr_indices: *const VindT, index_count: i32);
    pub fn mesh_get_inds(
        mesh: MeshT,
        out_arr_indices: *mut *mut VindT,
        out_index_count: *mut i32,
        reference_mode: Memory,
    );
    pub fn mesh_get_ind_count(mesh: MeshT) -> i32;
    pub fn mesh_set_draw_inds(mesh: MeshT, index_count: i32);
    pub fn mesh_set_bounds(mesh: MeshT, bounds: *const Bounds);
    pub fn mesh_get_bounds(mesh: MeshT) -> Bounds;
    pub fn mesh_has_skin(mesh: MeshT) -> Bool32T;
    pub fn mesh_set_skin(
        mesh: MeshT,
        in_arr_bone_ids_4: *const u16,
        bone_id_4_count: i32,
        in_arr_bone_weights: *const Vec4,
        bone_weight_count: i32,
        bone_resting_transforms: *const Matrix,
        bone_count: i32,
    );
    pub fn mesh_update_skin(mesh: MeshT, in_arr_bone_transforms: *const Matrix, bone_count: i32);
    pub fn mesh_ray_intersect(
        mesh: MeshT,
        model_space_ray: Ray,
        cull_mode: Cull,
        out_pt: *mut Ray,
        out_start_inds: *mut u32,
    ) -> Bool32T;
    pub fn mesh_ray_intersect_bvh(
        mesh: MeshT,
        model_space_ray: Ray,
        cull_mode: Cull,
        out_pt: *mut Ray,
        out_start_inds: *mut u32,
    ) -> Bool32T;
    pub fn mesh_get_triangle(
        mesh: MeshT,
        triangle_index: u32,
        out_a: *mut Vertex,
        out_b: *mut Vertex,
        out_c: *mut Vertex,
    ) -> Bool32T;
    pub fn mesh_gen_plane(
        dimensions: Vec2,
        plane_normal: Vec3,
        plane_top_direction: Vec3,
        subdivisions: i32,
        double_sided: Bool32T,
    ) -> MeshT;
    pub fn mesh_gen_circle(
        diameter: f32,
        plane_normal: Vec3,
        plane_top_direction: Vec3,
        spokes: i32,
        double_sided: Bool32T,
    ) -> MeshT;
    pub fn mesh_gen_cube(dimensions: Vec3, subdivisions: i32) -> MeshT;
    pub fn mesh_gen_sphere(diameter: f32, subdivisions: i32) -> MeshT;
    pub fn mesh_gen_rounded_cube(dimensions: Vec3, edge_radius: f32, subdivisions: i32) -> MeshT;
    pub fn mesh_gen_cylinder(diameter: f32, depth: f32, direction: Vec3, subdivisions: i32) -> MeshT;
    pub fn mesh_gen_cone(diameter: f32, depth: f32, direction: Vec3, subdivisions: i32) -> MeshT;
    pub fn mesh_asset_state(mesh: MeshT) -> AssetState;
    pub fn mesh_on_load(
        mesh: MeshT,
        asset_on_load_callback: Option<unsafe extern "C" fn(mesh: MeshT, context: *mut c_void)>,
        context: *mut c_void,
    );
    pub fn mesh_on_load_remove(
        mesh: MeshT,
        asset_on_load_callback: Option<unsafe extern "C" fn(mesh: MeshT, context: *mut c_void)>,
    );
}

/// Trampoline that forwards a C `mesh_on_load` callback to a boxed Rust closure.
///
/// The `context` pointer must point to a heap-allocated `Box<dyn Fn(Mesh)>` created by
/// [`Mesh::on_load`]. The box remains alive after the call; it is only freed when the
/// corresponding [`MeshOnLoadHandle`] is dropped.
unsafe extern "C" fn mesh_on_load_trampoline(mesh: MeshT, context: *mut c_void) {
    let callback = unsafe { &*(context as *const Box<dyn Fn(Mesh)>) };
    unsafe { mesh_addref(mesh) };
    if let Some(nn) = NonNull::new(mesh) {
        callback(Mesh(nn));
    }
}

impl IAsset for Mesh {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }

    fn as_asset(&self) -> crate::system::AssetT {
        self.0.as_ptr() as crate::system::AssetT
    }
}

impl Default for Mesh {
    /// Creates an empty Mesh asset. Use SetVerts and SetInds to add data to it!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Mesh.html>
    ///
    /// see also: [`Vertex`] [`Mesh::new`]
    fn default() -> Self {
        Self::new()
    }
}

impl Mesh {
    /// Creates an empty Mesh asset. Use SetVerts and SetInds to add data to it!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Mesh.html>
    ///
    /// see also [`mesh_create`] [`Mesh::default`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::new();
    ///
    /// assert_eq!(mesh.get_inds().len(), 0);
    /// assert_eq!(mesh.get_verts().len(), 0);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn new() -> Mesh {
        Mesh(NonNull::new(unsafe { mesh_create() }).expect("Mesh::new should work!"))
    }

    /// Creates a Mesh asset and sets its vertex and index data with control over upload behavior. This is a shorthand for
    /// creating a Mesh with [`Mesh::new`] and calling [`Mesh::set_data`].
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Mesh.html>
    /// * `vertices` - An array of vertices for the mesh. An empty slice is okay here, but may require a special shader.
    /// * `indices` - A list of face indices, must be a multiple of 3.
    /// * `flags` - Flags controlling upload behavior. Defaults to [`MeshData::CalcBounds`]. Pass [`MeshData::Async`]
    ///   for background upload.
    /// * `priority` - Loading priority for async upload. Lower values load sooner. Defaults to 0.
    ///
    /// see also [`mesh_create`] [`mesh_set_data`] [`Mesh::set_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{mesh::{Mesh, Vertex}, system::AssetState};
    ///
    /// let vertices = [
    ///     Vertex::new([-0.5, -0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    ///     Vertex::new([ 0.5, -0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    ///     Vertex::new([ 0.5,  0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    ///     Vertex::new([-0.5,  0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    /// ];
    /// let indices = [2u32, 1, 0, 3, 2, 0];
    ///
    /// let mesh_sync = Mesh::from_data(&vertices, &indices, None, None);
    /// assert_eq!(mesh_sync.get_asset_state(), AssetState::Loaded);
    /// assert_eq!(mesh_sync.get_vert_count(), 4);
    /// assert_eq!(mesh_sync.get_ind_count(), 6);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn from_data(vertices: &[Vertex], indices: &[u32], flags: Option<MeshData>, priority: Option<i32>) -> Mesh {
        let mut mesh = Mesh::new();
        mesh.set_data(vertices, indices, flags, priority);
        mesh
    }

    /// Generates a plane with an arbitrary orientation that is optionally subdivided, pre-sized to the given
    /// dimensions. UV coordinates start at the top left indicated with plane_top_direction.
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene. You may also be interested in using the pre-generated `Mesh.Quad` asset if it already meets your
    /// needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GeneratePlane.html>
    /// * `dimension` - How large is this plane on the XZ axis,  in meters?
    /// * `plane_normal` - What is the normal of the surface this plane is generated on?
    /// * `plane_top_direction` - A normal defines the plane, but this is technically a rectangle on the plane. So which
    ///   direction is up? It's important for UVs, but doesn't need to be exact. This function takes the planeNormal as
    ///   law, and uses this vector to find the right and up vectors via cross-products.
    /// * `subdivisions` - Use this to add extra slices of vertices across the plane. This can be useful for some types of
    ///   vertex-based effects! None is 0.
    /// * `double_sided` - Should both sides of the plane be rendered?
    ///
    /// Returns a plane mesh, pre-sized to the given dimensions.
    /// see also [`mesh_gen_plane`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_plane([1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], None, false);
    /// assert_eq!(mesh.get_ind_count(), 6);
    /// assert_eq!(mesh.get_vert_count(), 4);
    ///
    /// let mesh = Mesh::generate_plane([1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], None, true);
    /// assert_eq!(mesh.get_ind_count(), 12);
    /// assert_eq!(mesh.get_vert_count(), 8);
    ///
    /// let mesh = Mesh::generate_plane([1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], Some(1), true);
    /// assert_eq!(mesh.get_ind_count(), 48);
    /// assert_eq!(mesh.get_vert_count(), 18);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_plane<V: Into<Vec3>>(
        dimensions: impl Into<Vec2>,
        plane_normal: V,
        plane_top_direction: V,
        subdivisions: Option<i32>,
        double_sided: bool,
    ) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(0);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_plane(
                    dimensions.into(),
                    plane_normal.into(),
                    plane_top_direction.into(),
                    subdivisions,
                    double_sided as Bool32T,
                )
            })
            .expect("Mesh::generate_plane should work!"),
        )
    }

    /// Generates a plane on the XZ axis facing up that is optionally subdivided, pre-sized to the given dimensions. UV
    /// coordinates start at 0,0 at the -X,-Z corner, and go to 1,1 at the +X,+Z corner!
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene. You may also be interested in using the pre-generated `Mesh.Quad` asset if it already meets your
    /// needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GeneratePlane.html>
    /// * `dimension` - How large is this plane on the XZ axis,  in meters?
    /// * `subdivisions` - Use this to add extra slices of vertices across the plane. This can be useful for some types of
    ///   vertex-based effects! None is 0.
    /// * `double_sided` - Should both sides of the plane be rendered?
    ///
    /// Returns a plane mesh, pre-sized to the given dimensions.
    /// see also [`mesh_gen_plane`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{mesh::{Mesh, Vertex}, maths::{Vec2, Vec3}};
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_plane_up([1.0, 1.0],  None, false);
    /// let mesh_b = Mesh::generate_plane([1.0, 1.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0], None, false);
    /// assert_eq!(mesh.get_verts(), mesh_b.get_verts());
    /// assert_eq!(mesh.get_inds(), mesh_b.get_inds());
    /// assert_eq!(mesh.get_ind_count(), 6);
    /// assert_eq!(mesh.get_vert_count(), 4);
    /// let vertices0 = [
    ///    Vertex::new([-0.5, 0.0,-0.5].into(),Vec3::UP,Some(Vec2::ZERO), None),
    ///    Vertex::new([ 0.5, 0.0,-0.5].into(),Vec3::UP,Some(Vec2::X)   , None),
    ///    Vertex::new([-0.5, 0.0, 0.5].into(),Vec3::UP,Some(Vec2::Y)   , None),
    ///    Vertex::new([ 0.5, 0.0, 0.5].into(),Vec3::UP,Some(Vec2::ONE) , None),
    ///    ];
    /// assert_eq!(mesh.get_verts(), vertices0);
    ///
    /// let mesh = Mesh::generate_plane_up([1.0, 1.0], None, true);
    /// assert_eq!(mesh.get_inds().len(), 12);
    /// assert_eq!(mesh.get_verts().len(), 8);
    ///
    /// let mesh = Mesh::generate_plane_up([1.0, 1.0], Some(1), true);
    /// assert_eq!(mesh.get_inds().len(), 48);
    /// assert_eq!(mesh.get_verts().len(), 18);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_plane_up(dimensions: impl Into<Vec2>, subdivisions: Option<i32>, double_sided: bool) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(0);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_plane(dimensions.into(), Vec3::UP, Vec3::FORWARD, subdivisions, double_sided as Bool32T)
            })
            .expect("Mesh::generate_plane_up should work!"),
        )
    }

    /// Generates a circle with an arbitrary orientation that is pre-sized to the given diameter. UV coordinates start
    /// at the top  left indicated with 'plane_top_direction' and correspond to a unit circle centered at 0.5, 0.5.
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GenerateCircle.html>
    /// * `diameter` - The diameter of the circle in meters, or  2*radius. This is the full length from one side to the
    ///   other.
    /// * `plane_normal` - What is the normal of the surface this circle is generated on?
    /// * `plane_top_direction` - A normal defines the plane, but this is technically a rectangle on the plane. So which
    ///   direction is up? It's important for UVs, but doesn't need to be exact. This function takes the plane_normal as
    ///   law, and uses this vector to find the right and up vectors via cross-products.
    /// * `spokes` - How many vertices compose the circumference of the circle? Clamps to a minimum of 3. More is smoother,
    ///   but less performant. if None has default value of 16.
    /// * `double_sided` - Should both sides of the circle be  rendered?
    ///
    /// Returns A circle mesh, pre-sized to the given dimensions.
    ///
    /// see also [`mesh_gen_circle`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_circle(1.0, [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], None, false);
    /// assert_eq!(mesh.get_ind_count(), 42);
    /// assert_eq!(mesh.get_vert_count(), 16);
    ///
    /// let mesh = Mesh::generate_circle(1.0, [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], None, true);
    /// assert_eq!(mesh.get_inds().len(), 84);
    /// assert_eq!(mesh.get_verts().len(), 32);
    ///
    /// let mesh = Mesh::generate_circle(1.0, [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], Some(1), true);
    /// assert_eq!(mesh.get_inds().len(), 6);
    /// assert_eq!(mesh.get_verts().len(), 6);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_circle<V: Into<Vec3>>(
        diameter: f32,
        plane_normal: V,
        plane_top_direction: V,
        spokes: Option<i32>,
        double_sided: bool,
    ) -> Mesh {
        let spokes = spokes.unwrap_or(16);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_circle(
                    diameter,
                    plane_normal.into(),
                    plane_top_direction.into(),
                    spokes,
                    double_sided as Bool32T,
                )
            })
            .expect("Mesh::generate_circle should work!"),
        )
    }

    /// Generates a circle on the XZ axis facing up that is  pre-sized to the given diameter. UV coordinates correspond
    /// to a unit  circle centered at 0.5, 0.5! That is, the right-most point on the  circle has UV coordinates 1, 0.5
    /// and the top-most point has UV  coordinates 0.5, 1.
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GenerateCircle.html>
    /// * `diameter` - The diameter of the circle in meters, or  2*radius. This is the full length from one side to the
    ///   other.
    /// * `spokes` - How many vertices compose the circumference of the circle? Clamps to a minimum of 3. More is smoother,
    ///   but less performant. if None has default value of 16.
    /// * `double_sided` - Should both sides of the circle be  rendered?
    ///
    /// Returns A circle mesh, pre-sized to the given dimensions.
    /// see also [`mesh_gen_circle`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_circle_up(1.0 , None, false);
    /// let mesh_b = Mesh::generate_circle(1.0, [0.0, 1.0, 0.0], [0.0, 0.0, -1.0], None, false);
    /// assert_eq!(mesh.get_verts(), mesh_b.get_verts());
    /// assert_eq!(mesh.get_inds(), mesh_b.get_inds());
    /// assert_eq!(mesh.get_ind_count(), 42);
    /// assert_eq!(mesh.get_vert_count(), 16);
    ///
    /// let mesh = Mesh::generate_circle_up(1.0 , None, true);
    /// assert_eq!(mesh.get_inds().len(), 84);
    /// assert_eq!(mesh.get_verts().len(), 32);
    ///
    /// let mesh = Mesh::generate_circle_up(1.0 , Some(1), true);
    /// assert_eq!(mesh.get_inds().len(), 6);
    /// assert_eq!(mesh.get_verts().len(), 6);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_circle_up(diameter: f32, spokes: Option<i32>, double_sided: bool) -> Mesh {
        let spokes = spokes.unwrap_or(16);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_circle(diameter, Vec3::UP, Vec3::FORWARD, spokes, double_sided as Bool32T)
            })
            .expect("Mesh::generate_circle_up should work!"),
        )
    }

    /// Generates a flat-shaded cube mesh, pre-sized to the given dimensions. UV coordinates are projected flat on each
    /// face, 0,0 -> 1,1.
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene. You may also be interested in using the pre-generated Mesh::cube() asset if it already meets your
    /// needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GenerateCube.html>
    /// * `dimension` - How large is this cube on each axis, in meters?
    /// * `subdivisions` - Use this to add extra slices of vertices across the cube's faces. This can be useful for some
    ///   types of vertex-based effects! None is 0.
    ///
    /// Returns a flat-shaded cube mesh, pre-sized to the given  dimensions.
    /// see also [`mesh_gen_circle`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_cube([1.0, 1.0, 1.0], None);
    /// assert_eq!(mesh.get_ind_count(), 36);
    /// assert_eq!(mesh.get_vert_count(), 24);
    ///
    /// let mesh = Mesh::generate_cube([1.0, 1.0, 1.0], Some(1));
    /// assert_eq!(mesh.get_inds().len(), 144);
    /// assert_eq!(mesh.get_verts().len(), 54);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_cube(dimensions: impl Into<Vec3>, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(0);
        Mesh(
            NonNull::new(unsafe { mesh_gen_cube(dimensions.into(), subdivisions) })
                .expect("Mesh::generate_cube should work!"),
        )
    }

    /// Generates a cube mesh with rounded corners, pre-sized to the given dimensions. UV coordinates are 0,0 -> 1,1 on
    /// each face, meeting at the middle of the rounded corners.
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GenerateRoundedCube.html>
    /// * `dimension` - How large is this cube on each axis, in meters?
    /// * `edge_radius` - Radius of the corner rounding, in meters.
    /// * `subdivisions` -How many subdivisions should be used for creating the corners? A larger value results in
    ///   smoother corners, but can decrease performance.! None is 4.
    ///
    /// Returns a cube mesh with rounded corners, pre-sized to the given dimensions
    /// see also [`mesh_gen_rounded_cube`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_rounded_cube([1.0, 1.0, 1.0], 0.1, None);
    /// assert_eq!(mesh.get_ind_count(), 900);
    /// assert_eq!(mesh.get_vert_count(), 216);
    ///
    /// let mesh = Mesh::generate_rounded_cube([1.0, 1.0, 1.0], 0.1, Some(1));
    /// assert_eq!(mesh.get_inds().len(), 324);
    /// assert_eq!(mesh.get_verts().len(), 96);
    ///
    /// let mesh = Mesh::generate_rounded_cube([1.0, 1.0, 1.0], 0.2, Some(1));
    /// assert_eq!(mesh.get_inds().len(), 324);
    /// assert_eq!(mesh.get_verts().len(), 96);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_rounded_cube(dimensions: impl Into<Vec3>, edge_radius: f32, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(4);
        Mesh(
            NonNull::new(unsafe { mesh_gen_rounded_cube(dimensions.into(), edge_radius, subdivisions) })
                .expect("Mesh::generate_rounded_cube should work!"),
        )
    }

    /// Generates a sphere mesh, pre-sized to the given diameter, created by sphereifying a subdivided cube! UV
    /// coordinates are taken from the initial unspherified cube.
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene. You may also be interested in using the pre-generated `Mesh::sphere()` asset if it already meets your
    /// needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GenerateSphere.html>
    /// * `diameter` - The diameter of the sphere in meters, or 2*radius. This is the full length from one side to the other.
    /// * `subdivisions` - How many times should the initial cube be subdivided? None is 4.
    ///
    /// Returns - A sphere mesh, pre-sized to the given diameter, created by sphereifying a subdivided cube! UV
    /// coordinates are taken from the initial unspherified cube.
    /// see also [`mesh_gen_sphere`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_sphere(1.0 , None);
    /// assert_eq!(mesh.get_ind_count(), 900);
    /// assert_eq!(mesh.get_vert_count(), 216);
    ///
    ///
    /// let mesh = Mesh::generate_sphere(1.0 , Some(1));
    /// assert_eq!(mesh.get_inds().len(), 144);
    /// assert_eq!(mesh.get_verts().len(), 54);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_sphere(diameter: f32, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(4);
        Mesh(
            NonNull::new(unsafe { mesh_gen_sphere(diameter, subdivisions) })
                .expect("Mesh::generate_sphere should work!"),
        )
    }

    /// Generates a cylinder mesh, pre-sized to the given diameter and depth, UV coordinates are from a flattened top
    /// view right now. Additional development is needed for making better UVs for the edges.
    ///
    /// NOTE: This generates a completely new Mesh asset on the GPU, and is best done during 'initialization' of your
    /// app/scene.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GenerateCylinder.html>
    /// * `diameter` - Diameter of the circular part of the cylinder in meters. Diameter is 2*radius.
    /// * `depth` - How tall is this cylinder, in meters?
    /// * `direction` - What direction do the circular surfaces face? This is the surface normal for the top, it does not
    ///   need to be normalized.
    /// * `subdivisions` - How many vertices compose the edges of the cylinder? More is smoother, but less performant.
    ///   None is 16.
    ///
    /// Returns a cylinder mesh, pre-sized to the given diameter and depth, UV coordinates are from a flattened top view
    /// right now.
    /// see also [`mesh_gen_cylinder`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh = Mesh::generate_cylinder(1.0, 1.0, [0.0, 1.0, 0.0], None);
    /// assert_eq!(mesh.get_ind_count(), 192);
    /// assert_eq!(mesh.get_vert_count(), 70);
    ///
    /// let mesh = Mesh::generate_cylinder(1.0, 1.0, [0.0, 1.0, 0.0], Some(1));
    /// assert_eq!(mesh.get_inds().len(), 12);
    /// assert_eq!(mesh.get_verts().len(), 10);
    /// # system::Assets::block_for_priority(i32::MAX);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn generate_cylinder(diameter: f32, depth: f32, direction: impl Into<Vec3>, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(16);
        Mesh(
            NonNull::new(unsafe { mesh_gen_cylinder(diameter, depth, direction.into(), subdivisions) })
                .expect("Mesh::generate_cylinder should work!"),
        )
    }

    /// Finds the Mesh with the matching id, and returns a reference to it. If no Mesh is found, it returns
    /// StereoKitError::MeshFind.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Find.html>
    /// * `id` - The id of the Mesh to find.
    ///
    /// see also [`mesh_find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mut mesh = Mesh::generate_circle_up(1.0 , None, false);
    /// mesh.id("my_circle");
    ///
    /// let same_mesh = Mesh::find("my_circle").expect("Mesh should be here");
    ///
    /// assert_eq!(mesh, same_mesh);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<Mesh, StereoKitError> {
        let cstr = CString::new(id.as_ref())?;
        match NonNull::new(unsafe { mesh_find(cstr.as_ptr()) }) {
            Some(mesh) => Ok(Mesh(mesh)),
            None => Err(StereoKitError::MeshFind(id.as_ref().to_owned())),
        }
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Find.html>
    ///
    /// see also [`mesh_find()`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mesh =          Mesh::generate_circle_up(1.0 , None, false);
    /// let not_same_mesh = Mesh::generate_circle_up(1.0 , None, false);
    ///
    /// let same_mesh = mesh.clone_ref();
    ///
    /// assert_eq!(mesh, same_mesh);
    /// assert_ne!(mesh, not_same_mesh);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn clone_ref(&self) -> Mesh {
        Mesh(NonNull::new(unsafe { mesh_find(mesh_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    /// Sets the unique identifier of this asset resource! This can be helpful for debugging, managing your assets, or
    /// finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Id.html>
    /// * `id` - The unique identifier for this asset! Be sure it's unique!
    ///
    /// see also [`mesh_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Create Meshes
    /// let mut mesh = Mesh::generate_circle_up(1.0 , None, false);
    /// assert!(mesh.get_id().starts_with("auto/mesh_"));
    /// mesh.id("my_circle");
    ///
    /// assert_eq!(mesh.get_id(), "my_circle");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr = CString::new(id.as_ref()).unwrap_or_default();
        unsafe { mesh_set_id(self.0.as_ptr(), cstr.as_ptr()) };
        self
    }

    /// This is a bounding box that encapsulates the Mesh! It's used for collision, visibility testing, UI layout, and
    /// probably other things. While it's normally calculated from the mesh vertices, you can also override this to
    /// suit your needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Bounds.html>
    /// * `bounds` - The bounding box to set.
    ///
    /// see also [`mesh_set_bounds`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Bounds},
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let mut sphere = Mesh::generate_sphere(1.0, None);
    /// let material_sphere = Material::pbr();
    /// let transform = Matrix::IDENTITY;
    ///
    /// let cube =   Mesh::cube();
    /// let mut material_before = Material::ui_box();
    /// material_before.color_tint(named_colors::GOLD)
    ///                .border_size(0.025);
    ///
    /// let mut material_after = material_before.copy();
    /// material_after.color_tint(named_colors::RED);
    ///
    /// let bounds = sphere.get_bounds();
    /// let transform_before = Matrix::t_s( bounds.center, bounds.dimensions);
    ///
    /// sphere.bounds( Bounds::bounds_centered(Vec3::ONE * 0.7));
    /// let new_bounds = sphere.get_bounds();
    /// let transform_after = Matrix::t_s( new_bounds.center, new_bounds.dimensions);
    ///
    /// filename_scr = "screenshots/mesh_bounds.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sphere.draw(&material_sphere, transform, None, None);
    ///     cube.draw( &material_before, transform_before, None, None);
    ///     cube.draw( &material_after,  transform_after,  None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_bounds.jpeg" alt="screenshot" width="200">
    pub fn bounds(&mut self, bounds: impl AsRef<Bounds>) -> &mut Self {
        unsafe { mesh_set_bounds(self.0.as_ptr(), bounds.as_ref() as *const Bounds) };
        self
    }

    /// Should StereoKit keep the mesh data on the CPU for later access, or collision detection? Defaults to true. If you
    /// set this to false before setting data, the data won't be stored. If you call this after setting data, that
    /// stored data will be freed! If you set this to true again later on, it will not contain data until it's set again.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/KeepData.html>
    ///
    /// see also [`mesh_set_keep_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{mesh::Mesh, maths::Bounds};
    ///
    /// // Create Meshes
    /// let mut mesh = Mesh::generate_circle_up(1.0 , None, false);
    /// assert_eq!(mesh.get_keep_data(), true);
    /// assert_ne!(mesh.get_bounds(), Bounds::default());
    ///
    /// mesh.keep_data(false);
    /// assert_eq!(mesh.get_keep_data(), false);
    /// mesh.keep_data(true);
    /// assert_eq!(mesh.get_keep_data(), true);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn keep_data(&mut self, keep_data: bool) -> &mut Self {
        unsafe { mesh_set_keep_data(self.0.as_ptr(), keep_data as Bool32T) };
        self
    }

    /// Assigns the vertices and indices for this Mesh with control over upload behavior via flags! This will create a
    /// vertex buffer and index buffer object on the graphics card.
    ///
    /// Remember to set all the relevant values! Your material will often show black if the Normals or Colors are left
    /// at their default values.
    ///
    /// Calling SetData is slightly more efficient than calling SetVerts and SetInds separately.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetData.html>
    /// * `vertices` - An array of vertices to add to the mesh. Remember to set all the relevant values! Your material
    ///   will often show black if the Normals or Colors are left at their default values. An empty slice is okay here,
    ///   but may require a special shader.
    /// * `indices` - A list of face indices, must be a multiple of 3. Each index represents a vertex from the provided
    ///   vertex array.
    /// * `flags` - Flags controlling upload behavior. See [`MeshData`] for options. Use [`MeshData::CalcBounds`] to
    ///   recalculate bounds, [`MeshData::Async`] for background upload. None has default value of MeshData::CalcBounds.
    /// * `priority` - Loading priority for async upload. Lower values load sooner. None has default value of 0.
    ///
    /// see also [`mesh_set_data`] [`Mesh::from_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Vec3, Matrix}, mesh::{Mesh, Vertex},
    ///                      material::Material, util::named_colors};
    ///
    /// let material = Material::pbr();
    /// let mut square = Mesh::new();
    /// square.set_data(&[
    ///     Vertex::new([-1.0, -1.0, 0.0].into(), Vec3::UP, None,            Some(named_colors::BLUE)),
    ///     Vertex::new([ 1.0, -1.0, 0.0].into(), Vec3::UP, Some(Vec2::X),   None),
    ///     Vertex::new([-1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::Y),   None),
    ///     Vertex::new([ 1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::ONE), Some(named_colors::YELLOW)),
    ///     ], &[0, 1, 2, 2, 1, 3], None, None);
    ///
    /// filename_scr = "screenshots/mesh_set_data.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     square.draw(&material, Matrix::IDENTITY, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_data.jpeg" alt="screenshot" width="200">
    pub fn set_data(
        &mut self,
        vertices: &[Vertex],
        indices: &[u32],
        flags: Option<MeshData>,
        priority: Option<i32>,
    ) -> &mut Self {
        let flags = flags.unwrap_or(MeshData::CalcBounds);
        let priority = priority.unwrap_or(0);
        unsafe {
            mesh_set_data(
                self.0.as_ptr(),
                vertices.as_ptr(),
                vertices.len() as i32,
                indices.as_ptr(),
                indices.len() as i32,
                flags,
                priority,
            )
        };
        self
    }

    /// Assigns vertices with a custom vertex format along with face indices for this Mesh in a single call, with
    /// control over upload behavior via flags! Upload is synchronous by default — pass [`MeshData::Async`] for
    /// background upload. The format is derived from T's [`VertexLayout`] implementation, see [`Mesh::set_verts_fmt`]
    /// for details.
    ///
    /// Calling SetData is slightly more efficient than calling SetVerts and SetInds separately.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetData.html>
    /// * `vertices` - An array of vertices to add to the mesh. An empty slice is okay here, but may require a special
    ///   shader.
    /// * `indices` - A list of face indices, must be a multiple of 3. Each index represents a vertex from the provided
    ///   vertex array.
    /// * `flags` - Flags controlling upload behavior. See [`MeshData`] for options. None has default value of
    ///   [`MeshData::CalcBounds`].
    /// * `priority` - Loading priority for async upload. Lower values load sooner. None has default value of 0.
    ///
    /// see also [`mesh_set_data_fmt`] [`Mesh::set_verts_fmt`] [`VertexLayout`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, util::{named_colors, Color32},
    ///                      mesh::{Mesh, VertFmt, VertSemantic, VertComponent, VertexLayout}};
    ///
    /// #[derive(Default, Debug, Copy, Clone, PartialEq)]
    /// #[repr(C)]
    /// struct CustomVertex {
    ///     pos: Vec3,
    ///     col: Color32,
    /// }
    /// impl VertexLayout for CustomVertex {
    ///     const COMPONENTS: &'static [VertComponent] = &[
    ///         VertComponent::new(VertSemantic::Position, VertFmt::F32, 3, 0),
    ///         VertComponent::new(VertSemantic::Color, VertFmt::U8Normalized, 4, 0),
    ///     ];
    /// }
    ///
    /// let mut square = Mesh::new();
    /// square.set_data_fmt(&[
    ///     CustomVertex{ pos: [-1.0, -1.0, 0.0].into(), col: named_colors::BLUE  },
    ///     CustomVertex{ pos: [ 1.0, -1.0, 0.0].into(), col: named_colors::WHITE },
    ///     CustomVertex{ pos: [-1.0,  1.0, 0.0].into(), col: named_colors::WHITE },    
    ///     CustomVertex{ pos: [ 1.0,  1.0, 0.0].into(), col: named_colors::YELLOW},
    ///     ], &[0, 1, 2, 2, 1, 3], None, None);
    ///
    /// assert_eq!(square.get_vert_count(), 4);
    /// let verts = square.get_verts_fmt::<CustomVertex>().expect("4 vertices should be returned");
    /// assert_eq!(verts.len(), 4);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_data_fmt<T: VertexLayout>(
        &mut self,
        vertices: &[T],
        indices: &[u32],
        flags: Option<MeshData>,
        priority: Option<i32>,
    ) -> &mut Self {
        let format = T::COMPONENTS;
        let flags = flags.unwrap_or(MeshData::CalcBounds);
        let priority = priority.unwrap_or(0);
        unsafe {
            mesh_set_data_fmt(
                self.0.as_ptr(),
                format.as_ptr(),
                format.len() as i32,
                vertices.as_ptr() as *const c_void,
                vertices.len() as i32,
                indices.as_ptr(),
                indices.len() as i32,
                flags,
                priority,
            )
        };
        self
    }

    /// Use [`Mesh::set_data`] or [`Mesh::from_data`] instead!
    /// Assigns the vertices for this Mesh! This will create a vertex buffer object on the graphics card. If you're
    /// calling this a second time, the buffer will be marked as dynamic and re-allocated. If you're calling this a
    /// third time, the buffer will only re-allocate if the buffer is too small, otherwise it just copies in the data!
    ///
    /// Remember to set all the relevant values! Your material will often show black if the Normals or Colors are left
    /// at their default values.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetVerts.html>
    /// * `vertices` - An array of vertices to add to the mesh. Remember to set all the relevant values! Your material
    ///   will often show black if the Normals or Colors are left at their default values. An empty slice is okay here,
    ///   but may require a special shader.
    /// * `calculate_bounds` - If true, this will also update the Mesh's bounds based on the vertices provided. Since this
    ///   does require iterating through all the verts with some logic, there is performance cost to doing this. If
    ///   you're updating a mesh frequently or need all the performance you can get, setting this to false is a nice way
    ///   to gain some speed!
    ///
    /// see also [`mesh_set_verts`] [`Vertex`] [`Mesh::set_data`] [`Mesh::from_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Vec3}, mesh::{Mesh, Vertex}, util::named_colors};
    ///
    /// let mut square = Mesh::new();
    /// square.set_verts(&[
    ///     Vertex::new([-1.0, -1.0, 0.0].into(), Vec3::UP, None,            Some(named_colors::BLUE)),
    ///     Vertex::new([ 1.0, -1.0, 0.0].into(), Vec3::UP, Some(Vec2::X),   None),
    ///     Vertex::new([-1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::Y),   None),
    ///     Vertex::new([ 1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::ONE), Some(named_colors::YELLOW)),
    ///     ], true)
    ///    .set_inds(&[0, 1, 2, 2, 1, 3]);
    ///
    /// assert_eq!(square.get_vert_count(), 4);
    /// let verts = square.get_verts();
    /// assert_eq!(verts.len(), 4);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_verts(&mut self, vertices: &[Vertex], calculate_bounds: bool) -> &mut Self {
        unsafe {
            mesh_set_verts(self.0.as_ptr(), vertices.as_ptr(), vertices.len() as i32, calculate_bounds as Bool32T)
        };
        self
    }

    /// Assigns vertices with a custom vertex format to this Mesh! The format is derived from T's fields via its
    /// [`VertexLayout`] implementation, each component describing what it is. The shader this Mesh is drawn with must be
    /// one that works with the components this format provides, StereoKit's built-in shaders all expect position,
    /// normal, texcoord and color.
    ///
    /// A T that doesn't exactly describe its own memory layout will produce incorrect results, see [`VertexLayout`]
    /// docs for the rules.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetVerts.html>
    /// * `vertices` - An array of vertices to add to the mesh. An empty slice is okay here, but may require a special
    ///   shader.
    /// * `calculate_bounds` - If true, this will also update the Mesh's bounds based on the vertices provided. This
    ///   requires the format to contain a float3 position component.
    ///
    /// see also [`mesh_set_verts_fmt`] [`VertexLayout`] [`Mesh::set_data_fmt`] [`Mesh::get_verts_fmt`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Vec3, util::{named_colors, Color32},
    ///                      mesh::{Mesh, VertFmt, VertSemantic, VertComponent, VertexLayout}};
    ///
    /// #[derive(Default, Debug, Copy, Clone, PartialEq)]
    /// #[repr(C)]
    /// struct CustomVertex {
    ///     pos: Vec3,
    ///     col: Color32,
    /// }
    /// impl VertexLayout for CustomVertex {
    ///     const COMPONENTS: &'static [VertComponent] = &[
    ///         VertComponent::new(VertSemantic::Position, VertFmt::F32, 3, 0),
    ///         VertComponent::new(VertSemantic::Color, VertFmt::U8Normalized, 4, 0),
    ///     ];
    /// }
    ///
    /// let mut square = Mesh::new();
    /// square.set_verts_fmt(&[
    ///     CustomVertex{ pos: [-1.0, -1.0, 0.0].into(), col: named_colors::BLUE  },
    ///     CustomVertex{ pos: [ 1.0, -1.0, 0.0].into(), col: named_colors::WHITE },
    ///     CustomVertex{ pos: [-1.0,  1.0, 0.0].into(), col: named_colors::WHITE },    
    ///     CustomVertex{ pos: [ 1.0,  1.0, 0.0].into(), col: named_colors::YELLOW},
    ///     ], true)
    ///    .set_inds(&[0, 1, 2, 2, 1, 3]);
    ///
    /// assert_eq!(square.get_vert_count(), 4);
    /// let verts = square.get_verts_fmt::<CustomVertex>().expect("4 vertices should be returned");
    /// assert_eq!(verts.len(), 4);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn set_verts_fmt<T: VertexLayout>(&mut self, vertices: &[T], calculate_bounds: bool) -> &mut Self {
        let format = T::COMPONENTS;
        unsafe {
            mesh_set_verts_fmt(
                self.0.as_ptr(),
                format.as_ptr(),
                format.len() as i32,
                vertices.as_ptr() as *const c_void,
                vertices.len() as i32,
                calculate_bounds as Bool32T,
            )
        };
        self
    }

    /// Assigns the face indices for this Mesh! Faces are always triangles, there are only ever three indices per face.
    /// This function will create a index buffer object on the graphics card. If you're calling this a second time, the
    /// buffer will be marked as dynamic and re-allocated. If you're calling this a third time, the buffer will only
    /// re-allocate if the buffer is too small, otherwise it just copies in the data!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetInds.html>
    /// * `indices` - A list of face indices, must be a multiple of 3. Each index represents a vertex from the array
    ///   assigned using SetVerts.
    ///
    /// see also [`mesh_set_inds`] [`Vertex`] [`Mesh::set_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix, mesh::Mesh,
    ///                      material::Material, util::named_colors};
    ///
    /// let material = Material::pbr();
    /// let mut sphere = Mesh::generate_sphere(1.5, Some(16));
    ///
    /// // Let's remove half of the triangles.
    /// let indices = sphere.get_inds();
    /// let mut new_indices = vec![];
    /// let mut iter = 0;
    /// for i in 0..indices.len() {
    ///     if iter < 3 {   
    ///        new_indices.push(indices[i]);
    ///     } else if iter == 5 {
    ///        iter = -1;
    ///     }
    ///    iter += 1;
    /// }
    ///
    /// sphere.set_inds(&new_indices);
    ///
    /// filename_scr = "screenshots/mesh_set_inds.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sphere.draw(&material , Matrix::IDENTITY,  Some(named_colors::PINK.into()), None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_inds.jpeg" alt="screenshot" width="200">
    pub fn set_inds(&mut self, indices: &[u32]) -> &mut Self {
        unsafe { mesh_set_inds(self.0.as_ptr(), indices.as_ptr(), indices.len() as i32) };
        self
    }

    /// Indicates whether this Mesh has CPU skinning data attached. A Mesh gains skin data when [`Mesh::set_skin`] is
    /// called, or when it's loaded from a skinned glTF.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/HasSkin.html>
    ///
    /// see also [`mesh_has_skin`] [`Mesh::set_skin`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec4, Matrix}, mesh::{Mesh, Vertex}, material::Material,
    ///                      util::named_colors::{RED, BLUE, YELLOW,GREEN}};
    ///
    /// let material = Material::pbr();
    /// // 4-vertex strip along Y: top pair at y=+0.1, bottom pair at y=-0.1
    /// let mut mesh = Mesh::from_data(&[
    ///         Vertex::new([-0.05,  0.1, 0.0], [0.0, 0.0, 1.0], None, Some(RED)),
    ///         Vertex::new([ 0.05,  0.1, 0.0], [0.0, 0.0, 1.0], None, Some(BLUE)),
    ///         Vertex::new([-0.05, -0.1, 0.0], [0.0, 0.0, 1.0], None, Some(YELLOW)),
    ///         Vertex::new([ 0.05, -0.1, 0.0], [0.0, 0.0, 1.0], None, Some(GREEN)),
    ///     ], &[0, 2, 1, 1, 2, 3], None, None);
    /// assert!(!mesh.has_skin());
    ///
    /// // Top vertices → bone 0 (resting at y=+0.1), bottom → bone 1 (resting at y=-0.1)
    /// let bone_ids = [0u16, 0, 0, 0,  0, 0, 0, 0,  1, 0, 0, 0,  1, 0, 0, 0];
    /// let bone_weights = vec![Vec4::new(1.0, 0.0, 0.0, 0.0); 4];
    /// mesh.set_skin(&bone_ids, &bone_weights,
    ///               &[Matrix::t([0.0,  0.1, 0.0]), Matrix::t([0.0, -0.1, 0.0])]);
    /// assert!(mesh.has_skin());
    ///
    /// // Deform: bone 0 stays at rest, bone 1 inclined 90° around Z
    /// let mut deformed = mesh.copy();
    /// deformed.update_skin(&[Matrix::t([0.0, 0.1, 0.0]), Matrix::r([0.0, 0.0, -90.0])]);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     deformed.draw(&material, Matrix::IDENTITY, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    pub fn has_skin(&self) -> bool {
        unsafe { mesh_has_skin(self.0.as_ptr()) != 0 }
    }

    /// Creates an independent duplicate of this Mesh. Vertices, indices, bounds, and (if present) skin data are copied;
    /// the new Mesh has its own GPU buffers and shares no state with the source.
    ///
    /// This is useful when one source mesh is shared across N animated entities: [`Mesh::update_skin`] mutates the
    /// target mesh's vertex buffer in place, so each entity needs its own Mesh instance to deform independently.
    ///
    /// The source Mesh must have `keep_data` set to true.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Copy.html>
    ///
    /// see also [`mesh_copy`] [`Mesh::set_skin`] [`Mesh::update_skin`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec4, Matrix}, mesh::{Mesh, Vertex}, material::Material,
    ///                      util::named_colors::{RED, BLUE, YELLOW,GREEN}};
    ///
    /// let material = Material::pbr();
    /// // 4-vertex strip along Y: top pair at y=+0.1, bottom pair at y=-0.1
    /// let mut mesh = Mesh::from_data(&[
    ///         Vertex::new([-0.05,  0.1, 0.0], [0.0, 0.0, 1.0], None, Some(RED)),
    ///         Vertex::new([ 0.05,  0.1, 0.0], [0.0, 0.0, 1.0], None, Some(BLUE)),
    ///         Vertex::new([-0.05, -0.1, 0.0], [0.0, 0.0, 1.0], None, Some(YELLOW)),
    ///         Vertex::new([ 0.05, -0.1, 0.0], [0.0, 0.0, 1.0], None, Some(GREEN)),
    ///     ], &[0, 2, 1, 1, 2, 3], None, None);
    /// let bone_ids = [0u16, 0, 0, 0,  0, 0, 0, 0,  1, 0, 0, 0,  1, 0, 0, 0];
    /// let bone_weights = vec![Vec4::new(1.0, 0.0, 0.0, 0.0); 4];
    /// mesh.set_skin(&bone_ids, &bone_weights,
    ///              &[Matrix::t([0.0,  0.1, 0.0]), Matrix::t([0.0, -0.1, 0.0])]);
    ///
    /// // Each copy has its own vertex buffer — update_skin on one won't affect the other
    /// let mut a = mesh.copy();
    /// let mut b = mesh.copy();
    /// assert!(a.has_skin() && b.has_skin());
    /// assert_eq!(a.get_vert_count(), mesh.get_vert_count());
    ///
    /// // Same source, two different deformations: bone 1 inclined -90° and -45° around Z
    /// a.update_skin(&[Matrix::t([0.0, 0.1, 0.0]), Matrix::r([0.0, 0.0, -90.0])]);
    /// b.update_skin(&[Matrix::t([0.0, 0.1, 0.0]), Matrix::r([0.0, 0.0, -45.0])]);
    ///
    /// filename_scr = "screenshots/mesh_copy.jpeg"; fov_scr = 12.0;
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     a.draw(&material, Matrix::t([-0.05, 0.0, 0.0]), None, None);
    ///     b.draw(&material, Matrix::t([ 0.05, 0.0, 0.0]), None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_copy.jpeg" alt="screenshot" width="200">
    pub fn copy(&self) -> Mesh {
        Mesh(NonNull::new(unsafe { mesh_copy(self.0.as_ptr()) }).expect("Mesh::copy failed!"))
    }

    /// Attaches CPU skinning data to this Mesh. Once skin data is set, call [`Mesh::update_skin`] each frame with the
    /// current bone palette to deform the vertex buffer.
    ///
    /// `keep_data` must be true and vertex data must already be set before calling this — the deformation runs on the
    /// CPU and needs a copy of the rest-pose vertices to work from.
    ///
    /// The bone palette passed to [`Mesh::update_skin`] is expected to be bone world transforms in the same coordinate
    /// system the resting transforms were authored in. The skinning matrix for bone `i` is computed as
    /// `bone_palette[i] * inverse(bone_resting_transforms[i])`.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetSkin.html>
    /// * `bone_ids` - Per-vertex bone indices, packed 4 per vertex (so this slice has length `vert_count * 4`). Each
    ///   index references a slot in the bone palette and resting transforms.
    /// * `bone_weights` - Per-vertex bone weights, one [`Vec4`] per vertex (length must equal `vert_count`). The four
    ///   components correspond to the four bone ids for that vertex. Weights should sum to ~1 for a stable result.
    /// * `bone_resting_transforms` - Bind-pose transform for each bone, expressed in the mesh's model space. StereoKit
    ///   inverts these internally to produce the inverse-bind matrices used by the skinning math.
    ///
    /// see also [`mesh_set_skin`] [`Mesh::update_skin`] [`Mesh::has_skin`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec4, Matrix}, mesh::{Mesh, Vertex}, material::Material,
    ///                      util::named_colors::{RED, BLUE, YELLOW,GREEN}};
    ///
    /// let material = Material::pbr();
    /// // 4-vertex strip along Y: top pair at y=+0.1, bottom pair at y=-0.1
    /// let mut mesh = Mesh::from_data(&[
    ///         Vertex::new([-0.05,  0.1, 0.0], [0.0, 0.0, 1.0], None, Some(RED)),
    ///         Vertex::new([ 0.05,  0.1, 0.0], [0.0, 0.0, 1.0], None, Some(BLUE)),
    ///         Vertex::new([-0.05, -0.1, 0.0], [0.0, 0.0, 1.0], None, Some(YELLOW)),
    ///         Vertex::new([ 0.05, -0.1, 0.0], [0.0, 0.0, 1.0], None, Some(GREEN)),
    ///     ], &[0, 2, 1, 1, 2, 3], None, None);
    ///
    /// // 4 slots per vertex: [bone_idx, 0, 0, 0]. Top verts → bone 0, bottom → bone 1
    /// let bone_ids = [0u16, 0, 0, 0,  0, 0, 0, 0,  1, 0, 0, 0,  1, 0, 0, 0];
    /// // Single bone per vertex → weight 1.0 on slot 0
    /// let bone_weights = vec![Vec4::new(1.0, 0.0, 0.0, 0.0); 4];
    /// // Resting positions match vertex positions: bone 0 at top, bone 1 at bottom
    /// let resting = [Matrix::t([0.0,  0.1, 0.0]), Matrix::t([0.0, -0.1, 0.0])];
    /// mesh.set_skin(&bone_ids, &bone_weights, &resting);
    /// assert!(mesh.has_skin());
    ///
    /// // Deform: bone 0 stays at rest, bone 1 inclined 45° around Z
    /// let mut deformed = mesh.copy();
    /// deformed.update_skin(&[Matrix::t([0.0, 0.1, 0.0]), Matrix::r([0.0, 0.0, -90.0])]);
    ///
    /// filename_scr = "screenshots/mesh_set_skin.jpeg"; fov_scr = 12.0;
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     deformed.draw(&material, Matrix::IDENTITY, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_skin.jpeg" alt="screenshot" width="200">
    pub fn set_skin(
        &mut self,
        bone_ids: &[u16],
        bone_weights: &[Vec4],
        bone_resting_transforms: &[Matrix],
    ) -> &mut Self {
        unsafe {
            mesh_set_skin(
                self.0.as_ptr(),
                bone_ids.as_ptr(),
                (bone_ids.len() / 4) as i32,
                bone_weights.as_ptr(),
                bone_weights.len() as i32,
                bone_resting_transforms.as_ptr(),
                bone_resting_transforms.len() as i32,
            )
        };
        self
    }

    /// Drives the per-frame CPU deformation for a skinned Mesh. [`Mesh::set_skin`] must have been called first. This
    /// walks every vertex, blends the bone transforms by weight, and re-uploads the deformed vertices to the GPU.
    ///
    /// `bone_palette` holds the current world-space transform for each bone, in the same coordinate system the resting
    /// transforms passed to [`Mesh::set_skin`] were authored in. Its length must match the bone count supplied to
    /// [`Mesh::set_skin`].
    ///
    /// Because deformation mutates this Mesh's vertex buffer in place, two entities driven by different bone palettes
    /// need their own Mesh instance — use [`Mesh::copy`] on a shared source mesh to get per-instance deformation.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/UpdateSkin.html>
    /// * `bone_palette` - World-space transform per bone for this frame. Length must match the bone count supplied to
    ///   [`Mesh::set_skin`].
    ///
    /// see also [`mesh_update_skin`] [`Mesh::set_skin`] [`Mesh::copy`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec4, Matrix}, mesh::{Mesh, Vertex}, material::Material};
    ///
    /// let mut material = Material::pbr();
    /// material.color_tint([0.5, 1.0, 0.5, 0.4]);
    /// // Stack N thin cylinder segments along Y to get intermediate rings.
    /// // generate_cylinder alone only has top/bottom rings, so stacking gives
    /// // the per-height vertices needed for smooth bone blending.
    /// let (n, sides, rad, hh) = (4usize, 10usize, 0.04f32, 0.2f32);
    /// let seg_h = 2.0 * hh / n as f32;
    /// let (mut verts, mut inds) = (Vec::<Vertex>::new(), Vec::<u32>::new());
    /// for i in 0..n {
    ///     let y = -hh + (i as f32 + 0.5) * seg_h;
    ///     let seg = Mesh::generate_cylinder(rad * 2.0, seg_h, [0.0, 1.0, 0.0], Some(sides as i32));
    ///     let base = verts.len() as u32;
    ///     verts.extend(seg.get_verts().iter()
    ///         .map(|v| { let mut nv = *v; nv.pos.y += y; nv }));
    ///     inds.extend(seg.get_inds().iter().map(|&idx| idx + base));
    /// }
    /// let mut src = Mesh::from_data(&verts, &inds, None, None);
    ///
    /// // Both bones anchored at origin (resting = IDENTITY → ball-joint pivot at y=0)
    /// let bone_ids: Vec<u16> = (0..verts.len()).flat_map(|_| [0u16, 1, 0, 0]).collect();
    /// let bone_weights: Vec<Vec4> = verts.iter()
    ///     .map(|v| { let w = ((v.pos.y + hh) / (2.0 * hh)).clamp(0.0, 1.0);
    ///                Vec4::new(w, 1.0 - w, 0.0, 0.0) }).collect();
    /// src.set_skin(&bone_ids, &bone_weights, &[Matrix::IDENTITY, Matrix::IDENTITY]);
    ///
    /// let mut mesh = src.copy();
    /// // Bone 0 stays fixed, bone 1 rotated 60° around Z → smooth elbow bend at center
    /// mesh.update_skin(&[Matrix::IDENTITY, Matrix::r([0.0, -60.0, 60.0])]);
    /// assert!(mesh.has_skin());
    ///
    /// filename_scr = "screenshots/mesh_update_skin.jpeg"; fov_scr = 25.0;
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     mesh.draw(&material, Matrix::IDENTITY, None, None);
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_update_skin.jpeg" alt="screenshot" width="200">
    pub fn update_skin(&mut self, bone_palette: &[Matrix]) -> &mut Self {
        unsafe { mesh_update_skin(self.0.as_ptr(), bone_palette.as_ptr(), bone_palette.len() as i32) };
        self
    }

    /// Registers a Rust closure as a mesh-load callback. The closure is called once when
    /// the Mesh finishes uploading to the GPU. For synchronous uploads it fires before this
    /// call returns; for async uploads ([`MeshData::Async`]) it fires on a future frame.
    ///
    /// You have to keep the closure alive until on_load_remove
    /// <https://stereokit.net/Pages/StereoKit/Mesh/OnLoaded.html>
    /// * `callback` - A `Fn(Mesh)` closure. The `Mesh` argument is the loaded mesh (with its
    ///   own addref'd reference — it will be released when that `Mesh` is dropped).
    ///
    /// see also [`mesh_on_load`] [`Mesh::on_load`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{mesh::{Mesh, Vertex}, system::Assets};
    /// use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
    ///
    /// let fired = Arc::new(AtomicBool::new(false));
    /// let fired2 = fired.clone();
    /// let triggered = Arc::new(AtomicBool::new(false));
    /// let triggered2 = triggered.clone();
    ///
    /// // Upload data first so the mesh is already in loaded state …
    /// let mut mesh = Mesh::new();
    /// let vertices = [
    ///     Vertex::new([-0.5, -0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    ///     Vertex::new([ 0.5, -0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    ///     Vertex::new([ 0.5,  0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    ///     Vertex::new([-0.5,  0.5, 0.0], [0.0, 0.0, -1.0], None, None),
    /// ];
    /// mesh.set_data(&vertices, &[2u32, 1, 0, 3, 2, 0], None, None);
    /// mesh.on_load(move |_m| {
    ///     fired2.store(true, Ordering::SeqCst);
    /// });
    /// mesh.on_load(move |_m| {
    ///     triggered2.store(true, Ordering::SeqCst);
    /// });
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Assets::block_for_priority(i32::MAX);
    /// );
    /// assert!(fired.load(Ordering::SeqCst));
    /// assert!(triggered.load(Ordering::SeqCst));
    /// # sk::Sk::shutdown();
    /// ```
    pub fn on_load<F: Fn(Mesh) + 'static>(&mut self, callback: F) {
        // Double-box: outer Box<Box<dyn Fn>> gives a thin pointer for FFI context.
        let boxed: Box<dyn Fn(Mesh)> = Box::new(callback);
        let context = Box::into_raw(Box::new(boxed)) as *mut c_void;
        let mesh = self.0.as_ptr();
        unsafe {
            mesh_on_load(mesh, Some(mesh_on_load_trampoline), context);
        }
    }

    /// Unregisters the trampoline.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/OnLoaded.html>
    ///
    /// see also [`mesh_on_load_remove`] [`Mesh::on_load`]
    pub fn on_load_remove(&mut self) {
        let mesh = self.0.as_ptr();
        unsafe {
            mesh_on_load_remove(mesh, Some(mesh_on_load_trampoline));
        }
    }

    /// Adds a mesh to the render queue for this frame! If the Hierarchy has a transform on it, that transform is
    /// combined with the Matrix provided here.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Draw.html>
    /// * `material` - A Material to apply to the Mesh.
    /// * `transform` - A Matrix that will transform the mesh from Model Space into the current Hierarchy Space.
    /// * `color_linear` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in
    ///   extra per-instance data for the shader! If None has default value of WHITE
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the
    ///   user's head from a 3rd person perspective, but filtering it out from the 1st person perspective.If None has
    ///   default value of Layer0
    ///
    /// see also [`mesh_draw`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix}, mesh::Mesh,
    ///                      material::Material, util::named_colors, render::RenderLayer};
    ///
    /// let material = Material::pbr();
    /// let cylinder1 = Mesh::generate_cylinder(0.25, 1.5, Vec3::ONE,        None);
    /// let cylinder2 = Mesh::generate_cylinder(0.25, 1.5, [-0.5, 0.5, 0.5], None);
    /// let cylinder3 = Mesh::generate_cylinder(0.25, 1.2, [0.0, -0.5, 0.5], None);
    ///
    /// filename_scr = "screenshots/mesh_draw.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     cylinder1.draw(&material , Matrix::IDENTITY, None, None);
    ///     cylinder2.draw(&material , Matrix::IDENTITY, Some(named_colors::RED.into()),
    ///         Some(RenderLayer::Layer1));
    ///     cylinder3.draw(&material , Matrix::IDENTITY, Some(named_colors::GREEN.into()),
    ///         Some(RenderLayer::ThirdPerson));
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_draw.jpeg" alt="screenshot" width="200">
    pub fn draw(
        &self,
        material: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color_linear: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color_linear: Color128 = color_linear.unwrap_or(Color128::WHITE);
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe { mesh_draw(self.0.as_ptr(), material.as_ref().0.as_ptr(), transform.into(), color_linear, layer) }
    }

    /// Gets the unique identifier of this asset resource! This can be helpful for debugging, managing your assets, or
    /// finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/id.html>
    ///
    /// see also [`mesh_get_id`]
    /// see example in [`Mesh::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(mesh_get_id(self.0.as_ptr())) }.to_str().unwrap_or_default()
    }

    /// Gets the loading state of this Mesh asset. The AssetState will be AssetState::Loaded once all mesh data has
    /// been uploaded to the GPU. For synchronous uploads this will be immediate; for async uploads (MeshData::Async)
    /// check this each frame.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/AssetState.html>
    ///
    /// see also [`mesh_asset_state`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{mesh::{Mesh, MeshData}, system::{Assets, AssetState}};
    ///
    /// let mut mesh = Mesh::new();
    /// mesh.set_data(&[], &[], Some(MeshData::Async | MeshData::CalcBounds), Some(139));
    /// assert_ne!(mesh.get_asset_state(), AssetState::Loaded);
    ///
    /// let cube_mesh = Mesh::generate_cube( [0.1, 0.1, 0.1], None);
    ///
    /// Assets::block_for_priority(i32::MAX);
    /// assert_eq!(cube_mesh.get_asset_state(), AssetState::Loaded);
    ///
    /// number_of_steps = 100;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    /// );
    /// assert_eq!(mesh.get_asset_state(), AssetState::LoadedMeta);
    /// assert_eq!(cube_mesh.get_asset_state(), AssetState::Loaded);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_asset_state(&self) -> AssetState {
        unsafe { mesh_asset_state(self.0.as_ptr()) }
    }

    /// This is a bounding box that encapsulates the Mesh! It's used for collision, visibility testing, UI layout, and
    /// probably  other things. While it's normally calculated from the mesh vertices, you can also override this to
    /// suit your needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Bounds.html>
    ///
    /// see also [`mesh_get_bounds`]
    /// see example in [`Mesh::bounds`]
    pub fn get_bounds(&self) -> Bounds {
        unsafe { mesh_get_bounds(self.0.as_ptr()) }
    }

    /// Should StereoKit keep the mesh data on the CPU for later access, or collision detection? Defaults to true. If you
    /// set this to false before setting data, the data won't be stored. If you call this after setting data, that
    /// stored data will be freed! If you set this to true again later on, it will not contain data until it's set again.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/KeepData.html>
    ///
    /// see also [`mesh_get_keep_data`]
    /// see example in [`Mesh::keep_data`]
    pub fn get_keep_data(&self) -> bool {
        unsafe { mesh_get_keep_data(self.0.as_ptr()) != 0 }
    }

    /// Get the number of indices stored in this Mesh! This is available to you regardless of whether or not keep_data
    /// is set.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/IndCount.html>
    ///
    /// see also [`mesh_get_ind_count`]
    pub fn get_ind_count(&self) -> i32 {
        unsafe { mesh_get_ind_count(self.0.as_ptr()) }
    }

    /// Get the number of vertices stored in this Mesh! This is available to you regardless of whether or not keep_data
    /// is set.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/VertCount.html>
    ///
    /// see also [`mesh_get_vert_count`]
    pub fn get_vert_count(&self) -> i32 {
        unsafe { mesh_get_vert_count(self.0.as_ptr()) }
    }

    /// This marshalls the Mesh's index data into an array. If keep_data is false, then the Mesh is **not** storing
    /// indices on the CPU, and this information will **not** be available.
    ///
    /// Due to the way marshalling works, this is **not** a cheap function!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetInds.html>
    /// Returns - An array of indices representing the Mesh, or null if keep_data is false.
    ///
    /// see also [Mesh::get_inds_copy] [`mesh_get_inds`]
    /// see example in [`Mesh::set_inds`]
    pub fn get_inds(&self) -> &[u32] {
        let inds_ptr = CString::new("H").unwrap_or_default().into_raw() as *mut *mut u32;
        let mut inds_len = 0;
        unsafe {
            mesh_get_inds(self.0.as_ptr(), inds_ptr, &mut inds_len, Memory::Reference);
            &mut *slice_from_raw_parts_mut(*inds_ptr, inds_len as usize)
        }
    }

    /// Get the indices by value
    /// This marshalls the Mesh's index data into an array. If keep_data is false, then the Mesh is **not** storing
    /// indices on the CPU, and this information will **not** be available.
    ///
    /// Due to the way marshalling works, this is **not** a cheap function!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetInds.html>
    ///
    /// see also [Mesh::get_inds] [`mesh_get_inds`]
    pub fn get_inds_copy(&self) -> Vec<u32> {
        self.get_inds().to_vec()
    }

    /// This marshalls the Mesh's vertex data into an array. If keep_data is false, then the Mesh is **not** storing
    /// verts
    /// on the CPU, and this information will **not** be available.
    ///
    /// Due to the way marshalling works, this is **not** a cheap function!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetVerts.html>
    ///
    /// see also [Mesh::get_verts_copy] [`mesh_get_verts`]
    /// see example in [`Mesh::set_verts`]
    pub fn get_verts(&self) -> &[Vertex] {
        let verts_pointer = CString::new("H").unwrap_or_default().into_raw() as *mut *mut Vertex;
        let mut verts_len = 0;
        unsafe {
            mesh_get_verts(self.0.as_ptr(), verts_pointer, &mut verts_len, Memory::Reference);
            &mut *slice_from_raw_parts_mut(*verts_pointer, verts_len as usize)
        }
    }

    /// Get the vertices by value
    /// This marshalls the Mesh's vertex data into an array. If keep_data is false, then the Mesh is **not** storing
    /// verts
    /// on the CPU, and this information will **not** be available.
    ///
    /// Due to the way marshalling works, this is **not** a cheap function!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetVerts.html>
    ///
    /// see also [Mesh::get_verts] [`mesh_get_verts`]
    pub fn get_verts_copy(&self) -> Vec<Vertex> {
        self.get_verts().to_vec()
    }

    /// This marshalls the vertex data of a custom format Mesh into an array of T. T's [`VertexLayout`] derived format
    /// must exactly match the format the Mesh was created with, and keep_data must be true for vertex data to be
    /// available.
    ///
    /// Due to the way marshalling works, this is **not** a cheap function!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetVerts.html>
    /// * `reference_mode` - Reference mode to use, see [`Memory`]. [`Memory::Reference`] is the fastest.
    ///
    /// Returns - A reference to the vertex data if available and matching, else None.
    ///
    /// see also [`mesh_get_verts_fmt`] [`VertexLayout`] [`Mesh::get_verts`]
    /// see example in [`Mesh::set_verts_fmt`]
    pub fn get_verts_fmt<T: VertexLayout>(&self) -> Option<&[T]> {
        let mut fmt_ptr: *mut VertComponent = std::ptr::null_mut();
        let mut fmt_count = 0i32;
        let mut data_ptr: *mut c_void = std::ptr::null_mut();
        let mut data_count = 0i32;
        unsafe {
            mesh_get_verts_fmt(
                self.0.as_ptr(),
                &mut fmt_ptr,
                &mut fmt_count,
                &mut data_ptr,
                &mut data_count,
                Memory::Reference,
            );
        }
        if data_ptr.is_null() {
            return None;
        }
        let expected = T::COMPONENTS;
        if fmt_count as usize != expected.len() {
            return None;
        }
        for (i, expected_comp) in expected.iter().enumerate() {
            let comp = unsafe { &*fmt_ptr.add(i) };
            if comp != expected_comp {
                return None;
            }
        }
        let slice = unsafe { std::slice::from_raw_parts(data_ptr as *const T, data_count as usize) };
        // Extend the lifetime to the borrow of self; reference_mode::Reference data is alive while the Mesh is.
        Some(unsafe { &*(slice as *const [T]) })
    }

    /// Retrieves the vertices associated with a particular triangle on the Mesh.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetTriangle.html>
    /// * `triangle_index` - Starting index of the triangle, should be a multiple of 3.
    ///
    /// Returns an array of 3 vertices if triangle index was valid.
    /// see also [`mesh_get_triangle`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Vec3}, mesh::{Mesh, Vertex}};
    ///
    /// let plane = Mesh::generate_plane_up(Vec2::ONE, None, false);
    /// assert_eq!(plane.get_vert_count(), 4, "plane should have 4 vertices");
    /// assert_eq!(plane.get_ind_count(), 6, "plane should have 6 indices");
    ///
    /// let triangle0 = plane.get_triangle(0 * 3).expect("triangle 0 should exist");    
    /// let triangle1 = plane.get_triangle(1 * 3).expect("triangle 1 should exist");
    /// //assert!(plane.get_triangle(5).is_some(), "triangle 5 should exist");
    /// assert!(plane.get_triangle(2 * 3).is_none(), "triangle 6 should not exist");
    ///
    /// let vertices0 = [
    ///    Vertex::new([ 0.5, 0.0, 0.5].into(),Vec3::UP,Some(Vec2::ONE) , None),
    ///    Vertex::new([ 0.5, 0.0,-0.5].into(),Vec3::UP,Some(Vec2::X)   , None),
    ///    Vertex::new([-0.5, 0.0,-0.5].into(),Vec3::UP,Some(Vec2::ZERO), None),
    ///    ];
    /// assert_eq!(triangle0, vertices0);
    ///
    /// let vertices1 = [
    ///    Vertex::new([-0.5, 0.0, 0.5].into(),Vec3::UP,Some(Vec2::Y)   , None),
    ///    Vertex::new([ 0.5, 0.0, 0.5].into(),Vec3::UP,Some(Vec2::ONE) , None),
    ///    Vertex::new([-0.5, 0.0,-0.5].into(),Vec3::UP,Some(Vec2::ZERO), None),
    ///    ];
    /// assert_eq!(triangle1, vertices1);
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_triangle(&self, triangle_index: u32) -> Option<[Vertex; 3]> {
        let mut v_a = Vertex::default();
        let mut v_b = Vertex::default();
        let mut v_c = Vertex::default();
        let out_a: *mut Vertex = &mut v_a;
        let out_b: *mut Vertex = &mut v_b;
        let out_c: *mut Vertex = &mut v_c;
        unsafe {
            match mesh_get_triangle(self.0.as_ptr(), triangle_index, out_a, out_b, out_c) != 0 {
                true => Some([v_a, v_b, v_c]),
                false => None,
            }
        }
    }

    /// Checks the intersection point of a ray and this Mesh with collision data stored on the CPU. A mesh without
    /// collision data will always return None. Ray must be in model space, intersection point will be in model
    /// space too. You can use the inverse of the mesh’s world transform matrix to bring the ray into model space,
    /// see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Intersect.html>
    /// * `model_space_ray` - Ray must be in model space, the intersection point will be in model space too. You can use the
    ///   inverse of the mesh's world transform matrix to bring the ray into model space, see the example in the docs!
    /// * `cull` - If None has default value of Cull::Back.
    ///
    /// Returns a tuple with
    /// - The intersection point of the ray and the mesh, if an intersection  occurs. This is in model space, and must
    ///   be transformed back into world space later.
    /// - The indice of the mesh where the intersection occurs.
    ///
    /// see also [`mesh_ray_intersect`] [`Ray::intersect_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Quat, Ray}, system::Lines,
    ///     util::{named_colors}, mesh::Mesh, material::{Material, Cull}};
    ///
    /// // Create Meshes
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
    /// let sphere = Mesh::generate_sphere(1.0, Some(4));
    ///
    /// let material = Material::pbr().copy();
    /// let transform = Matrix::r(Quat::from_angles(40.0, 50.0, 20.0));
    /// let inv = transform.get_inverse();
    ///
    /// let ray = Ray::new([-1.0, 2.0, 2.5 ], [1.0, -2.0, -2.25]);
    /// let inv_ray = inv.transform_ray(ray);
    ///
    /// let (contact_cube, ind_cube) = cube.intersect( inv_ray, Some(Cull::Back))
    ///     .expect("Ray should touch cube");
    /// assert_eq!(ind_cube, 12);
    ///
    /// let transform_contact_cube = Matrix::t_s(
    ///     transform.transform_point(contact_cube), Vec3::ONE * 0.1);
    ///
    /// filename_scr = "screenshots/mesh_intersect.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     cube.draw(&material, transform, Some(named_colors::CYAN.into()), None);
    ///     Lines::add_ray( ray, 2.2, named_colors::WHITE, None, 0.02);
    ///     sphere.draw(&material, transform_contact_cube, Some(named_colors::YELLOW.into()), None );
    /// );
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_intersect.jpeg" alt="screenshot" width="200">
    #[inline]
    pub fn intersect(&self, model_space_ray: Ray, cull: Option<Cull>) -> Option<(Vec3, VindT)> {
        model_space_ray.intersect_mesh(self, cull)
    }

    /// Checks the intersection point of a Ray and this Mesh with collision data stored on the CPU. A mesh without
    /// collision data will always return false. Ray must be in model space, intersection point will be in model
    /// space too. You can use the inverse of the mesh’s world transform matrix to bring the ray into model space,
    /// see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Intersect.html>
    /// * `model_space_ray` - Ray must be in model space, the intersection point will be in model space too. You can use the
    ///   inverse of the mesh's world transform matrix to bring the ray into model space, see the example in the docs!
    /// * `cull` - If None has default value of Cull::Back.
    /// * `out_model_space_ray` -The intersection point and surface direction of the ray and the mesh, if an intersection
    ///   occurs. This is in model space, and must be transformed back into world space later. Direction is not
    ///   guaranteed to be normalized, especially if your own model->world transform contains scale/skew in it.
    /// * `out_start_inds` - The index of the first index of the triangle that was hit
    ///
    /// Returns true if an intersection occurs.
    /// see also [`mesh_ray_intersect`] [`Ray::intersect_mesh`] [`Mesh::intersect`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Quat, Ray},
    ///                      mesh::Mesh, material::Cull};
    ///
    /// // Create Meshes
    /// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
    /// let sphere = Mesh::generate_sphere(1.0, Some(4));
    ///
    /// let transform = Matrix::r(Quat::from_angles(40.0, 50.0, 20.0));
    /// let inv = transform.get_inverse();
    ///
    /// let ray = Ray::new([-3.0, 2.0, 0.5 ], [3.0, -2.0, -0.25]);
    /// let inv_ray = inv.transform_ray(ray);
    ///
    /// let (mut contact_sphere_ray, mut ind_sphere) = (Ray::default(), 0u32);
    /// assert!(sphere.intersect_to_ptr(inv_ray, Some(Cull::Front),
    ///             &mut contact_sphere_ray, &mut ind_sphere)
    ///     ,"Ray should touch sphere");
    ///
    /// let (mut contact_cube_ray, mut ind_cube) = (Ray::default(), 0u32);
    /// assert!( cube.intersect_to_ptr(
    ///             inv_ray, Some(Cull::Back),
    ///             &mut contact_cube_ray, &mut ind_cube)
    ///     ,"Ray should touch cube");
    ///
    /// assert_eq!(ind_sphere, 672);
    /// assert_eq!(ind_cube, 9);
    ///
    /// assert_eq!(transform.transform_ray(contact_sphere_ray),
    ///         Ray { position:  Vec3 { x: 0.36746234, y: -0.244975, z: 0.21937825 },
    ///               direction: Vec3 { x: 0.58682406, y: -0.6427875, z: 0.49240398 }});
    /// assert_eq!(transform.transform_ray(contact_cube_ray),
    ///         Ray { position:  Vec3 { x: -0.39531866, y: 0.26354572, z: 0.2829433 },
    ///               direction: Vec3 { x: -0.77243483, y: -0.2620026, z: 0.57853174 } });
    /// # sk::Sk::shutdown();
    /// ```
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn intersect_to_ptr(
        &self,
        ray: Ray,
        cull: Option<Cull>,
        out_model_space_ray: *mut Ray,
        out_start_inds: *mut u32,
    ) -> bool {
        ray.intersect_mesh_to_ptr(self, cull, out_model_space_ray, out_start_inds)
    }

    /// A cube with dimensions of (1,1,1), this is equivalent to Mesh.GenerateCube(Vec3.One).
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Cube.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Get the mesh
    /// let mesh = Mesh::cube();
    /// assert_eq!(mesh.get_id(), "default/mesh_cube");
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn cube() -> Self {
        Mesh::find("default/mesh_cube").unwrap_or_default()
    }

    /// A default quad mesh, 2 triangles, 4 verts, from (-0.5,-0.5,0) to (0.5,0.5,0) and facing forward on the Z axis
    /// (0,0,-1). White vertex colors, and UVs from (1,1) at vertex (-0.5,-0.5,0) to (0,0) at vertex (0.5,0.5,0).
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Quad.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Get the mesh
    /// let mesh = Mesh::screen_quad();
    /// assert_eq!(mesh.get_id(), "default/mesh_screen_quad");
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn screen_quad() -> Self {
        Mesh::find("default/mesh_screen_quad").unwrap_or_default()
    }

    // see screen_quad instead ! TODO: Why this ?
    // <https://stereokit.net/Pages/StereoKit/Mesh/Quad.html>
    // pub fn quad() -> Self {
    //     Mesh::find("default/mesh_quad").unwrap_or_default()
    // }

    /// A sphere mesh with a diameter of 1. This is equivalent to Mesh.GenerateSphere(1,4).
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Sphere.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Get the mesh
    /// let mesh = Mesh::sphere();
    /// assert_eq!(mesh.get_id(), "default/mesh_sphere");
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn sphere() -> Self {
        Mesh::find("default/mesh_sphere").unwrap_or_default()
    }

    /// A clone mesh of the left hand
    /// <https://stereokit.net/Pages/StereoKit/Mesh.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Get the mesh
    /// let mesh = Mesh::left_hand();
    /// assert_eq!(mesh.get_id(), "default/mesh_lefthand");
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/left_hand.jpeg" alt="screenshot" width="200">
    pub fn left_hand() -> Self {
        Mesh::find("default/mesh_lefthand").unwrap_or_default()
    }

    /// A clone mesh of the right hand
    /// <https://stereokit.net/Pages/StereoKit/Mesh.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::mesh::Mesh;
    ///
    /// // Get the mesh
    /// let mesh = Mesh::right_hand();
    /// assert_eq!(mesh.get_id(), "default/mesh_righthand");
    /// # test_steps!();
    /// # sk::Sk::shutdown();
    /// ```
    pub fn right_hand() -> Self {
        Mesh::find("default/mesh_righthand").unwrap_or_default()
    }
}
