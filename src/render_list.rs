use std::{self, ffi::c_void, ptr::NonNull};

use crate::{
    material::{Material, MaterialT},
    maths::{Matrix, Rect},
    mesh::{Mesh, MeshT},
    model::{Model, ModelT},
    system::{assets_releaseref_threadsafe, RenderClear, RenderLayer},
    tex::{Tex, TexT},
    util::Color128,
};

/// A RenderList is a collection of Draw commands that can be submitted to various surfaces. RenderList.Primary is
/// where all normal Draw calls get added to, and this RenderList is renderer to primary display surface.
///
/// Manually working with a RenderList can be useful for "baking down matrices" or caching a scene of objects. Or
/// for drawing a separate scene to an offscreen surface, like for thumbnails of Models.
/// <https://stereokit.net/Pages/StereoKit/RenderList.html>
#[repr(C)]
#[derive(Debug)]
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
#[repr(C)]
#[derive(Debug)]
pub struct _RenderListT {
    _unused: [u8; 0],
}
pub type RenderListT = *mut _RenderListT;

extern "C" {
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
        viewport_px: Rect,
        layer_filter: RenderLayer,
        clear: RenderClear,
    );
    pub fn render_list_push(list: RenderListT);
    pub fn render_list_pop();

}

impl Default for RenderList {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderList {
    /// Creates a new empty RenderList.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/RenderList.html>
    ///
    /// see also [`crate::render_list::render_list_create`]
    pub fn new() -> Self {
        RenderList(NonNull::new(unsafe { render_list_create() }).unwrap())
    }

    /// The number of Mesh/Material pairs that have been submitted to the render list so far this frame.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Count.html>
    ///
    /// see also [`crate::render_list::render_list_item_count`]
    pub fn get_count(&self) -> i32 {
        unsafe { render_list_item_count(self.0.as_ptr()) }
    }

    /// This is the number of items in the RenderList before it was most recently cleared. If this is a list that is
    /// drawn and cleared each frame, you can think of this as "last frame's count".
    /// <https://stereokit.net/Pages/StereoKit/RenderList/PrevCount.html>
    ///
    /// see also [`crate::render_list::render_list_prev_count`]
    pub fn get_prev_count(&self) -> i32 {
        unsafe { render_list_prev_count(self.0.as_ptr()) }
    }

    /// Clears out and de-references all Draw items currently in the RenderList.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Clear.html>
    ///
    /// see also [`crate::render_list::render_list_clear`]
    pub fn clear(&mut self) {
        unsafe { render_list_clear(self.0.as_ptr()) }
    }

    /// Add a Mesh/Material to the RenderList. The RenderList will hold a reference to these Assets until the list is
    /// cleared.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Add.html>
    /// * mesh - A valid Mesh you wish to draw.
    /// * material - A Material to apply to the Mesh.
    /// * transform - A transformation Matrix relative to the current Hierarchy.
    /// * colorLinear - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    /// material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in extra
    /// per-instance data for the shader!
    /// * layer - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    /// useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the user's
    /// head from a 3rd person perspective, but filtering it out from the 1st person perspective.
    ///
    /// see also [`crate::render_list::render_list_add_mesh`]
    pub fn add_mesh(
        &mut self,
        mesh: impl AsRef<Mesh>,
        material: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color_linear: impl Into<Color128>,
        layer: Option<RenderLayer>,
    ) {
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
    }

    /// Add a Model to the RenderList. The RenderList will hold a reference to these Assets until the list is cleared.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Add.html>
    /// * model - A valid Model you wish to draw.
    /// * material - Allows you to override the Material.
    /// * transform - A transformation Matrix relative to the current Hierarchy.
    /// * colorLinear - A per-instance linear space color value to pass into the shader! Normally this gets used like a
    /// material tint. If you're  adventurous and don't need per-instance colors, this is a great spot to pack in extra
    /// per-instance data for the shader!
    /// * layer - All visuals are rendered using a layer bit-flag. By default, all layers are rendered, but this can be
    /// useful for filtering out objects for different rendering purposes! For example: rendering a mesh over the user's
    /// head from a 3rd person perspective, but filtering it out from the 1st person perspective.
    ///
    /// see also [`crate::render_list::render_list_add_model`] [`crate::render_list::render_list_add_model_mat`]
    pub fn add_model(
        &mut self,
        model: impl AsRef<Model>,
        material_override: Option<Material>,
        transform: impl Into<Matrix>,
        color_linear: impl Into<Color128>,
        layer: Option<RenderLayer>,
    ) {
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
    }

    /// Draws the RenderList to a rendertarget texture immediately. It does _not_ clear the list
    /// <https://stereokit.net/Pages/StereoKit/RenderList/DrawNow.html>
    /// * to_render_target - The rendertarget texture to draw to.
    /// * camera - A TRS matrix representing the location and orientation of the camera. This matrix gets inverted
    /// later on, so no need to do it yourself.
    /// * projection - The projection matrix describes how the geometry is flattened onto the draw surface. Normally,
    /// you'd use Matrix.Perspective, and occasionally Matrix.Orthographic might be helpful as well.
    /// * viewport - Allows you to specify a region of the rendertarget to draw to! This is in normalized
    /// coordinates, 0-1. If the width of this value is zero, then this will render to the entire texture.
    /// * layerFilter - This is a bit flag that allows you to change which layers StereoKit renders for this
    /// particular render viewpoint. To change what layers a visual is on, use a Draw method that includes a
    /// RenderLayer as a parameter.
    /// * clear - Describes if and how the rendertarget should be cleared before rendering. Note that clearing the
    /// target is unaffected by the viewport, so this will clean the entire surface!
    ///
    /// see also [`crate::render_list::render_list_draw_now`]
    pub fn draw_now(
        &mut self,
        to_rendertarget: impl AsRef<Tex>,
        camera: impl Into<Matrix>,
        projection: impl Into<Matrix>,
        viewport_px: Rect,
        layer_filter: Option<RenderLayer>,
        clear: Option<RenderClear>,
    ) {
        let layer_filter = layer_filter.unwrap_or(RenderLayer::all());
        let clear = clear.unwrap_or(RenderClear::All);
        unsafe {
            render_list_draw_now(
                self.0.as_ptr(),
                to_rendertarget.as_ref().0.as_ptr(),
                camera.into(),
                projection.into(),
                viewport_px,
                layer_filter,
                clear,
            )
        }
    }

    /// The default RenderList used by the Renderer for the primary display surface.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Primary.html>
    ///
    /// see also [`crate::render_list::render_get_primary_list`]
    pub fn primary() -> Self {
        RenderList(NonNull::new(unsafe { render_get_primary_list() }).unwrap())
    }

    /// All draw calls that don't specify a render list will get submitted to the active RenderList at the top of the
    /// stack. By default, that's RenderList.Primary, but you can push your own list onto the stack here to capture draw
    /// calls, like those done in the UI.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Push.html>
    ///
    /// see also [`crate::render_list::render_list_push`]
    pub fn push(&mut self) {
        unsafe { render_list_push(self.0.as_ptr()) }
    }

    /// This removes the current top of the RenderList stack, making the next list as active
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Pop.html>
    ///
    /// see also [`crate::render_list::render_list_pop`]
    pub fn pop() {
        unsafe { render_list_pop() }
    }
}
