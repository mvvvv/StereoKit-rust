use std::fmt;

use crate::maths::Bool32T;

#[cfg(not(feature = "no-event-loop"))]
use crate::{
    framework::Appearence,
    maths::{Pose, Vec2, Vec3},
    prelude::*,
    sprite::Sprite,
    system::{Align, TextFit},
    tools::os_api::{SystemAction, app_package_name, system_deep_link},
    ui::{Ui, UiPad},
    util::{Color128, named_colors},
};

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
    /// For detecting walls, floors, tables and other surfaces in the user's space, via
    /// [`SpatialCapability::PlaneTracking`](crate::spatial::SpatialCapability::PlaneTracking). This is typically an
    /// interactive permission that the user will need to explicitly approve.
    /// This maps to android.permission.SCENE_UNDERSTANDING_COARSE on Android XR, but varies per-runtime.
    PlaneTracking = 9,
    /// For detecting QR codes, ArUco markers and AprilTags in the user's space, via the spatial marker capabilities.
    /// This is typically an interactive permission that the user will need to explicitly approve.
    /// This maps to android.permission.SCENE_UNDERSTANDING_COARSE on Android XR, but varies per-runtime.
    MarkerTracking = 10,
    /// This enum is for tracking the number of value in this enum.
    Max = 11,
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
            PermissionType::PlaneTracking => write!(f, "Plane Tracking"),
            PermissionType::MarkerTracking => write!(f, "Marker Tracking"),
            PermissionType::Max => write!(f, "Max"),
        }
    }
}

impl PermissionType {
    /// Every permission StereoKit knows about, in declaration order. [`PermissionType::Max`] is only the count of the
    /// values of the enum, it is not part of this list.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::permission::PermissionType;
    ///
    /// assert_eq!(PermissionType::ALL.len(), PermissionType::Max as usize);
    /// for permission in PermissionType::ALL {
    ///     println!("{permission}");
    /// }
    /// # sk::Sk::shutdown();
    /// ```
    pub const ALL: [PermissionType; 11] = [
        PermissionType::Microphone,
        PermissionType::Camera,
        PermissionType::EyeInput,
        PermissionType::HandTracking,
        PermissionType::FaceTracking,
        PermissionType::AmbientEstimation,
        PermissionType::ReflectionEstimation,
        PermissionType::DepthSensing,
        PermissionType::Anchors,
        PermissionType::PlaneTracking,
        PermissionType::MarkerTracking,
    ];
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

/// Event that shows (`"true"`) or hides (`"false"`) the [`PermissionPanel`] window. Send it with
/// [`StepperAction::event`] to the id the panel has been added under.
#[cfg(not(feature = "no-event-loop"))]
pub const SHOW_PERMISSION_PANEL: &str = "Permission_Panel_show";

/// A window listing every permission StereoKit knows about on the current runtime with its [`PermissionState`], and a
/// *Request* button for each permission that can still be granted ([`PermissionState::Capable`]). A request may bring
/// up a system dialog that the user answers on their own time: the OS settles the state asynchronously, so the panel
/// only reads [`Permission::get_state`] every frame and follows the answer by itself.
///
/// It is meant to be dropped in **any** application: add it as a stepper
/// ([`StepperAction::add_default`](crate::framework::StepperAction::add_default)), it draws its own window and that's
/// it. When only one or two permissions matter, call [`Permission::get_state`] and [`Permission::request`] directly
/// instead, see the examples of [`Permission`].
///
/// ### Fields that can be changed before initialization:
/// * `enabled` - Whether the window is shown at start. Default is `true`.
/// * `window_pose` - Where the window stands. Default is [`Ui::popup_pose`] of `Vec3::ZERO`, in front of the user.
/// * `appearence` - The look of the window: size, scaling and text styles, see [`Appearence`]. The default
///   `window_size` is `0.5 x 0.0` meters, the height being left to the number of rows, and the ui scale is 60%
///   ([`Appearence::set_ui_scale`]) so the whole list stays compact. The window is resizable down to the
///   [`Appearence::min_window_size`] floor.
/// * `placement` - Where the window is memorized between two sessions, see [`Appearence::placement`]. Left as it is,
///   the window is memorized under the id it is added under; nothing is memorized until the app installs a sink (see
///   [`crate::framework::set_sink`]).
/// * `title` - The title of the window. Default is `"Permissions"`.
/// * `description` - The explanatory text drawn under the title, with [`Appearence::label_style`]. Default is a short
///   reminder of what the list shows.
/// * `show_description` - Draw `description` when `true`. Default is `true`.
/// * `show_close` - Show the close button ([`Sprite::close`]) in the top right corner of the window, which removes
///   this stepper. Default is `true`.
/// * `show_request_all` - Show the *Request All Capable* button that chains every capable permission into a single OS
///   prompt. Default is `true`.
/// * `show_app_settings` - Show the *App Settings* button, which opens the system settings page of the app where a
///   permission refused for good ([`PermissionState::Blocked`]) can be turned back on. Android only, ignored
///   elsewhere. Default is `true`.
/// * `tint_granted` / `tint_pending` / `tint_refused` - The tints of the state column, chosen by
///   [`PermissionPanel::state_tint`]: granted permissions, permissions that can still be asked for, and permissions
///   the app won't get at runtime. Default are [`named_colors::LIGHT_GREEN`], [`named_colors::YELLOW`] and
///   [`named_colors::RED`].
///
/// ### Events this stepper is listening to:
/// * `SHOW_PERMISSION_PANEL` - Event that shows the window ("true") or hides it ("false"), the panel keeps running
///   either way.
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{
///     maths::{Pose, Quat, Vec3},
///     permission::PermissionPanel,
/// };
///
/// let mut permission_panel = PermissionPanel::default();
/// permission_panel.title = "My app permissions".into();
/// permission_panel.window_pose = Pose::new([0.0, 0.25, -0.4], Some(Quat::look_dir(Vec3::Z)));
///
/// sk.send_event(StepperAction::add("Permission_Panel", permission_panel));
///
/// (width_scr, height_scr, fov_scr) = (600, 600, 26.0);
/// filename_scr = "screenshots/permission_panel.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
/// );
/// # sk::Sk::shutdown();
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/permission_panel.jpeg" alt="screenshot" width="200">
#[cfg(not(feature = "no-event-loop"))]
#[derive(IStepper)]
pub struct PermissionPanel {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    enabled: bool,

    pub window_pose: Pose,
    pub appearence: Appearence,

    pub title: String,
    pub description: String,
    pub show_description: bool,
    pub show_close: bool,
    pub show_request_all: bool,
    pub show_app_settings: bool,

    pub tint_granted: Color128,
    pub tint_pending: Color128,
    pub tint_refused: Color128,

    /// The face of the close button, see [`PermissionPanel::show_close`].
    close_icon: Sprite,
}

#[cfg(not(feature = "no-event-loop"))]
unsafe impl Send for PermissionPanel {}

#[cfg(not(feature = "no-event-loop"))]
impl Default for PermissionPanel {
    fn default() -> Self {
        let mut appearence = Appearence::default();
        // As wide as a small panel, and as tall as its rows: the height is left to the content.
        appearence.window_size = Vec2::new(0.5, 0.0);
        // A compact list: 60% of the regular ui scale, the scale handle can still change it afterwards.
        appearence.set_ui_scale(0.6);

        Self {
            id: "PermissionPanel".to_string(),
            sk_info: None,
            enabled: true,

            window_pose: Ui::popup_pose(Vec3::ZERO),
            appearence,

            title: "Permissions".to_string(),
            description: "Permissions of this app on this runtime, and their current state.".to_string(),
            show_description: true,
            show_close: true,
            show_request_all: true,
            show_app_settings: true,

            tint_granted: named_colors::LIGHT_GREEN.into(),
            tint_pending: named_colors::YELLOW.into(),
            tint_refused: named_colors::RED.into(),

            close_icon: Sprite::close(),
        }
    }
}

#[cfg(not(feature = "no-event-loop"))]
impl PermissionPanel {
    /// Builds a panel with its own `title`, every other property as in [`PermissionPanel::default`].
    /// * `title` - The title of the window, also drawn as its header by [`Ui::window`].
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::permission::PermissionPanel;
    ///
    /// let permission_panel = PermissionPanel::new("My app permissions");
    /// assert_eq!(permission_panel.title, "My app permissions");
    /// # sk::Sk::shutdown();
    /// ```
    pub fn new(title: impl Into<String>) -> Self {
        Self { title: title.into(), ..Default::default() }
    }

    /// The tint of the state column for a given state, see [`PermissionPanel::tint_granted`],
    /// [`PermissionPanel::tint_pending`] and [`PermissionPanel::tint_refused`].
    /// * `state` - The current state of a permission, as given by [`Permission::get_state`].
    pub fn state_tint(&self, state: PermissionState) -> Color128 {
        match state {
            PermissionState::Granted => self.tint_granted,
            PermissionState::Capable | PermissionState::Requesting | PermissionState::Unknown => self.tint_pending,
            PermissionState::Denied | PermissionState::Blocked | PermissionState::Unavailable => self.tint_refused,
        }
    }

    /// Removes this stepper, what the close button does.
    pub fn close(&self) {
        SkInfo::send_event(&self.sk_info, StepperAction::remove(self.id.clone()));
    }

    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        // Captures the current (possibly user-tweaked) text heights as the base the ui scale multiplies,
        // and applies the current scale, see `Appearence::start`.
        self.appearence.start();
        // The id this window is added under is the key it is memorized under (see `Appearence::placement`), and the
        // placement of the previous session is applied now that the text styles have their base heights.
        #[cfg(feature = "placement")]
        {
            self.appearence.placement.ensure_key(&self.id);
            self.appearence.restore_placement(&mut self.window_pose);
        }
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, key: &str, value: &str) {
        if key.eq(SHOW_PERMISSION_PANEL) {
            self.enabled = value.parse().unwrap_or(false);
        }
    }

    /// Called from IStepper::step, after check_event here you can draw your UI
    fn draw(&mut self, _token: &MainThreadToken) {
        // The window is drawn with the Appearence-scaled UiSettings, restored exactly as they were afterwards.
        let prev_settings = Ui::get_settings();
        Ui::settings(self.appearence.get_ui_settings_scaled());
        Ui::push_id(&self.id);

        Ui::window(&self.title)
            .pose(&mut self.window_pose)
            .size(self.appearence.scaled_window_size())
            .begin();

        // Close button, alone on its row at the left edge of the window.
        if self.show_close {
            let diameter = Ui::get_line_height();
            if Ui::button_round("permission_panel_close", &self.close_icon, diameter).press() {
                self.close();
            }
        }

        // The width of the content area, in meters: it follows the window when it is resized or zoomed with the scale
        // handle, see `Appearence`.
        let usable_width = Ui::get_layout_remaining().x;

        if self.show_description && !self.description.is_empty() {
            Ui::same_line();
            Ui::hspace(0.8);
            // A `label` squeezes the text into one line, a `text` wraps it at the given width with an automatic height.
            Ui::same_line();
            Ui::push_text_style(self.appearence.title_style);
            Ui::text(&self.description).draw();
            Ui::pop_text_style();
        }

        Ui::hseparator();

        // Column widths, in fractions of the usable width of the window.
        let col_permission = usable_width * 0.45;
        let col_state = usable_width * 0.25;
        let col_interactive = usable_width * 0.30;
        let row_height = Ui::get_line_height();

        // The rows use the list style of the Appearence, like the file browser's list.
        Ui::push_text_style(self.appearence.list_style);
        for permission in PermissionType::ALL {
            let state = Permission::get_state(permission);
            Ui::push_id(format!("permission_{}", permission as u32));

            Ui::panel_begin(Some(UiPad::Outside));
            // Column 1: the permission name, squeezed instead of wrapped so each row stays on one line.
            Ui::text(permission.to_string())
                .size(Vec2::new(col_permission, row_height))
                .text_align(Align::CenterLeft)
                .fit(TextFit::Squeeze)
                .draw();
            // Column 2: the current state, tinted by what it means
            Ui::same_line();
            Ui::push_tint(self.state_tint(state));
            Ui::text(state.to_string())
                .size(Vec2::new(col_state, row_height))
                .text_align(Align::CenterLeft)
                .fit(TextFit::Squeeze)
                .draw();
            Ui::pop_tint();
            // Column 3: will requesting it show a system dialog the user has to answer?
            if Permission::is_interactive(permission) {
                Ui::same_line();
                Ui::text("(interactive)")
                    .size(Vec2::new(col_interactive, row_height))
                    .text_align(Align::CenterLeft)
                    .fit(TextFit::Squeeze)
                    .draw();
            }
            // Column 4: the request button, only while the OS is willing to ask for this permission
            if state == PermissionState::Capable {
                Ui::same_line();
                if Ui::button("Request").size(Vec2::new(0.0, row_height)).press() {
                    Permission::request(&[permission]);
                }
            }
            // Whatever the columns above used, the row spans the whole width of the window.
            Ui::hspace(Ui::get_layout_remaining().x.max(0.0));
            Ui::panel_end();
            Ui::pop_id();
        }
        Ui::pop_text_style();

        // Requesting several at once chains them into a single OS prompt.
        Ui::hseparator();
        Ui::push_text_style(self.appearence.label_style);
        if self.show_request_all && Ui::button("Request All Capable").press() {
            let capable: Vec<PermissionType> = PermissionType::ALL
                .into_iter()
                .filter(|permission| Permission::get_state(*permission) == PermissionState::Capable)
                .collect();
            if capable.is_empty() {
                Log::info("PermissionPanel: no permission is capable, nothing to request");
            } else {
                Permission::request(&capable);
            }
        }
        // A permission refused for good ("Blocked") can only be turned back on from the system settings of the app.
        if cfg!(target_os = "android") && self.show_app_settings && Ui::button("App Settings").press() {
            let result = system_deep_link(SystemAction::Settings {
                setting: app_package_name().map(|package| format!("/applications?package={package}")),
            });
            Log::info(format!("PermissionPanel: open app settings - {result:?}"));
        }
        Ui::pop_text_style();

        Ui::window_end();

        Ui::pop_id();
        Ui::settings(prev_settings);

        // Grab-able knob anchored to the window: dragging it resizes / zooms the window (see `Appearence`), and it
        // memorizes the placement of the window (see `Appearence::placement`).
        self.appearence.scale_handle(&self.window_pose, "permission_panel_scale");
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
        for permission in PermissionType::ALL {
            // Just make sure we can call the functions without crashing
            let _state = Permission::get_state(permission);
            let _interactive = Permission::is_interactive(permission);
        }
    }

    #[test]
    fn test_permission_all() {
        // `PermissionType::ALL` must list every value of the enum, in declaration order, `Max` excluded.
        assert_eq!(PermissionType::ALL.len(), PermissionType::Max as usize);
        for (index, permission) in PermissionType::ALL.into_iter().enumerate() {
            assert_eq!(permission as usize, index);
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
        assert_eq!(PermissionType::PlaneTracking as u32, 9);
        assert_eq!(PermissionType::MarkerTracking as u32, 10);
    }
}
