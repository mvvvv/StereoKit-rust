use crate::{
    StereoKitError,
    material::{Material, MaterialT},
    maths::{Matrix, Rect},
    mesh::{Mesh, MeshT},
    model::{Model, ModelT},
    system::{IAsset, RenderClear, RenderLayer, assets_releaseref_threadsafe},
    tex::{Tex, TexT},
    util::Color128,
};
use std::{
    self,
    ffi::{CStr, CString, c_char, c_void},
    ptr::NonNull,
};

/// A RenderList is a collection of Draw commands that can be submitted to various surfaces. RenderList.Primary is
/// where all normal Draw calls get added to, and this RenderList is renderer to primary display surface.
///
/// Manually working with a RenderList can be useful for "baking down matrices" or caching a scene of objects. Or
/// for drawing a separate scene to an offscreen surface, like for thumbnails of Models.
/// <https://stereokit.net/Pages/StereoKit/RenderList.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{maths::{Vec3, Matrix, Rect}, model::Model, util::Color128,
///                      tex::{Tex, TexType, TexFormat}, material::Material,
///                      mesh::Mesh, render_list::RenderList, system::RenderClear};
///
/// let model = Model::from_file("plane.glb", None).unwrap().copy();
///
/// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
///                       TexType::Rendertarget, TexFormat::RGBA32);
/// let mut render_mat = Material::unlit().copy();
/// render_mat.diffuse_tex(&render_tex);
///
/// let at = Vec3::new(-2.0, 1.0, 1000.9);
/// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 1010.0);
/// let transform_plane = Matrix::r([90.0, 90.0, 145.0]);
/// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, Some(Vec3::new(1.0, 1.0, 1.0)));
///
/// let mut render_list = RenderList::new();
/// render_list.add_model(&model, None, transform_plane, Color128::WHITE, None);
///
/// let screen = Mesh::screen_quad();
///
/// filename_scr = "screenshots/render_list.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     // The color will change so we redraw every frame
///     render_list.draw_now( &render_tex,
///            transform_cam,
///            perspective,
///            Some(Color128::new((iter % 100) as f32 * 0.01, 0.3, 0.2, 0.5)),
///            Some(RenderClear::Color),
///            Rect::new(0.0, 0.0, 1.0, 1.0),
///            None,
///        );
///
///     screen.draw(token, &render_mat, Matrix::IDENTITY, None, None);
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list.jpeg" alt="screenshot" width="200">
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RenderList(pub NonNull<_RenderListT>);

impl Drop for RenderList {
    fn drop(&mut self) {
        unsafe { assets_releaseref_threadsafe(self.0.as_ptr() as *mut c_void) };
    }
}

impl AsRef<RenderList> for RenderList {
    fn as_ref(&self) -> &RenderList {
        self
    }
}

/// StereoKit internal type.
#[repr(C)]
#[derive(Debug)]
pub struct _RenderListT {
    _unused: [u8; 0],
}
/// StereoKit ffi type.
pub type RenderListT = *mut _RenderListT;

unsafe extern "C" {
    pub fn render_list_find(id: *const c_char) -> RenderListT;
    pub fn render_list_set_id(render_list: RenderListT, id: *const c_char);
    pub fn render_list_get_id(render_list: RenderListT) -> *const c_char;
    pub fn render_get_primary_list() -> RenderListT;
    pub fn render_list_create() -> RenderListT;
    pub fn render_list_addref(list: RenderListT);
    pub fn render_list_release(list: RenderListT);
    pub fn render_list_clear(list: RenderListT);
    pub fn render_list_item_count(list: RenderListT) -> i32;
    pub fn render_list_prev_count(list: RenderListT) -> i32;
    pub fn render_list_add_mesh(
        list: RenderListT,
        mesh: MeshT,
        material: MaterialT,
        transform: Matrix,
        color_linear: Color128,
        render_layer: RenderLayer,
    );
    pub fn render_list_add_model(
        list: RenderListT,
        model: ModelT,
        transform: Matrix,
        color_linear: Color128,
        render_layer: RenderLayer,
    );
    pub fn render_list_add_model_mat(
        list: RenderListT,
        model: ModelT,
        material_override: MaterialT,
        transform: Matrix,
        color_linear: Color128,
        render_layer: RenderLayer,
    );
    pub fn render_list_draw_now(
        list: RenderListT,
        to_rendertarget: TexT,
        camera: Matrix,
        projection: Matrix,
        clear_color: Color128,
        clear: RenderClear,
        viewport_pct: Rect,
        layer_filter: RenderLayer,
    );
    pub fn render_list_push(list: RenderListT);
    pub fn render_list_pop();

}

impl Default for RenderList {
    fn default() -> Self {
        Self::new()
    }
}

impl IAsset for RenderList {
    // fn id(&mut self, id: impl AsRef<str>) {
    //     self.id(id);
    // }

    fn get_id(&self) -> &str {
        self.get_id()
    }
}

impl RenderList {
    /// Creates a new empty RenderList.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/RenderList.html>
    ///
    /// see also [`render_list_create`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render_list::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_count(), 1);
    /// ```
    pub fn new() -> Self {
        RenderList(NonNull::new(unsafe { render_list_create() }).unwrap())
    }

    /// Looks for a RenderList matching the given id!
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Find.html>
    /// * `id` - The id of the RenderList we are looking for.
    ///
    /// see also [`render_list_find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render_list::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// render_list.id("my_render_list");
    ///
    /// let same_list = RenderList::find("my_render_list").expect("my_render_list should be found");
    /// assert_eq!(render_list, same_list);
    /// ```
    pub fn find<S: AsRef<str>>(id: S) -> Result<RenderList, StereoKitError> {
        let c_str = CString::new(id.as_ref())?;
        let render_list = NonNull::new(unsafe { render_list_find(c_str.as_ptr()) });
        match render_list {
            Some(render_list) => Ok(RenderList(render_list)),
            None => Err(StereoKitError::RenderListFind(id.as_ref().to_owned(), "not found".to_owned())),
        }
    }

    /// Creates a clone of the same reference. Basically, the new variable is the same asset. This is what you get by
    /// calling find() method.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Find.html>
    ///
    /// see also [`render_list_find`] [`RenderList::find`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render_list::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    ///
    /// let same_list = render_list.clone_ref();
    /// assert_eq!(render_list, same_list);
    /// ```
    pub fn clone_ref(&self) -> RenderList {
        RenderList(
            NonNull::new(unsafe { render_list_find(render_list_get_id(self.0.as_ptr())) })
                .expect("<asset>::clone_ref failed!"),
        )
    }

    /// sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Id.html>
    ///
    /// see also [`render_list_set_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render_list::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// render_list.id("my_render_list");
    ///
    /// assert_eq!(render_list.get_id(), "my_render_list");
    /// ```
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let cstr_id = CString::new(id.as_ref()).unwrap();
        unsafe { render_list_set_id(self.0.as_ptr(), cstr_id.as_ptr()) };
        self
    }

    /// Clears out and de-references all Draw items currently in the RenderList.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Clear.html>
    ///
    /// see also [`render_list_clear`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render_list::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_count(), 1);
    /// assert_eq!(render_list.get_prev_count(), 0);
    ///
    /// render_list.clear();
    /// assert_eq!(render_list.get_count(), 0);
    /// assert_eq!(render_list.get_prev_count(), 1);
    /// ```
    pub fn clear(&mut self) -> &mut Self {
        unsafe { render_list_clear(self.0.as_ptr()) }
        self
    }

    /// Add a Mesh/Material to the RenderList. The RenderList will hold a reference to these Assets until the list is
    /// cleared.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Add.html>
    /// * `mesh` - A valid Mesh you wish to draw.
    /// * `material` - A Material to apply to the Mesh.
    /// * `transform` - A transformation Matrix relative to the current Hierarchy.
    /// * `colorLinear` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in extra
    ///   per-instance data for the shader!
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the user's
    ///   head from a 3rd person perspective, but filtering it out from the 1st person perspective.
    ///
    /// see also [`render_list_add_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect},  util::{named_colors, Color128},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render_list::RenderList, system::{RenderClear, RenderLayer}};
    ///
    /// let cylinder1 = Mesh::generate_cylinder(0.3, 1.5, [ 0.5, 0.5, 0.0],None);
    /// let cylinder2 = Mesh::generate_cylinder(0.3, 1.5, [-0.5, 0.5, 0.0],None);
    /// let cylinder_mat = Material::pbr();
    ///
    /// let at = Vec3::new(-80.0, 1.0, 80.0);
    /// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 120.0);
    /// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, None);
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_mesh(&cylinder1, &cylinder_mat, Matrix::IDENTITY, named_colors::CYAN, None)
    ///     .add_mesh(&cylinder2, &cylinder_mat, Matrix::IDENTITY, named_colors::FUCHSIA,
    ///               Some(RenderLayer::Layer1));
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::RGBA32);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let screen = Mesh::screen_quad();
    /// render_list.draw_now( &render_tex,
    ///     transform_cam,
    ///     perspective,
    ///     Some(Color128::new(0.99, 0.3, 0.2, 0.5)),
    ///     Some(RenderClear::Color),
    ///     Rect::new(0.0, 0.0, 1.0, 1.0),
    ///     Some(RenderLayer::AllThirdPerson),
    /// );
    ///
    /// filename_scr = "screenshots/render_list_add_mesh.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     screen.draw(token, &render_mat, Matrix::IDENTITY, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_mesh.jpeg" alt="screenshot" width="200">
    pub fn add_mesh(
        &mut self,
        mesh: impl AsRef<Mesh>,
        material: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color_linear: impl Into<Color128>,
        layer: Option<RenderLayer>,
    ) -> &mut Self {
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe {
            render_list_add_mesh(
                self.0.as_ptr(),
                mesh.as_ref().0.as_ptr(),
                material.as_ref().0.as_ptr(),
                transform.into(),
                color_linear.into(),
                layer,
            )
        }
        self
    }

    /// Add a Model to the RenderList. The RenderList will hold a reference to these Assets until the list is cleared.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Add.html>
    /// * `model` - A valid Model you wish to draw.
    /// * `material` - Allows you to override the Material.
    /// * `transform` - A transformation Matrix relative to the current Hierarchy.
    /// * `colorLinear` - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    ///   material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in extra
    ///   per-instance data for the shader!
    /// * `layer` - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    ///   useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the user's
    ///   head from a 3rd person perspective, but filtering it out from the 1st person perspective.
    ///
    /// see also [`render_list_add_model`] [`render_list_add_model_mat`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect}, model::Model, util::{named_colors,Color128},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render_list::RenderList, system::{RenderClear, RenderLayer}};
    ///
    /// let model = Model::from_file("plane.glb", None).unwrap().copy();
    ///
    /// let at = Vec3::new(-2.0, 400.0, 1000.9);
    /// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 1550.0);
    /// let transform_plane1 = Matrix::t([ 5.0, 2.0, 0.0]);
    /// let transform_plane2 = Matrix::t([2.0, -6.0, -10.0]);
    /// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, Some(Vec3::new(0.0, -1.0, 0.0)));
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_model(&model, None, transform_plane1, named_colors::RED, None)
    ///     .add_model(&model, None, transform_plane2, named_colors::BLUE,
    ///                Some(RenderLayer::Layer1));
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::RGBA32);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let screen = Mesh::screen_quad();
    /// render_list.draw_now( &render_tex,
    ///     transform_cam,
    ///     perspective,
    ///     Some(Color128::new(0.0, 0.3, 0.2, 0.5)),
    ///     Some(RenderClear::Color),
    ///     Rect::new(0.0, 0.0, 1.0, 1.0),
    ///     Some(RenderLayer::AllFirstPerson),
    /// );
    ///
    /// filename_scr = "screenshots/render_list_add_model.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     screen.draw(token, &render_mat, Matrix::IDENTITY, None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_model.jpeg" alt="screenshot" width="200">
    pub fn add_model(
        &mut self,
        model: impl AsRef<Model>,
        material_override: Option<Material>,
        transform: impl Into<Matrix>,
        color_linear: impl Into<Color128>,
        layer: Option<RenderLayer>,
    ) -> &mut Self {
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        match material_override {
            Some(material) => unsafe {
                render_list_add_model_mat(
                    self.0.as_ptr(),
                    model.as_ref().0.as_ptr(),
                    material.as_ref().0.as_ptr(),
                    transform.into(),
                    color_linear.into(),
                    layer,
                )
            },
            None => unsafe {
                render_list_add_model(
                    self.0.as_ptr(),
                    model.as_ref().0.as_ptr(),
                    transform.into(),
                    color_linear.into(),
                    layer,
                )
            },
        }
        self
    }

    /// Draws the RenderList to a rendertarget texture immediately. It does _not_ clear the list
    /// <https://stereokit.net/Pages/StereoKit/RenderList/DrawNow.html>
    /// * `to_render_target` - The rendertarget texture to draw to.
    /// * `camera` - A TRS matrix representing the location and orientation of the camera. This matrix gets inverted
    ///   later on, so no need to do it yourself.
    /// * `projection` - The projection matrix describes how the geometry is flattened onto the draw surface. Normally,
    ///   you'd use Matrix.Perspective, and occasionally Matrix.Orthographic might be helpful as well.
    /// * `clear_color` * If the `clear` parameter is set to clear the color of `to_render_target`, then this is the color
    ///   it will clear to. `default` would be a transparent black.
    /// * `clear` - Describes if and how the render_target should be cleared before rendering. Note that clearing the
    ///   target is unaffected by the viewport, so this will clean the entire surface! None is All.
    /// * `viewport_pct` - Allows you to specify a region of the rendertarget to draw to! This is in normalized
    ///   coordinates, 0-1. If the width of this value is zero, then this will render to the entire texture.
    /// * `layerFilter` - This is a bit flag that allows you to change which layers StereoKit renders for this
    ///   particular render viewpoint. To change what layers a visual is on, use a Draw method that includes a
    ///   RenderLayer as a parameter.
    ///
    /// see also [`render_list_draw_now`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix, Rect},  util::{named_colors, Color128, Color32},
    ///                      tex::{Tex, TexType, TexFormat}, material::Material,
    ///                      mesh::Mesh, render_list::RenderList, system::{RenderClear, RenderLayer}};
    ///
    /// let cylinder1 = Mesh::generate_cylinder(0.3, 1.5, [ 0.5, 0.5, 0.0],None);
    /// let cylinder2 = Mesh::generate_cylinder(0.3, 1.5, [-0.5, 0.5, 0.0],None);
    /// let cylinder_mat = Material::pbr().copy();
    ///
    /// let render_tex = Tex::gen_color(Color128::WHITE, 128, 128,
    ///                       TexType::Rendertarget, TexFormat::RGBA32);
    /// let mut render_mat = Material::unlit().copy();
    /// render_mat.diffuse_tex(&render_tex);
    /// let mut screen = Mesh::generate_cube([1.0, 1.0, 1.0], None);
    /// let transform_screen = Matrix::t([0.0, 0.0, -1.0]);
    ///
    /// let at = Vec3::new(-80.0, 1.0, 80.0);
    /// let perspective = Matrix::perspective(45.0, 1.0, 0.01, 220.0);
    /// let transform_cam  = Matrix::look_at(at, Vec3::ZERO, None);
    ///
    /// let mut render_list = RenderList::new();
    /// render_list
    ///     .add_mesh(&cylinder1, &cylinder_mat, Matrix::IDENTITY, named_colors::CYAN, None)
    ///     .add_mesh(&cylinder2, &cylinder_mat, Matrix::IDENTITY, named_colors::FUCHSIA,None)
    ///     .add_mesh(&screen,    &render_mat,   transform_screen, named_colors::GRAY, None);
    ///
    /// filename_scr = "screenshots/render_list_draw_now.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     screen.draw(token, &render_mat, Matrix::IDENTITY, None, None);
    ///     render_list.draw_now( &render_tex,
    ///         transform_cam,
    ///         perspective,
    ///         None,
    ///         Some(RenderClear::None),
    ///         Rect::new(0.0, 0.0, 1.0, 1.0),
    ///         None
    ///     );
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_draw_now.jpeg" alt="screenshot" width="200">
    #[allow(clippy::too_many_arguments)]
    pub fn draw_now(
        &mut self,
        to_rendertarget: impl AsRef<Tex>,
        camera: impl Into<Matrix>,
        projection: impl Into<Matrix>,
        clear_color: Option<Color128>,
        clear: Option<RenderClear>,
        viewport_pct: Rect,
        layer_filter: Option<RenderLayer>,
    ) {
        let layer_filter = layer_filter.unwrap_or(RenderLayer::all());
        let clear = clear.unwrap_or(RenderClear::All);
        let clear_color = clear_color.unwrap_or_default();
        unsafe {
            render_list_draw_now(
                self.0.as_ptr(),
                to_rendertarget.as_ref().0.as_ptr(),
                camera.into(),
                projection.into(),
                clear_color,
                clear,
                viewport_pct,
                layer_filter,
            )
        }
    }

    /// The default RenderList used by the Renderer for the primary display surface.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Primary.html>
    ///
    /// see also [`render_get_primary_list`]
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::render_list::RenderList;
    ///
    /// let primary_list = RenderList::primary();
    /// assert_eq!   (primary_list.get_id(),"sk/render/primary_renderlist");
    /// assert_eq!   (primary_list.get_count(), 0);
    /// ```
    pub fn primary() -> Self {
        RenderList(NonNull::new(unsafe { render_get_primary_list() }).unwrap())
    }

    /// All draw calls that don't specify a render list will get submitted to the active RenderList at the top of the
    /// stack. By default, that's RenderList.Primary, but you can push your own list onto the stack here to capture draw
    /// calls, like those done in the UI.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Push.html>
    ///
    /// see also [`render_list_push`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::{Vec3, Matrix},  util::named_colors, material::Material,
    ///                      mesh::Mesh, render_list::RenderList};
    ///
    /// let cylinder1 = Mesh::generate_cylinder(0.3, 1.5, [ 0.5, 0.5, 0.0],None);
    /// let cylinder2 = Mesh::generate_cylinder(0.3, 1.5, [-0.5, 0.5, 0.0],None);
    /// let cylinder_mat = Material::pbr().copy();
    ///
    /// let mut render_list = RenderList::new();
    /// // render_list.add_mesh(&cylinder1, &cylinder_mat, Matrix::IDENTITY,  None);
    ///
    /// filename_scr = "screenshots/render_list_push.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     render_list.push();
    ///     cylinder1.draw(token, &cylinder_mat, Matrix::IDENTITY, Some(named_colors::GOLD.into()), None);
    ///     RenderList::pop();
    ///     cylinder2.draw(token, &cylinder_mat, Matrix::IDENTITY, Some(named_colors::RED.into()), None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_push.jpeg" alt="screenshot" width="200">
    pub fn push(&mut self) {
        unsafe { render_list_push(self.0.as_ptr()) }
    }

    /// This removes the current top of the RenderList stack, making the next list as active
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Pop.html>
    ///
    /// see also [`render_list_pop`]
    /// see example [`RenderList::push`]
    pub fn pop() {
        unsafe { render_list_pop() }
    }

    /// The id of this render list
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Id.html>
    ///
    /// see also [`render_list_get_id`]
    /// see example [`RenderList::id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(render_list_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// The number of Mesh/Material pairs that have been submitted to the render list so far this frame.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Count.html>
    ///
    /// see also [`render_list_item_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render_list::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_count(), 1);
    /// ```
    pub fn get_count(&self) -> i32 {
        unsafe { render_list_item_count(self.0.as_ptr()) }
    }

    /// This is the number of items in the RenderList before it was most recently cleared. If this is a list that is
    /// drawn and cleared each frame, you can think of this as "last frame's count".
    /// <https://stereokit.net/Pages/StereoKit/RenderList/PrevCount.html>
    ///
    /// see also [`render_list_prev_count`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{maths::Matrix,  util::Color128,
    ///                      material::Material, mesh::Mesh, render_list::RenderList};
    ///
    /// let mut render_list = RenderList::new();
    /// assert!   (render_list.get_id().starts_with("auto/render_list_"));
    /// assert_eq!(render_list.get_prev_count(), 0);
    ///
    /// render_list.add_mesh(Mesh::cube(), Material::unlit(), Matrix::IDENTITY, Color128::WHITE, None);
    /// assert_eq!(render_list.get_prev_count(), 0);
    ///
    /// render_list.clear();
    /// assert_eq!(render_list.get_count(), 0);
    /// assert_eq!(render_list.get_prev_count(), 1);
    /// ```
    pub fn get_prev_count(&self) -> i32 {
        unsafe { render_list_prev_count(self.0.as_ptr()) }
    }
}
