use std::fmt;

use crate::maths::Bool32T;

/// A list of permissions that StereoKit knows about. On some platforms (like Android), these permissions may need to be
/// explicitly requested before using certain features.
/// <https://stereokit.net/Pages/StereoKit/PermissionType.html>
///
/// see also: [`Permission`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum PermissionType {
    /// For access to microphone data, this is typically an interactive permission that the user will need to explicitly
    /// approve.
    Microphone = 0,
    /// For access to camera data, this is typically an interactive permission that the user will need to explicitly
    /// approve. SK doesn't use this permission internally yet, but is often a useful permission for XR apps.
    Camera = 1,
    /// For access to input quality eye tracking data, this is typically an interactive permission that the user will
    /// need to explicitly approve.
    EyeInput = 2,
    /// For access to per-joint hand tracking data. Some runtimes may have this permission interactive, but many do not.
    HandTracking = 3,
    /// For access to facial expression data, this is typically an interactive permission that the user will need to
    /// explicitly approve.
    FaceTracking = 4,
    /// For access to data in the user's space, this can be for things like spatial anchors, plane detection, hit
    /// testing, etc. This is typically an interactive permission that the user will need to explicitly approve.
    Scene = 5,
    /// This enum is for tracking the number of values in this enum.
    Max = 6,
}

impl fmt::Display for PermissionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionType::Microphone => write!(f, "Microphone"),
            PermissionType::Camera => write!(f, "Camera"),
            PermissionType::EyeInput => write!(f, "Eye Input"),
            PermissionType::HandTracking => write!(f, "Hand Tracking"),
            PermissionType::FaceTracking => write!(f, "Face Tracking"),
            PermissionType::Scene => write!(f, "Scene"),
            PermissionType::Max => write!(f, "Max"),
        }
    }
}

/// Permissions can be in a variety of states, depending on how users interact with them. Sometimes they're
/// automatically granted, user denied, or just unknown for the current runtime!
/// <https://stereokit.net/Pages/StereoKit/PermissionState.html>
///
/// see also: [`Permission`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(i32)]
pub enum PermissionState {
    /// This permission is known to StereoKit, but not available to request. Typically this means the correct permission
    /// string is not listed in the AndroidManifest.xml or similar.
    Unavailable = -2,
    /// This app is capable of using the permission, but it needs to be requested first with [`Permission::request`].
    Capable = -1,
    /// StereoKit doesn't know about the permission on the current runtime. This happens when the runtime has a unique
    /// permission string (or not) and StereoKit doesn't know what it is to look up its current status.
    Unknown = 0,
    /// This permission is entirely approved and you can go ahead and use the associated features!
    Granted = 1,
}

impl fmt::Display for PermissionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionState::Unavailable => write!(f, "Unavailable"),
            PermissionState::Capable => write!(f, "Capable"),
            PermissionState::Unknown => write!(f, "Unknown"),
            PermissionState::Granted => write!(f, "Granted"),
        }
    }
}

#[link(name = "StereoKitC")]
unsafe extern "C" {
    pub fn permission_state(permission: PermissionType) -> PermissionState;
    pub fn permission_is_interactive(permission: PermissionType) -> Bool32T;
    pub fn permission_request(permission: PermissionType);
}

/// Certain features in XR require explicit permissions from the operating system and user! This is typically for
/// feature that surface sensitive data like eye gaze, or objects in the user's room. This is often complicated by the
/// fact that permissions aren't standardized across XR runtimes, making these permissions fragile and a pain to work
/// with.
///
/// This class attempts to manage feature permissions in a nice cross-platform manner that handles runtime specific
/// differences. You will still need to add permission strings to your app's metadata file (like AndroidManifest.xml),
/// but this class will handle figuring out which strings in the metadata to actually use.
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
    ///     PermissionState::Unavailable => println!("Microphone access unavailable"),
    ///     PermissionState::Unknown => println!("Microphone permission state unknown"),
    /// }
    ///
    /// assert_eq!(microphone_state, PermissionState::Granted); // On desktop, this is typically granted automatically
    /// ```
    pub fn get_state(permission: PermissionType) -> PermissionState {
        unsafe { permission_state(permission) }
    }

    /// Does this permission need the user to approve it? This typically means a popup window will come up when you
    /// request this permission, and the user has a chance to decline it.
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
    /// ```
    pub fn is_interactive(permission: PermissionType) -> bool {
        unsafe { permission_is_interactive(permission) != 0 }
    }

    /// This sends off a request to the OS for a particular permission! If the permission IsInteractive, then this will
    /// bring up a popup that the user may need to interact with. Otherwise, this will silently approve the permission.
    /// This means that the permission may take an arbitrary amount of time before it's approved, or declined.
    /// <https://stereokit.net/Pages/StereoKit/Permission/Request.html>
    /// * `permission` - The permission to request.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::permission::{Permission, PermissionType, PermissionState};
    ///
    /// // Check if we need to request microphone permission
    /// if Permission::get_state(PermissionType::Microphone) == PermissionState::Capable {
    ///     println!("Requesting microphone permission...");
    ///     Permission::request(PermissionType::Microphone);
    ///     panic!("On desktop, Microphone permission is automatic");
    /// }
    ///
    /// // Check for eye tracking permission
    /// if Permission::get_state(PermissionType::EyeInput) == PermissionState::Capable {
    ///     Permission::request(PermissionType::EyeInput);
    ///     panic!("On desktop, EyeInput permission is automatic");
    /// }
    /// ```
    pub fn request(permission: PermissionType) {
        unsafe { permission_request(permission) }
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
        assert_eq!(PermissionType::Scene as u32, 5);
    }
}
