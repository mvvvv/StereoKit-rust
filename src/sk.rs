use crate::{
    StereoKitError,
    maths::Bool32T,
    system::{Log, LogLevel},
    tools::os_api::get_assets_dir,
};
#[cfg(target_os = "android")]
#[cfg(feature = "no-event-loop")]
use android_activity::{AndroidApp, MainEvent, PollEvent};
use std::{
    cell::RefCell,
    ffi::{CStr, CString, c_char, c_void},
    fmt::{self, Formatter},
    path::Path,
    ptr::null_mut,
    rc::Rc,
};
#[cfg(target_os = "android")]
#[cfg(feature = "event-loop")]
use winit::platform::android::{
    EventLoopBuilderExtAndroid,
    activity::{AndroidApp, MainEvent, PollEvent},
};

#[cfg(feature = "event-loop")]
use crate::framework::{StepperAction, Steppers};
#[cfg(feature = "event-loop")]
use std::collections::VecDeque;
#[cfg(feature = "event-loop")]
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
};

/// Specifies a type of display mode StereoKit uses, like Mixed Reality headset display vs. a PC display, or even just
/// rendering to an offscreen surface, or not rendering at all!
/// <https://stereokit.net/Pages/StereoKit/DisplayMode.html>
///
/// see also: [`Sk::get_active_display_mode`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DisplayMode {
    /// Creates an OpenXR instance, and drives display/input through that.
    MixedReality = 0,
    /// Creates a flat, Win32 window, and simulates some MR functionality. Great for debugging.
    Flatscreen = 1,
    /// Not tested yet, but this is meant to run StereoKit without rendering to any display at all. This would allow
    /// for rendering to textures, running a server that can do MR related tasks, etc.
    None = 2,
}

/// Which operation mode should we use for this app? Default is XR, and by default the app will fall back to Simulator
/// if XR fails or is unavailable.
/// <https://stereokit.net/Pages/StereoKit/AppMode.html>
///
/// see also: [`SkSettings::mode`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum AppMode {
    /// No mode has been specified, default behavior will be used. StereoKit will pick XR in this case.
    None = 0,
    /// Creates an OpenXR or WebXR instance, and drives display/input through that.
    XR = 1,
    /// Creates a flat window, and simulates some XR functionality. Great for development and debugging.
    Simulator = 2,
    /// Creates a flat window and displays to that, but doesn't simulate XR at all. You will need to control your own
    /// camera here. This can be useful if using StereoKit for non-XR 3D applications.
    Window = 3,
    /// No display at all! StereoKit won't even render to a texture unless requested to. This may be good for running
    /// tests on a server, or doing graphics related tool or CLI work.
    Offscreen = 4,
}

/// This is used to determine what kind of depth buffer StereoKit uses!
/// <https://stereokit.net/Pages/StereoKit/DepthMode.html>
///
/// see also: [`SkSettings::depth_mode`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DepthMode {
    /// Default mode, uses 16 bit on mobile devices like HoloLens and Quest, and 32 bit on higher powered platforms
    /// like PC. If you need a far view distance even on mobile devices, prefer D32 or Stencil instead.
    Balanced = 0,
    /// 16 bit depth buffer, this is fast and recommended for devices like the HoloLens. This is especially important
    /// for fast depth based reprojection. Far view distances will suffer here though, so keep your clipping far plane
    /// as close as possible.
    D16 = 1,
    /// 32 bit depth buffer, should look great at any distance! If you must have the best, then this is the best. If
    /// you’re interested in this one, Stencil may also be plenty for you, as 24 bit depth is also pretty peachy.
    D32 = 2,
    /// 24 bit depth buffer with 8 bits of stencil data. 24 bits is generally plenty for a depth buffer, so using the
    /// rest for stencil can open up some nice options! StereoKit has limited stencil support right now though (v0.3).
    Stencil = 3,
}

/// This describes the way the display’s content blends with whatever is behind it. VR headsets are normally Opaque,
/// but some VR headsets provide passthrough video, and can support Opaque as well as Blend, like the Varjo.
/// Transparent AR displays like the HoloLens would be Additive.
/// <https://stereokit.net/Pages/StereoKit/DisplayBlend.html>
///
/// see also [`SkSettings::blend_preference`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum DisplayBlend {
    /// Default value, when using this as a search type, it will fall back to default behavior which defers to platform
    /// preference.
    None = 0,
    /// This display is opaque, with no view into the real world! This is equivalent to a VR headset, or a PC screen.
    Opaque = 1,
    /// This display is transparent, and adds light on top of the real world. This is equivalent to a HoloLens type of
    /// device.
    Additive = 2,
    /// This is a physically opaque display, but with a camera passthrough displaying the world behind it anyhow. This
    /// would be like a Varjo XR-1, or phone-camera based AR.
    Blend = 4,
    /// This matches either transparent display type! Additive or Blend. For use when you just want to see the world
    /// behind your application.
    AnyTransparent = 6,
}

/// Information about a system’s capabilities and properties!
/// <https://stereokit.net/Pages/StereoKit/SystemInfo.html>
///
/// see also: [`SkInfo::get_system`] [`Sk::get_system`]
/// ### Examples
/// ```
/// use stereokit_rust::sk::{Sk, SkSettings, SystemInfo, AppMode};
/// use stereokit_rust::system::LogLevel;
///
/// let mut settings = SkSettings::default();
/// settings.app_name("Test").mode(AppMode::Offscreen);
/// let sk = Sk::init(&settings).unwrap();
/// let system_info: SystemInfo = sk.get_system();
/// // 0 everywhere:
/// assert_eq!(system_info, SystemInfo::default());
/// Sk::shutdown();
/// ```
#[derive(Default, Debug, Clone, PartialEq)]
#[repr(C)]
pub struct SystemInfo {
    display_width: i32,
    display_height: i32,
    spatial_bridge_present: Bool32T,
    perception_bridge_present: Bool32T,
    eye_tracking_present: Bool32T,
    overlay_app: Bool32T,
    world_occlusion_present: Bool32T,
    world_raycast_present: Bool32T,
}

impl SystemInfo {
    /// Width of the display surface, in pixels! For a stereo display, this will be the width of a single eye.
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/displayWidth.html>
    pub fn get_display_width(&self) -> i32 {
        self.display_width
    }

    /// Height of the display surface, in pixels! For a stereo display, this will be the height of a single eye.
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/displayHeight.html>
    pub fn get_display_height(&self) -> i32 {
        self.display_height
    }

    /// Does the device we’re currently on have the spatial graph bridge extension? The extension is provided through
    /// the function [`crate::system::World::from_spatial_node`]. This allows OpenXR to talk with certain windows APIs, such
    /// as the QR code API that provides Graph Node GUIDs for the pose.
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/spatialBridgePresent.html>
    pub fn get_spatial_bridge_present(&self) -> bool {
        self.spatial_bridge_present != 0
    }

    /// Can the device work with externally provided spatial anchors, like UWP’s Windows.Perception.Spatial.SpatialAnchor
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/perceptionBridgePresent.html>
    pub fn get_perception_bridge_present(&self) -> bool {
        self.perception_bridge_present != 0
    }

    /// Does the device we’re on have eye tracking support present? This is not an indicator that the user has given
    /// the application permission to access this information. See Input.Gaze for how to use this data.
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/eyeTrackingPresent.html>
    pub fn get_eye_tracking_present(&self) -> bool {
        self.eye_tracking_present != 0
    }

    /// This tells if the app was successfully started as an overlay application. If this is true, then expect this
    /// application to be composited with other content below it!
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/overlayApp.html>
    pub fn get_overlay_app(&self) -> bool {
        self.overlay_app != 0
    }

    /// Does this device support world occlusion of digital objects? If this is true, then World.OcclusionEnabled can
    /// be set to true, and [`crate::system::World::occlusion_material`] can be modified.
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/worldOcclusionPresent.html>
    pub fn get_world_occlusion_present(&self) -> bool {
        self.world_occlusion_present != 0
    }

    /// Can this device get ray intersections from the environment? If this is true, then [`crate::system::World::raycast_enabled`]
    /// can be set to true, and [`crate::system::World::raycast`] can be used.
    /// <https://stereokit.net/Pages/StereoKit/SystemInfo/worldRaycastPresent.html>
    pub fn get_world_raycast_present(&self) -> bool {
        self.world_raycast_present != 0
    }
}

/// This describes where the origin of the application should be. While these origins map closely to OpenXR features,
/// not all runtimes support each feature. StereoKit will provide reasonable fallback behavior in the event the origin
/// mode isn’t directly supported.
/// <https://stereokit.net/Pages/StereoKit/OriginMode.html>
///
/// se also [`SkSettings::origin`] [`crate::system::World::get_origin_mode`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum OriginMode {
    /// Default value : The origin will be at the location of the user’s head when the application starts, facing the
    /// same direction as the user. This mode is available on all runtimes, and will never fall back to another mode!
    /// However, due to variances in underlying behavior, StereoKit may introduce an origin offset to ensure consistent
    /// behavior.
    Local = 0,
    /// The origin will be at the floor beneath where the user starts, facing the direction of the user. If this mode is
    /// not natively supported, StereoKit will use the stage mode with an offset. If stage mode is unavailable, it will
    /// fall back to local mode with a -1.5 Y axis offset.
    Floor = 1,
    /// The origin will be at the center of a safe play area or stage that the user or OS has defined, and will face one
    /// of the edges of the play area. If this mode is not natively supported, StereoKit will use the floor origin mode.
    /// If floor mode is unavailable, it will fall back to local mode with a -1.5 Y axis offset.
    Stage = 2,
}

/// This tells about the app’s current focus state, whether it’s active and receiving input, or if it’s backgrounded
/// or hidden. This can be important since apps may still run and render when unfocused, as the app may still be
/// visible behind the app that does have focus.
/// <https://stereokit.net/Pages/StereoKit/AppFocus.html>
///
/// see also [`Sk::get_app_focus`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum AppFocus {
    /// This StereoKit app is active, focused, and receiving input from the user. Application should behave as normal.
    Active = 0,
    /// This StereoKit app has been unfocused, something may be compositing on top of the app such as an OS dashboard.
    /// The app is still visible, but some other thing has focus and is receiving input. You may wish to pause,
    /// disable input tracking, or other such things.
    Background = 1,
    /// This app is not rendering currently.
    Hidden = 2,
}

/// When the device StereoKit is running on goes into standby mode, how should StereoKit react? Typically the app should
/// pause, stop playing sound, and consume as little power as possible, but some scenarios such as multiplayer games may
/// need the app to continue running.
/// <https://stereokit.net/Pages/StereoKit/StandbyMode.html>
///
/// see also [`SkSettings::standby_mode`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum StandbyMode {
    /// This will let StereoKit pick a mode based on its own preferences. On v0.3 and lower, this will be Slow, and on
    /// v0.4 and higher, this will be Pause.
    Default = 0,
    /// The entire main thread will pause, and wait until the device has come out of standby. This is the most power
    /// efficient mode for the device to take when the device is in standby, and is recommended for the vast majority
    /// of apps. This will also disable sound.
    Pause = 1,
    /// The main thread will continue to execute, but with 100ms sleeps each frame. This allows the app to continue
    /// polling and processing, but reduces power consumption by throttling a bit. This will not disable sound. In the
    /// Simulator, this will behave as Slow.
    Slow = 2,
    /// The main thread will continue to execute, but with a very short sleep each frame. This allows the app to
    /// continue polling and processing, but without flooding the CPU with polling work while vsync is no longer the
    /// throttle. This will not disable sound.
    None = 3,
}

unsafe extern "C" {
    pub fn sk_init(settings: SkSettings) -> Bool32T;
    pub fn sk_set_window(window: *mut c_void);
    pub fn sk_set_window_xam(window: *mut c_void);
    pub fn sk_shutdown();
    pub fn sk_shutdown_unsafe();
    pub fn sk_quit(quit_reason: QuitReason);
    pub fn sk_step(app_step: Option<unsafe extern "C" fn()>) -> Bool32T;
    pub fn sk_run(app_step: Option<unsafe extern "C" fn()>, app_shutdown: Option<unsafe extern "C" fn()>);
    pub fn sk_run_data(
        app_step: Option<unsafe extern "C" fn(step_data: *mut c_void)>,
        step_data: *mut c_void,
        app_shutdown: Option<unsafe extern "C" fn(shutdown_data: *mut c_void)>,
        shutdown_data: *mut c_void,
    );
    pub fn sk_is_stepping() -> Bool32T;
    pub fn sk_active_display_mode() -> DisplayMode;
    pub fn sk_get_settings() -> SkSettings;
    pub fn sk_system_info() -> SystemInfo;
    pub fn sk_version_name() -> *const c_char;
    pub fn sk_version_id() -> u64;
    pub fn sk_app_focus() -> AppFocus;
    pub fn sk_get_quit_reason() -> QuitReason;
}

/// Default name of the applications
pub const DEFAULT_NAME: *const c_char = {
    const BYTES: &[u8] = b"StereoKitApp\0";
    BYTES.as_ptr().cast()
};

/// StereoKit initialization settings! Setup SkSettings with your data before calling SkSetting.Init().
/// <https://stereokit.net/Pages/StereoKit/SKSettings.html>
///
/// see also [`Sk::init`] [`Sk::get_settings`] [`SkInfo::settings_from`] [`sk_init`]
/// ### Examples
/// ```
/// use stereokit_rust::sk::{Sk, SkSettings, AppMode, DisplayBlend, DepthMode, OriginMode, StandbyMode};
/// use stereokit_rust::system::LogLevel;
///
/// let mut settings = SkSettings::default();
/// settings.app_name("Test").mode(AppMode::Offscreen);
/// let sk = Sk::init(&settings).expect("StereoKit should initialize");
/// let settings = sk.get_settings();
/// assert_eq!(settings.mode,                        AppMode::Offscreen);
/// assert_eq!(settings.blend_preference,            DisplayBlend::None);
/// assert_eq!(settings.no_flatscreen_fallback,      0);
/// assert_eq!(settings.depth_mode,                  DepthMode::Balanced);
/// assert_eq!(settings.log_filter,                  LogLevel::None);
/// assert_eq!(settings.overlay_app,                 0);
/// assert_eq!(settings.overlay_priority,            0);
/// assert_eq!(settings.flatscreen_pos_x,            0);
/// assert_eq!(settings.flatscreen_pos_y,            0);
/// assert_eq!(settings.flatscreen_width,            1280);
/// assert_eq!(settings.flatscreen_height,           720);
/// assert_eq!(settings.disable_desktop_input_window,0);
/// assert_eq!(settings.disable_unfocused_sleep,     0);
/// assert_eq!(settings.render_scaling,              1.0);
/// assert_eq!(settings.render_multisample,          1);
/// assert_eq!(settings.origin,                      OriginMode::Local);
/// assert_eq!(settings.omit_empty_frames,           0);
/// assert_eq!(settings.standby_mode,                StandbyMode::Pause);
/// Sk::shutdown();
/// ```
#[derive(Debug, Clone)]
#[repr(C)]
pub struct SkSettings {
    pub app_name: *const c_char,
    pub assets_folder: *const c_char,
    pub mode: AppMode,
    pub blend_preference: DisplayBlend,
    pub no_flatscreen_fallback: Bool32T,
    pub depth_mode: DepthMode,
    pub log_filter: LogLevel,
    pub overlay_app: Bool32T,
    pub overlay_priority: u32,
    pub flatscreen_pos_x: i32,
    pub flatscreen_pos_y: i32,
    pub flatscreen_width: i32,
    pub flatscreen_height: i32,
    pub disable_desktop_input_window: Bool32T,
    pub disable_unfocused_sleep: Bool32T,
    pub render_scaling: f32,
    pub render_multisample: i32,
    pub origin: OriginMode,
    pub omit_empty_frames: Bool32T,
    pub standby_mode: StandbyMode,
    pub android_java_vm: *mut c_void,
    pub android_activity: *mut c_void,
}
impl Default for SkSettings {
    fn default() -> Self {
        Self {
            app_name: DEFAULT_NAME,
            assets_folder: Self::assets_folder(get_assets_dir()),
            mode: AppMode::XR,
            blend_preference: DisplayBlend::None,
            no_flatscreen_fallback: 0,
            depth_mode: DepthMode::Balanced,
            log_filter: LogLevel::None,
            overlay_app: 0,
            overlay_priority: 0,
            flatscreen_pos_x: 0,
            flatscreen_pos_y: 0,
            flatscreen_width: 0,
            flatscreen_height: 0,
            disable_desktop_input_window: 0,
            disable_unfocused_sleep: 0,
            render_scaling: 1.0,
            render_multisample: 1,
            origin: OriginMode::Local,
            omit_empty_frames: 0,
            standby_mode: StandbyMode::Default,
            android_java_vm: null_mut(),
            android_activity: null_mut(),
        }
    }
}

impl fmt::Display for SkSettings {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl SkSettings {
    /// Name of the application, this shows up an the top of the Win32 window, and is submitted to OpenXR. OpenXR caps
    /// this at 128 characters. Default is "StereoKitApp"
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/appName.html>
    pub fn app_name(&mut self, app_name: impl AsRef<str>) -> &mut Self {
        let c_str = CString::new(app_name.as_ref()).unwrap();
        self.app_name = c_str.into_raw();
        self
    }

    /// Where to look for assets when loading files! Final path will look like ‘\[assetsFolder\]/\[file\]’, so a
    /// trailing ‘/’ is unnecessary. Default is "assets"
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/assetsFolder.html>
    ///
    /// Not pub anymore, please change variable SK_RUST_ASSET_DIR in config.toml [env]
    fn assets_folder(assets_folder: impl AsRef<Path>) -> *mut c_char {
        let c_str = CString::new(assets_folder.as_ref().to_str().unwrap()).unwrap();
        c_str.into_raw()
    }

    /// Which operation mode should we use for this app? Default is XR, and by default the app will fall back to
    /// Simulator if XR fails or is unavailable.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/appMode.html>
    pub fn mode(&mut self, app_mode: AppMode) -> &mut Self {
        self.mode = app_mode;
        self
    }

    ///If the preferred display fails, should we avoid falling back to flatscreen and just crash out? Default is false.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/noFlatscreenFallback.html>
    pub fn no_flatscreen_fallback(&mut self, no_flatscreen_fallback: bool) -> &mut Self {
        self.no_flatscreen_fallback = no_flatscreen_fallback as Bool32T;
        self
    }

    /// What type of background blend mode do we prefer for this application? Are you trying to build an
    /// Opaque/Immersive/VR app, or would you like the display to be AnyTransparent, so the world will show up behind
    /// your content, if that’s an option? Note that this is a preference only, and if it’s not available on this device,
    /// the app will fall back to the runtime’s preference instead! By default, (DisplayBlend.None) this uses the
    /// runtime’s preference.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/blendPreference.html>
    pub fn blend_preference(&mut self, blend_preference: DisplayBlend) -> &mut Self {
        self.blend_preference = blend_preference;
        self
    }

    /// What kind of depth buffer should StereoKit use? A fast one, a detailed one, one that uses stencils? By default,
    /// StereoKit uses a balanced mix depending on platform, prioritizing speed but opening up when there’s headroom.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/depthMode.html>
    pub fn depth_mode(&mut self, depth_mode: DepthMode) -> &mut Self {
        self.depth_mode = depth_mode;
        self
    }

    //// The default log filtering level. This can be changed at runtime, but this allows you to set the log filter
    /// before Initialization occurs, so you can choose to get information from that. Default is LogLevel.Diagnostic.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/logFilter.html>
    pub fn log_filter(&mut self, log_filter: LogLevel) -> &mut Self {
        self.log_filter = log_filter;
        self
    }

    /// If the runtime supports it, should this application run as an overlay above existing applications? Check
    /// SK.System.overlayApp after initialization to see if the runtime could comply with this flag. This will always
    /// force StereoKit to work in a blend compositing mode.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/overlayApp.html>
    pub fn overlay_app(&mut self, overlay_app: bool) -> &mut Self {
        self.overlay_app = overlay_app as Bool32T;
        self
    }

    /// For overlay applications, this is the order in which apps should be composited together. 0 means first, bottom
    /// of the stack, and uint.MaxValue is last, on top of the stack.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/overlayPriority.html>
    pub fn overlay_priority(&mut self, overlay_priority: u32) -> &mut Self {
        self.overlay_priority = overlay_priority;
        self
    }

    /// If using DisplayMode::Flatscreen, the pixel position of the window on the screen.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/flatscreenPosX.html>
    pub fn flatscreen_pos_x(&mut self, flatscreen_pos_x: i32) -> &mut Self {
        self.flatscreen_pos_x = flatscreen_pos_x;
        self
    }

    /// If using DisplayMode::Flatscreen, the pixel position of the window on the screen.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/flatscreenPosY.html>
    pub fn flatscreen_pos_y(&mut self, flatscreen_pos_y: i32) -> &mut Self {
        self.flatscreen_pos_y = flatscreen_pos_y;
        self
    }

    /// If using DisplayMode::Flatscreen, the pixel position of the window on the screen.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings.html>
    pub fn flatscreen_pos(&mut self, flatscreen_pos_x: i32, flatscreen_pos_y: i32) -> &mut Self {
        self.flatscreen_pos_x = flatscreen_pos_x;
        self.flatscreen_pos_y = flatscreen_pos_y;
        self
    }

    /// If using DisplayMode::Flatscreen, the pixel size of the window on the screen.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/flatscreenWidth.html>
    pub fn flatscreen_width(&mut self, flatscreen_width: i32) -> &mut Self {
        self.flatscreen_width = flatscreen_width;
        self
    }

    /// If using DisplayMode::Flatscreen, the pixel size of the window on the screen.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/flatscreenHeight.html>
    pub fn flatscreen_height(&mut self, flatscreen_height: i32) -> &mut Self {
        self.flatscreen_height = flatscreen_height;
        self
    }

    /// If using DisplayMode::Flatscreen, the pixel size of the window on the screen.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings.html>
    pub fn flatscreen_size(&mut self, flatscreen_width: i32, flatscreen_height: i32) -> &mut Self {
        self.flatscreen_width = flatscreen_width;
        self.flatscreen_height = flatscreen_height;
        self
    }

    /// By default, StereoKit will open a desktop window for keyboard input due to lack of XR-native keyboard APIs on
    /// many platforms. If you don’t want this, you can disable it with this setting!
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/disableDesktopInputWindow.html>
    pub fn disable_desktop_input_window(&mut self, disabled_desktop_input_window: bool) -> &mut Self {
        self.disable_desktop_input_window = disabled_desktop_input_window as Bool32T;
        self
    }

    /// By default, StereoKit will slow down when the application is out of focus. This is useful for saving processing
    /// power while the app is out-of-focus, but may not always be desired. In particular, running multiple copies of a
    /// SK app for testing networking code may benefit from this setting.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/disableUnfocusedSleep.html>
    #[deprecated(since = "0.40.0", note = "please use `standby_mode = StandbyMode::None` instead")]
    pub fn disable_unfocused_sleep(&mut self, disable_unfocused_sleep: bool) -> &mut Self {
        self.disable_unfocused_sleep = disable_unfocused_sleep as Bool32T;
        self
    }

    /// If you know in advance that you need this feature, this setting allows you to set Renderer::scaling before
    /// initialization. This avoids creating and discarding a large and unnecessary swapchain object. Default value
    /// is 1.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/renderScaling.html>
    pub fn render_scaling(&mut self, render_scaling: f32) -> &mut Self {
        self.render_scaling = render_scaling;
        self
    }

    /// If you know in advance that you need this feature, this setting allows you to set Renderer::multisample before
    /// initialization. This avoids creating and discarding a large and unnecessary swapchain object. Default value is 1.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/renderMultisample.html>
    pub fn render_multisample(&mut self, render_multisample: i32) -> &mut Self {
        self.render_multisample = render_multisample;
        self
    }

    /// Set the behavior of StereoKit’s initial origin. Default behavior is OriginMode.Local, which is the most
    /// universally supported origin mode. Different origin modes have varying levels of support on different XR
    /// runtimes, and StereoKit will provide reasonable fallbacks for each. NOTE that when falling back, StereoKit
    /// will use a different root origin mode plus an offset. You can check World.OriginMode and World.OriginOffset
    /// to inspect what StereoKit actually landed on.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/origin.html>
    pub fn origin(&mut self, origin_mode: OriginMode) -> &mut Self {
        self.origin = origin_mode;
        self
    }

    /// If StereoKit has nothing to render for this frame, it skips submitting a projection layer to OpenXR entirely.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/omitEmptyFrames.html>
    pub fn omit_empty_frames(&mut self, origin_mode: bool) -> &mut Self {
        self.omit_empty_frames = origin_mode as Bool32T;
        self
    }

    /// Configures StereoKit's behavior during device standby. By default in v0.4, SK will completely pause the main
    /// thread and disable audio. In v0.3, SK will continue to execute at a throttled pace, and audio will remain on.
    /// <https://stereokit.net/Pages/StereoKit/SKSettings/omitEmptyFrames.html>
    pub fn standby_mode(&mut self, mode: StandbyMode) -> &mut Self {
        self.standby_mode = mode;
        self
    }

    // fn to_string(&self) -> String {
    //     unsafe { CStr::from_ptr(self.app_name) }.to_str().unwrap().to_string()
    // }
}
#[cfg(feature = "event-loop")]
impl SkSettings {
    /// Initialize StereoKit with the given settings (here for Android platform)
    /// This should be done after Sk::poll_first_event(...) has been called in the main thread.
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    /// * `app` - The AndroidApp instance to use for initialization given by android_main() from <https://github.com/rust-mobile>
    ///
    /// see also [`Sk::init_with_event_loop`]
    #[cfg(target_os = "android")]
    pub fn init_with_event_loop(&mut self, app: AndroidApp) -> Result<(Sk, EventLoop<StepperAction>), StereoKitError> {
        Sk::init_with_event_loop(self, app)
    }

    /// Initialise Sk with the given settings parameter (here for non Android platform)
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    ///
    /// see also [`Sk::init_with_event_loop`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{prelude::*, system::{LogLevel,Renderer}, framework::SkClosures,
    ///                      maths::{Vec3, Matrix, Pose} ,tools::title::Title, util::named_colors};
    /// use stereokit_rust::sk::{Sk, SkSettings, AppMode, DisplayBlend, DepthMode,
    ///                          OriginMode, StandbyMode, QuitReason};
    ///
    /// let mut settings = SkSettings::default();
    /// settings
    ///     .app_name("Test")
    ///     .mode(AppMode::Offscreen)
    ///     .origin(OriginMode::Floor)
    ///     .render_multisample(4)
    ///     .render_scaling(0.8)
    ///     .depth_mode(DepthMode::D32)
    ///     .omit_empty_frames(true)
    ///     .log_filter(LogLevel::Diagnostic)
    ///     .no_flatscreen_fallback(true);
    ///
    /// let (mut sk, event_loop) = settings.init_with_event_loop()
    ///                                .expect("StereoKit should initialize");
    ///
    /// let mut title = Title::new("Sk basic example", Some(named_colors::BLUE), None, None);
    /// title.transform = Matrix::tr(&([0.5, 0.5, -1.9].into()),
    ///                              &([0.0, 200.0, 0.0].into()));
    /// sk.send_event(StepperAction::add("Title_blue_ID1", title));
    ///
    /// let mut iter = 0;
    /// let number_of_steps = 3;
    /// let filename_scr = "screenshots/sk_basic_example.jpeg";
    /// SkClosures::new(sk, |sk, token|  {
    ///     // Main loop where we draw stuff and do things!!
    ///     if iter > number_of_steps {sk.quit(None)}
    ///
    ///     if iter == number_of_steps {
    ///         // render screenshot
    ///         Renderer::screenshot(token, filename_scr, 90, Pose::look_at(Vec3::Z, Vec3::ZERO),
    ///             200, 200, Some(99.0) );
    ///     }
    ///
    ///     iter+=1;
    /// })
    /// .shutdown(|sk| {
    ///    // This is called when the app is shutting down
    ///     assert_eq!(sk.get_quit_reason(), QuitReason::User);
    ///     Log::info(format!("QuitReason is {:?}", sk.get_quit_reason()));
    /// })
    /// .run(event_loop);
    ///
    /// Sk::shutdown();
    ///
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sk_basic_example.jpeg" alt="screenshot" width="200">
    #[cfg(not(target_os = "android"))]
    pub fn init_with_event_loop(&mut self) -> Result<(Sk, EventLoop<StepperAction>), StereoKitError> {
        Sk::init_with_event_loop(self)
    }
}
#[cfg(feature = "no-event-loop")]
impl SkSettings {
    /// Initialize StereoKit with the given settings (here for Android)
    /// This should be done after Sk::poll_first_event(...) has been called in the main thread.
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    /// * `app` - The AndroidApp instance to use for initialization given by android_main() from <https://github.com/rust-mobile>
    ///
    /// see also [Sk::init]
    #[cfg(target_os = "android")]
    pub fn init(&mut self, app: AndroidApp) -> Result<Sk, StereoKitError> {
        Sk::init(self, app)
    }

    /// Initialize StereoKit with the given settings (here for non Android platform)
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    ///
    /// see also [Sk::init]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{prelude::*, system::LogLevel};
    /// use stereokit_rust::sk::{Sk, SkSettings, AppMode, DisplayBlend, DepthMode,
    ///                          OriginMode, StandbyMode, QuitReason};
    ///
    /// let mut settings = SkSettings::default();
    /// settings
    ///     .app_name("Test")
    ///     .mode(AppMode::Offscreen)
    ///     .origin(OriginMode::Floor)
    ///     .render_multisample(4)
    ///     .render_scaling(0.8)
    ///     .depth_mode(DepthMode::D32)
    ///     .omit_empty_frames(true)
    ///     .log_filter(LogLevel::Diagnostic)
    ///     .no_flatscreen_fallback(true);
    ///
    /// let sk = settings.init().expect("StereoKit should initialize");
    /// let settings = sk.get_settings();
    /// assert_eq!(settings.mode,                        AppMode::Offscreen);
    /// assert_eq!(settings.blend_preference,            DisplayBlend::None);
    /// assert_eq!(settings.origin,                      OriginMode::Floor);
    /// assert_eq!(settings.standby_mode,                StandbyMode::Pause);
    ///
    /// let mut iter = 0;
    /// let number_of_steps = 3;
    /// while let Some(token) = sk.step() {
    ///     // Main loop where we draw stuff and do things!!
    ///     if iter > number_of_steps {sk.quit(None)}
    ///
    ///     //model.draw(token,  transform ,  None, None);
    ///     //mesh.draw(token, &material,transform, None, None);
    ///
    ///     iter+=1;
    /// }
    ///
    /// Sk::shutdown();
    /// ```
    #[cfg(not(target_os = "android"))]
    pub fn init(&mut self) -> Result<Sk, StereoKitError> {
        Sk::init(self)
    }
}

// Trampoline for Sk.run closures
// unsafe extern "C" fn sk_trampoline<F: FnMut(&mut Sk)>(context: *mut c_void) {
//     let (closure, sk) = &mut *(context as *mut (&mut F, &mut Sk));
//     closure(*sk);
// }

/// Provides a reason on why StereoKit has quit.
/// <https://stereokit.net/Pages/StereoKit/QuitReason.html>
///
/// see also: [`Sk::quit`] [`Sk::get_quit_reason`]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum QuitReason {
    /// Default state when SK has not quit.
    None = 0,
    /// The user (or possibly the OS) has explicitly asked to exit the application under normal circumstances.
    User = 1,
    /// Some runtime error occurred, causing the application to quit gracefully.
    Error = 2,
    /// If initialization failed, StereoKit won't run to begin with!
    InitializationFailed = 3,
    /// The runtime under StereoKit has encountered an issue and has been lost.
    SessionLost = 4,
}

/// Non canonical structure whose purpose is to expose infos far from [`Sk`]
///
/// see also: [`Sk::get_sk_info_clone`]
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::sk::{Sk, SkSettings, AppMode, SkInfo};
/// use stereokit_rust::{system::LogLevel, ui::Ui, maths::Pose};
///
/// let sk_info = Some(sk.get_sk_info_clone());
/// let settings = SkInfo::settings_from(&sk_info);
/// assert_eq!(settings.mode, AppMode::Offscreen);
///
/// let system = SkInfo::system_from(&sk_info);
/// assert_eq!(system.get_display_width(), 0);
///
/// #[cfg(feature = "event-loop")]
/// {
///     use stereokit_rust::tools::screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW};
///     // send event
///     SkInfo::send_event(&sk_info,
///                    StepperAction::add_default::<ScreenshotViewer>("SCR_ID1"));
///    
///     // get event_loop_proxy clone to use in other threads
///     let event_loop_proxy = SkInfo::event_loop_proxy_from(&sk_info);
///
///     // get a closure, ready to send an event with a given value
///     let show_screenshot = SkInfo::get_message_closure(sk_info,
///                               "SCR_ID1", SHOW_SCREENSHOT_WINDOW);
///     let mut window_pose = Pose::IDENTITY;
///     test_steps!(// !!!! Get a proper main loop !!!!
///         Ui::window_begin("Default Font", &mut window_pose, None, None, None);
///         if Ui::button("Show screenshot", None) {
///             show_screenshot("true".into())
///         }
///         Ui::window_end();
///     );
/// }
/// ```
#[allow(dead_code)]
#[derive(Debug)]
pub struct SkInfo {
    settings: SkSettings,
    system_info: SystemInfo,
    #[cfg(feature = "event-loop")]
    event_loop_proxy: Option<EventLoopProxy<StepperAction>>,
    #[cfg(target_os = "android")]
    android_app: AndroidApp,
}

impl SkInfo {
    /// This is a copy of the settings that StereoKit was initialized with, so you can refer back to them a little
    /// easier.  Some of these values will be different than provided, as StereoKit will resolve some default values
    /// based on the platform capabilities or internal preference. These are read only, and keep in mind that some
    /// settings are only requests! Check [`Sk::get_system`] and other properties for the current state of StereoKit.
    /// [`SkInfo::settings_from`] is more easy to use.
    /// <https://stereokit.net/Pages/StereoKit/SK/Settings.html>
    pub fn get_settings(&self) -> SkSettings {
        self.settings.clone()
    }

    /// This structure contains information about the current system and its capabilities. There’s a lot of different MR
    /// devices, so it’s nice to have code for systems with particular characteristics!
    /// [`SkInfo::system_from`] is more easy to use.
    /// <https://stereokit.net/Pages/StereoKit/SK/System.html>
    pub fn get_system(&self) -> SystemInfo {
        self.system_info.clone()
    }

    /// Get an event_loop_proxy clone to send events.
    /// [`SkInfo::event_loop_proxy_from`] is more easy to use.
    #[cfg(feature = "event-loop")]
    pub fn get_event_loop_proxy(&self) -> Option<EventLoopProxy<StepperAction>> {
        self.event_loop_proxy.clone()
    }

    /// Non canonical function to get the rust ndk AndroidApp
    #[cfg(target_os = "android")]
    pub fn get_android_app(&self) -> &AndroidApp {
        &self.android_app
    }

    /// This is a copy of the settings that StereoKit was initialized with, so you can refer back to them a little
    /// easier.  Some of these values will be different than provided, as StereoKit will resolve some default values
    /// based on the platform capabilities or internal preference. These are read only, and keep in mind that some
    /// settings are only requests! Check SK.System and other properties for the current state of StereoKit.
    /// <https://stereokit.net/Pages/StereoKit/SK/Settings.html>
    /// * `sk_info` - The SkInfo smart pointer to get the settings from
    ///
    /// see also [`Sk::get_settings`]
    pub fn settings_from(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> SkSettings {
        if sk_info.is_none() {
            Log::err("The stepper must be initialized. SkInfo::setting_from(??) returns an invalid default value.");
            return SkSettings::default();
        }
        let rc_sk = sk_info.as_ref().unwrap();
        let sk = rc_sk.as_ref();
        sk.borrow().get_settings()
    }

    /// This structure contains information about the current system and its capabilities. There’s a lot of different MR
    /// devices, so it’s nice to have code for systems with particular characteristics!
    /// <https://stereokit.net/Pages/StereoKit/SK/System.html>
    /// * `sk_info` - The SkInfo smart pointer to get the SystemInfo from.
    ///
    /// see also [`Sk::get_system`]
    pub fn system_from(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> SystemInfo {
        if sk_info.is_none() {
            Log::err("The stepper must be initialized. SkInfo::system_from(??) returns an invalid default value.");
            return SystemInfo::default();
        }
        let rc_sk = sk_info.as_ref().unwrap();
        let sk = rc_sk.as_ref();
        sk.borrow().get_system()
    }

    /// Get an event_loop_proxy clone to send events
    /// * `sk_info` - The SkInfo smart pointer to get the event_loop_proxy from.
    ///
    /// see also [`Sk::get_sk_info_clone`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::{Sk, SkSettings, AppMode, SkInfo, QuitReason};
    /// use stereokit_rust::framework::SkClosures;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let sk_info = Some(sk.get_sk_info_clone());
    /// let event_loop_proxy = SkInfo::event_loop_proxy_from(&sk_info);
    ///
    /// // Spawn a new thread to send an event after a delay
    /// let handle = thread::spawn(move || {
    ///     thread::sleep(Duration::from_millis(100));
    ///     if let Some(proxy) = event_loop_proxy {
    ///         proxy.send_event(StepperAction::quit("thread", "I'm done!")).unwrap();
    ///     }
    /// });
    ///
    /// SkClosures::new(sk, |sk, token|  {
    ///     // Only the thread can stop this test
    ///     // model.draw(token, Matrix::IDENTITY, None, None);
    /// })
    /// .shutdown(|sk| {
    ///     assert_eq!(sk.get_quit_reason(), QuitReason::User);
    /// })
    /// .run(event_loop);
    ///
    /// // If we are here the thread has finished
    /// handle.join().unwrap();
    /// ```
    #[cfg(feature = "event-loop")]
    pub fn event_loop_proxy_from(sk_info: &Option<Rc<RefCell<SkInfo>>>) -> Option<EventLoopProxy<StepperAction>> {
        if sk_info.is_none() {
            Log::err("The stepper must be initialized. SkInfo::event_loop_proxy_from(??) returns None.");
            return None;
        }
        let rc_sk = sk_info.as_ref().unwrap();
        let sk = rc_sk.as_ref();
        sk.borrow().get_event_loop_proxy()
    }

    /// Send a StepperAction to the event loop
    /// * `sk_info` - The SkInfo smart pointer to get the event loop proxy from
    /// * `message` - The StepperAction to send to the event loop
    ///
    /// see also [`SkInfo::get_message_closure`]
    #[cfg(feature = "event-loop")]
    pub fn send_event(sk_info: &Option<Rc<RefCell<SkInfo>>>, message: StepperAction) {
        if let Some(proxy) = Self::event_loop_proxy_from(sk_info) {
            proxy.send_event(message).unwrap();
        } else {
            Log::err("The stepper must be initialized. SkInfo::send_event(??) not sent.");
        }
    }

    /// Get a closure to send a message to the event loop from a given ID for a given key. This is useful for
    /// HandMenuRadial for example.
    ///
    /// see also [`SkInfo::send_event`]
    /// see examples/demos/hand_menu_radial1.rs
    #[cfg(feature = "event-loop")]
    pub fn get_message_closure(
        sk_info: Option<Rc<RefCell<SkInfo>>>,
        id: impl AsRef<str>,
        key: &str,
    ) -> impl Fn(String) {
        let sk_info = sk_info.clone();
        let key = key.to_string();
        let id = id.as_ref().to_string();
        Box::new(move |value: String| {
            SkInfo::send_event(&sk_info, StepperAction::event(id.as_str(), &key, &value));
        })
    }
}

/// A token you only find on the main thread. It is required to call rendering functions
///
/// see also [`Sk::step`]
pub struct MainThreadToken {
    /// Event report of one step
    ///
    /// see also [`Sk::step`] [`check_event`](`crate::IStepper`) [`crate::framework::IStepper::step`]
    #[cfg(feature = "event-loop")]
    pub(crate) event_report: Vec<StepperAction>,
}

#[cfg(feature = "event-loop")]
impl MainThreadToken {
    /// Get the event report of this step. You don't have to call this function if you use the derive macro
    /// [`crate::IStepper`] and implement the `check_event` function.
    ///
    /// see also [`Sk::step`] [`check_event`](`crate::IStepper`)
    pub fn get_event_report(&self) -> &Vec<StepperAction> {
        &self.event_report
    }
}

/// This class contains functions for running the StereoKit library!
/// <https://stereokit.net/Pages/StereoKit/SK.html>
pub struct Sk {
    /// Where to find information about the current system
    sk_info: Rc<RefCell<SkInfo>>,
    /// The main thread token
    token: MainThreadToken,
    /// The current pool of ISteppers.
    #[cfg(feature = "event-loop")]
    pub(crate) steppers: Steppers,
    /// Some actions to run on the main thread.
    #[cfg(feature = "event-loop")]
    pub(crate) actions: VecDeque<Box<dyn FnMut()>>,
}

impl Sk {
    /// OpenXR won't leave IDLE state if we do not purge the first events :
    /// PostSessionStateChange: XR_SESSION_STATE_IDLE -> XR_SESSION_STATE_READY
    ///
    /// This must be done in the main thread. see [`Sk::init`]
    #[cfg(target_os = "android")]
    pub fn poll_first_events(app: &AndroidApp) {
        let mut ready_to_go = false;
        while !ready_to_go {
            app.poll_events(None, |event| match event {
                PollEvent::Main(main_event) => {
                    Log::diag(format!("MainEvent {:?} ", main_event));
                    match main_event {
                        MainEvent::GainedFocus { .. } => {
                            ready_to_go = true;
                        }
                        _ => {
                            ready_to_go = false;
                        }
                    }
                }
                otherwise => Log::diag(format!("PollEvent {:?} ", otherwise)),
            })
        }
    }

    /// Initialize StereoKit with the given settings (here for Android platform)
    /// This should be done after Sk::poll_first_event(...) has been called in the main thread.
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    /// * `settings` - The settings to use for initialization
    /// * `app` - The AndroidApp instance to use for initialization given by android_main() from <https://github.com/rust-mobile>
    ///
    /// It is best to use [`SkSettings::init`]
    #[cfg(feature = "no-event-loop")]
    #[cfg(target_os = "android")]
    pub fn init(settings: &mut SkSettings, app: AndroidApp) -> Result<Sk, StereoKitError> {
        // this must be done in the main thread : Sk::poll_first_events(&app);
        let (vm_pointer, jobject_pointer) = {
            {
                let context = ndk_context::android_context();
                (context.vm(), context.context())
            }
        };
        settings.android_java_vm = vm_pointer;
        settings.android_activity = jobject_pointer;

        Log::diag(format!("sk_init : context: {:?} / jvm: {:?}", vm_pointer, jobject_pointer));

        match unsafe {
            Log::info("Before init >>>");
            let val = sk_init(settings.clone()) != 0;
            Log::info("<<< After init");
            val
        } {
            true => {
                let sk_info = Rc::new(RefCell::new(SkInfo {
                    android_app: app,
                    settings: unsafe { sk_get_settings() },
                    system_info: unsafe { sk_system_info() },
                    #[cfg(feature = "event-loop")]
                    event_loop_proxy: None,
                }));
                Ok(Sk {
                    sk_info: sk_info.clone(),
                    token: MainThreadToken {
                        #[cfg(feature = "event-loop")]
                        event_report: vec![],
                    },
                    #[cfg(feature = "event-loop")]
                    steppers: Steppers::new(sk_info.clone()),
                    #[cfg(feature = "event-loop")]
                    actions: VecDeque::new(),
                })
            }
            false => Err(StereoKitError::SkInit(settings.to_string())),
        }
    }

    /// Initialize StereoKit with the given settings (here for non Android platform)
    /// Even if this function can be called with the feature `event_loop` (for tests) it is more logical to use
    /// `init_with_event_loop`.
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    /// * `settings` - The settings to use for initialization
    ///
    /// It is best to use `SkSettings::init` see example with [`Sk::step`]
    #[cfg(not(target_os = "android"))]
    pub fn init(settings: &SkSettings) -> Result<Sk, StereoKitError> {
        match unsafe {
            Log::info("Before init >>>");
            let val = sk_init(settings.clone()) != 0;
            Log::info("<<< After init");
            val
        } {
            true => {
                let sk_info = Rc::new(RefCell::new(SkInfo {
                    settings: unsafe { sk_get_settings() },
                    system_info: unsafe { sk_system_info() },
                    #[cfg(feature = "event-loop")]
                    event_loop_proxy: None,
                }));
                Ok(Sk {
                    sk_info: sk_info.clone(),
                    token: MainThreadToken {
                        #[cfg(feature = "event-loop")]
                        event_report: vec![],
                    },
                    #[cfg(feature = "event-loop")]
                    steppers: Steppers::new(sk_info.clone()),
                    #[cfg(feature = "event-loop")]
                    actions: VecDeque::new(),
                })
            }
            false => Err(StereoKitError::SkInit(settings.to_string())),
        }
    }

    /// Steps all StereoKit systems, and inserts user code via callback between the appropriate system updates.
    /// For feature `event-loop` you don't have to call this yourself thanks to `SkClosures`.
    /// <https://stereokit.net/Pages/StereoKit/SK/Step.html>
    ///
    /// Returns `None` If an exit message is received from the platform, or SK.Quit() is called. Or the MainThreadToken
    /// if a step is to be drawn.
    /// see also [`sk_step`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::{prelude::*, system::LogLevel};
    /// use stereokit_rust::sk::{Sk, SkSettings, AppMode, QuitReason};
    ///
    /// let mut settings = SkSettings::default();
    /// settings
    ///     .app_name("Test")
    ///     .mode(AppMode::Offscreen);
    ///
    /// let sk = Sk::init(&settings).expect("StereoKit should initialize");
    ///
    /// let mut iter = 0;
    /// let number_of_steps = 3;
    /// while let Some(token) = sk.step() {
    ///     // Main loop where we draw stuff and do things!!
    ///     if iter > number_of_steps {sk.quit(None)}
    ///
    ///     //model.draw(token,  transform ,  None, None);
    ///     //mesh.draw(token, &material,transform, None, None);
    ///
    ///     iter+=1;
    /// }
    /// Sk::shutdown();
    /// ```
    pub fn step(&self) -> Option<&MainThreadToken> {
        if unsafe { sk_step(None) } == 0 {
            return None;
        }

        Some(&self.token)
    }

    /// Returns the MainThreadToken that is used to draw stuff.
    pub fn main_thread_token(&mut self) -> &MainThreadToken {
        &self.token
    }

    /// Since we can fallback to a different DisplayMode, this lets you check to see which Runtime was successfully
    /// initialized.
    /// <https://stereokit.net/Pages/StereoKit/SK/ActiveDisplayMode.html>
    ///
    /// see also [`sk_active_display_mode`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::DisplayMode;
    ///
    /// let display_mode: DisplayMode = sk.get_active_display_mode();
    /// assert_eq!(display_mode, DisplayMode::Flatscreen);
    /// ```
    pub fn get_active_display_mode(&self) -> DisplayMode {
        unsafe { sk_active_display_mode() }
    }

    /// This tells about the app’s current focus state, whether it’s active and receiving input, or if it’s backgrounded
    /// or hidden. This can be important since apps may still run and render when unfocused, as the app may still be
    /// visible behind the app that does have focus.
    /// <https://stereokit.net/Pages/StereoKit/SK/AppFocus.html>
    ///
    /// see also [`sk_app_focus`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::AppFocus;
    ///
    /// let display_mode: AppFocus = sk.get_app_focus();
    /// assert_eq!(display_mode, AppFocus::Active);
    /// ```
    pub fn get_app_focus(&self) -> AppFocus {
        unsafe { sk_app_focus() }
    }

    /// Return a clone of SkInfo smart pointer
    /// <https://stereokit.net/Pages/StereoKit/SK.html>
    ///
    /// see also [`Sk::get_settings`] [`Sk::get_system`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::{Sk, SkSettings, AppMode, SkInfo};
    ///
    /// let sk_info = Some(sk.get_sk_info_clone());
    /// let settings = SkInfo::settings_from(&sk_info);
    /// assert_eq!(settings.mode, AppMode::Offscreen);
    ///
    /// let system = SkInfo::system_from(&sk_info);
    /// assert_eq!(system.get_display_width(), 0);
    /// ```
    pub fn get_sk_info_clone(&self) -> Rc<RefCell<SkInfo>> {
        self.sk_info.clone()
    }

    /// This is a copy of the settings that StereoKit was initialized with, so you can refer back to them a little
    /// easier.  Some of these values will be different than provided, as StereoKit will resolve some default values
    /// based on the platform capabilities or internal preference. These are read only, and keep in mind that some
    /// settings are only requests! Check SK.System and other properties for the current state of StereoKit.
    /// <https://stereokit.net/Pages/StereoKit/SK/Settings.html>
    ///
    /// see also [`sk_get_settings`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::{Sk, SkSettings, AppMode};
    ///
    /// let settings: SkSettings = sk.get_settings();
    /// assert_eq!(settings.mode, AppMode::Offscreen);
    /// ```
    pub fn get_settings(&self) -> SkSettings {
        unsafe { sk_get_settings() }
    }

    /// This structure contains information about the current system and its capabilities. There’s a lot of different MR
    /// devices, so it’s nice to have code for systems with particular characteristics!
    /// <https://stereokit.net/Pages/StereoKit/SK/System.html>
    ///
    /// see also [`sk_system_info`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::{Sk, SystemInfo};
    ///
    /// let system_info: SystemInfo = sk.get_system();
    /// assert_eq!(system_info.get_display_height(), 0);
    /// ```
    pub fn get_system(&self) -> SystemInfo {
        unsafe { sk_system_info() }
    }

    /// An integer version Id! This is defined using a hex value with this format: 0xMMMMiiiiPPPPrrrr in order of
    /// Major.mInor.Patch.pre-Release
    /// <https://stereokit.net/Pages/StereoKit/SK/VersionId.html>
    ///
    /// see also [`sk_version_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    ///
    /// assert_ne!(sk.get_version_id(), 0x0);
    /// ```
    pub fn get_version_id(&self) -> u64 {
        unsafe { sk_version_id() }
    }

    /// Human-readable version name embedded in the StereoKitC library.
    /// <https://stereokit.net/Pages/StereoKit/SK/VersionName.html>
    ///
    /// see also [`sk_version_name`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    ///
    /// assert_ne!(sk.get_version_name(), "3");
    /// ```
    pub fn get_version_name(&self) -> &str {
        unsafe { CStr::from_ptr(sk_version_name()) }.to_str().unwrap()
    }

    /// Lets StereoKit know it should quit! It’ll finish the current frame, and after that Step will return that it
    /// wants to exit.
    /// <https://stereokit.net/Pages/StereoKit/SK/Quit.html>
    /// * `quit_reason` - if None has default value of QuitReason::User
    ///
    /// see also [`sk_quit`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::QuitReason;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // Here we can draw some stuff.
    ///     //model.draw(token, Matrix::IDENTITY, None, None);
    ///
    ///     // Quit the app at first step
    ///     sk.quit(Some(QuitReason::Error));
    /// );
    /// ```
    pub fn quit(&self, quit_reason: Option<QuitReason>) {
        let quit_reason = quit_reason.unwrap_or(QuitReason::User);
        unsafe { sk_quit(quit_reason) }
    }

    /// This tells the reason why StereoKit has quit and
    /// developer can take appropriate action to debug.
    /// <https://stereokit.net/Pages/StereoKit/SK/QuitReason.html>
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::QuitReason;
    ///
    /// // Quit the app before a single step.
    /// sk.quit(Some(QuitReason::User));
    ///
    /// // a single step.
    /// sk.step();
    ///
    /// assert_eq!(sk.get_quit_reason(), QuitReason::User);
    /// ```
    pub fn get_quit_reason(&self) -> QuitReason {
        unsafe { sk_get_quit_reason() }
    }

    /// Cleans up all StereoKit initialized systems. Release your own StereoKit created assets before calling this. This
    /// is for cleanup only, and should not be used to exit the application, use [`Sk::quit`] for that instead. Calling
    /// this function is unnecessary if using [`sk_run_data`]`, as it is called automatically there.
    /// <https://stereokit.net/Pages/StereoKit/SK/Shutdown.html>
    ///
    /// see also [`sk_shutdown`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::QuitReason;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // Here we can draw some stuff.
    ///     //model.draw(token, Matrix::IDENTITY, None, None);
    ///
    ///     // Quit the app at first step
    ///     sk.quit(Some(QuitReason::Error));
    /// );
    ///
    /// // Shutdown StereoKit
    /// Sk::shutdown();
    /// ```
    pub fn shutdown() {
        unsafe { sk_shutdown() }
        if cfg!(target_os = "android") {
            std::process::exit(0);
        }
    }
}

/// <h2>Sk methods only available for event-loop feature.</h2>
#[cfg(feature = "event-loop")]
impl Sk {
    /// Initializes StereoKit window, default resources, systems, etc.
    /// Here for Android plaforms
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    /// * `settings` - The settings to use for initialization
    /// * `app` - The AndroidApp instance to use for initialization given by android_main() from <https://github.com/rust-mobile>
    ///
    /// It is best to use [`SkSettings::init_with_event_loop`] see also [`sk_init`] [`crate::event_loop::SkClosures`]
    #[cfg(target_os = "android")]
    pub fn init_with_event_loop(
        settings: &mut SkSettings,
        app: AndroidApp,
    ) -> Result<(Sk, EventLoop<StepperAction>), StereoKitError> {
        Sk::poll_first_events(&app);

        let event_loop = EventLoop::<StepperAction>::with_user_event().with_android_app(app.clone()).build()?;
        let event_loop_proxy = event_loop.create_proxy();

        let (vm_pointer, jobject_pointer) = {
            {
                let context = ndk_context::android_context();
                (context.vm(), context.context())
            }
        };
        settings.android_java_vm = vm_pointer;
        settings.android_activity = jobject_pointer;

        Log::diag(format!("sk_init : context: {:?} / jvm: {:?}", vm_pointer, jobject_pointer));

        match unsafe {
            Log::info("Before init >>>");
            let val = sk_init(settings.clone()) != 0;
            Log::info("<<< After init");
            val
        } {
            true => {
                let sk_info = Rc::new(RefCell::new(SkInfo {
                    settings: settings.clone(),
                    system_info: unsafe { sk_system_info() },
                    event_loop_proxy: Some(event_loop_proxy),
                    android_app: app,
                }));
                Ok((
                    Sk {
                        sk_info: sk_info.clone(),
                        token: MainThreadToken { event_report: vec![] },
                        steppers: Steppers::new(sk_info.clone()),
                        actions: VecDeque::new(),
                    },
                    event_loop,
                ))
            }
            false => Err(StereoKitError::SkInit(settings.to_string())),
        }
    }

    /// Initializes StereoKit window, default resources, systems, etc.
    /// Here for non Android platforms!
    /// <https://stereokit.net/Pages/StereoKit/SK/Initialize.html>
    /// * `settings` - The settings to use for initialization.
    ///
    /// It is best to use [`SkSettings::init_with_event_loop`] see also [`sk_init`] [`crate::framework::SkClosures`]    
    #[cfg(not(target_os = "android"))]
    pub fn init_with_event_loop(settings: &mut SkSettings) -> Result<(Sk, EventLoop<StepperAction>), StereoKitError> {
        let event_loop = EventLoop::<StepperAction>::with_user_event().build()?;
        let event_loop_proxy = event_loop.create_proxy();
        let (vm_pointer, jobject_pointer) = (null_mut::<c_void>(), null_mut::<c_void>());

        settings.android_java_vm = vm_pointer;
        settings.android_activity = jobject_pointer;

        Log::info(format!("SK_INIT ::: context {:?}/jvm : {:?}", vm_pointer, jobject_pointer));

        match unsafe {
            Log::info("Before init >>>");
            let val = sk_init(settings.clone()) != 0;
            Log::info("<<< After init");
            val
        } {
            true => {
                let sk_info = Rc::new(RefCell::new(SkInfo {
                    settings: settings.clone(),
                    system_info: unsafe { sk_system_info() },
                    event_loop_proxy: Some(event_loop_proxy),
                }));
                Ok((
                    Sk {
                        sk_info: sk_info.clone(),
                        token: MainThreadToken { event_report: vec![] },
                        steppers: Steppers::new(sk_info.clone()),
                        actions: VecDeque::new(),
                    },
                    event_loop,
                ))
            }
            false => Err(StereoKitError::SkInit(settings.to_string())),
        }
    }

    /// This is a non canonical function that let you swap the current steppers with a new set of steppers
    /// <https://stereokit.net/Pages/StereoKit.Framework/IStepper.html>
    /// * `steppers` - The new set of steppers to use. This will contain the previous steppers so take care of them.
    ///
    /// see example in [`Steppers`]
    pub fn swap_steppers(&mut self, steppers: &mut Steppers) {
        std::mem::swap(&mut self.steppers, steppers);
    }

    /// This will queue up some code to be run on StereoKit’s main thread! Immediately after StereoKit’s Step, all
    /// callbacks registered here will execute, and then removed from the list.
    /// <https://stereokit.net/Pages/StereoKit/SK/ExecuteOnMain.html>
    /// * `action` - Some code to run! This Action will persist in a list until after Step, at which point it is removed
    ///   and dropped.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::{SkInfo, QuitReason};
    ///
    /// let sk_info = Some(sk.get_sk_info_clone());
    /// sk.execute_on_main(move || {
    ///     SkInfo::send_event(&sk_info, StepperAction::quit("main", "I'm done!"));
    /// });
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // Quit the app after a single step.
    /// );
    ///
    /// // 3 steps later we are out
    /// assert_eq!(iter, 3);
    /// ```
    pub fn execute_on_main<F: FnMut() + 'static>(&mut self, action: F) {
        self.actions.push_back(Box::new(action))
    }

    /// Convenient way to push some steppers action.
    /// * `action` - the action to push
    ///
    /// see also [`SkInfo::send_event`] [`winit::event_loop::EventLoopProxy::send_event`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{sk::{SkInfo, QuitReason},
    ///                      tools::{screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW},
    ///                              title::Title}};
    ///
    /// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("SCR_ID1"));
    /// sk.send_event(StepperAction::event("main", SHOW_SCREENSHOT_WINDOW, "true"));
    ///
    /// let title = Title::new("Title", None, None, None);
    /// sk.send_event(StepperAction::add("TITLE_ID1", title));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if (iter == 1) {
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///         // Remove the screenshot viewer after 1 steps.
    ///         sk.send_event(StepperAction::remove("SCR_ID1"));
    ///     } else if (iter == 2) {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///         // Remove the all the Title ISteppers after 2 steps.
    ///         sk.send_event(StepperAction::remove_all(std::any::TypeId::of::<Title>()));
    ///     } else if (iter == 3) {
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///         sk.send_event(StepperAction::quit("main", "I'm done!"));
    ///     }
    /// );
    ///
    /// // 6 steps later we are out
    /// assert_eq!(iter, number_of_steps + 3);
    /// ```
    pub fn send_event(&mut self, action: StepperAction) {
        self.steppers.send_event(action);
    }

    /// The number of ISteppers registered in the current [Steppers]. This does not include
    /// Steppers that have been added, but are not yet initialized. Stepper initialization happens at the beginning of
    /// the frame, before the app's Step.
    ///
    /// see also [`Steppers`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tools::{screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW},
    ///                              title::Title}};
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter == 0 {
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///         sk.send_event(StepperAction::add_default::<ScreenshotViewer>("SCR_ID1"));
    ///     } else if iter == 1 {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///         let title = Title::new("Title", None, None, None);
    ///         sk.send_event(StepperAction::add("TITLE_ID1", title));
    ///     } else if iter == 2 {
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///         sk.send_event(StepperAction::remove("TITLE_ID1"));
    ///     } else if iter < number_of_steps + 2 {
    ///         assert_eq!(sk.get_steppers_count(), 1);
    ///     } else {
    ///         assert_eq!(sk.get_steppers_count(), 0);
    ///     }
    /// );
    ///
    /// // 6 steps later we are out
    /// assert_eq!(iter, number_of_steps + 3);
    /// ```
    pub fn get_steppers_count(&self) -> usize {
        self.steppers.get_count()
    }

    /// Currently active ISteppers registered with [`Sk`]. This does not include Steppers
    /// that have been added, but are not yet initialized. Stepper initialization happens at the beginning of the frame,
    /// before the app's Step.
    ///
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{tools::{screenshot::{ScreenshotViewer, SHOW_SCREENSHOT_WINDOW},
    ///                              title::Title}};
    ///
    /// sk.send_event(StepperAction::add_default::<ScreenshotViewer>("SCR_ID1"));
    ///
    /// let title = Title::new("Title", None, None, None);
    /// sk.send_event(StepperAction::add("TITLE_ID1", title));
    ///
    ///
    /// let steppers = sk.get_steppers();
    /// assert_eq!(steppers.get_count(), 0);
    /// assert_eq!(sk.get_steppers_count(), 0);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     if iter <= number_of_steps + 1 {
    ///         let steppers = sk.get_steppers();
    ///         assert_eq!(steppers.get_count(), 2);
    ///         assert_eq!(sk.get_steppers_count(), 2);
    ///
    ///         let stepper_list = steppers.get_stepper_handlers();
    ///         for (iter, stepper) in stepper_list.iter().enumerate() {
    ///             match iter {
    ///                 0 => assert_eq!(stepper.get_id(), "SCR_ID1"),
    ///                 1 => assert_eq!(stepper.get_id(), "TITLE_ID1"),
    ///                 _ => panic!("Only 2 steppers should be present"),
    ///             }
    ///         }
    ///     } else {
    ///        let steppers = sk.get_steppers();
    ///        assert_eq!(steppers.get_count(), 0);
    ///        assert_eq!(sk.get_steppers_count(), 0);
    ///     }
    /// );
    /// ```
    pub fn get_steppers(&self) -> &Steppers {
        &self.steppers
    }

    /// Get an event_loop_proxy clone to send events
    ///
    /// see also [`SkInfo::event_loop_proxy_from`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::sk::{Sk, SkSettings, AppMode, SkInfo, QuitReason};
    /// use stereokit_rust::framework::SkClosures;
    /// use std::thread;
    /// use std::time::Duration;
    ///
    /// let event_loop_proxy = sk.get_event_loop_proxy();
    ///
    /// // Spawn a new thread to send an event after a delay
    /// let handle = thread::spawn(move || {
    ///     thread::sleep(Duration::from_millis(100));
    ///     if let Some(proxy) = event_loop_proxy {
    ///         proxy.send_event(StepperAction::quit("thread", "I'm done!")).unwrap();
    ///     }
    /// });
    ///
    /// SkClosures::new(sk, |sk, token|  {
    ///     // Only the thread can stop this test
    ///     // model.draw(token, Matrix::IDENTITY, None, None);
    /// })
    /// .shutdown(|sk| {
    ///     assert_eq!(sk.get_quit_reason(), QuitReason::User);
    /// })
    /// .run(event_loop);
    ///
    /// // If we are here the thread has finished
    /// handle.join().unwrap();
    /// ```
    pub fn get_event_loop_proxy(&self) -> Option<EventLoopProxy<StepperAction>> {
        let sk = self.sk_info.as_ref();
        sk.borrow().get_event_loop_proxy()
    }

    /// Steps all StereoKit systems, and inserts user code via callback between the appropriate system updates.
    /// <https://stereokit.net/Pages/StereoKit/SK/Step.html>
    ///
    /// see also [`sk_step`]
    #[deprecated(since = "0.40.0", note = "see [crate::framework::SkClosures] instead")]
    pub fn step_looped<F: FnMut(&mut Sk)>(&mut self, on_step: &mut F) -> bool {
        if unsafe { sk_step(None) } == 0 {
            return false;
        }
        if !self.steppers.step(&mut self.token) {
            self.quit(None)
        };

        while let Some(mut action) = self.actions.pop_front() {
            action();
        }

        on_step(self);

        true
    }

    // A way to execute without event_loop frame. This can be use only for PC programs
    // or android ones having a _main() derived with #ndk-glue (warning ndk-glue is deprecated)
    // <https://stereokit.net/Pages/StereoKit/SK/Run.html>
    //
    // see also [`sk_run_data`]
    // pub fn run_raw<U: FnMut(&mut Sk), S: FnMut(&mut Sk)>(mut self, mut on_step: U, mut on_shutdown: S) {
    //     while self.step(&mut on_step) {}
    //     on_shutdown(&mut self);
    //     self.shutdown();
    // }

    // An alternative and basic way to execute a stereokit without ISteppers. This can be use only for PC programs
    // or android ones having a _main() derived with #ndk-glue (warning ndk-glue is deprecated)
    // <https://stereokit.net/Pages/StereoKit/SK.html>
    //
    // see also [`sk_run_data`]
    // pub fn run_basic<U: FnMut(&mut Sk), S: FnMut(&mut Sk)>(mut self, mut on_update: U, mut on_shutdown: S) {
    //     let mut update_ref: (&mut U, &mut &mut Sk) = (&mut on_update, &mut &mut self);
    //     let update_raw = &mut update_ref as *mut (&mut U, &mut &mut Sk) as *mut c_void;
    //     let mut shutdown_ref: (&mut S, &mut &mut Sk) = (&mut on_shutdown, &mut &mut self);
    //     let shutdown_raw = &mut shutdown_ref as *mut (&mut S, &mut &mut Sk) as *mut c_void;
    //     unsafe {
    //         sk_run_data(Some(sk_trampoline::<U>), update_raw, Some(sk_trampoline::<S>), shutdown_raw);
    //     }
    // }

    /// This passes application execution over to StereoKit. This continuously steps all StereoKit systems, and inserts
    /// user code via callback between the appropriate system updates. Once execution completes, or SK.Quit is called,
    /// it properly calls the shutdown callback and shuts down StereoKit for you.
    ///
    /// This method is a basic way to handle event_loop. You can, instead, implement this loop in your main thread.
    /// <https://stereokit.net/Pages/StereoKit/SK/Run.html>
    ///
    /// see also [`sk_run_data`]
    #[deprecated(since = "0.40.0", note = "see [crate::framework::SkClosures] instead")]
    pub fn run<U: FnMut(&mut Sk), S: FnMut(&mut Sk)>(
        mut self,
        event_loop: EventLoop<StepperAction>,
        mut on_step: U,
        mut on_shutdown: S,
    ) {
        event_loop.set_control_flow(ControlFlow::Poll);
        #[allow(deprecated)]
        event_loop
            .run(move |event, elwt| match event {
                Event::NewEvents(_start_cause) => {} // Quest flood this : Log::diag(format!("NewEvents {:?}", start_cause)),
                Event::WindowEvent { window_id, event } => {
                    Log::diag(format!("WindowEvent {:?} -> {:?}", window_id, event))
                }
                Event::DeviceEvent { device_id, event } => {
                    Log::diag(format!("DeviceEvent {:?} -> {:?}", device_id, event))
                }
                Event::UserEvent(action) => {
                    Log::diag(format!("UserEvent {:?}", action));
                    self.send_event(action);
                }
                Event::Suspended => Log::info("Suspended !!"),
                Event::Resumed => Log::info("Resumed !!"),
                Event::AboutToWait => {
                    if !&self.step_looped(&mut on_step) {
                        elwt.exit()
                    }
                }
                Event::LoopExiting => {
                    Log::info("LoopExiting !!");
                    on_shutdown(&mut self);
                }
                Event::MemoryWarning => Log::warn("MemoryWarning !!"),
            })
            .unwrap_or_else(|e| {
                Log::err(format!("!!!event_loop error closing!! : {}", e));
            });
    }
}
