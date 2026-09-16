use std::fmt;

use crate::maths::Bool32T;

/// A list of permissions that StereoKit knows about, each named for the feature it unlocks. On some platforms
/// (like Android), these permissions may need to be explicitly requested before using certain features. Runtimes
/// group features into system permissions differently, so several of these may resolve to the same underlying system
/// permission.
/// <https://stereokit.net/Pages/StereoKit/PermissionType.html>
///
/// see also: [`Permission`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum PermissionType {
    /// For access to microphone data, this is typically an interactive permission that the user will need to explicitly
    /// approve.
    /// This maps to android.permission.RECORD_AUDIO on Android.
    Microphone = 0,
    /// For access to camera data, this is typically an interactive permission that the user will need to explicitly
    /// approve. SK doesn't use this permission internally yet, but is often a useful permission for XR apps.
    /// This maps to android.permission.CAMERA on Android.
    Camera = 1,
    /// For access to input quality eye tracking data, this is typically an interactive permission that the user will
    /// need to explicitly approve.
    /// This maps to android.permission.EYE_TRACKING_FINE on Android XR, but varies per-runtime.
    EyeInput = 2,
    /// For access to per-joint hand tracking data. Some runtimes may have this permission interactive, but many do not.
    /// This maps to android.permission.HAND_TRACKING on Android XR, but varies per-runtime.
    HandTracking = 3,
    /// For access to facial expression data, this is typically an interactive permission that the user will need to
    /// explicitly approve.
    /// This maps to android.permission.FACE_TRACKING on Android XR, but varies per-runtime.
    FaceTracking = 4,
    /// For estimating ambient lighting from the user's surroundings, this is what the world lighting source feeds into
    /// Lighting.Ambient. This is typically an interactive permission that the user will need to explicitly approve.
    /// This maps to android.permission.SCENE_UNDERSTANDING_COARSE on Android XR, but varies per-runtime.
    AmbientEstimation = 5,
    /// For estimating an environment cubemap from the user's surroundings, this is what the world lighting source
    /// feeds into [`Lighting::reflection`](crate::lighting::Lighting::reflection). The estimate shows imagery of the
    /// user's space, so runtimes may treat it more strictly than ambient estimation. This is typically an interactive
    /// permission that the user will need to explicitly approve. This maps to
    /// android.permission.SCENE_UNDERSTANDING_FINE on Android XR, but varies per-runtime.
    ReflectionEstimation = 6,
    /// For reading depth data about the user's surroundings via Sensor.Depth, useful for things like occlusion. This
    /// is typically an interactive permission that the user will need to explicitly approve. This maps to
    /// android.permission.SCENE_UNDERSTANDING_FINE on Android XR, but varies per-runtime.
    DepthSensing = 7,
    /// For creating and persisting spatial anchors in the user's space, via StereoKit's Anchor API. Some runtimes
    /// grant this automatically from the manifest entry, while others treat it as an interactive permission.
    /// This maps to android.permission.SCENE_UNDERSTANDING_COARSE on Android XR and
    /// com.oculus.permission.USE_ANCHOR_API on Meta, but varies per-runtime.
    Anchors = 8,
    /// This enum is for tracking the number of value in this enum.
    Max = 9,
}

impl fmt::Display for PermissionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionType::Microphone => write!(f, "Microphone"),
            PermissionType::Camera => write!(f, "Camera"),
            PermissionType::EyeInput => write!(f, "Eye Input"),
            PermissionType::HandTracking => write!(f, "Hand Tracking"),
            PermissionType::FaceTracking => write!(f, "Face Tracking"),
            PermissionType::AmbientEstimation => write!(f, "Ambient Estimation"),
            PermissionType::ReflectionEstimation => write!(f, "Reflection Estimation"),
            PermissionType::DepthSensing => write!(f, "Depth Sensing"),
            PermissionType::Anchors => write!(f, "Anchors"),
            PermissionType::Max => write!(f, "Max"),
        }
    }
}

/// Permissions can be in a variety of states, depending on how users interact with them. Sometimes they're
/// automatically granted, user denied, or just unknown for the current runtime! A positive value means you're clear to
/// use the feature, zero or negative means you're not.
/// <https://stereokit.net/Pages/StereoKit/PermissionState.html>
///
/// see also: [`Permission`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum PermissionState {
    /// StereoKit knows this permission, but nothing you do at runtime can get it granted. Usually the permission
    /// string is missing from the AndroidManifest.xml or equivalent, so check there first. Some runtimes also report
    /// this when an administrator or parental control has locked the feature off, which no amount of asking will
    /// change. Fix your app's manifest, or work without the feature.
    Unavailable = -5,
    /// The permission was refused, and the system will not prompt for it again. Requesting it is legal, but nothing
    /// will happen. Only the user can undo this, from the system's settings. Work without the feature, and if it
    /// matters, tell the user where to turn it back on.
    Blocked = -4,
    /// The permission was refused, but the system is still willing to prompt for it. Not every platform has this
    /// state; where the first refusal is final, you'll get blocked instead. You can request it again, ideally at
    /// a moment where the user understands why you need it.
    Denied = -3,
    /// A permission request is in flight: a dialog may be up, or StereoKit is waiting to see if the system will answer
    /// one. This settles on its own once the system answers, or the user dismisses the dialog. Wait, and check back
    /// later.
    Requesting = -2,
    /// This app can use the permission, but hasn't been granted it yet. Ask for it with [`Permission::request`].
    Capable = -1,
    /// StereoKit doesn't know about the permission on the current runtime. This happens when the runtime has a unique
    /// permission string (or not) and StereoKit doesn't know what it is to look up its current status. There's no
    /// reliable action here, try the feature and see if it works.
    Unknown = 0,
    /// This permission is entirely approved and you can go ahead and use the associated features!
    Granted = 1,
}

impl fmt::Display for PermissionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionState::Unavailable => write!(f, "Unavailable"),
            PermissionState::Blocked => write!(f, "Blocked"),
            PermissionState::Denied => write!(f, "Denied"),
            PermissionState::Requesting => write!(f, "Requesting"),
            PermissionState::Capable => write!(f, "Capable"),
            PermissionState::Unknown => write!(f, "Unknown"),
            PermissionState::Granted => write!(f, "Granted"),
        }
    }
}

unsafe extern "C" {
    pub fn permission_state(permission: PermissionType) -> PermissionState;
    pub fn permission_is_interactive(permission: PermissionType) -> Bool32T;
    pub fn permission_request(in_arr_permissions: *const PermissionType, permission_count: i32);
}

/// Certain features in XR require explicit permissions from the operating system and user! This is typically for
/// features that surface sensitive data like eye gaze, or objects in the user's room. This is often complicated by the
/// fact that permissions aren't standardized across XR runtimes, making these permissions fragile and a pain to work
/// with.
///
/// This class attempts to manage feature permissions in a nice cross-platform manner that handles runtime specific
/// differences. You will still need to add permission strings to your app's metadata file (like AndroidManifest.xml),
/// but this class will handle figuring out which strings in the metadata to actually use.
///
/// On Android, if you use a Service or Context instead of an Activity for your app (unusual), StereoKit will not be
/// able to manage permissions for you!
///
/// On platforms that don't use permissions, like Win32 or Linux, these functions will behave as though everything is
/// granted automatically.
/// <https://stereokit.net/Pages/StereoKit/Permission.html>
pub struct Permission;

impl Permission {
    /// Retreives the current state of a particular permission. This is a fast check, so it's fine to call frequently.
    /// <https://stereokit.net/Pages/StereoKit/Permission/GetState.html>
    /// * `permission` - The permission you're interested in.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::permission::{Permission, PermissionType, PermissionState};
    ///
    /// let microphone_state = Permission::get_state(PermissionType::Microphone);
    /// match microphone_state {
    ///     PermissionState::Granted => println!("Microphone access granted"),
    ///     PermissionState::Capable => println!("Microphone access needs to be requested"),
    ///     PermissionState::Requesting => println!("Microphone access is being requested"),
    ///     PermissionState::Denied => println!("Microphone access denied"),
    ///     PermissionState::Blocked => println!("Microphone access blocked"),
    ///     PermissionState::Unavailable => println!("Microphone access unavailable"),
    ///     PermissionState::Unknown => println!("Microphone permission state unknown"),
    /// }
    ///
    /// assert_eq!(microphone_state, PermissionState::Granted); // On desktop, this is typically granted automatically
    /// # sk::Sk::shutdown();
    /// ```
    pub fn get_state(permission: PermissionType) -> PermissionState {
        unsafe { permission_state(permission) }
    }

    /// Might requesting this permission interrupt the user with a popup? This is a prediction from how sensitive the
    /// platform considers the permission, not a promise. The system can still grant one silently, most often when the
    /// user already approved a related permission earlier in the session, so a true here means "may interrupt" rather
    /// than "will".
    ///
    /// There's no way to know for certain in advance, since the answer depends on what the user has already agreed to.
    /// <https://stereokit.net/Pages/StereoKit/Permission/IsInteractive.html>
    /// * `permission` - The permission you're interested in.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::permission::{Permission, PermissionType};
    ///
    /// if Permission::is_interactive(PermissionType::Microphone) {
    ///     println!("Microphone permission requires user approval");
    ///     panic!("On desktop, Microphone permission is automatic");
    /// } else {
    ///     println!("Microphone permission is automatic");
    /// }
    /// # sk::Sk::shutdown();
    /// ```
    pub fn is_interactive(permission: PermissionType) -> bool {
        unsafe { permission_is_interactive(permission) != 0 }
    }

    /// This sends off a request to the OS for one or more permissions! If a permission IsInteractive, then this will
    /// bring up a popup that the user may need to interact with. Otherwise, this will silently approve the permission.
    /// This means that the permission may take an arbitrary amount of time before it's approved, or declined.
    ///
    /// Requesting multiple permissions in a single call is preferable to chaining individual requests yourself, since
    /// the OS gets to present them together and you avoid the risk of a follow-up request getting dropped while an
    /// earlier popup is still up.
    ///
    /// Any permissions that aren't known on the current platform are skipped with a warning. If your app is an Android
    /// Service, this function will do nothing.
    /// <https://stereokit.net/Pages/StereoKit/Permission/Request.html>
    /// * `permissions` - The permission(s) to request.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::permission::{Permission, PermissionType, PermissionState};
    ///
    /// // Check if we need to request microphone permission
    ///
    /// if Permission::get_state(PermissionType::Microphone) == PermissionState::Capable {
    ///     if Permission::is_interactive(PermissionType::Microphone) {
    ///         println!("Microphone permission requires user approval. We can't request it in an unit test");
    ///     } else {
    ///         println!("Requesting microphone permission...");
    ///         Permission::request(&[PermissionType::Microphone]);
    ///     }
    /// }
    ///
    /// // Check for eye tracking permission
    /// if Permission::get_state(PermissionType::EyeInput) == PermissionState::Capable {
    ///     if Permission::is_interactive(PermissionType::EyeInput) {
    ///         println!("Eye Input permission requires user approval. We can't request it in an unit test");
    ///     } else {
    ///         println!("Requesting Eye Input permission...");
    ///         Permission::request(&[PermissionType::EyeInput]);
    ///     }
    /// }
    /// # sk::Sk::shutdown();
    /// ```
    pub fn request(permissions: &[PermissionType]) {
        unsafe { permission_request(permissions.as_ptr(), permissions.len() as i32) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_state() {
        // For unit tests, we'll just test the enum values and basic functionality
        // without initializing StereoKit since the macro doesn't work from inside the crate

        // Test that we can iterate through permission types
        for i in 0..(PermissionType::Max as u32) {
            let permission = unsafe { std::mem::transmute::<u32, PermissionType>(i) };
            // Just make sure we can call the functions without crashing
            let _state = Permission::get_state(permission);
            let _interactive = Permission::is_interactive(permission);
        }
    }

    #[test]
    fn test_permission_display() {
        // Test that display formatting works
        assert_eq!(format!("{}", PermissionType::Microphone), "Microphone");
        assert_eq!(format!("{}", PermissionType::Camera), "Camera");
        assert_eq!(format!("{}", PermissionState::Granted), "Granted");
        assert_eq!(format!("{}", PermissionState::Capable), "Capable");
    }

    #[test]
    fn test_permission_enum_values() {
        // Test that enum values match the expected constants
        assert_eq!(PermissionType::Microphone as u32, 0);
        assert_eq!(PermissionType::Camera as u32, 1);
        assert_eq!(PermissionType::EyeInput as u32, 2);
        assert_eq!(PermissionType::HandTracking as u32, 3);
        assert_eq!(PermissionType::FaceTracking as u32, 4);
        assert_eq!(PermissionType::AmbientEstimation as u32, 5);
        assert_eq!(PermissionType::ReflectionEstimation as u32, 6);
        assert_eq!(PermissionType::DepthSensing as u32, 7);
        assert_eq!(PermissionType::Anchors as u32, 8);
    }
}
