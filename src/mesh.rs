use crate::{
    StereoKitError,
    material::{Cull, Material, MaterialT},
    maths::{Bool32T, Bounds, Matrix, Ray, Vec2, Vec3, Vec4},
    sk::MainThreadToken,
    system::{IAsset, RenderLayer},
    util::{Color32, Color128},
};
use std::{
    ffi::{CStr, CString, c_char},
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
/// use stereokit_rust::{maths::{Vec3, Vec2, Matrix}, util::Color32, mesh::{Mesh,Vertex}, material::Material};
///
/// // Creating vertices with all fields specified
/// let vertices = [
///     Vertex::new(Vec3::ZERO,Vec3::UP,None,         Some(Color32::rgb(255, 0, 0))),
///     Vertex::new(Vec3::X,   Vec3::UP,Some(Vec2::X),Some(Color32::rgb(255, 255, 0))),
///     Vertex::new(Vec3::Y,   Vec3::UP,Some(Vec2::Y),Some(Color32::rgb(0, 0, 255))),
/// ];
/// let indices = [0, 1, 2, 2, 1, 0];
/// let mut mesh = Mesh::new();
/// mesh.id("most_basic_mesh").keep_data(true).set_data(&vertices, &indices, true);
/// let material = Material::pbr();
///
/// filename_scr = "screenshots/basic_mesh.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     mesh.draw(token, &material, Matrix::IDENTITY, None, None);
/// );
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
/// use stereokit_rust::{maths::{Vec3, Matrix, Quat}, util::{named_colors,Color32},
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
///     cube.draw(token, &material_cube, cube_transform, None, None);
///     sphere.draw(token, &material_sphere, Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/meshes.jpeg" alt="screenshot" width="200">
#[derive(Debug, PartialEq)]
pub struct Mesh(pub NonNull<_MeshT>);
impl Drop for Mesh {
    fn drop(&mut self) {
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
        calculate_bounds: Bool32T,
    );
    pub fn mesh_set_verts(mesh: MeshT, in_arr_vertices: *const Vertex, vertex_count: i32, calculate_bounds: Bool32T);
    pub fn mesh_get_verts(
        mesh: MeshT,
        out_arr_vertices: *mut *mut Vertex,
        out_vertex_count: *mut i32,
        reference_mode: Memory,
    );
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
}

impl IAsset for Mesh {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
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
    /// ```
    pub fn new() -> Mesh {
        Mesh(NonNull::new(unsafe { mesh_create() }).unwrap())
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
            .unwrap(),
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
    /// ```
    pub fn generate_plane_up(dimensions: impl Into<Vec2>, subdivisions: Option<i32>, double_sided: bool) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(0);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_plane(dimensions.into(), Vec3::UP, Vec3::FORWARD, subdivisions, double_sided as Bool32T)
            })
            .unwrap(),
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
            .unwrap(),
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
    /// ```
    pub fn generate_circle_up(diameter: f32, spokes: Option<i32>, double_sided: bool) -> Mesh {
        let spokes = spokes.unwrap_or(16);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_circle(diameter, Vec3::UP, Vec3::FORWARD, spokes, double_sided as Bool32T)
            })
            .unwrap(),
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
    /// ```
    pub fn generate_cube(dimensions: impl Into<Vec3>, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(0);
        Mesh(NonNull::new(unsafe { mesh_gen_cube(dimensions.into(), subdivisions) }).unwrap())
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
    /// ```
    pub fn generate_rounded_cube(dimensions: impl Into<Vec3>, edge_radius: f32, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(4);
        Mesh(NonNull::new(unsafe { mesh_gen_rounded_cube(dimensions.into(), edge_radius, subdivisions) }).unwrap())
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
    /// ```
    pub fn generate_sphere(diameter: f32, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(4);
        Mesh(NonNull::new(unsafe { mesh_gen_sphere(diameter, subdivisions) }).unwrap())
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
    /// ```
    pub fn generate_cylinder(diameter: f32, depth: f32, direction: impl Into<Vec3>, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(16);
        Mesh(NonNull::new(unsafe { mesh_gen_cylinder(diameter, depth, direction.into(), subdivisions) }).unwrap())
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
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    /// material_before .color_tint(named_colors::GOLD)
    ///                 .get_all_param_info().set_float("border_size", 0.025);
    ///
    /// let mut material_after = material_before.copy();
    /// material_after.color_tint(named_colors::RED);
    ///
    /// let bounds = sphere.get_bounds();
    /// let transform_before = Matrix::ts( bounds.center, bounds.dimensions);
    ///
    /// sphere.bounds( Bounds::bounds_centered(Vec3::ONE * 0.7));
    /// let new_bounds = sphere.get_bounds();
    /// let transform_after = Matrix::ts( new_bounds.center, new_bounds.dimensions);
    ///
    /// filename_scr = "screenshots/mesh_bounds.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     sphere.draw(token, &material_sphere, transform, None, None);
    ///     cube.draw(  token, &material_before, transform_before, None, None);
    ///     cube.draw(  token, &material_after,  transform_after,  None, None);
    /// );
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
    /// ```
    pub fn keep_data(&mut self, keep_data: bool) -> &mut Self {
        unsafe { mesh_set_keep_data(self.0.as_ptr(), keep_data as Bool32T) };
        self
    }

    /// Assigns the vertices and indices for this Mesh! This will create a vertex buffer and index buffer object on the
    /// graphics card. If you're calling this a second time, the buffers will be marked as dynamic and re-allocated. If
    /// you're calling this a third time, the buffer will only re-allocate if the buffer is too small, otherwise it just
    /// copies in the data!
    ///
    /// Remember to set all the relevant values! Your material will often show black if the Normals or Colors are left
    /// at their default values.
    ///
    /// Calling SetData is slightly more efficient than calling SetVerts and SetInds separately.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetData.html>
    /// * `vertices` - An array of vertices to add to the mesh. Remember to set all the relevant values! Your material
    ///   will often show black if the Normals or Colors are left at their default values.
    /// * `indices` - A list of face indices, must be a multiple of 3. Each index represents a vertex from the provided
    ///   vertex array.
    /// * `calculate_bounds` - If true, this will also update the Mesh's bounds based on the vertices provided. Since this
    ///   does require iterating through all the verts with some logic, there is performance cost to doing this. If
    ///   you're updating a mesh frequently or need all the performance you can get, setting this to false is a nice way
    ///   to gain some speed!
    ///
    /// see also [`mesh_set_data`]
    /// see example[`Vertex`]
    pub fn set_data(&mut self, vertices: &[Vertex], indices: &[u32], calculate_bounds: bool) -> &mut Self {
        unsafe {
            mesh_set_data(
                self.0.as_ptr(),
                vertices.as_ptr(),
                vertices.len() as i32,
                indices.as_ptr(),
                indices.len() as i32,
                calculate_bounds as Bool32T,
            )
        };
        self
    }

    /// Assigns the vertices for this Mesh! This will create a vertex buffer object on the graphics card. If you're
    /// calling this a second time, the buffer will be marked as dynamic and re-allocated. If you're calling this a
    /// third time, the buffer will only re-allocate if the buffer is too small, otherwise it just copies in the data!
    ///
    /// Remember to set all the relevant values! Your material will often show black if the Normals or Colors are left
    /// at their default values.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetVerts.html>
    /// * `vertices` - An array of vertices to add to the mesh. Remember to set all the relevant values! Your material
    ///   will often show black if the Normals or Colors are left at their default values.
    /// * `calculate_bounds` - If true, this will also update the Mesh's bounds based on the vertices provided. Since this
    ///   does require iterating through all the verts with some logic, there is performance cost to doing this. If
    ///   you're updating a mesh frequently or need all the performance you can get, setting this to false is a nice way
    ///   to gain some speed!
    ///
    /// see also [`mesh_set_verts`] [`Vertex`] [`Mesh::set_data`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Vec3, Matrix, Bounds}, mesh::{Mesh, Vertex},
    ///                      material::Material, util::named_colors};
    ///
    /// let material = Material::pbr();
    /// let mut square = Mesh::new();
    /// square.set_verts(&[
    ///     Vertex::new([-1.0, -1.0, 0.0].into(), Vec3::UP, None,            Some(named_colors::BLUE)),
    ///     Vertex::new([ 1.0, -1.0, 0.0].into(), Vec3::UP, Some(Vec2::X),   None),
    ///     Vertex::new([-1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::Y),   None),
    ///     Vertex::new([ 1.0,  1.0, 0.0].into(), Vec3::UP, Some(Vec2::ONE), Some(named_colors::RED)),
    ///     ], true)
    ///    .set_inds(&[0, 1, 2, 2, 1, 3]);
    ///
    /// filename_scr = "screenshots/mesh_set_verts.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     square.draw(token, &material , Matrix::IDENTITY, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_verts.jpeg" alt="screenshot" width="200">
    pub fn set_verts(&mut self, vertices: &[Vertex], calculate_bounds: bool) -> &mut Self {
        unsafe {
            mesh_set_verts(self.0.as_ptr(), vertices.as_ptr(), vertices.len() as i32, calculate_bounds as Bool32T)
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
    /// use stereokit_rust::{maths::{Vec2, Vec3, Matrix, Bounds}, mesh::{Mesh, Vertex},
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
    ///     sphere.draw(token, &material , Matrix::IDENTITY,  Some(named_colors::PINK.into()), None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_inds.jpeg" alt="screenshot" width="200">
    pub fn set_inds(&mut self, indices: &[u32]) -> &mut Self {
        unsafe { mesh_set_inds(self.0.as_ptr(), indices.as_ptr(), indices.len() as i32) };
        self
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
    /// use stereokit_rust::{maths::{Vec2, Vec3, Matrix, Bounds}, mesh::{Mesh, Vertex},
    ///                      material::Material, util::named_colors, system::RenderLayer};
    ///
    /// let material = Material::pbr();
    /// let cylinder1 = Mesh::generate_cylinder(0.25, 1.5, Vec3::ONE,        None);
    /// let cylinder2 = Mesh::generate_cylinder(0.25, 1.5, [-0.5, 0.5, 0.5], None);
    /// let cylinder3 = Mesh::generate_cylinder(0.25, 1.2, [0.0, -0.5, 0.5], None);
    ///
    /// filename_scr = "screenshots/mesh_draw.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     cylinder1.draw(token, &material , Matrix::IDENTITY, None, None);
    ///     cylinder2.draw(token, &material , Matrix::IDENTITY, Some(named_colors::RED.into()),
    ///         Some(RenderLayer::Layer1));
    ///     cylinder3.draw(token, &material , Matrix::IDENTITY, Some(named_colors::GREEN.into()),
    ///         Some(RenderLayer::Layer_third_person));
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_draw.jpeg" alt="screenshot" width="200">
    pub fn draw(
        &self,
        _token: &MainThreadToken,
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
        unsafe { CStr::from_ptr(mesh_get_id(self.0.as_ptr())) }.to_str().unwrap()
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
        let inds_ptr = CString::new("H").unwrap().into_raw() as *mut *mut u32;
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
        let verts_pointer = CString::new("H").unwrap().into_raw() as *mut *mut Vertex;
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

    /// Retrieves the vertices associated with a particular triangle on the Mesh.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetTriangle.html>
    /// * `triangle_index` - Starting index of the triangle, should be a multiple of 3.
    ///
    /// Returns an array of 3 vertices if triangle index was valid.
    /// see also [`mesh_get_triangle`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec2, Vec3, Matrix, Bounds}, mesh::{Mesh, Vertex},
    ///                      material::Material, util::named_colors, system::RenderLayer};
    ///
    /// let material = Material::pbr();
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
    /// let transform_contact_cube = Matrix::ts(
    ///     transform.transform_point(contact_cube), Vec3::ONE * 0.1);
    ///
    /// filename_scr = "screenshots/mesh_intersect.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     cube.draw(token, &material, transform, Some(named_colors::CYAN.into()), None);
    ///     Lines::add_ray(token, ray, 2.2, named_colors::WHITE, None, 0.02);
    ///     sphere.draw(token, &material, transform_contact_cube, Some(named_colors::YELLOW.into()), None );
    /// );
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
    /// ```
    pub fn cube() -> Self {
        Mesh::find("default/mesh_cube").unwrap()
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
    /// ```
    pub fn screen_quad() -> Self {
        Mesh::find("default/mesh_screen_quad").unwrap()
    }

    // see screen_quad instead ! TODO: Why this ?
    // <https://stereokit.net/Pages/StereoKit/Mesh/Quad.html>
    // pub fn quad() -> Self {
    //     Mesh::find("default/mesh_quad").unwrap()
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
    /// ```
    pub fn sphere() -> Self {
        Mesh::find("default/mesh_sphere").unwrap()
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
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/left_hand.jpeg" alt="screenshot" width="200">
    pub fn left_hand() -> Self {
        Mesh::find("default/mesh_lefthand").unwrap()
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
    /// ```
    pub fn right_hand() -> Self {
        Mesh::find("default/mesh_righthand").unwrap()
    }
}
