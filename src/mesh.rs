use std::{
    ffi::{c_char, CStr, CString},
    ptr::{slice_from_raw_parts_mut, NonNull},
};

use crate::{
    material::{Cull, Material, MaterialT},
    maths::{Bool32T, Matrix, Vec2, Vec3, Vec4},
    maths::{Bounds, Ray},
    system::{IAsset, RenderLayer},
    util::{Color128, Color32},
    StereoKitError,
};

/// This represents a single vertex in a Mesh, all StereoKit Meshes currently use this exact layout!
/// It’s good to fill out all values of a Vertex explicitly, as default values for the normal (0,0,0) and color
/// (0,0,0,0) will cause your mesh to appear completely black, or even transparent in most shaders!
/// <https://stereokit.net/Pages/StereoKit/Vertex.html>
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
/// ## Examples
///
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
extern "C" {
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
    /// Creates an empty mesh.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Mesh.html>
    fn default() -> Self {
        Self::new()
    }
}

impl Mesh {
    /// Creates an empty mesh.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Mesh.html>
    ///
    /// see also [`crate::mesh::mesh_create`]
    pub fn new() -> Mesh {
        Mesh(NonNull::new(unsafe { mesh_create() }).unwrap())
    }

    /// Creates a plane
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GeneratePlane.html>
    /// * `subdivisions` - if None has default value of 0
    /// * `double_side` - if None has default value of false
    ///
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

    /// Creates a face up plane
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GeneratePlane.html>
    /// * `subdivisions` - if None has default value to 0
    /// * `double_side` - if None has default value of false
    ///
    /// see also [`crate::mesh::mesh_gen_plane`]
    pub fn generate_plane_up(dimensions: impl Into<Vec2>, subdivisions: Option<i32>, double_sided: bool) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(0);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_plane(
                    dimensions.into(),
                    Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                    Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                    subdivisions,
                    double_sided as Bool32T,
                )
            })
            .unwrap(),
        )
    }

    ///<https://stereokit.net/Pages/StereoKit/Mesh/GenerateCircle.html>
    /// * `spoke` - if None has default value of 16
    /// * `double_side` - if None has default value of false
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

    ///<https://stereokit.net/Pages/StereoKit/Mesh/GenerateCircle.html>
    /// * `spoke` - if None has default value to 16
    /// * `double_side` - if None has default value of false
    ///
    /// see also [`crate::mesh::mesh_gen_circle`]
    pub fn generate_circle_up(diameter: f32, spokes: Option<i32>, double_sided: bool) -> Mesh {
        let spokes = spokes.unwrap_or(16);
        Mesh(
            NonNull::new(unsafe {
                mesh_gen_circle(
                    diameter,
                    Vec3 { x: 0.0, y: 1.0, z: 0.0 },
                    Vec3 { x: 0.0, y: 0.0, z: -1.0 },
                    spokes,
                    double_sided as Bool32T,
                )
            })
            .unwrap(),
        )
    }

    ///<https://stereokit.net/Pages/StereoKit/Mesh/GenerateCube.html>
    /// * `subdivisions` - if None has default value of 0
    ///
    /// see also [`crate::mesh::mesh_gen_circle`]
    pub fn generate_cube(dimensions: impl Into<Vec3>, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(0);
        Mesh(NonNull::new(unsafe { mesh_gen_cube(dimensions.into(), subdivisions) }).unwrap())
    }

    ///<https://stereokit.net/Pages/StereoKit/Mesh/GenerateRoundedCube.html>
    /// * `subdivisions` - if None has default value of 4
    ///
    /// see also [`crate::mesh::mesh_gen_rounded_circle`]
    pub fn generate_rounded_cube(dimensions: impl Into<Vec3>, edge_radius: f32, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(4);
        Mesh(NonNull::new(unsafe { mesh_gen_rounded_cube(dimensions.into(), edge_radius, subdivisions) }).unwrap())
    }

    ///<https://stereokit.net/Pages/StereoKit/Mesh/GenerateSphere.html>
    /// * `subdivisions` - if None has default value of 4
    ///
    /// see also [`crate::mesh::mesh_gen_sphere`]
    pub fn generate_sphere(diameter: f32, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(4);
        Mesh(NonNull::new(unsafe { mesh_gen_sphere(diameter, subdivisions) }).unwrap())
    }

    ///<https://stereokit.net/Pages/StereoKit/Mesh/GenerateCylinder.html>
    /// * `subdivisions` - if None has default value of 16
    ///
    /// see also [`crate::mesh::mesh_gen_cylinder`]
    pub fn generate_cylinder(diameter: f32, depth: f32, direction: impl Into<Vec3>, subdivisions: Option<i32>) -> Mesh {
        let subdivisions = subdivisions.unwrap_or(16);
        Mesh(NonNull::new(unsafe { mesh_gen_cylinder(diameter, depth, direction.into(), subdivisions) }).unwrap())
    }

    /// Looks for a Mesh asset that’s already loaded, matching the given id!
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

    /// Sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Id.html>
    ///
    /// see also [`crate::mesh::mesh_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { mesh_set_id(self.0.as_ptr(), cstr.as_ptr()) };
        self
    }

    /// Set the bounds of this mesh
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Bounds.html>
    ///
    /// see also [`crate::mesh::mesh_set_bounds`]
    pub fn bounds(&mut self, bounds: impl AsRef<Bounds>) -> &mut Self {
        unsafe { mesh_set_bounds(self.0.as_ptr(), bounds.as_ref() as *const Bounds) };
        self
    }

    /// Set the keep data flag. Default is true
    /// <https://stereokit.net/Pages/StereoKit/Mesh/KeepData.html>
    ///
    /// see also [`crate::mesh::mesh_set_keep_data`]
    pub fn keep_data(&mut self, keep_data: bool) -> &mut Self {
        unsafe { mesh_set_keep_data(self.0.as_ptr(), keep_data as Bool32T) };
        self
    }

    /// Set the data
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetData.html>
    /// * calculate_bounds - if None has the default value of true
    ///
    /// see also [`crate::mesh::mesh_set_data`]
    pub fn set_data(&mut self, vertices: &[Vertex], indices: &[u32], calculate_bounds: Option<bool>) -> &mut Self {
        let calculate_bounds = calculate_bounds.unwrap_or(true);
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

    /// Set the vertices
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetVerts.html>
    /// * calculate_bounds - has the default value of true
    ///
    /// see also [`crate::mesh::mesh_set_verts`]
    pub fn set_verts(&mut self, vertices: &[Vertex], calculate_bounds: bool) -> &mut Self {
        unsafe {
            mesh_set_verts(self.0.as_ptr(), vertices.as_ptr(), vertices.len() as i32, calculate_bounds as Bool32T)
        };
        self
    }

    /// Set the indices
    /// <https://stereokit.net/Pages/StereoKit/Mesh/SetInds.html>
    ///
    /// see also [`crate::mesh::mesh_set_indss`]
    pub fn set_inds(&mut self, indices: &[u32]) -> &mut Self {
        unsafe { mesh_set_inds(self.0.as_ptr(), indices.as_ptr(), indices.len() as i32) };
        self
    }

    /// Adds the mesh to the render queue of this frame
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Draw.html>
    /// * color_linear - if None has default value of WHITE
    /// * layer - if None has default value of Layer0
    ///
    /// see also [`stereokit::StereoKitDraw::mesh_draw`]
    pub fn draw(
        &self,
        material: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color_linear: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color_linear: Color128 = match color_linear {
            Some(c) => c,
            None => Color128::WHITE,
        };
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe { mesh_draw(self.0.as_ptr(), material.as_ref().0.as_ptr(), transform.into(), color_linear, layer) }
    }

    /// Get the id of this mesh
    /// <https://stereokit.net/Pages/StereoKit/Mesh/id.html>
    ///
    /// see also [`crate::mesh::mesh_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(mesh_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// Get the bounds
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Bounds.html>
    ///
    /// see also [`crate::mesh::mesh_get_bounds`]
    pub fn get_bounds(&self) -> Bounds {
        unsafe { mesh_get_bounds(self.0.as_ptr()) }
    }

    /// Get the keep data flag.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/KeepData.html>
    ///
    /// see also [`crate::mesh::mesh_get_keep_data`]
    pub fn get_keep_data(&self) -> bool {
        unsafe { mesh_get_keep_data(self.0.as_ptr()) != 0 }
    }

    /// Get the number of indices.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/IndCount.html>
    ///
    /// see also [`crate::mesh::mesh_get_ind_count`]
    pub fn get_ind_count(&self) -> i32 {
        unsafe { mesh_get_ind_count(self.0.as_ptr()) }
    }

    /// Get the number of vertices.
    /// <https://stereokit.net/Pages/StereoKit/Mesh/VertCount.html>
    ///
    /// see also [`crate::mesh::mesh_get_vert_count`]
    pub fn get_vert_count(&self) -> i32 {
        unsafe { mesh_get_vert_count(self.0.as_ptr()) }
    }

    /// Get the indices by ref
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetInds.html>
    ///
    /// see also [`crate::mesh::mesh_get_inds_ref`]
    pub fn get_inds(&self) -> &[u32] {
        let inds_ptr = CString::new("H").unwrap().into_raw() as *mut *mut u32;
        let mut inds_len = 0;
        unsafe {
            mesh_get_inds(self.0.as_ptr(), inds_ptr, &mut inds_len, Memory::Reference);
            &mut *slice_from_raw_parts_mut(*inds_ptr, inds_len as usize)
        }
    }

    /// Get the indices by value
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetInds.html>
    ///
    /// see also [`crate::mesh::mesh_get_inds_copy`]
    pub fn get_inds_copy(&self) -> Vec<u32> {
        self.get_inds().to_vec()
    }

    /// Get the vertices by ref
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetVerts.html>
    ///
    /// see also [`crate::mesh::mesh_get_verts`]
    pub fn get_verts(&self) -> &[Vertex] {
        let verts_pointer = CString::new("H").unwrap().into_raw() as *mut *mut Vertex;
        let mut verts_len = 0;
        unsafe {
            mesh_get_verts(self.0.as_ptr(), verts_pointer, &mut verts_len, Memory::Reference);
            &mut *slice_from_raw_parts_mut(*verts_pointer, verts_len as usize)
        }
    }

    /// Get the vertices by value
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetVerts.html>
    ///
    /// see also [`crate::mesh::mesh_get_verts_copy`]
    pub fn get_verts_copy(&self) -> Vec<Vertex> {
        self.get_verts().to_vec()
    }

    /// Get the triangle by value
    /// <https://stereokit.net/Pages/StereoKit/Mesh/GetTriangle.html>
    ///
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
    /// collision data will always return false. Ray must be in model space, intersection point will be in model
    /// space too. You can use the inverse of the mesh’s world transform matrix to bring the ray into model space,
    /// see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Intersect.html>
    /// * cull - If None has default value of Cull::Back.
    ///
    /// see also [`stereokit::mesh_ray_intersect`]
    #[inline]
    pub fn intersect_mesh(&self, ray: Ray, cull: Option<Cull>) -> Option<(Vec3, VindT)> {
        ray.intersect_mesh(self, cull)
    }

    /// Checks the intersection point of a Ray and this Mesh with collision data stored on the CPU. A mesh without
    /// collision data will always return false. Ray must be in model space, intersection point will be in model
    /// space too. You can use the inverse of the mesh’s world transform matrix to bring the ray into model space,
    /// see the example in the docs!
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Intersect.html>
    /// * cull - If None has default value of Cull::Back.
    ///
    /// see also [`stereokit::mesh_ray_intersect`]
    #[inline]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn intersect_mesh_to_ptr(&self, ray: Ray, cull: Option<Cull>, out_ray: *mut Ray, out_inds: *mut u32) -> bool {
        ray.intersect_mesh_to_ptr(self, cull, out_ray, out_inds)
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

    /// see screen_quad instead ! TODO: Why this ?
    /// <https://stereokit.net/Pages/StereoKit/Mesh/Quad.html>
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
