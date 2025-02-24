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
///
/// filename_scr = "screenshots/basic_mesh.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     mesh.draw(token, Material::pbr(), Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/basic_mesh.jpeg" alt="screenshot" width="200">
#[derive(Default, Debug, Copy, Clone)]
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
    /// * texture_coordinate - If None, set the value to Vec2::ZERO
    /// * color - If None, set the value to Color32::WHITE
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
/// use stereokit_rust::{maths::{Vec3, Matrix, Quat}, util::{named_colors,Color32}, mesh::Mesh, material::Material};
///
/// // Create Meshes
/// let cube = Mesh::generate_cube(Vec3::ONE * 0.8, None);
/// let sphere = Mesh::generate_sphere(1.0, None);
///
/// let material_cube = Material::pbr().copy();
/// let mut material_sphere = Material::pbr().copy();
/// material_sphere.color_tint(named_colors::GREEN);
/// let cube_transform = Matrix::r(Quat::from_angles(40.0, 50.0, 20.0));
///
/// filename_scr = "screenshots/meshes.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     cube.draw(token, &material_cube, cube_transform, None, None);
///     sphere.draw(token, &material_sphere, Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/meshes.jpeg" alt="screenshot" width="200">
#[derive(Debug)]
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

#[repr(C)]
#[derive(Debug)]
pub struct _MeshT {
    _unused: [u8; 0],
}
pub type MeshT = *mut _MeshT;
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
    fn default() -> Self {
        Self::new()
    }
}

impl Mesh {
    /// Creates an empty Mesh asset. Use SetVerts and SetInds to add data to it!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Mesh.html>
    ///
    /// see also [`crate::mesh::mesh_create`]
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
    /// * dimension - How large is this plane on the XZ axis,  in meters?
    /// * plane_normal - What is the normal of the surface this plane is generated on?
    /// * plane_top_direction - A normal defines the plane, but this is technically a rectangle on the plane. So which
    ///   direction is up? It's important for UVs, but doesn't need to be exact. This function takes the planeNormal as
    ///   law, and uses this vector to find the right and up vectors via cross-products.
    /// * subdivisions - Use this to add extra slices of vertices across the plane. This can be useful for some types of
    ///   vertex-based effects! None is 0.
    /// * double_sided - Should both sides of the plane be rendered?
    ///
    /// Returns a plane mesh, pre-sized to the given dimensions.
    /// see also [`crate::mesh::mesh_gen_plane`]
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
    /// * dimension - How large is this plane on the XZ axis,  in meters?
    /// * subdivisions - Use this to add extra slices of vertices across the plane. This can be useful for some types of
    ///   vertex-based effects! None is 0.
    /// * double_sided - Should both sides of the plane be rendered?
    ///
    /// Returns a plane mesh, pre-sized to the given dimensions.
    /// see also [`crate::mesh::mesh_gen_plane`]
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
    /// * diameter - The diameter of the circle in meters, or  2*radius. This is the full length from one side to the
    ///   other.
    /// * plane_normal - What is the normal of the surface this circle is generated on?
    /// * plane_top_direction - A normal defines the plane, but this is technically a rectangle on the plane. So which
    ///   direction is up? It's important for UVs, but doesn't need to be exact. This function takes the plane_normal as
    ///   law, and uses this vector to find the right and up vectors via cross-products.
    /// * spoke - How many vertices compose the circumference of the circle? Clamps to a minimum of 3. More is smoother,
    ///   but less performant. if None has default value of 16.
    /// * double_side - Should both sides of the circle be  rendered?
    ///
    /// Returns A circle mesh, pre-sized to the given dimensions.
    ///
    /// see also [`crate::mesh::mesh_gen_circle`]
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
    /// * diameter - The diameter of the circle in meters, or  2*radius. This is the full length from one side to the
    ///   other.
    /// * spoke - How many vertices compose the circumference of the circle? Clamps to a minimum of 3. More is smoother,
    ///   but less performant. if None has default value of 16.
    /// * double_side - Should both sides of the circle be  rendered?
    ///
    /// Returns A circle mesh, pre-sized to the given dimensions.
    /// see also [`crate::mesh::mesh_gen_circle`]
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
    /// * dimension - How large is this cube on each axis, in meters?
    /// * subdivisions - Use this to add extra slices of vertices across the cube's faces. This can be useful for some
    ///   types of vertex-based effects! None is 0.
    ///
    /// Returns a flat-shaded cube mesh, pre-sized to the given  dimensions.
    /// see also [`crate::mesh::mesh_gen_circle`]
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
    /// * dimension - How large is this cube on each axis, in meters?
    /// * edge-radius - Radius of the corner rounding, in meters.
    /// * subdivisions -How many subdivisions should be used for creating the corners? A larger value results in
    ///   smoother corners, but can decrease performance.! None is 4.
    ///
    /// Returns a cube mesh with rounded corners, pre-sized to the given dimensions
    /// see also [`crate::mesh::mesh_gen_rounded_cube`]
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
    /// * diameter - The diameter of the sphere in meters, or 2*radius. This is the full length from one side to the other.
    /// * subdivisions - How many times should the initial cube be subdivided? None is 4.
    ///
    /// Returns - A sphere mesh, pre-sized to the given diameter, created by sphereifying a subdivided cube! UV
    /// coordinates are taken from the initial unspherified cube.
    /// see also [`crate::mesh::mesh_gen_sphere`]
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
    /// * diameter - Diameter of the circular part of the cylinder in meters. Diameter is 2*radius.
    /// * depth - How tall is this cylinder, in meters?
    /// * direction - What direction do the circular surfaces face? This is the surface normal for the top, it does not
    ///   need to be normalized.
    /// * subdivisions - How many vertices compose the edges of the cylinder? More is smoother, but less performant.
    ///   None is 16.
    ///
    /// Returns a cylinder mesh, pre-sized to the given diameter and depth, UV coordinates are from a flattened top view
    /// right now.
    /// see also [`crate::mesh::mesh_gen_cylinder`]
    pub fn generate_cylinder(diameter: f32, depth: f32, direction: impl Into<Vec3>, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(16);
        Mesh(NonNull::new(unsafe { mesh_gen_cylinder(diameter, depth, direction.into(), subdivisions) }).unwrap())
    }

    /// Finds the Mesh with the matching id, and returns a reference to it. If no Mesh is found, it returns
    /// StereoKitError::MeshFind.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Find.html>
    ///
    /// see also [`crate::mesh::mesh_find`]
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
    /// see also [`crate::mesh::mesh_find()`]
    pub fn clone_ref(&self) -> Mesh {
        Mesh(NonNull::new(unsafe { mesh_find(mesh_get_id(self.0.as_ptr())) }).expect("<asset>::clone_ref failed!"))
    }

    /// Sets the unique identifier of this asset resource! This can be helpful for debugging, managing your assets, or
    /// finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Id.html>
    ///
    /// see also [`crate::mesh::mesh_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { mesh_set_id(self.0.as_ptr(), cstr.as_ptr()) };
        self
    }

    /// This is a bounding box that encapsulates the Mesh! It's used for collision, visibility testing, UI layout, and
    /// probably other things. While it's normally calculated from the mesh vertices, you can also override this to
    /// suit your needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Bounds.html>
    ///
    /// see also [`crate::mesh::mesh_set_bounds`]
    pub fn bounds(&mut self, bounds: impl AsRef<Bounds>) -> &mut Self {
        unsafe { mesh_set_bounds(self.0.as_ptr(), bounds.as_ref() as *const Bounds) };
        self
    }

    /// Should StereoKit keep the mesh data on the CPU for later access, or collision detection? Defaults to true. If you
    /// set this to false before setting data, the data won't be stored. If you call this after setting data, that
    /// stored data will be freed! If you set this to true again later on, it will not contain data until it's set again.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/KeepData.html>
    ///
    /// see also [`crate::mesh::mesh_set_keep_data`]
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
    /// * vertices - An array of vertices to add to the mesh. Remember to set all the relevant values! Your material
    ///   will often show black if the Normals or Colors are left at their default values.
    /// * indices - A list of face indices, must be a multiple of 3. Each index represents a vertex from the provided
    ///   vertex array.
    /// * calculate_bounds - If true, this will also update the Mesh's bounds based on the vertices provided. Since this
    ///   does require iterating through all the verts with some logic, there is performance cost to doing this. If
    ///   you're updating a mesh frequently or need all the performance you can get, setting this to false is a nice way
    ///   to gain some speed!
    ///
    /// see also [`crate::mesh::mesh_set_data`]
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
    /// * vertices - An array of vertices to add to the mesh. Remember to set all the relevant values! Your material
    ///   will often show black if the Normals or Colors are left at their default values.
    /// * calculate_bounds - If true, this will also update the Mesh's bounds based on the vertices provided. Since this
    ///   does require iterating through all the verts with some logic, there is performance cost to doing this. If
    ///   you're updating a mesh frequently or need all the performance you can get, setting this to false is a nice way
    ///   to gain some speed!
    ///
    /// see also [`crate::mesh::mesh_set_verts`]
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
    /// * indices - A list of face indices, must be a multiple of 3. Each index represents a vertex from the array
    ///   assigned using SetVerts.
    ///
    /// see also [`crate::mesh::mesh_set_inds`]
    pub fn set_inds(&mut self, indices: &[u32]) -> &mut Self {
        unsafe { mesh_set_inds(self.0.as_ptr(), indices.as_ptr(), indices.len() as i32) };
        self
    }

    /// Adds a mesh to the render queue for this frame! If the Hierarchy has a transform on it, that transform is
    /// combined with the Matrix provided here.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Draw.html>
    /// * material - A Material to apply to the Mesh.
    /// * transform - A Matrix that will transform the mesh from Model Space into the current Hierarchy Space.
    /// * color_linear - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in
    ///   extra per-instance data for the shader! If None has default value of WHITE
    /// * layer - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the
    ///   user's head from a 3rd person perspective, but filtering it out from the 1st person perspective.If None has
    ///   default value of Layer0
    ///
    /// see also [`crate::mesh::mesh_draw`]
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
    /// see also [`crate::mesh::mesh_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(mesh_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }
    /// This is a bounding box that encapsulates the Mesh! It's used for collision, visibility testing, UI layout, and
    /// probably  other things. While it's normally calculated from the mesh vertices, you can also override this to
    /// suit your needs.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Bounds.html>
    ///
    /// see also [`crate::mesh::mesh_get_bounds`]
    pub fn get_bounds(&self) -> Bounds {
        unsafe { mesh_get_bounds(self.0.as_ptr()) }
    }

    /// Should StereoKit keep the mesh data on the CPU for later access, or collision detection? Defaults to true. If you
    /// set this to false before setting data, the data won't be stored. If you call this after setting data, that
    /// stored data will be freed! If you set this to true again later on, it will not contain data until it's set again.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/KeepData.html>
    ///
    /// see also [`crate::mesh::mesh_get_keep_data`]
    pub fn get_keep_data(&self) -> bool {
        unsafe { mesh_get_keep_data(self.0.as_ptr()) != 0 }
    }

    /// Get the number of indices stored in this Mesh! This is available to you regardless of whether or not keep_data
    /// is set.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/IndCount.html>
    ///
    /// see also [`crate::mesh::mesh_get_ind_count`]
    pub fn get_ind_count(&self) -> i32 {
        unsafe { mesh_get_ind_count(self.0.as_ptr()) }
    }

    /// Get the number of vertices stored in this Mesh! This is available to you regardless of whether or not keep_data
    /// is set.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/VertCount.html>
    ///
    /// see also [`crate::mesh::mesh_get_vert_count`]
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
    /// see also [Mesh::get_inds_copy] [`crate::mesh::mesh_get_inds`]
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
    /// see also [Mesh::get_inds] [`crate::mesh::mesh_get_inds`]
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
    /// see also [Mesh::get_verts_copy] [`crate::mesh::mesh_get_verts`]
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
    /// see also [Mesh::get_verts] [`crate::mesh::mesh_get_verts`]
    pub fn get_verts_copy(&self) -> Vec<Vertex> {
        self.get_verts().to_vec()
    }

    /// Retrieves the vertices associated with a particular triangle on the Mesh.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetTriangle.html>
    /// * triangle_index - Starting index of the triangle, should be a multiple of 3.
    ///
    /// Returns an array of 3 vertices if triangle index was valid.
    /// see also [`crate::mesh::mesh_get_triangle`]
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
    /// * model_space_ray - Ray must be in model space, the intersection point will be in model space too. You can use the
    ///   inverse of the mesh's world transform matrix to bring the ray into model space, see the example in the docs!
    /// * cull - If None has default value of Cull::Back.
    ///
    /// Returns a tuple with
    /// - The intersection point of the ray and the mesh, if an intersection  occurs. This is in model space, and must
    ///   be transformed back into world space later.
    /// - The indice of the mesh where the intersection occurs.
    ///
    /// see also [`crate::mesh::mesh_ray_intersect`]
    #[inline]
    pub fn intersect_mesh(&self, model_space_ray: Ray, cull: Option<Cull>) -> Option<(Vec3, VindT)> {
        model_space_ray.intersect_mesh(self, cull)
    }

    /// Checks the intersection point of a Ray and this Mesh with collision data stored on the CPU. A mesh without
    /// collision data will always return false. Ray must be in model space, intersection point will be in model
    /// space too. You can use the inverse of the mesh’s world transform matrix to bring the ray into model space,
    /// see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Intersect.html>
    /// * model_space_ray - Ray must be in model space, the intersection point will be in model space too. You can use the
    ///   inverse of the mesh's world transform matrix to bring the ray into model space, see the example in the docs!
    /// * cull - If None has default value of Cull::Back.
    /// * out_model_space_ray -The intersection point and surface direction of the ray and the mesh, if an intersection
    ///   occurs. This is in model space, and must be transformed back into world space later. Direction is not
    ///   guaranteed to be normalized, especially if your own model->world transform contains scale/skew in it.
    /// * out_start_inds - The index of the first index of the triangle that was hit
    ///
    /// Returns true if an intersection occurs.
    /// see also [`crate::mesh::mesh_ray_intersect`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn intersect_mesh_to_ptr(
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
    pub fn cube() -> Self {
        Mesh::find("default/mesh_cube").unwrap()
    }

    /// A default quad mesh, 2 triangles, 4 verts, from (-0.5,-0.5,0) to (0.5,0.5,0) and facing forward on the Z axis
    /// (0,0,-1). White vertex colors, and UVs from (1,1) at vertex (-0.5,-0.5,0) to (0,0) at vertex (0.5,0.5,0).
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Quad.html>
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
    pub fn sphere() -> Self {
        Mesh::find("default/mesh_sphere").unwrap()
    }

    /// A clone mesh of the left hand
    /// <https://stereokit.net/Pages/StereoKit/Mesh.html>
    pub fn left_hand() -> Self {
        Mesh::find("default/mesh_lefthand").unwrap()
    }

    /// A clone mesh of the right hand
    /// <https://stereokit.net/Pages/StereoKit/Mesh.html>
    pub fn right_hand() -> Self {
        Mesh::find("default/mesh_righthand").unwrap()
    }
}
