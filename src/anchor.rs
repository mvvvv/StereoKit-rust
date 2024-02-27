use std::{
    ffi::{c_char, c_void, CStr, CString},
    ptr::{null_mut, NonNull},
};

use crate::{
    maths::{Bool32T, Pose},
    system::{BtnState, Log},
    StereoKitError,
};

/// An Anchor in StereoKit is a completely virtual pose that is pinned to a real-world location. They are creatable via
/// code, generally can persist across sessions, may provide additional stability beyond the system’s 6dof tracking,
/// and are not physical objects!
///
/// This functionality is backed by extensions like the Microsoft Spatial Anchor, or the Facebook Spatial Entity. If a
/// proper anchoring system isn’t present on the device, StereoKit will fall back to a stage- relative anchor.
/// Stage-relative anchors may be a good solution for devices with a consistent stage, but may be troublesome if the user
/// adjusts their stage frequently.
///
/// A conceptual guide to Anchors:
///
/// * A cloud anchor is an Anchor
/// * A QR code is not an Anchor (it’s physical)
/// * That spot around where your coffee usually sits can be an Anchor
/// * A semantically labeled floor plane is not an Anchor (it’s physical)
/// <https://stereokit.net/Pages/StereoKit/Anchor.html>
///
#[repr(C)]
#[derive(Debug)]
pub struct Anchor(pub NonNull<_AnchorT>);
impl Drop for Anchor {
    fn drop(&mut self) {
        unsafe { anchor_release(self.0.as_ptr()) };
    }
}
impl AsRef<Anchor> for Anchor {
    fn as_ref(&self) -> &Anchor {
        self
    }
}
#[repr(C)]
#[derive(Debug)]
pub struct _AnchorT {
    _unused: [u8; 0],
}
pub type AnchorT = *mut _AnchorT;

/// This is a bit flag that describes what an anchoring system is capable of doing.
/// <https://stereokit.net/Pages/StereoKit/AnchorCaps.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum AnchorCaps {
    /// This anchor system can store/persist anchors across sessions. Anchors must still be explicitly marked as
    /// persistent.
    Storable = 1,

    /// This anchor system will provide extra accuracy in locating the Anchor, so if the SLAM/6dof tracking drifts over
    /// time or distance, the anchor may remain fixed in the correct physical space, instead of drifting with the
    /// virtual content.
    Stability = 2,
}

extern "C" {
    pub fn anchor_find(asset_id_utf8: *const c_char) -> AnchorT;
    pub fn anchor_create(pose: Pose) -> AnchorT;
    pub fn anchor_set_id(anchor: AnchorT, asset_id_utf8: *const c_char);
    pub fn anchor_get_id(anchor: AnchorT) -> *const c_char;
    pub fn anchor_addref(anchor: AnchorT);
    pub fn anchor_release(anchor: AnchorT);
    pub fn Anchor_try_set_persistent(anchor: AnchorT, persistent: Bool32T) -> Bool32T;
    pub fn anchor_get_persistent(anchor: AnchorT) -> Bool32T;
    pub fn anchor_get_pose(anchor: AnchorT) -> Pose;
    pub fn anchor_get_changed(anchor: AnchorT) -> Bool32T;
    pub fn anchor_get_name(anchor: AnchorT) -> *const c_char;
    pub fn anchor_get_tracked(anchor: AnchorT) -> BtnState;
    pub fn anchor_clear_stored();
    pub fn anchor_get_capabilities() -> AnchorCaps;
    pub fn anchor_get_count() -> i32;
    pub fn anchor_get_index(index: i32) -> AnchorT;
    pub fn anchor_get_new_count() -> i32;
    pub fn anchor_get_new_index(index: i32) -> AnchorT;
    pub fn anchor_get_perception_anchor(anchor: AnchorT, perception_spatial_anchor: *mut *mut c_void) -> Bool32T; //TODO: Check this

}

impl Anchor {
    /// Searches the asset list for an anchor with the given Id.
    /// <https://stereokit.net/Pages/StereoKit/Anchor/Find.html>
    ///
    /// see also [`crate::anchor::anchor_find`]
    pub fn find<S: AsRef<str>>(id: S) -> Result<Anchor, StereoKitError> {
        let c_str = CString::new(id.as_ref())
            .map_err(|_| StereoKitError::AnchorFind(id.as_ref().into(), "CString conversion error".to_string()))?;
        Ok(Anchor(
            NonNull::new(unsafe { anchor_find(c_str.as_ptr()) })
                .ok_or(StereoKitError::AnchorFind(id.as_ref().into(), "anchor_find failed".to_string()))?,
        ))
    }

    /// This creates a new Anchor from a world space pose.
    /// <https://stereokit.net/Pages/StereoKit/Anchor/FromPose.html>
    ///
    /// see also [`crate::anchor::anchor_create`]
    pub fn from_pose(pose: impl Into<Pose>) -> Anchor {
        Anchor(NonNull::new(unsafe { anchor_create(pose.into()) }).unwrap())
    }

    /// Gets or sets the unique identifier of this asset resource! This can be helpful for debugging,
    /// managing your assets, or finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/Anchor/Id.html>
    ///
    /// see also [`crate::anchor::anchor_set_id`]
    pub fn id<S: AsRef<str>>(&mut self, id: S) -> &mut Self {
        let c_str = CString::new(id.as_ref()).unwrap();
        unsafe { anchor_set_id(self.0.as_ptr(), c_str.as_ptr()) };
        self
    }

    /// Clear the Store
    /// <https://stereokit.net/Pages/StereoKit/Anchor/ClearStore.html>
    ///
    /// see also [`crate::anchor::anchor_clear_stored`]
    pub fn clear_store() {
        unsafe { anchor_clear_stored() };
    }

    /// Get an iterator of all Anchors that exist in StereoKit at the current moment.
    /// <https://stereokit.net/Pages/StereoKit/Anchor/Anchors.html>
    ///
    /// see also [`crate::anchor::anchor_get_count`] [`crate::anchor::anchor_get_index`]
    pub fn anchors() -> AnchorIter {
        AnchorIter::anchors()
    }

    /// Get an iterator of all Anchors that are new to StereoKit this frame.
    /// <https://stereokit.net/Pages/StereoKit/Anchor/Anchors.html>
    ///
    /// see also [`crate::anchor::anchor_get_new_count`] [`crate::anchor::anchor_get_new_index`]
    pub fn new_anchors() -> AnchorIter {
        AnchorIter::new_anchors()
    }

    /// <https://stereokit.net/Pages/StereoKit/Anchor/Persistent.html>
    ///
    /// see also [`crate::anchor::anchor_get_capabilities`]
    pub fn get_capabilities() -> AnchorCaps {
        unsafe { anchor_get_capabilities() }
    }

    /// The id of this font
    /// <https://stereokit.net/Pages/StereoKit/Anchor/Id.html>
    ///
    /// see also [`crate::anchor::anchor_get_id`]
    pub fn get_id(&self) -> &str {
        unsafe { CStr::from_ptr(anchor_get_id(self.0.as_ptr())) }.to_str().unwrap()
    }

    /// <https://stereokit.net/Pages/StereoKit/Anchor/Pose.html>
    ///
    /// see also [`crate::anchor::anchor_get_pose`]
    pub fn get_pose(&self) -> Pose {
        unsafe { anchor_get_pose(self.0.as_ptr()) }
    }

    /// <https://stereokit.net/Pages/StereoKit/Anchor/Tracked.html>
    ///
    /// see also [`crate::anchor::anchor_get_tracked`]
    pub fn get_tracked(&self) -> BtnState {
        unsafe { anchor_get_tracked(self.0.as_ptr()) }
    }

    /// <https://stereokit.net/Pages/StereoKit/Anchor/Persistent.html>
    ///
    /// see also [`crate::anchor::anchor_get_persistent`]
    pub fn get_persistent(&self) -> bool {
        unsafe { anchor_get_persistent(self.0.as_ptr()) != 0 }
    }

    /// <https://stereokit.net/Pages/StereoKit/Anchor/Name.html>
    ///
    /// see also [`crate::anchor::anchor_get_name`]
    pub fn get_name(&self) -> &str {
        unsafe { CStr::from_ptr(anchor_get_name(self.0.as_ptr())).to_str().unwrap() }
    }

    /// Tries to get the underlying perception spatial anchor
    /// for platforms using Microsoft spatial anchors.
    /// <https://stereokit.net/Pages/StereoKit/Anchor/TryGetPerceptionAnchor.html>
    /// * "<T>" - The type of the spatial anchor. Must corresponds to the the Windows API type
    /// of Windows.Perception.Spatial.SpatialAnchor.
    /// * spatial_anchor - The spatial anchor.
    /// returns Some(anchor) if the perception spatial anchor was successfully obtained, false otherwise.
    ///
    /// see also [`crate::anchor::anchor_get_name`]
    pub fn try_get_perception_anchor<T>(&self) -> Option<*mut T> {
        let out_anchor: *mut T = null_mut();
        if unsafe { anchor_get_perception_anchor(self.0.as_ptr(), out_anchor as *mut *mut c_void) } != 0 {
            Some(out_anchor)
        } else {
            None
        }
    }
}

pub struct AnchorIter {
    index: i32,
    only_new: bool,
}

impl Iterator for AnchorIter {
    type Item = Anchor;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        if self.only_new {
            let count = unsafe { anchor_get_new_count() };
            if self.index < count {
                match NonNull::new(unsafe { anchor_get_new_index(self.index) }) {
                    None => {
                        Log::err(format!(
                            "new anchor at index {:?}, is missing when {:?} new anchors are expected",
                            self.index, count,
                        ));
                        None
                    }
                    Some(anchor) => Some(Anchor(anchor)),
                }
            } else {
                None
            }
        } else {
            let count = unsafe { anchor_get_count() };
            if self.index < count {
                match NonNull::new(unsafe { anchor_get_index(self.index) }) {
                    None => {
                        Log::err(format!(
                            "anchor at index {:?}, is missing when {:?} anchors are expected",
                            self.index, count,
                        ));
                        None
                    }
                    Some(anchor) => Some(Anchor(anchor)),
                }
            } else {
                None
            }
        }
    }
}

impl AnchorIter {
    /// Get an iterator of all Anchors that exist in StereoKit at the current moment.
    /// <https://stereokit.net/Pages/StereoKit/Anchor/Anchors.html>
    ///
    /// see also [`crate::anchor::anchor_get_count`] [`crate::anchor::anchor_get_index`]
    pub fn anchors() -> AnchorIter {
        AnchorIter { index: -1, only_new: false }
    }

    /// Get an iterator of all Anchors that are new to StereoKit this frame.
    /// <https://stereokit.net/Pages/StereoKit/Anchor/Anchors.html>
    ///
    /// see also [`crate::anchor::anchor_get_new_count`] [`crate::anchor::anchor_get_new_index`]
    pub fn new_anchors() -> AnchorIter {
        AnchorIter { index: -1, only_new: true }
    }
}
