use std::{self, ffi::c_void, ptr::NonNull};

use crate::system::assets_releaseref_threadsafe;

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

    /// The default RenderList used by the Renderer for the primary display surface.
    /// <https://stereokit.net/Pages/StereoKit/RenderList/Primary.html>
    ///
    /// see also [`crate::render_list::render_get_primary_list`]
    pub fn primary() -> Self {
        RenderList(NonNull::new(unsafe { render_get_primary_list() }).unwrap())
    }
}
