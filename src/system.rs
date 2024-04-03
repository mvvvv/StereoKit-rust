use std::{
    char,
    ffi::{c_char, c_ushort, c_void, CStr, CString},
    fmt,
    mem::size_of,
    path::Path,
    ptr::{null, null_mut, NonNull},
};

use crate::{
    anchor::{Anchor, _AnchorT},
    font::{Font, FontT, _FontT},
    material::{Material, MaterialT, _MaterialT},
    maths::{ray_from_mouse, Bool32T, Matrix, Pose, Quat, Ray, Rect, Vec2, Vec3},
    mesh::{Mesh, MeshT, _MeshT},
    model::{Model, ModelT, _ModelT},
    shader::{Shader, ShaderT, _ShaderT},
    sk::OriginMode,
    sound::{Sound, SoundT, _SoundT},
    sprite::{Sprite, _SpriteT},
    tex::{Tex, TexFormat, TexT, _TexT},
    util::{Color128, Color32, SphericalHarmonics},
    StereoKitError,
};

/// All StereoKit assets implement this interface! This is mostly to help group and hold Asset objects, and is
/// particularly useful when working with Assets at a high level with the Assets class.
/// <https://stereokit.net/Pages/StereoKit/IAsset.html>
pub trait IAsset {
    /// sets the unique identifier of this asset resource! This can be helpful for debugging, managine your assets, or
    /// finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/IAsset/Id.html>
    //fn id(&mut self, id: impl AsRef<str>);

    /// gets the unique identifier of this asset resource! This can be helpful for debugging, managine your assets, or
    /// finding them later on!
    /// <https://stereokit.net/Pages/StereoKit/IAsset/Id.html>
    fn get_id(&self) -> &str;
}

/// StereoKit uses an asynchronous loading system to prevent assets from blocking execution! This means that asset
/// loading systems will return an asset to you right away, even though it is still being processed in the background.
/// <https://stereokit.net/Pages/StereoKit/AssetState.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum AssetState {
    /// This asset encountered an issue when parsing the source data. Either the format is unrecognized by StereoKit,
    /// or the data may be corrupt. Check the logs for additional details.
    Unsupported = -3,
    /// The asset data was not found! This is most likely an issue with a bad file path, or file permissions. Check
    /// the logs for additional details.
    NotFound = -2,
    /// An unknown error occurred when trying to load the asset! Check the logs for additional details.
    Error = -1,
    /// This asset is in its default state. It has not been told to load anything, nor does it have any data!
    None = 0,
    /// This asset is currently queued for loading, but hasn’t received any data yet. Attempting to access metadata or
    /// asset data will result in blocking the app’s execution until that data is loaded!
    Loading = 1,
    /// This asset is still loading, but some of the higher level data is already available for inspection without
    /// blocking the app. Attempting to access the core asset data will result in blocking the app’s execution until
    /// that data is loaded!
    LoadedMeta = 2,
    /// This asset is completely loaded without issues, and is ready for use!
    Loaded = 3,
}

/// A flag for what ‘type’ an Asset may store.
///
/// None -> No type, this may come from some kind of invalid Asset id.
/// <https://stereokit.net/Pages/StereoKit/AssetType.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum AssetType {
    None = 0,
    Mesh = 1,
    Tex = 2,
    Shader = 3,
    Material = 4,
    Model = 5,
    Font = 6,
    Sprite = 7,
    Sound = 8,
    Solid = 9,
    Anchor = 10,
}

/// If you want to manage loading assets, this is the class for you!
///  <https://stereokit.net/Pages/StereoKit/Assets.html>
pub struct Assets;

pub type AssetT = *mut c_void;

extern "C" {
    pub fn assets_releaseref_threadsafe(asset: *mut c_void);
    pub fn assets_current_task() -> i32;
    pub fn assets_total_tasks() -> i32;
    pub fn assets_current_task_priority() -> i32;
    pub fn assets_block_for_priority(priority: i32);
    pub fn assets_count() -> i32;
    pub fn assets_get_index(index: i32) -> AssetT;
    pub fn assets_get_type(index: i32) -> AssetType;
    pub fn asset_get_type(asset: AssetT) -> AssetType;
    pub fn asset_set_id(asset: AssetT, id: *const c_char);
    pub fn asset_get_id(asset: AssetT) -> *const c_char;
    pub fn asset_addref(asset: AssetT);
    pub fn asset_release(asset: AssetT);
}

/// Non-canonical structure to store an asset and avoid reducer Box<dyn Asset>
#[derive(Debug)]
pub enum Asset {
    None,
    Mesh(Mesh),
    Tex(Tex),
    Shader(Shader),
    Material(Material),
    Model(Model),
    Font(Font),
    Sprite(Sprite),
    Sound(Sound),
    Solid(*mut c_void),
    Anchor(Anchor),
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Asset::None => write!(f, "None"),
            Asset::Mesh(v) => write!(f, "Mesh : {}", v.get_id()),
            Asset::Tex(v) => write!(f, "Tex : {}", v.get_id()),
            Asset::Shader(v) => write!(f, "Shader : {}", v.get_id()),
            Asset::Material(v) => write!(f, "Material : {}", v.get_id()),
            Asset::Model(v) => write!(f, "Model : {}", v.get_id()),
            Asset::Font(v) => write!(f, "Font : {}", v.get_id()),
            Asset::Sprite(v) => write!(f, "Sprite : {}", v.get_id()),
            Asset::Sound(v) => write!(f, "Sound : {}", v.get_id()),
            Asset::Solid(_) => write!(f, "Solid : ... deprecated ..."),
            Asset::Anchor(v) => write!(f, "Anchor : {}", v.get_id()),
        }
    }
}

/// Iterator on assets
///
/// see also [Assets::all][Assets::type]
#[derive(Debug, Copy, Clone)]
pub struct AssetIter {
    index: i32,
    asset_type: AssetType,
}

impl Iterator for AssetIter {
    type Item = Asset;

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;
        let count = unsafe { assets_count() };
        if self.asset_type == AssetType::None {
            if self.index < count {
                match unsafe { assets_get_type(self.index) } {
                    AssetType::None => {
                        Log::err(format!("Asset at index {:?}/{:?} is AssetType::None", self.index, count));
                        None
                    }
                    asset_type => {
                        let asset_id = unsafe { assets_get_index(self.index) };
                        Some(self.to_asset(asset_type, asset_id))
                    }
                }
            } else {
                None
            }
        } else {
            while self.index < count {
                if unsafe { assets_get_type(self.index) } == self.asset_type {
                    let asset_id = unsafe { assets_get_index(self.index) };
                    return Some(self.to_asset(self.asset_type, asset_id));
                } else {
                    self.index += 1;
                }
            }
            None
        }
    }
}

impl AssetIter {
    /// Get the asset
    fn to_asset(self, asset_type: AssetType, c_id: *mut c_void) -> Asset {
        match asset_type {
            AssetType::None => Asset::None,
            AssetType::Mesh => Asset::Mesh(Mesh(NonNull::new(c_id as *mut _MeshT).unwrap())),
            AssetType::Tex => Asset::Tex(Tex(NonNull::new(c_id as *mut _TexT).unwrap())),
            AssetType::Shader => Asset::Shader(Shader(NonNull::new(c_id as *mut _ShaderT).unwrap())),
            AssetType::Material => Asset::Material(Material(NonNull::new(c_id as *mut _MaterialT).unwrap())),
            AssetType::Model => Asset::Model(Model(NonNull::new(c_id as *mut _ModelT).unwrap())),
            AssetType::Font => Asset::Font(Font(NonNull::new(c_id as *mut _FontT).unwrap())),
            AssetType::Sprite => Asset::Sprite(Sprite(NonNull::new(c_id as *mut _SpriteT).unwrap())),
            AssetType::Sound => Asset::Sound(Sound(NonNull::new(c_id as *mut _SoundT).unwrap())),
            AssetType::Solid => todo!("Solids are deprecated!"),
            AssetType::Anchor => Asset::Anchor(Anchor(NonNull::new(c_id as *mut _AnchorT).unwrap())),
        }
    }

    /// Get an iterator upon all assets loaded if asset_type is None or only assets of the given AssetType
    /// <https://stereokit.net/Pages/StereoKit/Assets.html>
    pub fn iterate(asset_type: Option<AssetType>) -> AssetIter {
        let asset_type = asset_type.unwrap_or(AssetType::None);
        AssetIter { index: -1, asset_type }
    }
}

impl Assets {
    /// A list of supported model format extensions. This pairs pretty well with Platform::file_picker when attempting to
    /// load a Model!
    /// <https://stereokit.net/Pages/StereoKit/Assets/ModelFormats.html>
    pub const MODEL_FORMATS: [&'static str; 5] = [".gltf", ".glb", ".obj", ".stl", ".ply"];

    /// A list of supported texture format extensions. This pairs pretty well with Platform::file_picker when attempting
    /// to load a Tex!
    /// <https://stereokit.net/Pages/StereoKit/Assets/TextureFormats.html>
    pub const TEXTURE_FORMATS: [&'static str; 10] =
        [".jpg", ".jpeg", ".png", ".hdr", ".tga", ".bmp", ".psd", ".pic", ".qoi", ".gif"];

    /// This is an iterator upon all assets loaded by StereoKit at the current moment.
    /// <https://stereokit.net/Pages/StereoKit/Assets/All.html>
    pub fn all() -> AssetIter {
        AssetIter::iterate(None)
    }

    /// This is an iterator upon all assets matching the specified type.
    /// <https://stereokit.net/Pages/StereoKit/Assets/Type.html>
    pub fn all_of_type(asset_type: AssetType) -> AssetIter {
        AssetIter::iterate(Some(asset_type))
    }

    /// This is the index of the current asset loading task. Note that to load one asset, multiple tasks are generated.
    /// <https://stereokit.net/Pages/StereoKit/Assets/CurrentTask.html>
    ///
    /// see also [crate::system::asset_current_task]
    pub fn current_task() -> i32 {
        unsafe { assets_current_task() }
    }

    /// StereoKit processes tasks in order of priority. This returns the priority of the current task, and can be used
    /// to wait until all tasks within a certain priority range have been completed.
    /// <https://stereokit.net/Pages/StereoKit/Assets/CurrentTaskPriority.html>
    ///
    /// see also [crate::system::asset_current_task_priority]
    pub fn current_task_priority() -> i32 {
        unsafe { assets_current_task_priority() }
    }

    /// This is the total number of tasks that have been added to the loading system, including all completed and
    /// pending tasks. Note that to load one asset, multiple tasks are generated.
    /// <https://stereokit.net/Pages/StereoKit/Assets/TotalTasks.html>
    ///
    /// see also [crate::system::assets_total_tasks]
    pub fn total_tasks() -> i32 {
        unsafe { assets_total_tasks() }
    }

    /// This will block the execution of the application until all asset tasks below the priority value have completed
    /// loading. To block until all assets are loaded, pass in i32::MAX for the priority.
    /// <https://stereokit.net/Pages/StereoKit/Assets/BlockForPriority.html>
    ///
    /// see also [crate::system::assets_block_for_priority]
    pub fn block_for_priority(priority: i32) {
        unsafe { assets_block_for_priority(priority) }
    }
}

/// This describes what technology is being used to power StereoKit’s XR backend.
/// <https://stereokit.net/Pages/StereoKit/BackendXRType.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum BackendXRType {
    /// StereoKit is not using an XR backend of any sort. That means the application is flatscreen and has the simulator
    /// disabled.
    None = 0,
    /// StereoKit is using the flatscreen XR simulator. Inputs are emulated, and some advanced XR functionality may not
    /// be available.
    Simulator = 1,
    /// StereoKit is currently powered by OpenXR! This means we’re running on a real XR device. Not all OpenXR runtimes
    /// provide the same functionality, but we will have access to more fun stuff :)
    OpenXR = 2,
    /// StereoKit is running in a browser, and is using WebXR!
    WebXR = 3,
}

/// This describes the platform that StereoKit is running on.
/// <https://stereokit.net/Pages/StereoKit/BackendPlatform.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum BackendPlatform {
    /// This is running as a Windows app using the Win32 APIs.
    Win32 = 0,
    /// This is running as a Windows app using the UWP APIs.
    Uwp = 1,
    /// This is running as a Linux app.
    Linux = 2,
    /// This is running as an Android app.
    Android = 3,
    /// This is running in a browser.
    Web = 4,
}

/// This describes the graphics API thatStereoKit is using for rendering.
/// <https://stereokit.net/Pages/StereoKit/BackendGraphics.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum BackendGraphics {
    /// An invalid default value.
    None = 0,
    /// DirectX’s Direct3D11 is used for rendering! This is used by default on Windows.
    D3D11 = 1,
    /// OpenGL is used for rendering, using GLX (OpenGL Extension to the X Window System) for loading. This is used by
    /// default on Linux.
    OpenGLGLX = 2,
    /// OpenGL is used for rendering, using WGL (Windows Extensions to OpenGL) for loading. Native developers can
    /// configure SK to use this on Windows.
    OpenGLWGL = 3,
    /// OpenGL ES is used for rendering, using EGL (EGL Native Platform Graphics Interface) for loading. This is used by
    /// default on Android, and native developers can configure SK to use this on Linux.
    OpenGLESEGL = 4,
    /// WebGL is used for rendering. This is used by default on Web.
    WebGL = 5,
}

pub type OpenXRHandleT = u64;

///  <https://stereokit.net/Pages/StereoKit/Backend.html>
pub struct Backend;

extern "C" {
    pub fn backend_xr_get_type() -> BackendXRType;
    pub fn backend_openxr_get_instance() -> OpenXRHandleT;
    pub fn backend_openxr_get_session() -> OpenXRHandleT;
    pub fn backend_openxr_get_system_id() -> OpenXRHandleT;
    pub fn backend_openxr_get_space() -> OpenXRHandleT;
    pub fn backend_openxr_get_time() -> i64;
    pub fn backend_openxr_get_eyes_sample_time() -> i64;
    pub fn backend_openxr_get_function(function_name: *const c_char) -> *mut c_void;
    pub fn backend_openxr_ext_enabled(extension_name: *const c_char) -> Bool32T;
    pub fn backend_openxr_ext_request(extension_name: *const c_char);
    pub fn backend_openxr_ext_exclude(extension_name: *const c_char);
    pub fn backend_openxr_use_minimum_exts(use_minimum_exts: Bool32T);
    pub fn backend_openxr_composition_layer(XrCompositionLayerBaseHeader: *mut c_void, data_size: i32, sort_order: i32);
    pub fn backend_openxr_end_frame_chain(XrBaseHeader: *mut c_void, data_size: i32);
    pub fn backend_openxr_set_hand_joint_scale(joint_scale_factor: f32);
    pub fn backend_openxr_add_callback_pre_session_create(
        xr_pre_session_create_callback: ::std::option::Option<unsafe extern "C" fn(context: *mut c_void)>,
        context: *mut c_void,
    );
    pub fn backend_openxr_add_callback_poll_event(
        xr_poll_event_callback: ::std::option::Option<
            unsafe extern "C" fn(context: *mut c_void, XrEventDataBuffer: *mut c_void),
        >,
        context: *mut c_void,
    );
    pub fn backend_openxr_remove_callback_poll_event(
        xr_poll_event_callback: ::std::option::Option<
            unsafe extern "C" fn(context: *mut c_void, XrEventDataBuffer: *mut c_void),
        >,
    );
    pub fn backend_platform_get() -> BackendPlatform;
    pub fn backend_android_get_java_vm() -> *mut c_void;
    pub fn backend_android_get_activity() -> *mut c_void;
    pub fn backend_android_get_jni_env() -> *mut c_void;
    pub fn backend_graphics_get() -> BackendGraphics;
    pub fn backend_d3d11_get_d3d_device() -> *mut c_void;
    pub fn backend_d3d11_get_d3d_context() -> *mut c_void;
    pub fn backend_d3d11_get_deferred_d3d_context() -> *mut c_void;
    pub fn backend_d3d11_get_deferred_mtx() -> *mut c_void;
    pub fn backend_d3d11_get_main_thread_id() -> u32;
    pub fn backend_opengl_wgl_get_hdc() -> *mut c_void;
    pub fn backend_opengl_wgl_get_hglrc() -> *mut c_void;
    pub fn backend_opengl_glx_get_context() -> *mut c_void;
    pub fn backend_opengl_glx_get_display() -> *mut c_void;
    pub fn backend_opengl_glx_get_drawable() -> *mut c_void;
    pub fn backend_opengl_egl_get_context() -> *mut c_void;
    pub fn backend_opengl_egl_get_config() -> *mut c_void;
    pub fn backend_opengl_egl_get_display() -> *mut c_void;
}

impl Backend {
    /// This describes the graphics API thatStereoKit is using for rendering. StereoKit uses D3D11 for Windows platforms,
    /// and a flavor of OpenGL for Linux, Android, and Web.
    /// <https://stereokit.net/Pages/StereoKit/Backend/Graphics.html>
    ///
    /// see also [crate::system::backend_graphics_get]
    pub fn graphics() -> BackendGraphics {
        unsafe { backend_graphics_get() }
    }

    /// What kind of platform is StereoKit running on? This can be important to tell you what APIs or functionality is
    /// available to the app.
    /// <https://stereokit.net/Pages/StereoKit/Backend/Platform.html>
    ///
    /// see also [crate::system::backend_platform_get]
    pub fn platform() -> BackendPlatform {
        unsafe { backend_platform_get() }
    }

    /// What technology is being used to drive StereoKit’s XR functionality? OpenXR is the most likely candidate here,
    /// but if you’re running the flatscreen Simulator, or running in the web with WebXR, then this will reflect that.
    /// <https://stereokit.net/Pages/StereoKit/Backend/XRType.html>
    ///
    /// see also [crate::system::backend_xr_get_type]
    pub fn xr_type() -> BackendXRType {
        unsafe { backend_xr_get_type() }
    }
}

/// This class is NOT of general interest, unless you are trying to add support for some unusual OpenXR extension!
/// StereoKit should do all the OpenXR work that most people will need. If you find yourself here anyhow for something
/// you feel StereoKit should support already, please add a feature request on GitHub!
///
/// This class contains handles and methods for working directly with OpenXR. This may allow you to activate or work
/// with OpenXR extensions that StereoKit hasn’t implemented or exposed yet. Check that Backend.XRType is OpenXR before
/// using any of this.
///
/// These properties may best be used with some external OpenXR binding library, but you may get some limited mileage
/// with the API as provided here.
/// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR.html>
pub struct BackendOpenXR;

impl BackendOpenXR {
    /// Type: XrTime. This is the OpenXR time of the eye tracker sample associated with the current value of .
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/EyesSampleTime.html>
    ///
    /// see also [crate::system::backend_openxr_get_eyes_sample_time]
    pub fn eyes_sample_time() -> i64 {
        unsafe { backend_openxr_get_eyes_sample_time() }
    }

    /// Type: XrInstance. StereoKit’s instance handle, valid after Sk.initialize.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/Instance.html>
    ///
    /// see also [crate::system::backend_openxr_get_instance]
    pub fn instance() -> OpenXRHandleT {
        unsafe { backend_openxr_get_instance() }
    }

    /// Type: XrSession. StereoKit’s current session handle, this will be valid after SK.Initialize, but the session may
    /// not be started quite so early.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/Session.html>
    ///
    /// see also [crate::system::backend_openxr_get_session]
    pub fn session() -> OpenXRHandleT {
        unsafe { backend_openxr_get_session() }
    }

    /// Type: XrSpace. StereoKit’s primary coordinate space, valid after SK.Initialize, this will most likely be created
    /// from XR_REFERENCE_SPACE_TYPE_UNBOUNDED_MSFT or XR_REFERENCE_SPACE_TYPE_LOCAL.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/Space.html>
    ///
    /// see also [crate::system::backend_openxr_get_space]
    pub fn space() -> OpenXRHandleT {
        unsafe { backend_openxr_get_space() }
    }

    /// Type: XrSystemId. This is the id of the device StereoKit is currently using! This is the result of calling
    /// xrGetSystem with XR_FORM_FACTOR_HEAD_MOUNTED_DISPLAY.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/SystemId.html>
    ///
    /// see also [crate::system::backend_openxr_get_system_id]
    pub fn system_id() -> OpenXRHandleT {
        unsafe { backend_openxr_get_system_id() }
    }

    /// Type: XrTime. This is the OpenXR time for the current frame, and is available after Sk.initialize.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/Time.html>
    ///
    /// see also [crate::system::backend_openxr_get_time]
    pub fn time() -> i64 {
        unsafe { backend_openxr_get_time() }
    }

    /// Tells StereoKit to request only the extensions that are absolutely critical to StereoKit. You can still request
    /// extensions via OpenXR.RequestExt, and this can be used to opt-in to extensions that StereoKit would normally
    /// request automatically.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/UseMinimumExts.html>
    ///
    /// see also [crate::system::backend_openxr_use_minimum_exts]
    pub fn use_minimum_exts(value: bool) {
        unsafe { backend_openxr_use_minimum_exts(value as Bool32T) }
    }

    /// This allows you to add XrCompositionLayers to the list that StereoKit submits to xrEndFrame. You must call this
    /// every frame you wish the layer to be included.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/AddCompositionLayer.html>
    ///
    /// see also [crate::system::backend_openxr_composition_layer]
    pub fn add_composition_layer<T>(xr_composition_layer_x: &mut T, sort_order: i32) {
        let size = size_of::<T>();
        let ptr = xr_composition_layer_x as *mut _ as *mut c_void;
        unsafe { backend_openxr_composition_layer(ptr, size as i32, sort_order) }
    }

    /// This adds an item to the chain of objects submitted to StereoKit’s xrEndFrame call!
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/AddEndFrameChain.html>
    ///
    /// see also [crate::system::backend_openxr_end_frame_chain]
    pub fn add_end_frame_chain<T>(xr_base_header: &mut T) {
        let size = size_of::<T>();
        let ptr = xr_base_header as *mut _ as *mut c_void;
        unsafe { backend_openxr_end_frame_chain(ptr, size as i32) }
    }

    /// This ensures that StereoKit does not load a particular extension! StereoKit will behave as if the extension is
    /// not available on the device. It will also be excluded even if you explicitly requested it with RequestExt
    /// earlier, or afterwards. This MUST be called before SK.Initialize.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/ExcludeExt.html>
    ///
    /// see also [crate::system::backend_openxr_ext_exclude]
    pub fn exclude_ext(extension_name: impl AsRef<str>) {
        let c_str = CString::new(extension_name.as_ref()).unwrap();
        unsafe { backend_openxr_ext_exclude(c_str.as_ptr()) }
    }

    /// Requests that OpenXR load a particular extension. This MUST be called before SK.Initialize. Note that it’s
    /// entirely possible that your extension will not load on certain runtimes, so be sure to check ExtEnabled to see
    /// if it’s available to use.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/RequestExt.html>
    ///
    /// see also [crate::system::backend_openxr_ext_request]
    pub fn request_ext(extension_name: impl AsRef<str>) {
        let c_str = CString::new(extension_name.as_ref()).unwrap();
        unsafe { backend_openxr_ext_request(c_str.as_ptr()) }
    }

    /// This tells if an OpenXR extension has been requested and successfully loaded by the runtime. This MUST only be
    /// called after SK.Initialize.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/ExtEnabled.html>
    ///
    /// see also [crate::system::backend_openxr_ext_enabled]
    pub fn ext_enabled(extension_name: impl AsRef<str>) -> bool {
        let c_str = CString::new(extension_name.as_ref()).unwrap();
        unsafe { backend_openxr_ext_enabled(c_str.as_ptr()) != 0 }
    }

    /// This is basically xrGetInstanceProcAddr from OpenXR, you can use this to get and call functions from an
    /// extension you’ve loaded. You can use Marshal.GetDelegateForFunctionPointer to turn the result into a delegate
    /// that you can call.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/GetFunctionPtr.html>
    ///
    /// see also [crate::system::backend_openxr_get_function]
    pub fn get_function_ptr(function_name: impl AsRef<str>) -> *mut c_void {
        let c_str = CString::new(function_name.as_ref()).unwrap();
        unsafe { backend_openxr_get_function(c_str.as_ptr()) }
    }

    /// This sets a scaling value for joints provided by the articulated hand extension. Some systems just don’t seem to
    /// get their joint sizes right!
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenXR/SetHandJointScale.html>
    ///
    /// see also [crate::system::backend_openxr_set_hand_joint_scale]
    pub fn set_hand_joint_scale(joint_scale_factor: f32) {
        unsafe { backend_openxr_set_hand_joint_scale(joint_scale_factor) }
    }
}

/// This class contains variables that may be useful for interop with the Android operating system, or other Android
/// libraries.
/// <https://stereokit.net/Pages/StereoKit/Backend.Android.html>
pub struct BackendAndroid;

impl BackendAndroid {
    /// This is the jobject activity that StereoKit uses on Android. This is only valid after Sk.initialize, on Android
    /// systems.
    /// <https://stereokit.net/Pages/StereoKit/Backend.Android/Activity.html>
    ///
    /// see also [crate::system::backend_android_get_activity]
    pub fn activity() -> *mut c_void {
        unsafe { backend_android_get_activity() }
    }

    /// This is the JavaVM* object that StereoKit uses on Android. This is only valid after Sk.initialize, on Android
    /// systems.
    /// <https://stereokit.net/Pages/StereoKit/Backend.Android/JavaVM.html>
    ///
    /// see also [crate::system::backend_android_get_java_vm]
    pub fn java_vm() -> *mut c_void {
        unsafe { backend_android_get_java_vm() }
    }

    /// This is the JNIEnv* object that StereoKit uses on Android. This is only valid after Sk.initialize, on Android
    /// systems.
    /// <https://stereokit.net/Pages/StereoKit/Backend.Android/JNIEnvironment.html>
    ///
    /// see also [crate::system::backend_android_get_jni_env]
    pub fn jni_environment() -> *mut c_void {
        unsafe { backend_android_get_jni_env() }
    }
}

/// When using Direct3D11 for rendering, this contains a number of variables that may be useful for doing advanced
/// rendering tasks. This is the default rendering backend on Windows.
/// <https://stereokit.net/Pages/StereoKit/Backend.D3D11.html>
pub struct BackendD3D11;

impl BackendD3D11 {
    /// This is the main ID3D11DeviceContext* StereoKit uses for rendering.
    /// <https://stereokit.net/Pages/StereoKit/Backend.D3D11/D3DContext.html>
    ///
    /// see also [crate::system::backend_d3d11_get_d3d_context]
    pub fn d3d_context() -> *mut c_void {
        unsafe { backend_d3d11_get_d3d_context() }
    }

    /// This is the main ID3D11Device* StereoKit uses for rendering.
    /// <https://stereokit.net/Pages/StereoKit/Backend.D3D11/D3DDevice.html>
    ///
    /// see also [crate::system::backend_d3d11_get_d3d_device]
    pub fn d3d_device() -> *mut c_void {
        unsafe { backend_d3d11_get_d3d_device() }
    }
}

/// When using OpenGL with the WGL loader for rendering, this contains a number of variables that may be useful for
/// doing advanced rendering tasks. This is Windows only, and requires gloabally defining SKG_FORCE_OPENGL when building
/// the core StereoKitC library.
/// <https://stereokit.net/Pages/StereoKit/Backend.OpenGL_WGL.html>
pub struct OpenGLWGL;

impl OpenGLWGL {
    /// This is the Handle to Device Context HDC StereoKit uses with wglMakeCurrent.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenGL_WGL/HDC.html>
    ///
    /// see also [crate::system::backend_opengl_wgl_get_hdc]
    pub fn hdc() -> *mut c_void {
        unsafe { backend_opengl_wgl_get_hdc() }
    }

    /// This is the Handle to an OpenGL Rendering Context HGLRC StereoKit uses with wglMakeCurrent.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenGL_WGL/HGLRC.html>
    ///
    /// see also [crate::system::backend_opengl_wgl_get_hglrc]
    pub fn hglrc() -> *mut c_void {
        unsafe { backend_opengl_wgl_get_hglrc() }
    }
}

/// When using OpenGL ES with the EGL loader for rendering, this contains a number of variables that may be useful for
/// doing advanced rendering tasks. This is the default rendering backend for Android, and Linux builds can be
/// configured to use this with the SK_LINUX_EGL cmake option when building the core StereoKitC library.
/// <https://stereokit.net/Pages/StereoKit/Backend.OpenGLES_EGL.html>
pub struct OpenGLESEGL;

impl OpenGLESEGL {
    /// This is the EGLContext StereoKit receives from eglCreateContext.
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenGLES_EGL/Context.html>
    ///
    /// see also [crate::system::backend_opengl_egl_get_context]
    pub fn context() -> *mut c_void {
        unsafe { backend_opengl_egl_get_context() }
    }

    /// This is the EGLDisplay StereoKit receives from eglGetDisplay
    /// <https://stereokit.net/Pages/StereoKit/Backend.OpenGLES_EGL/Display.html>
    ///
    /// see also [crate::system::backend_opengl_egl_get_display]
    pub fn display() -> *mut c_void {
        unsafe { backend_opengl_egl_get_display() }
    }
}

/// This class represents a stack of transform matrices that build up a transform hierarchy! This can be used like an
/// object-less parent-child system, where you push a parent’s transform onto the stack, render child objects relative
/// to that parent transform and then pop it off the stack.
///
/// Performance note: if any matrices are on the hierarchy stack, any render will cause a matrix multiplication to
/// occur! So if you have a collection of objects with their transforms baked and cached into matrices for performance
/// reasons, you’ll want to ensure there are no matrices in the hierarchy stack, or that the hierarchy is disabled!
/// It’ll save you a matrix multiplication in that case :)
/// <https://stereokit.net/Pages/StereoKit/Hierarchy.html>
pub struct Hierarchy;

extern "C" {
    pub fn hierarchy_push(transform: *const Matrix);
    pub fn hierarchy_pop();
    pub fn hierarchy_set_enabled(enabled: Bool32T);
    pub fn hierarchy_is_enabled() -> Bool32T;
    pub fn hierarchy_to_world() -> *const Matrix;
    pub fn hierarchy_to_local() -> *const Matrix;
    pub fn hierarchy_to_local_point(world_pt: *const Vec3) -> Vec3;
    pub fn hierarchy_to_local_direction(world_dir: *const Vec3) -> Vec3;
    pub fn hierarchy_to_local_rotation(world_orientation: *const Quat) -> Quat;
    pub fn hierarchy_to_local_pose(world_pose: *const Pose) -> Pose;
    pub fn hierarchy_to_local_ray(world_ray: Ray) -> Ray;
    pub fn hierarchy_to_world_point(local_pt: *const Vec3) -> Vec3;
    pub fn hierarchy_to_world_direction(local_dir: *const Vec3) -> Vec3;
    pub fn hierarchy_to_world_rotation(local_orientation: *const Quat) -> Quat;
    pub fn hierarchy_to_world_pose(local_pose: *const Pose) -> Pose;
    pub fn hierarchy_to_world_ray(local_ray: Ray) -> Ray;
}

impl Hierarchy {
    /// This is enabled by default. Disabling this will cause any draw call to ignore any Matrices that are on the
    /// Hierarchy stack.
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/Enabled.html>
    ///
    /// see also [crate::system::hierarchy_set_enabled]
    pub fn enabled(enable: bool) {
        unsafe { hierarchy_set_enabled(enable as Bool32T) }
    }

    /// This is enabled by default. Disabling this will cause any draw call to ignore any Matrices that are on the
    /// Hierarchy stack.
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/Enabled.html>
    ///
    /// see also [crate::system::hierarchy_is_enabled]
    pub fn is_enabled() -> bool {
        unsafe { hierarchy_is_enabled() != 0 }
    }

    /// Removes the top Matrix from the stack!
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/Pop.html>
    ///
    /// see also [crate::system::hierarchy_pop]
    pub fn pop() {
        unsafe { hierarchy_pop() }
    }

    /// Pushes a transform Matrix onto the stack, and combines it with the Matrix below it. Any draw operation’s Matrix
    /// will now be combined with this Matrix to make it relative to the current hierarchy. Use Hierarchy.pop to remove
    /// it from the Hierarchy stack! All Push calls must have an accompanying Pop call.
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/Push.html>
    ///
    /// see also [crate::system::hierarchy_pop]
    pub fn push<M: Into<Matrix>>(transform: M) {
        unsafe { hierarchy_push(&transform.into()) }
    }

    /// Converts a world space point into the local space of the current Hierarchy stack!
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToLocal.html>
    ///
    /// see also [crate::system::hierarchy_to_local_point]
    pub fn to_local_point<V: Into<Vec3>>(world_point: V) -> Vec3 {
        unsafe { hierarchy_to_local_point(&world_point.into()) }
    }

    /// Converts a world space rotation into the local space of the current Hierarchy stack!
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToLocal.html>
    ///
    /// see also [crate::system::hierarchy_to_local_rotation]
    pub fn to_local_rotation<Q: Into<Quat>>(world_orientation: Q) -> Quat {
        unsafe { hierarchy_to_local_rotation(&world_orientation.into()) }
    }

    /// Converts a world pose relative to the current hierarchy stack into local space!
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToLocal.html>
    ///
    /// see also [crate::system::hierarchy_to_local_pose]
    pub fn to_local_pose<P: Into<Pose>>(world_pose: P) -> Pose {
        unsafe { hierarchy_to_local_pose(&world_pose.into()) }
    }

    /// Converts a world space direction into the local space of the current Hierarchy stack! This excludes the
    /// translation component normally applied to vectors, so it’s still a valid direction.
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToLocalDirection.html>
    ///
    /// see also [crate::system::hierarchy_to_local_direction]
    pub fn to_local_direction<V: Into<Vec3>>(world_direction: V) -> Vec3 {
        unsafe { hierarchy_to_local_direction(&world_direction.into()) }
    }

    /// Converts a local point relative to the current hierarchy stack into world space!
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToWorld.html>
    ///
    /// see also [crate::system::hierarchy_to_world_point]
    pub fn to_world_point<V: Into<Vec3>>(local_point: V) -> Vec3 {
        unsafe { hierarchy_to_world_point(&local_point.into()) }
    }

    /// Converts a local rotation relative to the current hierarchy stack into world space!
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToWorld.html>
    ///
    /// see also [crate::system::hierarchy_to_world_rotation]
    pub fn to_world_rotation<Q: Into<Quat>>(local_orientation: Q) -> Quat {
        unsafe { hierarchy_to_world_rotation(&local_orientation.into()) }
    }

    /// Converts a local pose relative to the current hierarchy stack into world space!
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToWorld.html>
    ///
    /// see also [crate::system::hierarchy_to_world_pose]
    pub fn to_world_pose<P: Into<Pose>>(local_pose: P) -> Pose {
        unsafe { hierarchy_to_world_pose(&local_pose.into()) }
    }

    /// Converts a local direction relative to the current hierarchy stack into world space! This excludes the
    /// translation component normally applied to vectors, so it’s still a valid direction.
    /// <https://stereokit.net/Pages/StereoKit/Hierarchy/ToWorldDirection.html>
    ///
    /// see also [crate::system::hierarchy_to_world_direction]
    pub fn to_world_direction<V: Into<Vec3>>(local_direction: V) -> Vec3 {
        unsafe { hierarchy_to_world_direction(&local_direction.into()) }
    }
}

bitflags::bitflags! {
/// What type of device is the source of the pointer? This is a bit-flag that can contain some input source family
/// information.
/// <https://stereokit.net/Pages/StereoKit/InputSource.html>
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct InputSource: u32 {
        /// Matches with all input sources!
        const Any = 2147483647;
        /// Matches with any hand input source.
        const Hand = 1;
        /// Matches with left hand input sources.
        const HandLeft = 2;
        /// Matches with right hand input sources.
        const HandRight = 4;
        /// Matches with Gaze category input sources.
        const Gaze = 16;
        /// Matches with the head gaze input source.
        const GazeHead = 32;
        /// Matches with the eye gaze input source.
        const GazeEyes = 64;
        /// Matches with mouse cursor simulated gaze as an input source.
        const GazeCurzor = 128;
        /// Matches with any input source that has an activation button!
        const CanPress = 256;
    }
}
/// An enum for indicating which hand to use!
/// <https://stereokit.net/Pages/StereoKit/Handed.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Handed {
    /// Left hand.
    Left = 0,
    /// Right hand.
    Right = 1,
    /// The number of hands one generally has, this is much nicer than doing a for loop with ‘2’ as the condition! It’s
    /// much clearer when you can loop Hand.Max times instead.
    Max = 2,
}
bitflags::bitflags! {
    /// A bit-flag for the current state of a button input.
    /// <https://stereokit.net/Pages/StereoKit/BtnState.html>
    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd)]
    #[repr(C)]
    pub struct BtnState: u32 {
        /// Is the button currently up, unpressed?
        const Inactive = 0;
        /// Is the button currently down, pressed?
        const Active = 1;
        ///	Has the button just been released? Only true for a single frame.
        const JustInactive = 2;
        ///	Has the button just been pressed? Only true for a single frame.
        const JustActive = 4;
        /// Has the button just changed state this frame?
        const Changed = 6;
        /// Matches with all states!
        const Any = 2147483647;
    }
}

impl BtnState {
    /// Is the button pressed?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsActive.html>
    pub fn is_active(&self) -> bool {
        (*self & BtnState::Active) > BtnState::Inactive
    }

    /// Has the button just been pressed this frame?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsJustActive.html>
    pub fn is_just_active(&self) -> bool {
        (*self & BtnState::JustActive) > BtnState::Inactive
    }

    /// Has the button just been released this frame?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsJustInactive.html>
    pub fn is_just_inactive(&self) -> bool {
        (*self & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Was the button either presses or released this frame?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsChanged.html>
    pub fn is_changed(&self) -> bool {
        (*self & BtnState::Changed) > BtnState::Inactive
    }
}

/// A collection of extension methods for the BtnState enum that makes bit-field checks a little easier.
/// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions.html>
pub struct BtnStateExtension;

impl BtnStateExtension {
    /// Is the button pressed?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsActive.html>
    pub fn is_active(state: BtnState) -> bool {
        (state & BtnState::Active) > BtnState::Inactive
    }

    /// Has the button just been pressed this frame?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsJustActive.html>
    pub fn is_just_active(state: BtnState) -> bool {
        (state & BtnState::JustActive) > BtnState::Inactive
    }

    /// Has the button just been released this frame?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsJustInactive.html>
    pub fn is_just_inactive(state: BtnState) -> bool {
        (state & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Was the button either presses or released this frame?
    /// <https://stereokit.net/Pages/StereoKit/BtnStateExtensions/IsChanged.html>
    pub fn is_changed(state: BtnState) -> bool {
        (state & BtnState::Changed) > BtnState::Inactive
    }
}

/// This is the tracking state of a sensory input in the world, like a controller’s position sensor, or a QR code
/// identified by a tracking system.
/// <https://stereokit.net/Pages/StereoKit/TrackState.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum TrackState {
    /// The system has no current knowledge about the state of this input. It may be out of visibility, or possibly just
    /// disconnected.
    Lost = 0,
    /// The system doesn’t know for sure where this is, but it has an educated guess that may be inferred from previous
    /// data at a lower quality. For example, a controller may still have accelerometer data after going out of view,
    /// which can still be accurate for a short time after losing optical tracking.
    Inferred = 1,
    /// The system actively knows where this input is. Within the constraints of the relevant hardware’s capabilities,
    /// this is as accurate as it gets!
    Known = 2,
}

/// Pointer is an abstraction of a number of different input sources, and a way to surface input events!
/// <https://stereokit.net/Pages/StereoKit/Pointer.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Pointer {
    /// What input source did this pointer come from? This is a bit-flag that contains input family and capability
    /// information.
    pub source: InputSource,
    /// Is the pointer source being tracked right now?
    pub tracked: BtnState,
    /// What is the state of the input source’s ‘button’, if it has one?
    pub state: BtnState,
    /// A ray in the direction of the pointer.
    pub ray: Ray,
    /// Orientation of the pointer! Since a Ray has no concept of ‘up’, this can be used to retrieve more orientation
    /// information.
    pub orientation: Quat,
}

impl Pointer {
    /// Convenience function that turns ray.position and orientation into a Pose.
    /// <https://stereokit.net/Pages/StereoKit/Pointer/Pose.html>
    pub fn get_pose(&self) -> Pose {
        Pose::new(self.ray.position, Some(self.orientation))
    }
}

/// Contains information to represents a joint on the hand.
/// <https://stereokit.net/Pages/StereoKit/HandJoint.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct HandJoint {
    /// The center of the joint’s world space location.
    pub position: Vec3,
    /// The joint’s world space orientation, where Forward points to the next joint down the finger, and Up will point
    /// towards the back of the hand. On the left hand, Right will point towards the thumb, and on the right hand, Right
    /// will point away from the thumb.
    pub orientation: Quat,
    /// The distance, in meters, to the surface of the hand from this joint.
    pub radius: f32,
}

/// Index values for each finger! From 0-4, from thumb to little finger.
/// <https://stereokit.net/Pages/StereoKit/FingerId.html>
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum FingerId {
    /// Thumb
    Thumb = 0,
    /// The primary index/pointer finger! Finger 1.
    Index = 1,
    /// next to the index finger.
    Middle = 2,
    /// ! What does one do with this finger? I guess… wear rings on it?
    Ring = 3,
    /// The smallest little finger! AKA, The Pinky.
    Little = 4,
}

/// Here’s where hands get crazy! Technical terms, and watch out for the thumbs!
/// <https://stereokit.net/Pages/StereoKit/JointId.html>
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum JointId {
    /// This is at the base of the hand, right above the wrist. For the thumb, Root and KnuckleMajor have the same
    /// value.
    Root = 0,
    /// Joint 1. These are the knuckles at the top of the palm! For the thumb, Root and KnuckleMajor have the same
    /// value.
    KnuckleMajor = 1,
    ///  These are the knuckles in the middle of the finger! First joints on the fingers themselves.
    KnuckleMid = 2,
    /// The joints right below the fingertip!
    KnuckleMinor = 3,
    /// The end/tip of each finger!
    Tip = 4,
}

/// This enum provides information about StereoKit’s hand tracking data source. It allows you to distinguish between
/// true hand data such as that provided by a Leap Motion Controller, and simulated data that StereoKit provides when
/// true hand data is not present.
/// <https://stereokit.net/Pages/StereoKit/HandSource.html>
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum HandSource {
    /// There is currently no source of hand data! This means there are no tracked controllers, or active hand tracking
    /// systems. This may happen if the user has hand tracking disabled, and no active controllers.
    None = 0,
    /// The current hand data is a simulation of hand data rather than true hand data. It is backed by either a
    /// controller, or a mouse, and may have a more limited range of motion.
    Simulated = 1,
    /// This is true hand data which exhibits the full range of motion of a normal hand. It is backed by something like
    /// a Leap Motion Controller, or some other optical (or maybe glove?) hand tracking system.
    Articulated = 2,
    /// This hand data is provided by your use of SK’s override functionality. What properties it exhibits depends on
    /// what override data you’re sending to StereoKit!
    Overridden = 3,
}

/// Id of a simulated hand pose, for use with Input.HandSimPoseRemove
/// <https://stereokit.net/Pages/StereoKit/HandSimId.html>
pub type HandSimId = i32;

#[derive(Debug, Copy, Clone)]
#[repr(C)]
/// Information about a hand!
/// <https://stereokit.net/Pages/StereoKit/Hand.html>
pub struct Hand {
    /// This is a 2D array with 25 HandJoints. You can get the right joint by finger*5 + joint
    pub fingers: [[HandJoint; 5usize]; 5usize],
    /// Pose of the wrist. TODO: Not populated right now.
    pub wrist: Pose,
    /// The position and orientation of the palm! Position is specifically defined as the middle of the middle finger’s
    /// root (metacarpal) bone. For orientation, Forward is the direction the flat of the palm is facing, “Iron Man”
    /// style. X+ is to the outside of the right hand, and to the inside of the left hand.
    pub palm: Pose,
    /// This is an approximation of where the center of a ‘pinch’ gesture occurs, and is used internally by StereoKit
    /// for some tasks, such as UI. For simulated hands, this position will give you the most stable pinch location
    /// possible. For real hands, it’ll be pretty close to the stablest point you’ll get. This is especially important
    /// for when the user begins and ends their pinch action, as you’ll often see a lot of extra movement in the fingers
    /// then.
    pub pinch_pt: Vec3,
    /// Is this a right hand, or a left hand?
    pub handed: Handed,
    /// Is the hand being tracked by the sensors right now?
    pub tracked: BtnState,
    /// Is the hand making a pinch gesture right now? Finger and thumb together.
    pub pinch: BtnState,
    /// Is the hand making a grip gesture right now? Fingers next to the palm.
    pub grip: BtnState,
    /// This is the size of the hand, calculated by measuring the length of the middle finger! This is calculated by
    /// adding the distances between each joint, then adding the joint radius of the root and tip. This value is
    /// recalculated at relatively frequent intervals, and can vary by as much as a centimeter.
    pub size: f32,
    /// What percentage of activation is the pinch gesture right now? Where 0 is a hand in an outstretched resting
    /// position, and 1 is fingers touching, within a device error tolerant threshold.
    pub pinch_activation: f32,
    /// What percentage of activation is the grip gesture right now? Where 0 is a hand in an outstretched resting
    /// position, and 1 is ring finger touching the base of the palm, within a device error tolerant threshold.
    pub grip_activation: f32,
}

impl Hand {
    /// Returns the joint information of the indicated hand joint! This also includes fingertips as a ‘joint’. This is
    /// the same as the [] operator. Note that for thumbs, there are only 4 ‘joints’ in reality, so StereoKit has
    /// JointId.Root and JointId.KnuckleMajor as the same pose, so JointId.Tip is still the tip of the thumb!
    /// <https://stereokit.net/Pages/StereoKit/Hand/Get.html>
    pub fn get(&self, finger: FingerId, joint: JointId) -> HandJoint {
        self.fingers[finger as usize][joint as usize]
    }

    /// Returns the joint information of the indicated hand joint! This also includes fingertips as a ‘joint’. This is
    /// the same as the [] operator. Note that for thumbs, there are only 4 ‘joints’ in reality, so StereoKit has
    /// JointId.Root and JointId.KnuckleMajor as the same pose, so JointId.Tip is still the tip of the thumb!
    /// <https://stereokit.net/Pages/StereoKit/Hand/Get.html>
    pub fn get_u(&self, finger: usize, joint: usize) -> HandJoint {
        self.fingers[finger][joint]
    }

    /// Are the fingers currently gripped?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsGripped.html>
    pub fn is_gripped(&self) -> bool {
        (self.grip & BtnState::Active) > BtnState::Inactive
    }

    /// Have the fingers just been gripped this frame?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsJustGripped.html>
    pub fn is_just_gripped(&self) -> bool {
        (self.grip & BtnState::JustActive) > BtnState::Inactive
    }

    /// Have the fingers just stopped being gripped this frame?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsJustUngripped.html>
    pub fn is_just_ungripped(&self) -> bool {
        (self.grip & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Are the fingers currently pinched?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsPinched.html>
    pub fn is_pinched(&self) -> bool {
        (self.pinch & BtnState::Active) > BtnState::Inactive
    }

    /// Have the fingers just been pinched this frame?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsJustPinched.html>
    pub fn is_just_pinched(&self) -> bool {
        (self.pinch & BtnState::JustActive) > BtnState::Inactive
    }

    /// Have the fingers just stopped being pinched this frame?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsJustUnpinched.html>
    pub fn is_just_unpinched(&self) -> bool {
        (self.pinch & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Is the hand being tracked by the sensors right now?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsTracked.html>
    pub fn is_tracked(&self) -> bool {
        (self.tracked & BtnState::Active) > BtnState::Inactive
    }

    /// Has the hand just started being tracked this frame?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsJustTracked.html>
    pub fn is_just_tracked(&self) -> bool {
        (self.tracked & BtnState::JustActive) > BtnState::Inactive
    }

    /// Has the hand just stopped being tracked this frame?
    /// <https://stereokit.net/Pages/StereoKit/Hand/IsJustUntracked.html>
    pub fn is_just_untracked(&self) -> bool {
        (self.tracked & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Set the Material used to render the hand! The default material uses an offset of 10 to ensure it gets drawn
    /// overtop of other elements.
    /// <https://stereokit.net/Pages/StereoKit/Hand/Material.html>
    ///
    /// see also [`crate::system::input_hand_material`]
    pub fn material(&mut self, material: impl AsRef<Material>) -> &mut Self {
        unsafe { input_hand_material(self.handed, material.as_ref().0.as_ptr()) }
        self
    }

    /// Sets whether or not StereoKit should render this hand for you. Turn this to false if you’re going to render your
    /// own, or don’t need the hand itself to be visible.
    /// <https://stereokit.net/Pages/StereoKit/Hand/Visible.html>
    ///
    /// see also [`crate::system::input_hand_visible`]
    pub fn visible(&mut self, visible: bool) -> &mut Self {
        unsafe { input_hand_visible(self.handed, visible as Bool32T) }
        self
    }
}

/// Represents an input from an XR headset’s controller!
/// <https://stereokit.net/Pages/StereoKit/ControllerKey.html>
#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum ControllerKey {
    /// Doesn’t represent a key, generally means this item has not been set to any particular value!
    None_ = 0,
    /// The trigger button on the controller, where the user’s index finger typically sits.
    Trigger = 1,
    /// The grip button on the controller, usually where the fingers that are not the index finger sit.
    Grip = 2,
    /// This is the lower of the two primary thumb buttons, sometimes labelled X, and sometimes A.
    X1_ = 3,
    /// This is the upper of the two primary thumb buttons, sometimes labelled Y, and sometimes B.
    X2 = 4,
    /// This is when the thumbstick on the controller is actually pressed. This has nothing to do with the horizontal
    /// or vertical movement of the stick.
    Stick = 5,
    /// This is the menu, or settings button of the controller.
    Menu = 6,
}

/// This represents a physical controller input device! Tracking information, buttons, analog sticks and triggers!
/// There’s also a Menu button that’s tracked separately at Input.ContollerMenu.
/// <https://stereokit.net/Pages/StereoKit/Controller.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Controller {
    /// The grip pose of the controller. This approximately represents the center of the hand’s position. Check
    /// trackedPos and trackedRot for the current state of the pose data.
    pub pose: Pose,
    pub palm: Pose,
    /// The aim pose of a controller is where the controller ‘points’ from and to. This is great for pointer rays and
    /// far interactions.
    pub aim: Pose,
    /// This tells the current tracking state of this controller overall. If either position or rotation are trackable,
    /// then this will report tracked. Typically, positional tracking will be lost first, when the controller goes out
    /// of view, and rotational tracking will often remain as long as the controller is still connected. This is a good
    /// way to check if the controller is connected to the system at all.
    pub tracked: BtnState,
    /// This tells the current tracking state of the controller’s position information. This is often the first part of
    /// tracking data to go, so it can often be good to account for this on occasions.
    pub tracked_pos: TrackState,
    /// This tells the current tracking state of the controller’s rotational information.
    pub tracked_rot: TrackState,
    /// This represents the click state of the controller’s analog stick or directional controller.
    pub stick_click: BtnState,
    /// The current state of the controller’s X1 button. Depending on the specific hardware, this is the first general
    /// purpose button on the controller. For example, on an Oculus Quest Touch controller this would represent ‘X’ on
    /// the left controller, and ‘A’ on the right controller.
    pub x1: BtnState,
    ///The current state of the controller’s X2 button. Depending on the specific hardware, this is the second general
    /// purpose button on the controller. For example, on an Oculus Quest Touch controller this would represent ‘Y’ on
    /// the left controller, and ‘B’ on the right controller.
    pub x2: BtnState,
    /// The trigger button at the user’s index finger. These buttons typically have a wide range of activation, so this
    /// is provided as a value from 0.0 -> 1.0, where 0 is no interaction, and 1 is full interaction. If a controller
    /// has binary activation, this will jump straight from 0 to 1.
    pub trigger: f32,
    /// The grip button typically sits under the user’s middle finger. These buttons occasionally have a wide range of
    /// activation, so this is provided as a value from 0.0 -> 1.0, where 0 is no interaction, and 1 is full
    /// interaction. If a controller has binary activation, this will jump straight from 0 to 1.
    pub grip: f32,
    /// This is the current 2-axis position of the analog stick or equivalent directional controller. This generally
    /// ranges from -1 to +1 on each axis. This is a raw input, so dead-zones and similar issues are not accounted for
    /// here, unless modified by the OpenXR platform itself.
    pub stick: Vec2,
}

impl Controller {
    /// Is the controller’s X1 button currently pressed? Depending on the specific hardware, this is the first
    /// general purpose button on the controller. For example, on an Oculus Quest Touch controller this would
    /// represent ‘X’ on the left controller, and ‘A’ on the right controller.
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsX1Pressed.html>
    pub fn is_x1_pressed(&self) -> bool {
        (self.x1 & BtnState::Active) > BtnState::Inactive
    }

    /// Has the controller’s X1 button just been pressed this frame? Depending on the specific hardware, this is the
    /// first general purpose button on the controller. For example, on an Oculus Quest Touch controller this would
    /// represent ‘X’ on the left controller, and ‘A’ on the right controller.
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsX1JustPressed.html>
    pub fn is_x1_just_pressed(&self) -> bool {
        (self.x1 & BtnState::JustActive) > BtnState::Inactive
    }

    /// Has the controller’s X1 button just been released this frame? Depending on the specific hardware, this is
    /// the first general purpose button on the controller. For example, on an Oculus Quest Touch controller this
    /// would represent ‘X’ on the left controller, and ‘A’ on the right controller.
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsX1JustUnPressed.html>
    pub fn is_x1_just_unpressed(&self) -> bool {
        (self.x1 & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Is the controller’s X2 button currently pressed? Depending on the specific hardware, this is the second
    /// general purpose button on the controller. For example, on an Oculus Quest Touch controller this would
    /// represent ‘Y’ on the left controller, and ‘B’ on the right controller.
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsX2Pressed.html>
    pub fn is_x2_pressed(&self) -> bool {
        (self.x2 & BtnState::Active) > BtnState::Inactive
    }

    /// Has the controller’s X2 button just been pressed this frame? Depending on the specific hardware, this is the
    /// second general purpose button on the controller. For example, on an Oculus Quest Touch controller this would
    /// represent ‘Y’ on the left controller, and ‘B’ on the right controller.
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsX2JustPressed.html>
    pub fn is_x2_just_pressed(&self) -> bool {
        (self.x2 & BtnState::JustActive) > BtnState::Inactive
    }

    /// Has the controller’s X2 button just been released this frame? Depending on the specific hardware, this is
    /// the second general purpose button on the controller. For example, on an Oculus Quest Touch controller this
    /// would represent ‘Y’ on the left controller, and ‘B’ on the right controller.
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsX2JustUnPressed.html>
    pub fn is_x2_just_unpressed(&self) -> bool {
        (self.x2 & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Is the analog stick/directional controller button currently being actively pressed?
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsStickClicked.html>
    pub fn is_stick_clicked(&self) -> bool {
        (self.stick_click & BtnState::Active) > BtnState::Inactive
    }

    /// Is the analog stick/directional controller button currently being actively pressed?
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsStickJustClicked.html>
    pub fn is_stick_just_clicked(&self) -> bool {
        (self.stick_click & BtnState::JustActive) > BtnState::Inactive
    }

    /// Has the analog stick/directional controller button just been released this frame?
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsStickJustUnclicked.html>
    pub fn is_stick_just_unclicked(&self) -> bool {
        (self.stick_click & BtnState::JustInactive) > BtnState::Inactive
    }

    /// Is the controller being tracked by the sensors right now?
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsTracked.html>
    pub fn is_tracked(&self) -> bool {
        (self.tracked & BtnState::Active) > BtnState::Inactive
    }

    /// Has the controller just started being tracked this frame?
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsJustTracked.html>
    pub fn is_just_tracked(&self) -> bool {
        (self.tracked & BtnState::JustActive) > BtnState::Inactive
    }

    /// Has the analog stick/directional controller button just been released this frame?
    /// <https://stereokit.net/Pages/StereoKit/Controller/IsJustUntracked.html>
    pub fn is_just_untracked(&self) -> bool {
        (self.tracked & BtnState::JustInactive) > BtnState::Inactive
    }
}

/// This stores information about the mouse! What’s its state, where’s it pointed, do we even have one?
/// <https://stereokit.net/Pages/StereoKit/Mouse.html>
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Mouse {
    /// Is the mouse available to use? Most MR systems likely won’t have a mouse!
    pub available: Bool32T,
    /// Position of the mouse relative to the window it’s in! This is the number of pixels from the top left corner of
    /// the screen.
    pub pos: Vec2,
    /// How much has the mouse’s position changed in the current frame? Measured in pixels.
    pub pos_change: Vec2,
    /// What’s the current scroll value for the mouse’s scroll wheel? TODO: Units
    pub scroll: f32,
    /// How much has the scroll wheel value changed during this frame? TODO: Units
    pub scroll_change: f32,
}

impl Mouse {
    /// Ray representing the position and orientation that the current Input.Mouse.pos is pointing in.
    /// <https://stereokit.net/Pages/StereoKit/Mouse/Ray.html>
    ///
    /// see also [`crate::system::ray_from_mouse`]
    pub fn get_ray(&self) -> Ray {
        let mut out_ray = Ray::default();
        unsafe { ray_from_mouse(self.pos, &mut out_ray) };
        out_ray
    }
}

/// A collection of system key codes, representing keyboard characters and mouse buttons. Based on VK codes.
/// <https://stereokit.net/Pages/StereoKit/Key.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Key {
    None = 0,
    MouseLeft = 1,
    MouseRight = 2,
    MouseCenter = 4,
    MouseForward = 5,
    MouseBack = 6,
    Backspace = 8,
    Tab = 9,
    Return = 13,
    Shift = 16,
    Ctrl = 17,
    Alt = 18,
    CapsLock = 20,
    Esc = 27,
    Space = 32,
    End = 35,
    Home = 36,
    Left = 37,
    Right = 39,
    Up = 38,
    Down = 40,
    PageUp = 33,
    PageDown = 34,
    PrintScreen = 42,
    KeyInsert = 45,
    Del = 46,
    Key0 = 48,
    Key1 = 49,
    Key2 = 50,
    Key3 = 51,
    Key4 = 52,
    Key5 = 53,
    Key6 = 54,
    Key7 = 55,
    Key8 = 56,
    Key9 = 57,
    A = 65,
    B = 66,
    C = 67,
    D = 68,
    E = 69,
    F = 70,
    G = 71,
    H = 72,
    I = 73,
    J = 74,
    K = 75,
    L = 76,
    M = 77,
    N = 78,
    O = 79,
    P = 80,
    Q = 81,
    R = 82,
    S = 83,
    T = 84,
    U = 85,
    V = 86,
    W = 87,
    X = 88,
    Y = 89,
    Z = 90,
    Numpad0 = 96,
    Numpad1 = 97,
    Numpad2 = 98,
    Numpad3 = 99,
    Numpad4 = 100,
    Numpad5 = 101,
    Numpad6 = 102,
    Numpad7 = 103,
    Numpad8 = 104,
    Numpad9 = 105,
    F1 = 112,
    F2 = 113,
    F3 = 114,
    F4 = 115,
    F5 = 116,
    F6 = 117,
    F7 = 118,
    F8 = 119,
    F9 = 120,
    F10 = 121,
    F11 = 122,
    F12 = 123,
    Comma = 188,
    Period = 190,
    SlashFwd = 191,
    SlashBack = 220,
    Semicolon = 186,
    Apostrophe = 222,
    BracketOpen = 219,
    BracketClose = 221,
    Minus = 189,
    Equals = 187,
    Backtick = 192,
    LCmd = 91,
    RCmd = 92,
    Multiply = 106,
    Add = 107,
    Subtract = 109,
    Decimal = 110,
    Divide = 111,
}

/// Input from the system come from this class! Hands, eyes, heads, mice and pointers!
/// <https://stereokit.net/Pages/StereoKit/Input.html>
pub struct Input;
extern "C" {
    pub fn input_pointer_count(filter: InputSource) -> i32;
    pub fn input_pointer(index: i32, filter: InputSource) -> Pointer;
    pub fn input_hand(hand: Handed) -> *const Hand;
    pub fn input_hand_override(hand: Handed, in_arr_hand_joints: *const HandJoint);
    pub fn input_hand_source(hand: Handed) -> HandSource;
    pub fn input_controller(hand: Handed) -> *const Controller;
    pub fn input_controller_menu() -> BtnState;
    pub fn input_controller_model_set(hand: Handed, model: ModelT);
    pub fn input_controller_model_get(hand: Handed) -> ModelT;
    pub fn input_head() -> *const Pose;
    pub fn input_eyes() -> *const Pose;
    pub fn input_eyes_tracked() -> BtnState;
    pub fn input_mouse() -> *const Mouse;
    pub fn input_key(key: Key) -> BtnState;
    pub fn input_key_inject_press(key: Key);
    pub fn input_key_inject_release(key: Key);
    pub fn input_text_consume() -> u32;
    pub fn input_text_reset();
    pub fn input_text_inject_char(character: u32);
    pub fn input_hand_visible(hand: Handed, visible: Bool32T);
    // Deprecated: pub fn input_hand_solid(hand: Handed, solid: Bool32T);
    pub fn input_hand_material(hand: Handed, material: MaterialT);
    pub fn input_hand_sim_pose_add(
        in_arr_palm_relative_hand_joints_25: *const Pose,
        button1: ControllerKey,
        and_button2: ControllerKey,
        or_hotkey1: Key,
        and_hotkey2: Key,
    ) -> HandSimId;
    pub fn input_hand_sim_pose_remove(id: HandSimId);
    pub fn input_hand_sim_pose_clear();
    pub fn input_subscribe(
        source: InputSource,
        input_event: BtnState,
        input_event_callback: Option<
            unsafe extern "C" fn(source: InputSource, input_event: BtnState, in_pointer: *const Pointer),
        >,
    );
    pub fn input_unsubscribe(
        source: InputSource,
        input_event: BtnState,
        input_event_callback: Option<
            unsafe extern "C" fn(source: InputSource, input_event: BtnState, in_pointer: *const Pointer),
        >,
    );
    pub fn input_fire_event(source: InputSource, input_event: BtnState, pointer: *const Pointer);
}

impl Input {
    /// When StereoKit is rendering the input source, this allows you to override the controller Model SK uses. The
    /// Model SK uses by default may be provided from the OpenXR runtime depending on extension support, but if not, SK
    /// does have a default Model.
    /// Setting this to null will restore SK's default.
    /// <https://stereokit.net/Pages/StereoKit/Input.html>
    /// * handed - The hand to assign the Model to.
    /// * model - The Model to use to represent the controller.
    /// None is valid, and will restore SK's default model.
    pub fn set_controller_model(handed: Handed, model: Option<Model>) {
        match model {
            Some(model) => unsafe { input_controller_model_set(handed, model.0.as_ptr()) },
            None => unsafe { input_controller_model_set(handed, null_mut()) },
        }
    }

    /// Gets raw controller input data from the system. Note that not all buttons provided here are guaranteed to be
    /// present on the user’s physical controller. Controllers are also not guaranteed to be available on the system,
    /// and are never simulated.
    /// <https://stereokit.net/Pages/StereoKit/Input/Controller.html>
    ///
    /// see also [`crate::system::input_controller`]    
    pub fn controller(hand: Handed) -> Controller {
        unsafe { *input_controller(hand) }
    }

    /// This function allows you to artifically insert an input event, simulating any device source and event type you
    /// want.
    /// <https://stereokit.net/Pages/StereoKit/Input/FireEvent.html>
    ///
    /// see also [`crate::system::input_fire_event`]    
    pub fn fire_event(event_source: InputSource, event_types: BtnState, pointer: &Pointer) {
        unsafe { input_fire_event(event_source, event_types, pointer) };
    }

    /// Retrieves all the information about the user’s hand! StereoKit will always provide hand information, however
    /// sometimes that information is simulated, like in the case of a mouse, or controllers.
    ///
    /// Note that this is a pointer to the hand information, and it’s a good chunk of data, so it’s a good idea to grab
    /// it once and keep it around for the frame, or at least function, rather than asking for it again and again each
    /// time you want to touch something.
    /// <https://stereokit.net/Pages/StereoKit/Input/Hand.html>
    ///
    /// see also [`crate::system::input_hand`]    
    pub fn hand(hand: Handed) -> Hand {
        unsafe { *input_hand(hand) }
    }

    /// Clear out the override status from Input::hand_override, and restore the user’s control over it again.
    /// <https://stereokit.net/Pages/StereoKit/Input/HandClearOverride.html>
    ///
    /// see also [`crate::system::input_hand_override`]    
    pub fn hand_clear_override(hand: Handed) {
        unsafe { input_hand_override(hand, null()) };
    }

    /// This allows you to completely override the hand’s pose information! It is still treated like the user’s hand,
    /// so this is great for simulating input for testing purposes. It will remain overridden until you call
    /// Input::hand_clear_override.
    /// <https://stereokit.net/Pages/StereoKit/Input/HandOverride.html>
    ///
    /// see also [`crate::system::input_hand_override`]    
    pub fn hand_override(hand: Handed, joints: &[HandJoint]) {
        unsafe { input_hand_override(hand, joints.as_ptr()) };
    }

    /// Set the Material used to render the hand! The default material uses an offset of 10 to ensure it gets drawn
    /// overtop of other elements.
    /// <https://stereokit.net/Pages/StereoKit/Input/HandMaterial.html>
    /// * material - If None, will reset to the default value
    ///
    /// see also [`crate::system::input_hand_material`]    
    pub fn hand_material(hand: Handed, material: Option<Material>) {
        match material {
            Some(material) => unsafe { input_hand_material(hand, material.0.as_ptr()) },
            None => unsafe { input_hand_material(hand, null_mut()) },
        }
    }

    /// StereoKit will use controller inputs to simulate an articulated hand. This function allows you to add new
    /// simulated poses to different controller or keyboard buttons!
    /// <https://stereokit.net/Pages/StereoKit/Input/HandSimPoseAdd.html>
    ///
    /// see also [`crate::system::input_hand_sim_pose_add`]    
    pub fn hand_sim_pose_add(
        hand_joints_25: &[Pose],
        button1: ControllerKey,
        and_button2: ControllerKey,
        or_hotkey1: Key,
        and_hotkey2: Key,
    ) -> HandSimId {
        unsafe { input_hand_sim_pose_add(hand_joints_25.as_ptr(), button1, and_button2, or_hotkey1, and_hotkey2) }
    }

    /// This clears all registered hand simulation poses, including the ones that StereoKit registers by default!
    /// <https://stereokit.net/Pages/StereoKit/Input/HandSimPoseClear.html>
    ///
    /// see also [`crate::system::input_hand_sim_pose_clear`]    
    pub fn hand_sim_pose_clear() {
        unsafe { input_hand_sim_pose_clear() };
    }

    /// Lets you remove an existing hand pose.
    /// <https://stereokit.net/Pages/StereoKit/Input/HandSimPoseRemove.html>
    ///
    /// see also [`crate::system::input_hand_sim_pose_remove`]    
    pub fn hand_sim_pose_remove(id: HandSimId) {
        unsafe { input_hand_sim_pose_remove(id) };
    }

    /// This gets the current source of the hand joints! This allows you to distinguish between fully articulated
    /// joints, and simulated hand joints that may not have the same range of mobility. Note that this may change during
    /// a session, the user may put down their controllers, automatically switching to hands, or visa versa.
    /// <https://stereokit.net/Pages/StereoKit/Input/HandSource.html>
    ///
    /// see also [`crate::system::input_hand_source`]    
    pub fn hand_source(hand: Handed) -> HandSource {
        unsafe { input_hand_source(hand) }
    }

    /// Sets whether or not StereoKit should render the hand for you. Turn this to false if you’re going to render your
    /// own, or don’t need the hand itself to be visible.
    /// <https://stereokit.net/Pages/StereoKit/Input/HandVisible.html>
    ///
    /// see also [`crate::system::input_hand_visible`]    
    pub fn hand_visible(hand: Handed, visible: bool) {
        unsafe { input_hand_visible(hand, visible as Bool32T) };
    }

    /// Keyboard key state! On desktop this is super handy, but even standalone MR devices can have bluetooth keyboards,
    /// or even just holographic system keyboards!
    /// <https://stereokit.net/Pages/StereoKit/Input/Key.html>
    ///
    /// see also [`crate::system::input_key`]    
    pub fn key(key: Key) -> BtnState {
        unsafe { input_key(key) }
    }

    /// This will inject a key press event into StereoKit’s input event queue. It will be processed at the start of the
    /// next frame, and will be indistinguishable from a physical key press. Remember to release your key as well!
    ///
    /// This will not submit text to StereoKit’s text queue, and will not show up in places like UI.Input. For that, you
    /// must submit a TextInjectChar call.
    /// <https://stereokit.net/Pages/StereoKit/Input/KeyInjectPress.html>
    ///
    /// see also [`crate::system::input_key_inject_press`]    
    pub fn key_inject_press(key: Key) {
        unsafe { input_key_inject_press(key) };
    }

    /// This will inject a key release event into StereoKit’s input event queue. It will be processed at the start of
    /// the next frame, and will be indistinguishable from a physical key release. This should be preceded by a key
    /// press!
    ///
    /// This will not submit text to StereoKit’s text queue, and will not show up in places like UI.Input. For that, you
    /// must submit a TextInjectChar call.
    /// <https://stereokit.net/Pages/StereoKit/Input/KeyInjectRelease.html>
    ///
    /// see also [`crate::system::input_key_inject_release`]    
    pub fn key_inject_release(key: Key) {
        unsafe { input_key_inject_release(key) };
    }

    /// This gets the pointer by filter based index.
    /// <https://stereokit.net/Pages/StereoKit/Input/Pointer.html>
    /// * filter - If None has default value of ANY
    ///
    /// see also [`crate::system::input_pointer`]    
    pub fn pointer(index: i32, filter: Option<InputSource>) -> Pointer {
        let filter = filter.unwrap_or(InputSource::Any);
        unsafe { input_pointer(index, filter) }
    }

    /// The number of Pointer inputs that StereoKit is tracking that match the given filter.
    /// <https://stereokit.net/Pages/StereoKit/Input/PointerCount.html>
    /// * filter - If None has default value of ANY
    ///
    /// see also [`crate::system::input_pointer_count`]    
    pub fn pointer_count(filter: Option<InputSource>) -> i32 {
        let filter = filter.unwrap_or(InputSource::Any);
        unsafe { input_pointer_count(filter) }
    }

    /// Returns the next text character from the list of characters that have been entered this frame! Will return ‘\0’
    /// if there are no more characters left in the list. These are from the system’s text entry system, and so can be
    /// unicode, will repeat if their ‘key’ is held down, and could arrive from something like a copy/paste operation.
    ///
    /// If you wish to reset this function to begin at the start of the read list on the next call, you can call
    /// Input::text_reset.
    /// <https://stereokit.net/Pages/StereoKit/Input/TextConsume.html>
    ///
    /// see also [`crate::system::input_text_consume`]    
    pub fn text_consume() -> Option<char> {
        char::from_u32(unsafe { input_text_consume() })
    }

    /// Resets the Input::text_consume read list back to the start. For example, UI.Input will not call text_reset, so
    /// it effectively will consume those characters, hiding them from any TextConsume calls following it. If you wanted
    /// to check the current frame’s text, but still allow UI.Input to work later on in the frame, you would read
    /// everything with TextConsume, and then TextReset afterwards to reset the read list for the following UI.Input.
    /// <https://stereokit.net/Pages/StereoKit/Input/TextReset.html>
    ///
    /// see also [`crate::system::input_text_reset`]    
    pub fn text_reset() {
        unsafe { input_text_reset() };
    }

    /// This will inject a UTF32 Unicode text character into StereoKit’s text input queue. It will be available at the
    /// start of the next frame, and will be indistinguishable from normal text entry.
    ///
    /// This will not submit key press/release events to StereoKit’s input queue, use key_inject_press/_release
    /// for that.
    /// <https://stereokit.net/Pages/StereoKit/Input/TextInjectChar.html>
    ///
    /// see also [`crate::system::input_text_inject_char`]    
    pub fn text_inject_char(character: char) {
        unsafe { input_text_inject_char(character as u32) };
    }

    /// This will convert an str into a number of UTF32 Unicode text characters, and inject them into StereoKit’s
    /// text input queue. It will be available at the start of the next frame, and will be indistinguishable from normal
    /// text entry.
    ///
    /// This will not submit key press/release events to StereoKit’s input queue, use key_inject_press/_release
    /// for that.
    /// <https://stereokit.net/Pages/StereoKit/Input/TextInjectChar.html>
    ///
    /// see also [`crate::system::input_text_inject_char`]    
    pub fn text_inject_chars(str: impl AsRef<str>) {
        for character in str.as_ref().chars() {
            unsafe { input_text_inject_char(character as u32) }
        }
    }

    /// You can subscribe to input events from Pointer sources here. StereoKit will call your callback and pass along a
    /// Pointer that matches the position of that pointer at the moment the event occurred. This can be more accurate
    /// than polling for input data, since polling happens specifically at frame start.
    /// <https://stereokit.net/Pages/StereoKit/Input/Subscribe.html>
    ///
    /// see also [`crate::system::input_subscribe`]    
    pub fn subscribe(
        event_source: InputSource,
        event_types: BtnState,
        cb: Option<unsafe extern "C" fn(source: InputSource, input_event: BtnState, in_pointer: *const Pointer)>,
    ) {
        unsafe { input_subscribe(event_source, event_types, cb) }
    }

    /// Unsubscribes a listener from input events.
    /// <https://stereokit.net/Pages/StereoKit/Input/Unsubscribe.html>
    ///
    /// see also [`crate::system::input_unsubscribe`]    
    pub fn unsubscribe(
        event_source: InputSource,
        event_types: BtnState,
        cb: Option<unsafe extern "C" fn(source: InputSource, input_event: BtnState, in_pointer: *const Pointer)>,
    ) {
        unsafe { input_unsubscribe(event_source, event_types, cb) }
    }

    /// This retreives the Model currently in use by StereoKit to represent the controller input source. By default,
    /// this will be a Model provided by OpenXR, or SK's fallback Model. This will never be null while SK is
    /// initialized.
    /// <https://stereokit.net/Pages/StereoKit/Input.html>
    /// * handed - The hand of the controller Model to retreive.
    /// Returns the current controller Model. By default, his will be a Model provided by OpenXR, or SK's fallback
    /// Model. This will never be null while SK is initialized.
    pub fn get_controller_model(handed: Handed) -> Model {
        match NonNull::new(unsafe { input_controller_model_get(handed) }) {
            Some(model) => Model(model),
            None => Model::new(),
        }
    }

    /// This is the state of the controller’s menu button, this is not attached to any particular hand, so it’s
    /// independent of a left or right controller.
    /// <https://stereokit.net/Pages/StereoKit/Input/ControllerMenuButton.html>
    ///
    /// see also [`crate::system::input_controller_menu`]    
    pub fn get_controller_menu_button() -> BtnState {
        unsafe { input_controller_menu() }
    }

    /// If the device has eye tracking hardware and the app has permission to use it, then this is the most recently
    /// tracked eye pose. Check Input.EyesTracked to see if the pose is up-to date, or if it’s a leftover!
    ///
    /// You can also check Sk::System::eye_tracking_present to see if the hardware is capable of providing eye tracking.
    ///
    /// On Flatscreen when the MR sim is still enabled, then eyes are emulated using the cursor position when the user
    /// holds down Alt.
    /// <https://stereokit.net/Pages/StereoKit/Input/Eyes.html>
    ///
    /// see also [`crate::system::input_eyes`]    
    pub fn get_eyes() -> Pose {
        unsafe { *input_eyes() }
    }

    /// If eye hardware is available and app has permission, then this is the tracking state of the eyes. Eyes may move
    /// out of bounds, hardware may fail to detect eyes, or who knows what else!
    ///
    /// On Flatscreen when MR sim is still enabled, this will report whether the user is simulating eye input with the
    /// Alt key.
    ///
    /// Permissions:
    /// * For UWP apps, permissions for eye tracking can be found in the project’s .appxmanifest file under
    /// Capabilities->Gaze Input.
    /// * For Xamarin apps, you may need to add an entry to your AndroidManifest.xml, refer to your device’s
    /// documentation for specifics.
    /// <https://stereokit.net/Pages/StereoKit/Input/EyesTracked.html>
    ///
    /// see also [`crate::system::input_eyes_tracked`]    
    pub fn get_eyes_tracked() -> BtnState {
        unsafe { input_eyes_tracked() }
    }

    /// The position and orientation of the user’s head! This is the center point between the user’s eyes, NOT the
    /// center of the user’s head. Forward points the same way the user’s face is facing.
    /// <https://stereokit.net/Pages/StereoKit/Input/Head.html>
    ///
    /// see also [`crate::system::input_eyes`]    
    pub fn get_head() -> Pose {
        unsafe { *input_head() }
    }

    /// Information about this system’s mouse, or lack thereof!
    /// <https://stereokit.net/Pages/StereoKit/Input/Mouse.html>
    ///
    /// see also [`crate::system::input_mouse`]    
    pub fn get_mouse() -> Mouse {
        unsafe { *input_mouse() }
    }
}

/// Used to represent lines for the line drawing functions! This is just a snapshot of information about each individual
/// point on a line.
/// <https://stereokit.net/Pages/StereoKit/LinePoint.html>
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
pub struct LinePoint {
    pub pt: Vec3,
    pub thickness: f32,
    pub color: Color32,
}

/// A line drawing class! This is an easy way to visualize lines or relationships between objects. The current
/// implementation uses a quad strip that always faces the user, via vertex shader manipulation.
/// <https://stereokit.net/Pages/StereoKit/Lines.html>
pub struct Lines;

extern "C" {
    pub fn line_add(start: Vec3, end: Vec3, color_start: Color32, color_end: Color32, thickness: f32);
    pub fn line_addv(start: LinePoint, end: LinePoint);
    pub fn line_add_axis(pose: Pose, size: f32);
    pub fn line_add_list(points: *const Vec3, count: i32, color: Color32, thickness: f32);
    pub fn line_add_listv(in_arr_points: *const LinePoint, count: i32);
}

impl Lines {
    /// Adds a line to the environment for the current frame.
    /// <https://stereokit.net/Pages/StereoKit/Lines/Add.html>
    /// * color_end - If None, uses color_start.
    ///
    /// see also [crate::system::line_add]
    pub fn add<V: Into<Vec3>>(start: V, end: V, color_start: Color32, color_end: Option<Color32>, thickness: f32) {
        let color_end = color_end.unwrap_or(color_start);
        unsafe { line_add(start.into(), end.into(), color_start, color_end, thickness) }
    }

    /// Adds a line based on a ray to the environment for the current frame.
    /// <https://stereokit.net/Pages/StereoKit/Lines/Add.html>
    /// * color_end - If None, uses color_start.
    ///
    /// see also [crate::system::line_add]
    pub fn add_ray<R: Into<Ray>>(
        ray: R,
        lenght: f32,
        color_start: Color32,
        color_end: Option<Color32>,
        thickness: f32,
    ) {
        let color_end = color_end.unwrap_or(color_start);
        let ray: Ray = ray.into();
        unsafe { line_add(ray.position, ray.get_at(lenght), color_start, color_end, thickness) }
    }

    /// Adds a line from a list of line points to the environment. This does not close the path, so if you want it
    /// closed, you’ll have to add an extra point or two at the end yourself!
    /// <https://stereokit.net/Pages/StereoKit/Lines/Add.html>
    /// * color_end - If None, uses color_start.
    ///
    /// see also [crate::system::line_add]
    pub fn add_many(points: &[LinePoint]) {
        unsafe { line_add_listv(points.as_ptr(), points.len() as i32) }
    }

    /// Displays an RGB/XYZ axis widget at the pose! Each line is extended along the positive direction of each axis, so
    /// the red line is +X, green is +Y, and blue is +Z. A white line is drawn along -Z to indicate the Forward vector
    /// of the pose (-Z is forward in StereoKit).
    /// <https://stereokit.net/Pages/StereoKit/Lines/AddAxis.html>
    /// * size - If None, is set to 1 cm
    /// * thickness - If None, will use a faster renderer with a thickness of one tenth of the size.
    ///
    /// see also [crate::system::line_add]
    pub fn add_axis<P: Into<Pose>>(at_pose: P, size: Option<f32>, thickness: Option<f32>) {
        let at_pose: Pose = at_pose.into();
        let size = size.unwrap_or(0.01);
        match thickness {
            Some(thickness) => {
                Self::add(
                    at_pose.position,
                    at_pose.orientation.mul_vec3(at_pose.position + Vec3::X) * size,
                    Color32::new(255, 0, 0, 255),
                    None,
                    thickness,
                );
                Self::add(
                    at_pose.position,
                    at_pose.orientation.mul_vec3(at_pose.position + Vec3::Y) * size,
                    Color32::new(0, 255, 0, 255),
                    None,
                    thickness,
                );
                Self::add(
                    at_pose.position,
                    at_pose.orientation.mul_vec3(at_pose.position + Vec3::Z) * size,
                    Color32::new(0, 0, 255, 255),
                    None,
                    thickness,
                );
                Self::add(
                    at_pose.position,
                    at_pose.orientation.mul_vec3(at_pose.position + Vec3::FORWARD) * size * 0.5,
                    Color32::new(255, 255, 255, 255),
                    None,
                    thickness,
                )
            }
            None => unsafe { line_add_axis(at_pose, size) },
        }
    }
}

/// The log tool will write to the console with annotations for console colors, which helps with readability, but isn’t
/// always supported. These are the options available for configuring those colors.
/// <https://stereokit.net/Pages/StereoKit/LogColors.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum LogColors {
    /// Use console coloring annotations.
    Ansi = 0,
    ///Scrape out any color annotations, so logs are all completely plain text.
    None = 1,
}

/// Severity of a log item.
/// <https://stereokit.net/Pages/StereoKit/LogLevel.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum LogLevel {
    None = 0,
    /// This is for diagnostic information, where you need to know details about what -exactly- is going on in the
    /// system. This info doesn’t surface by default.
    Diagnostic = 1,
    /// This is non-critical information, just to let you know what’s going on.
    Inform = 2,
    /// Something bad has happened, but it’s still within the realm of what’s expected.
    Warning = 3,
    /// Danger Will Robinson! Something really bad just happened and needs fixing!
    Error = 4,
}

/// A class for logging errors, warnings and information!
/// Different levels of information can be filtered out, and supports
/// coloration via &lt;`~colorCode`&gt; and &lt;`~clr`&gt; tags.
///
/// Text colors can be set with a tag, and reset back to default with
/// &lt;`~clr`&gt;. Color codes are as follows:
///
/// | Dark | Bright | Description |
/// |------|--------|-------------|
/// | DARK | BRIGHT | DESCRIPTION |
/// | blk  | BLK    | Black       |
/// | red  | RED    | Red         |
/// | grn  | GRN    | Green       |
/// | ylw  | YLW    | Yellow      |
/// | blu  | BLU    | Blue        |
/// | mag  | MAG    | Magenta     |
/// | cyn  | cyn    | Cyan        |
/// | grn  | GRN    | Green       |
/// | wht  | WHT    | White       |
/// <https://stereokit.net/Pages/StereoKit/Log.html>
///
///## Examples
/// ```
/// stereokit_rust::system::Log::info("model <~GRN>node count<~clr> : <~RED>6589<~clr> !!!");
/// ```
pub struct Log;
extern "C" {
    pub fn log_diag(text: *const c_char);
    //pub fn log_diagf(text: *const c_char, ...);
    pub fn log_info(text: *const c_char);
    //pub fn log_infof(text: *const c_char, ...);
    pub fn log_warn(text: *const c_char);
    //pub fn log_warnf(text: *const c_char, ...);
    pub fn log_err(text: *const c_char);
    //pub fn log_errf(text: *const c_char, ...);
    //pub fn log_writef(level: LogLevel, text: *const c_char, ...);
    pub fn log_write(level: LogLevel, text: *const c_char);
    pub fn log_set_filter(level: LogLevel);
    pub fn log_set_colors(colors: LogColors);
    pub fn log_subscribe(
        log_callback: Option<unsafe extern "C" fn(context: *mut c_void, level: LogLevel, text: *const c_char)>,
        context: *mut c_void,
    );
    pub fn log_unsubscribe(
        log_callback: Option<unsafe extern "C" fn(context: *mut c_void, level: LogLevel, text: *const c_char)>,
        context: *mut c_void,
    );
}

/// Log subscribe trampoline
///
/// see also [`crate::system::log_subscribe_data`]
unsafe extern "C" fn log_trampoline<'a, F: FnMut(LogLevel, &str) + 'a>(
    context: *mut c_void,
    log_level: LogLevel,
    text: *const c_char,
) {
    let closure = &mut *(context as *mut &mut F);
    let c_str = CStr::from_ptr(text).to_str().unwrap().trim_end();
    closure(log_level, c_str)
}

impl Log {
    /// What's the lowest level of severity logs to display on the console? Default is LogLevel.Info. This property
    /// can safely be set before SK initialization.
    /// <https://stereokit.net/Pages/StereoKit/Log.html>
    ///
    /// see also [`crate::system::log_set_filter`]
    pub fn filter(filter: LogLevel) {
        unsafe { log_set_filter(filter) }
    }

    /// Set the colors
    /// <https://stereokit.net/Pages/StereoKit/Log.html>
    ///
    /// see also [`crate::system::log_set_colors`]
    pub fn colors(colors: LogColors) {
        unsafe { log_set_colors(colors) }
    }

    /// Writes a formatted line to the log using a LogLevel.Error severity level!
    /// <https://stereokit.net/Pages/StereoKit/Log.html>
    ///
    /// see also [`crate::system::log_err`]
    pub fn err<S: AsRef<str>>(text: S) {
        let c_str = CString::new(text.as_ref()).unwrap();
        unsafe { log_err(c_str.as_ptr()) }
    }

    /// Writes a formatted line to the log using a LogLevel.Inform severity level!
    /// <https://stereokit.net/Pages/StereoKit/Log.html>
    ///
    /// see also [`crate::system::log_info`]
    pub fn info<S: AsRef<str>>(text: S) {
        let c_str = CString::new(text.as_ref()).unwrap();
        unsafe { log_info(c_str.as_ptr()) }
    }

    /// Writes a formatted line to the log using a LogLevel.Warning severity level!
    /// <https://stereokit.net/Pages/StereoKit/Log.html>
    ///
    /// see also [`crate::system::log_warn`]
    pub fn warn<S: AsRef<str>>(text: S) {
        let c_str = CString::new(text.as_ref()).unwrap();
        unsafe { log_warn(c_str.as_ptr()) }
    }

    /// Writes a formatted line to the log using a LogLevel.Diagnostic severity level!
    /// <https://stereokit.net/Pages/StereoKit/Log.html>
    ///
    /// see also [`crate::system::log_diag`]
    pub fn diag<S: AsRef<str>>(text: S) {
        let c_str = CString::new(text.as_ref()).unwrap();
        unsafe { log_diag(c_str.as_ptr()) }
    }

    /// Writes a formatted line to the log with the specified severity level!
    /// <https://stereokit.net/Pages/StereoKit/Log.html>
    ///
    /// see also [`crate::system::log_write`]
    pub fn write<S: AsRef<str>>(level: LogLevel, text: S) {
        let c_str = CString::new(text.as_ref()).unwrap();
        unsafe { log_write(level, c_str.as_ptr()) }
    }

    /// Allows you to listen in on log events! Any callback subscribed here will be called when something is logged.
    /// This does honor the Log.Filter, so filtered logs will not be received here. This method can safely be called
    /// before SK initialization.
    /// <https://stereokit.net/Pages/StereoKit/Log/Subscribe.html>
    ///
    /// see also [`crate::system::log_subscribe`]    
    pub fn subscribe<'a, F: FnMut(LogLevel, &str) + 'a>(mut on_log: F) {
        let mut closure = &mut on_log;
        unsafe { log_subscribe(Some(log_trampoline::<F>), &mut closure as *mut _ as *mut c_void) }
    }

    /// If you subscribed to the log callback, you can unsubscribe that callback here! This method can safely be
    /// called before initialization.
    /// <https://stereokit.net/Pages/StereoKit/Log/Unsubscribe.html>
    ///
    /// see also [`crate::system::log_unsubscribe`]    
    pub fn unsubscribe<'a, F: FnMut(LogLevel, &str) + 'a>(mut on_log: F) {
        let mut closure = &mut on_log;
        unsafe { log_unsubscribe(Some(log_trampoline::<F>), &mut closure as *mut _ as *mut c_void) }
    }
}

/// This class provides access to the hardware’s microphone, and stores it in a Sound stream. Start and Stop recording,
/// and check the Sound property for the results! Remember to ensure your application has microphone permissions
/// enabled!
/// <https://stereokit.net/Pages/StereoKit/Microphone.html>
///
///## Examples
pub struct Microphone;

extern "C" {
    pub fn mic_device_count() -> i32;
    pub fn mic_device_name(index: i32) -> *const c_char;
    pub fn mic_start(device_name: *const c_char) -> Bool32T;
    pub fn mic_stop();
    pub fn mic_get_stream() -> SoundT;
    pub fn mic_is_recording() -> Bool32T;
}

impl Microphone {
    /// This is the sound stream of the Microphone when it is recording. ~~This Asset is created the first time it is
    /// accessed via this property, or during Start, and will persist.~~ It is re-used for the Microphone stream if you
    /// start/stop/switch devices.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/Sound.html>
    ///
    /// see also [crate::system::mic_get_stream]
    pub fn sound() -> Result<Sound, StereoKitError> {
        Ok(Sound(
            NonNull::new(unsafe { mic_get_stream() })
                .ok_or(StereoKitError::SoundCreate("microphone stream".to_string()))?,
        ))
    }

    /// Tells if the Microphone is currently recording audio.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/IsRecording.html>
    ///
    /// see also [crate::system::mic_get_stream]
    pub fn is_recording() -> bool {
        unsafe { mic_is_recording() != 0 }
    }

    /// Constructs a list of valid Microphone devices attached to the system. These names can be passed into start to
    /// select a specific device to record from. It’s recommended to cache this list if you’re using it frequently, as
    /// this list is constructed each time you call it.
    ///
    /// It’s good to note that a user might occasionally plug or unplug microphone devices from their system, so this
    /// list may occasionally change.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/GetDevices.html>
    ///
    /// see also [crate::system::mic_device_count][crate::system::mic_device_name]
    pub fn get_devices() -> Vec<String> {
        let mut devices = Vec::new();
        for i in 0..unsafe { mic_device_count() } {
            let name = unsafe { CStr::from_ptr(mic_device_name(i)).to_str().unwrap() };
            devices.push(name.to_string());
        }
        devices
    }

    /// This begins recording audio from the Microphone! Audio is stored in Microphone.Sound as a stream of audio. If
    /// the Microphone is already recording with a different device, it will stop the previous recording and start again
    ///  with the new device.
    ///
    /// If null is provided as the device, then they system’s default input device will be used. Some systems may not
    /// provide access to devices other than the system’s default.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/Start.html>
    ///
    /// see also [crate::system::mic_start]
    pub fn start(device_name: impl AsRef<str>) -> bool {
        let c_str = CString::new(device_name.as_ref()).unwrap();
        unsafe { mic_start(c_str.as_ptr()) != 0 }
    }

    /// If the Microphone is recording, this will stop it.
    /// <https://stereokit.net/Pages/StereoKit/Microphone/Stop.html>
    ///
    /// see also [crate::system::mic_stop]
    pub fn stop() {
        unsafe { mic_stop() }
    }
}

/// When rendering to a rendertarget, this tells if and what of the rendertarget gets cleared before rendering. For
/// example, if you are assembling a sheet of images, you may want to clear everything on the first image draw, but not
/// clear on subsequent draws.
/// <https://stereokit.net/Pages/StereoKit/RenderClear.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum RenderClear {
    /// Don’t clear anything, leave it as it is.
    None = 0,
    /// Clear the rendertarget’s color data.
    Color = 1,
    /// Clear the rendertarget’s depth data, if present.
    Depth = 2,
    /// Clear both color and depth data.
    All = 3,
}

bitflags::bitflags! {
    /// When rendering content, you can filter what you’re rendering by the RenderLayer that they’re on. This allows
    /// you to draw items that are visible in one render, but not another. For example, you may wish to draw a player’s
    /// avatar in a ‘mirror’ rendertarget, but not in the primary display. See Renderer.LayerFilter for configuring
    /// what the primary display renders.
    /// <https://stereokit.net/Pages/StereoKit/RenderLayer.html>

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct RenderLayer: u32 {
        /// The default render layer. All Draw use this layer unless otherwise specified.
        const Layer0 = 1 << 0;
        /// Render layer 1.
        const Layer1 = 1 << 1;
        /// Render layer 2.
        const Layer2 = 1 << 2;
        /// Render layer 3.
        const Layer3 = 1 << 3;
        /// Render layer 4.
        const Layer4 = 1 << 4;
        /// Render layer 5.
        const Layer5 = 1 << 5;
        /// Render layer 6.
        const Layer6 = 1 << 6;
        /// Render layer 7.
        const Layer7 = 1 << 7;
        /// Render layer 8.
        const Layer8 = 1 << 8;
        /// Render layer 9.
        const Layer9 = 1 << 9;
        /// The default VFX layer, StereoKit draws some non-standard mesh content using this flag, such as lines.
        const Layer_VFX = 10;
        /// For items that should only be drawn from the first person perspective. By default, this is enabled for
        /// renders that are from a 1st person viewpoint.
        const Layer_first_person    = 1 << 11;
        /// For items that should only be drawn from the third person perspective. By default, this is enabled for
        /// renders that are from a 3rd person viewpoint.
        const Layer_third_person    = 1 << 12;
        /// This is a flag that specifies all possible layers. If you want to render all layers, then this is the layer
        ///  filter you would use. This is the default for render filtering.
        const Layer_all = 0xFFFF;
        /// This is a combination of all layers that are not the VFX layer.
        const Layer_all_regular = Self::Layer0.bits() | Self::Layer1.bits() | Self::Layer2.bits() | Self::Layer3.bits() | Self::Layer4.bits() | Self::Layer5.bits() | Self::Layer6.bits() | Self::Layer7.bits() | Self::Layer8.bits() | Self::Layer9.bits();
        /// All layers except for the third person layer.
        const Layer_all_first_person= Self::Layer_all.bits() & !Self::Layer_third_person.bits();
        ///All layers except for the first person layer.
        const Layer_all_third_person= Self::Layer_all.bits() & !Self::Layer_first_person.bits();
    }
}
impl Default for RenderLayer {
    fn default() -> Self {
        RenderLayer::Layer_all
    }
}
impl RenderLayer {
    pub fn as_u32(&self) -> u32 {
        self.bits()
    }
}

/// The projection mode used by StereoKit for the main camera! You can use this with Renderer.Projection. These options
/// are only available in flatscreen mode, as MR headsets provide very specific projection matrices.
/// <https://stereokit.net/Pages/StereoKit/Projection.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum Projection {
    /// This is the default projection mode, and the one you’re most likely to be familiar with! This is where parallel
    /// lines will converge as they go into the distance.
    Perspective = 0,
    /// Orthographic projection mode is often used for tools, 2D rendering, thumbnails of 3D objects, or other similar
    /// cases. In this mode, parallel lines remain parallel regardless of how far they travel.
    Orthographic = 1,
}

/// Do you need to draw something? Well, you’re probably in the right place! This static class includes a variety of
/// different drawing methods, from rendering Models and Meshes, to setting rendering options and drawing to offscreen
/// surfaces! Even better, it’s entirely a static class, so you can call it from anywhere :)
/// <https://stereokit.net/Pages/StereoKit/Renderer.html>
///
///## Examples
pub struct Renderer;

extern "C" {
    pub fn render_set_clip(near_plane: f32, far_plane: f32);
    pub fn render_set_fov(field_of_view_degrees: f32);
    pub fn render_set_ortho_clip(near_plane: f32, far_plane: f32);
    pub fn render_set_ortho_size(viewport_height_meters: f32);
    pub fn render_set_projection(proj: Projection);
    pub fn render_get_projection() -> Projection;
    pub fn render_get_cam_root() -> Matrix;
    pub fn render_set_cam_root(cam_root: *const Matrix);
    pub fn render_set_skytex(sky_texture: TexT);
    pub fn render_get_skytex() -> TexT;
    pub fn render_set_skymaterial(sky_material: MaterialT);
    pub fn render_get_skymaterial() -> MaterialT;
    pub fn render_set_skylight(light_info: *const SphericalHarmonics);
    pub fn render_get_skylight() -> SphericalHarmonics;
    pub fn render_set_filter(layer_filter: RenderLayer);
    pub fn render_get_filter() -> RenderLayer;
    pub fn render_set_scaling(display_tex_scale: f32);
    pub fn render_get_scaling() -> f32;
    pub fn render_set_multisample(display_tex_multisample: i32);
    pub fn render_get_multisample() -> i32;
    pub fn render_override_capture_filter(use_override_filter: Bool32T, layer_filter: RenderLayer);
    pub fn render_get_capture_filter() -> RenderLayer;
    pub fn render_has_capture_filter() -> Bool32T;
    pub fn render_set_clear_color(color_gamma: Color128);
    pub fn render_get_clear_color() -> Color128;
    pub fn render_enable_skytex(show_sky: Bool32T);
    pub fn render_enabled_skytex() -> Bool32T;

    pub fn render_global_texture(register_slot: i32, texture: TexT);
    pub fn render_add_mesh(
        mesh: MeshT,
        material: MaterialT,
        transform: *const Matrix,
        color_linear: Color128,
        layer: RenderLayer,
    );
    pub fn render_add_model(model: ModelT, transform: *const Matrix, color_linear: Color128, layer: RenderLayer);
    pub fn render_add_model_mat(
        model: ModelT,
        material_override: MaterialT,
        transform: *const Matrix,
        color_linear: Color128,
        layer: RenderLayer,
    );
    pub fn render_blit(to_rendertarget: TexT, material: MaterialT);

    pub fn render_screenshot(
        file_utf8: *const c_char,
        file_quality_100: i32,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view_degrees: f32,
    );
    pub fn render_screenshot_capture(
        render_on_screenshot_callback: ::std::option::Option<
            unsafe extern "C" fn(color_buffer: *mut Color32, width: i32, height: i32, context: *mut c_void),
        >,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view_degrees: f32,
        tex_format: TexFormat,
        context: *mut c_void,
    );
    pub fn render_screenshot_viewpoint(
        render_on_screenshot_callback: ::std::option::Option<
            unsafe extern "C" fn(color_buffer: *mut Color32, width: i32, height: i32, context: *mut c_void),
        >,
        camera: Matrix,
        projection: Matrix,
        width: i32,
        height: i32,
        layer_filter: RenderLayer,
        clear: RenderClear,
        viewport: Rect,
        tex_format: TexFormat,
        context: *mut c_void,
    );
    pub fn render_to(
        to_rendertarget: TexT,
        camera: *const Matrix,
        projection: *const Matrix,
        layer_filter: RenderLayer,
        clear: RenderClear,
        viewport: Rect,
    );
    pub fn render_MaterialTo(
        to_rendertarget: TexT,
        override_material: MaterialT,
        camera: *const Matrix,
        projection: *const Matrix,
        layer_filter: RenderLayer,
        clear: RenderClear,
        viewport: Rect,
    );
    pub fn render_get_device(device: *mut *mut c_void, context: *mut *mut c_void);

}

/// screenshot_capture trampoline
///
/// see also [`Renderer::screenshot_capture`]
unsafe extern "C" fn sc_capture_trampoline<F: FnMut(&[Color32], usize, usize)>(
    color_buffer: *mut Color32,
    width: i32,
    height: i32,
    context: *mut c_void,
) {
    let closure = &mut *(context as *mut &mut F);
    closure(std::slice::from_raw_parts(color_buffer, (width * height) as usize), width as usize, height as usize)
}

impl Renderer {
    /// Sets the root transform of the camera! This will be the identity matrix by default. The user’s head
    /// location will then be relative to this point. This is great to use if you’re trying to do teleportation,
    /// redirected walking, or just shifting the floor around.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/CameraRoot.html>
    ///
    /// see also [`crate::system::render_set_cam_root`]
    pub fn camera_root(transform: impl Into<Matrix>) {
        unsafe { render_set_cam_root(&transform.into()) }
    }

    /// This is the gamma space color the renderer will clear the screen to when beginning to draw a new frame.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/ClearColor.html>
    ///
    /// see also [`crate::system::render_set_clear_color`]
    pub fn clear_color(color_gamma: impl Into<Color128>) {
        unsafe { render_set_clear_color(color_gamma.into()) }
    }

    /// Enables or disables rendering of the skybox texture! It’s enabled by default on Opaque displays, and completely
    /// unavailable for transparent displays.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/EnableSky.html>
    ///
    /// see also [`crate::system::render_enable_skytex`]
    pub fn enable_sky(enable: bool) {
        unsafe { render_enable_skytex(enable as Bool32T) }
    }

    /// By default, StereoKit renders all first-person layers. This is a bit flag that allows you to change which layers
    /// StereoKit renders for the primary viewpoint. To change what layers a visual is on, use a Draw method that
    /// includes a RenderLayer as a parameter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/LayerFilter.html>
    ///
    /// see also [`crate::system::render_set_filter`]
    pub fn layer_filter(filter: RenderLayer) {
        unsafe { render_set_filter(filter) }
    }

    /// Allows you to set the multisample (MSAA) level of the render surface. Valid values are 1, 2, 4, 8, 16, though
    /// some OpenXR runtimes may clamp this to lower values. Note that while this can greatly smooth out edges, it also
    /// greatly increases RAM usage and fill rate, so use it sparingly. Only works in XR mode. If known in advance, set
    /// this via SKSettings in initialization. This is a very costly change to make.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Multisample.html>
    ///
    /// see also [`crate::system::render_set_multisample`]
    pub fn multisample(level: i32) {
        unsafe { render_set_multisample(level) }
    }

    /// For flatscreen applications only! This allows you to change the camera projection between perspective and
    /// orthographic projection. This may be of interest for some category of UI work, but is generally a niche piece of
    /// functionality.
    /// Swapping between perspective and orthographic will also switch the clipping planes and field of view to the
    /// values associated with that mode. See SetClip/SetFov for perspective, and SetOrthoClip/SetOrthoSize for
    /// orthographic.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Projection.html>
    ///
    /// see also [`crate::system::render_set_projection`]
    pub fn projection(projection: Projection) {
        unsafe { render_set_projection(projection) }
    }

    /// OpenXR has a recommended default for the main render surface, this value allows you to set SK’s surface to a
    /// multiple of the recommended size. Note that the final resolution may also be clamped or quantized. Only works in
    /// XR mode. If known in advance, set this via SKSettings in initialization. This is a very costly change to make.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Scaling.html>
    ///
    /// see also [`crate::system::render_set_scaling`]
    pub fn scaling(scaling: f32) {
        unsafe { render_set_scaling(scaling) }
    }

    /// Sets the lighting information for the scene! You can build one through SphericalHarmonics.FromLights, or grab
    /// one from Tex.FromEquirectangular or Tex.GenCubemap
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    ///
    /// see also [`crate::system::render_set_skylight`]
    pub fn skylight(sh: SphericalHarmonics) {
        unsafe { render_set_skylight(&sh) }
    }

    /// Set a cubemap skybox texture for rendering a background! This is only visible on Opaque displays, since
    /// transparent displays have the real world behind them already! StereoKit has a a default procedurally generated
    /// skybox. You can load one with Tex.FromEquirectangular, Tex.GenCubemap. If you’re trying to affect the lighting,
    /// see Renderer.SkyLight.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also [`crate::system::render_set_skytex`]
    pub fn skytex(tex: impl AsRef<Tex>) {
        unsafe { render_set_skytex(tex.as_ref().0.as_ptr()) }
    }

    /// This is the Material that StereoKit is currently using to draw the skybox! It needs a special shader that's
    /// tuned for a full-screen quad. If you just want to change the skybox image, try setting `Renderer.SkyTex`
    /// instead.
    ///  
    /// This value will never be null! If you try setting this to null, it will assign SK's built-in default sky
    /// material. If you want to turn off the skybox, see `Renderer.EnableSky` instead.
    ///  
    /// Recommended Material settings would be:
    /// - DepthWrite: false
    /// - DepthTest: LessOrEq
    /// - QueueOffset: 100</summary>
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyMaterial.html>
    ///
    /// see also [`crate::system::render_set_skymaterial`]
    pub fn skymaterial(material: impl AsRef<Material>) {
        unsafe { render_set_skymaterial(material.as_ref().0.as_ptr()) }
    }

    /// Adds a mesh to the render queue for this frame! If the Hierarchy has a transform on it, that transform is
    /// combined with the Matrix provided here.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Add.html>
    /// * color - If None has default value of WHITE
    /// * layer - If None has default value of RenderLayer::Layer0
    ///
    /// see also [`crate::system::render_add_mesh`]
    pub fn add_mesh(
        mesh: impl AsRef<Mesh>,
        material: impl AsRef<Material>,
        transform: impl Into<Matrix>,
        color: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color = color.unwrap_or(Color128::WHITE);
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe {
            render_add_mesh(mesh.as_ref().0.as_ptr(), material.as_ref().0.as_ptr(), &transform.into(), color, layer)
        }
    }

    /// Adds a Model to the render queue for this frame! If the Hierarchy has a transform on it, that transform is
    /// combined with the Matrix provided here.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Add.html>
    /// * color - If None has default value of WHITE
    /// * layer - If None has default value of RenderLayer::Layer0
    ///
    /// see also [`crate::system::render_add_model`]
    pub fn add_model(
        model: impl AsRef<Model>,
        transform: impl Into<Matrix>,
        color: Option<Color128>,
        layer: Option<RenderLayer>,
    ) {
        let color = color.unwrap_or(Color128::WHITE);
        let layer = layer.unwrap_or(RenderLayer::Layer0);
        unsafe { render_add_model(model.as_ref().0.as_ptr(), &transform.into(), color, layer) }
    }

    /// Renders a Material onto a rendertarget texture! StereoKit uses a 4 vert quad stretched over the surface of the
    /// texture, and renders the material onto it to the texture.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Blit.html>
    ///
    /// see also [`crate::system::render_blit`]
    pub fn blit(tex: impl AsRef<Tex>, material: impl AsRef<Material>) {
        unsafe { render_blit(tex.as_ref().0.as_ptr(), material.as_ref().0.as_ptr()) }
    }

    /// The CaptureFilter is a layer mask for Mixed Reality Capture, or 2nd person observer rendering. On HoloLens and
    /// WMR, this is the video rendering feature. This allows you to hide, or reveal certain draw calls when rendering
    /// video output.
    ///
    /// By default, the CaptureFilter will always be the same as Render.LayerFilter, overriding this will mean this
    /// filter no longer updates with LayerFilter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/OverrideCaptureFilter.html>
    ///
    /// see also [`crate::system::render_override_capture_filter`]
    pub fn override_capture_filter(use_override_filter: bool, override_filter: RenderLayer) {
        unsafe { render_override_capture_filter(use_override_filter as Bool32T, override_filter) }
    }

    /// This renders the current scene to the indicated rendertarget texture, from the specified viewpoint. This call
    /// enqueues a render that occurs immediately before the screen itself is rendered.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/RenderTo.html>
    /// * layer_filter - If None has default value of RenderLayer::ALL
    /// * clear - If None has default value of RenderClear::All
    /// * vieport - If None has default value of (0, 0, 0, 0)
    ///
    /// see also [`crate::system::render_to`]
    pub fn render_to<M: Into<Matrix>>(
        to_render_target: impl AsRef<Tex>,
        camera: M,
        projection: M,
        layer_filter: Option<RenderLayer>,
        clear: Option<RenderClear>,
        viewport: Option<Rect>,
    ) {
        let layer_filter = layer_filter.unwrap_or(RenderLayer::Layer_all);
        let clear = clear.unwrap_or(RenderClear::All);
        let viewport = viewport.unwrap_or_default();

        unsafe {
            render_to(
                to_render_target.as_ref().0.as_ptr(),
                &camera.into(),
                &projection.into(),
                layer_filter,
                clear,
                viewport,
            )
        }
    }

    /// Schedules a screenshot for the end of the frame! The view will be rendered from the given pose, with a
    /// resolution the same size as the screen’s surface. It’ll be saved as a JPEG or PNG file depending on the filename
    /// extension provided.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
    /// * file_quality - should be 90 in most of the case for 90%
    /// * viewpoint - is Pose::look_at(from_point, looking_at_point)
    /// * field_of_view - If None will use default value of 90°
    ///
    /// see also [`crate::system::render_screenshot_pose`]
    pub fn screenshot(
        filename: impl AsRef<Path>,
        file_quality: i32,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view: Option<f32>,
    ) {
        let path = filename.as_ref();
        let c_str = CString::new(path.to_str().unwrap_or("!!!path.to_str error!!!").to_owned()).unwrap();
        let field_of_view = field_of_view.unwrap_or(90.0);
        unsafe { render_screenshot(c_str.as_ptr(), file_quality, viewpoint, width, height, field_of_view) }
    }

    /// Schedules a screenshot for the end of the frame! The view will be rendered from the given position at the given
    /// point, with a resolution the same size as the screen’s surface. This overload allows for retrieval of the color
    /// data directly from the render thread! You can use the color data directly by saving/processing it inside your
    /// callback, or you can keep the data alive for as long as it is referenced.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
    /// * on_screenshot : closure |&[Color32], width:usize, height:usize|
    /// * viewpoint - is Pose::look_at(from_point, looking_at_point)
    /// * field_of_view - If None will use default value of 90°
    /// * tex_format - If None will use default value of TexFormat::RGBA32
    ///
    /// see also [`crate::system::render_screenshot_pose`]
    pub fn screenshot_capture<F: FnMut(&[Color32], usize, usize)>(
        mut on_screenshot: F,
        viewpoint: Pose,
        width: i32,
        height: i32,
        field_of_view: Option<f32>,
        tex_format: Option<TexFormat>,
    ) {
        let field_of_view = field_of_view.unwrap_or(90.0);
        let tex_format = tex_format.unwrap_or(TexFormat::RGBA32);
        let mut closure = &mut on_screenshot;
        unsafe {
            render_screenshot_capture(
                Some(sc_capture_trampoline::<F>),
                viewpoint,
                width,
                height,
                field_of_view,
                tex_format,
                &mut closure as *mut _ as *mut c_void,
            )
        }
    }

    /// <https://stereokit.net/Pages/StereoKit/Renderer/Screenshot.html>
    /// * on_screenshot : closure |&[Color32], width:usize, height:usize|
    /// * render_layer - If None will use default value of All
    /// * clear - If None wille use default value of All
    /// * tex_format - If None will use default value of TexFormat::RGBA32
    ///
    /// see also [`crate::system::render_screenshot_pose`]
    #[allow(clippy::too_many_arguments)]
    pub fn screenshot_viewpoint<M: Into<Matrix>, F: FnMut(&[Color32], usize, usize)>(
        mut on_screenshot: F,
        camera: M,
        projection: M,
        width: i32,
        height: i32,
        render_layer: Option<RenderLayer>,
        clear: Option<RenderClear>,
        viewport: Option<Rect>,
        tex_format: Option<TexFormat>,
    ) {
        let tex_format = tex_format.unwrap_or(TexFormat::RGBA32);
        let render_layer = render_layer.unwrap_or(RenderLayer::all());
        let clear = clear.unwrap_or(RenderClear::All);
        let viewport = viewport.unwrap_or_default();
        let mut closure = &mut on_screenshot;
        unsafe {
            render_screenshot_viewpoint(
                Some(sc_capture_trampoline::<F>),
                camera.into(),
                projection.into(),
                width,
                height,
                render_layer,
                clear,
                viewport,
                tex_format,
                &mut closure as *mut _ as *mut c_void,
            )
        }
    }

    /// Set the near and far clipping planes of the camera! These are important to z-buffer quality, especially when
    /// using low bit depth z-buffers as recommended for devices like the HoloLens. The smaller the range between the
    /// near and far planes, the better your z-buffer will look! If you see flickering on objects that are overlapping,
    /// try making the range smaller.
    ///
    /// These values only affect perspective mode projection, which is the default projection mode.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetClip.html>
    ///
    /// see also [`crate::system::render_set_clip`]
    pub fn set_clip(near_plane: f32, far_plane: f32) {
        unsafe { render_set_clip(near_plane, far_plane) }
    }

    /// Only works for flatscreen! This updates the camera’s projection matrix with a new field of view.
    ///
    /// This value only affects perspective mode projection, which is the default projection mode.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetFOV.html>
    ///
    /// see also [`crate::system::render_set_fov`]
    pub fn set_fov(field_of_view: f32) {
        unsafe { render_set_fov(field_of_view) }
    }

    /// Set the near and far clipping planes of the camera! These are important to z-buffer quality, especially when
    /// using low bit depth z-buffers as recommended for devices like the HoloLens. The smaller the range between the
    /// near and far planes, the better your z-buffer will look! If you see flickering on objects that are overlapping,
    /// try making the range smaller.
    ///
    /// These values only affect orthographic mode projection, which is only available in flatscreen.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetOrthoClip.html>
    ///
    /// see also [`crate::system::render_set_ortho_clip`]
    pub fn set_ortho_clip(near_plane: f32, far_plane: f32) {
        unsafe { render_set_ortho_clip(near_plane, far_plane) }
    }

    /// This sets the size of the orthographic projection’s viewport. You can use this feature to zoom in and out of the
    /// scene.
    ///
    /// This value only affects orthographic mode projection, which is only available in flatscreen.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SetOrthoSize.html>
    ///
    /// see also [`crate::system::render_set_ortho_size`]
    pub fn set_ortho_size(view_port_height_meters: f32) {
        unsafe { render_set_ortho_size(view_port_height_meters) }
    }

    /// Gets the root transform of the camera! This will be the identity matrix by default. The user’s head
    /// location will then be relative to this point. This is great to use if you’re trying to do teleportation,
    /// redirected walking, or just shifting the floor around.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/CameraRoot.html>
    ///
    /// see also [`crate::system::render_get_cam_root`]
    pub fn get_camera_root() -> Matrix {
        unsafe { render_get_cam_root() }
    }

    /// This is the current render layer mask for Mixed Reality Capture, or 2nd person observer rendering. By default,
    /// this is directly linked to Renderer.LayerFilter, but this behavior can be overridden via
    /// Renderer.OverrideCaptureFilter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/CaptureFilter.html>
    ///
    /// see also [`crate::system::render_get_capture_filter`]
    pub fn get_capture_filter() -> RenderLayer {
        unsafe { render_get_capture_filter() }
    }

    /// This is the gamma space color the renderer will clear the screen to when beginning to draw a new frame.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/ClearColor.html>
    ///
    /// see also [`crate::system::render_get_clear_color`]
    pub fn get_clear_color() -> Color128 {
        unsafe { render_get_clear_color() }
    }

    /// Enables or disables rendering of the skybox texture! It’s enabled by default on Opaque displays, and completely
    /// unavailable for transparent displays.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/EnableSky.html>
    ///
    /// see also [`crate::system::render_enabled_skytex`]
    pub fn get_enable_sky() -> bool {
        unsafe { render_enabled_skytex() != 0 }
    }

    /// This tells if CaptureFilter has been overridden to a specific value via Renderer.OverrideCaptureFilter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/HasCaptureFilter.html>
    ///
    /// see also [`crate::system::render_has_capture_filter`]
    pub fn has_capture_filter() -> bool {
        unsafe { render_has_capture_filter() != 0 }
    }

    /// By default, StereoKit renders all first-person layers. This is a bit flag that allows you to change which layers
    /// StereoKit renders for the primary viewpoint. To change what layers a visual is on, use a Draw method that
    /// includes a RenderLayer as a parameter.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/LayerFilter.html>
    ///
    /// see also [`crate::system::render_get_filter`]
    pub fn get_layer_filter() -> RenderLayer {
        unsafe { render_get_filter() }
    }

    /// Get the multisample (MSAA) level of the render surface. Valid values are 1, 2, 4, 8, 16, though
    /// some OpenXR runtimes may clamp this to lower values. Note that while this can greatly smooth out edges, it also
    /// greatly increases RAM usage and fill rate, so use it sparingly. Only works in XR mode. If known in advance, set
    /// this via SKSettings in initialization. This is a very costly change to make.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Multisample.html>
    ///
    /// see also [`crate::system::render_get_multisample`]
    pub fn get_multisample() -> i32 {
        unsafe { render_get_multisample() }
    }

    /// For flatscreen applications only! This allows you to get the camera projection between perspective and
    /// orthographic projection. This may be of interest for some category of UI work, but is generally a niche piece of
    /// functionality.
    /// Swapping between perspective and orthographic will also switch the clipping planes and field of view to the
    /// values associated with that mode. See SetClip/SetFov for perspective, and SetOrthoClip/SetOrthoSize for
    /// orthographic.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Projection.html>
    ///
    /// see also [`crate::system::render_get_projection`]
    pub fn get_projection() -> Projection {
        unsafe { render_get_projection() }
    }

    /// OpenXR has a recommended default for the main render surface, this value allows you to set SK’s surface to a
    /// multiple of the recommended size. Note that the final resolution may also be clamped or quantized. Only works in
    /// XR mode. If known in advance, set this via SKSettings in initialization. This is a very costly change to make.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/Scaling.html>
    ///
    /// see also [`crate::system::render_get_scaling`]
    pub fn get_scaling() -> f32 {
        unsafe { render_get_scaling() }
    }

    /// Gets the lighting information for the scene! You can build one through SphericalHarmonics.FromLights, or grab
    /// one from Tex.FromEquirectangular or Tex.GenCubemap
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyLight.html>
    ///
    /// see also [`crate::system::render_get_skylight`]
    pub fn get_skylight() -> SphericalHarmonics {
        unsafe { render_get_skylight() }
    }

    /// Get the cubemap skybox texture for rendering a background! This is only visible on Opaque displays, since
    /// transparent displays have the real world behind them already! StereoKit has a a default procedurally generated
    /// skybox. You can load one with Tex.FromEquirectangular, Tex.GenCubemap. If you’re trying to affect the lighting,
    /// see Renderer.SkyLight.
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyTex.html>
    ///
    /// see also [`crate::system::render_get_skytex`]
    pub fn get_skytex() -> Tex {
        Tex(NonNull::new(unsafe { render_get_skytex() }).unwrap())
    }

    /// This is the Material that StereoKit is currently using to draw the skybox! It needs a special shader that's
    /// tuned for a full-screen quad. If you just want to change the skybox image, try setting `Renderer.SkyTex`
    /// instead.
    ///  
    /// This value will never be null! If you try setting this to null, it will assign SK's built-in default sky
    /// material. If you want to turn off the skybox, see `Renderer.EnableSky` instead.
    ///  
    /// Recommended Material settings would be:
    /// - DepthWrite: false
    /// - DepthTest: LessOrEq
    /// - QueueOffset: 100</summary>
    /// <https://stereokit.net/Pages/StereoKit/Renderer/SkyMaterial.html>
    ///
    /// see also [`crate::system::render_get_skymaterial`]
    pub fn get_skymaterial() -> Material {
        Material(NonNull::new(unsafe { render_get_skymaterial() }).unwrap())
    }
}

/// A text style is a font plus size/color/material parameters, and are used to keep text looking more consistent
/// through the application by encouraging devs to re-use styles throughout the project. See Text.MakeStyle for making a
/// TextStyle object.
/// <https://stereokit.net/Pages/StereoKit/TextStyle.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct TextStyle {
    _id: u32,
}

extern "C" {
    pub fn text_make_style(font: FontT, character_height: f32, color_gamma: Color128) -> TextStyle;
    pub fn text_make_style_shader(
        font: FontT,
        character_height: f32,
        shader: ShaderT,
        color_gamma: Color128,
    ) -> TextStyle;
    pub fn text_make_style_mat(
        font: FontT,
        character_height: f32,
        material: MaterialT,
        color_gamma: Color128,
    ) -> TextStyle;
    pub fn text_style_get_material(style: TextStyle) -> MaterialT;
    pub fn text_style_get_char_height(style: TextStyle) -> f32;
    pub fn text_style_set_char_height(style: TextStyle, height_meters: f32);
}

impl Default for TextStyle {
    /// This is the default text style used by StereoKit.
    /// <https://stereokit.net/Pages/StereoKit/TextStyle/Default.html>
    fn default() -> Self {
        Self { _id: 0 }
    }
}

impl TextStyle {
    /// Create a text style for use with other text functions! A text style is a font plus size/color/material
    /// parameters, and are used to keep text looking more consistent through the application by encouraging devs to
    /// re-use styles throughout the project.
    ///
    /// This fn will create an unique Material for this style based on Default.ShaderFont.
    /// <https://stereokit.net/Pages/StereoKit/TextStyle/FromFont.html>
    ///
    /// see also [`crate::system::text_make_style`]
    pub fn from_font(font: impl AsRef<Font>, character_height_meters: f32, color_gamma: impl Into<Color128>) -> Self {
        unsafe { text_make_style(font.as_ref().0.as_ptr(), character_height_meters, color_gamma.into()) }
    }

    /// Create a text style for use with other text functions! A text style is a font plus size/color/material
    /// parameters, and are used to keep text looking more consistent through the application by encouraging devs to
    /// re-use styles throughout the project.
    ///
    /// This function will create an unique Material for this style based on the provided Shader.
    /// <https://stereokit.net/Pages/StereoKit/TextStyle/FromFont.html>
    ///
    /// see also [`crate::system::text_make_style_shader`]
    pub fn from_font_and_shader(
        font: impl AsRef<Font>,
        character_height_meters: f32,
        shader: impl AsRef<Shader>,
        color_gamma: impl Into<Color128>,
    ) -> Self {
        unsafe {
            text_make_style_shader(
                font.as_ref().0.as_ptr(),
                character_height_meters,
                shader.as_ref().0.as_ptr(),
                color_gamma.into(),
            )
        }
    }

    /// Create a text style for use with other text functions! A text style is a font plus size/color/material
    /// parameters, and are used to keep text looking more consistent through the application by encouraging devs to
    /// re-use styles throughout the project.
    ///
    /// This overload allows you to set the specific Material that is used. This can be helpful if you’re keeping styles
    /// similar enough to re-use the material and save on draw calls. If you don’t know what that means, then prefer
    /// using the overload that takes a Shader, or takes neither a Shader nor a Material!
    /// <https://stereokit.net/Pages/StereoKit/TextStyle/FromFont.html>
    ///
    /// see also [`crate::system::text_make_style_mat`]
    pub fn from_font_and_material(
        font: impl AsRef<Font>,
        character_height_meters: f32,
        material: impl AsRef<Material>,
        color_gamma: impl Into<Color128>,
    ) -> Self {
        unsafe {
            text_make_style_mat(
                font.as_ref().0.as_ptr(),
                character_height_meters,
                material.as_ref().0.as_ptr(),
                color_gamma.into(),
            )
        }
    }

    /// Height of a text glyph in meters. StereoKit currently bases this on the letter ‘T’.
    /// <https://stereokit.net/Pages/StereoKit/TextStyle/CharHeight.html>
    ///
    /// see also [`crate::system::text_style_set_char_height`]
    pub fn char_height(&mut self, char_height: f32) {
        unsafe { text_style_set_char_height(*self, char_height) }
    }

    /// Returns the maximum height of a text character using this style, in meters.
    /// <https://stereokit.net/Pages/StereoKit/TextStyle/CharHeight.html>
    ///
    /// see also [`crate::system::text_style_get_char_height`]
    pub fn get_char_height(&self) -> f32 {
        unsafe { text_style_get_char_height(*self) }
    }

    /// This provides a reference to the Material used by this style, so you can override certain features! Note that if
    /// you’re creating TextStyles with manually provided Materials, this Material may not be unique to this style.
    /// <https://stereokit.net/Pages/StereoKit/TextStyle/Material.html>
    ///
    /// see also [`crate::system::text_style_get_material`]
    pub fn get_material(&self) -> Material {
        Material(NonNull::new(unsafe { text_style_get_material(*self) }).unwrap())
    }
}

/// An enum for describing alignment or positioning
/// <https://stereokit.net/Pages/StereoKit/TextAlign.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum TextAlign {
    /// On the x axis, this item should start on the left.
    XLeft = 1,
    /// On the y axis, this item should start at the top.
    YTop = 2,
    /// On the x axis, the item should be centered.
    XCenter = 4,
    /// On the y axis, the item should be centered.
    YCenter = 8,
    /// On the x axis, this item should start on the right.
    XRight = 16,
    /// On the y axis, this item should start on the bottom.
    YBottom = 32,
    /// Center on both X and Y axes. This is a combination of XCenter and YCenter.
    Center = 12,
    /// Start on the left of the X axis, center on the Y axis. This is a combination of XLeft and YCenter.
    CenterLeft = 9,
    /// Start on the right of the X axis, center on the Y axis. This is a combination of XRight and YCenter.
    CenterRight = 24,
    /// Center on the X axis, and top on the Y axis. This is a combination of XCenter and YTop.
    TopCenter = 6,
    /// Start on the left of the X axis, and top on the Y axis. This is a combination of XLeft and YTop.
    TopLeft = 3,
    /// Start on the right of the X axis, and top on the Y axis. This is a combination of XRight and YTop.
    TopRight = 18,
    /// Center on the X axis, and bottom on the Y axis. This is a combination of XCenter and YBottom.
    BottomCenter = 36,
    /// Start on the left of the X axis, and bottom on the Y axis. This is a combination of XLeft and YBottom.
    BottomLeft = 33,
    /// Start on the right of the X axis, and bottom on the Y axis.This is a combination of XRight and YBottom.
    BottomRight = 48,
}

bitflags::bitflags! {
    /// This enum describes how text layout behaves within the space it is given.
    /// <https://stereokit.net/Pages/StereoKit/TextFit.html>
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    #[repr(C)]
    pub struct TextFit: u32 {
        /// The text will wrap around to the next line down when it reaches the end of the space on the X axis.
        const Wrap = 1;
        /// When the text reaches the end, it is simply truncated and no longer visible.
        const Clip = 2;
        /// If the text is too large to fit in the space provided, it will be scaled down to fit inside. This will not scale up.
        const Squeeze = 4;
        /// If the text is larger, or smaller than the space provided, it will scale down or up to fill the space.
        const Exact = 8;
        /// The text will ignore the containing space, and just keep on going.
        const Overflow = 16;
}
}

/// Soft keyboard layouts are often specific to the type of text that they’re editing! This enum is a collection of
/// common text contexts that SK can pass along to the OS’s soft keyboard for a more optimal layout.
/// <https://stereokit.net/Pages/StereoKit/TextContext.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum TextContext {
    /// General text editing, this is the most common type of text, and would result in a ‘standard’ keyboard layout.
    Text = 0,
    /// Numbers and numerical values.
    Number = 1,
    /// This text specifically represents some kind of URL/URI address.
    Uri = 2,
    /// This is a password, and should not be visible when typed!
    Password = 3,
}

/// A collection of functions for rendering and working with text. These are a lower level access to text rendering than
/// the UI text functions, and are completely unaware of the UI code.
/// <https://stereokit.net/Pages/StereoKit/Text.html>
pub struct Text;

extern "C" {
    pub fn text_add_at(
        text_utf8: *const c_char,
        transform: *const Matrix,
        style: TextStyle,
        position: TextAlign,
        align: TextAlign,
        off_x: f32,
        off_y: f32,
        off_z: f32,
        vertex_tint_linear: Color128,
    );
    pub fn text_add_at_16(
        text_utf16: *const c_ushort,
        transform: *const Matrix,
        style: TextStyle,
        position: TextAlign,
        align: TextAlign,
        off_x: f32,
        off_y: f32,
        off_z: f32,
        vertex_tint_linear: Color128,
    );
    pub fn text_add_in(
        text_utf8: *const c_char,
        transform: *const Matrix,
        size: Vec2,
        fit: TextFit,
        style: TextStyle,
        position: TextAlign,
        align: TextAlign,
        off_x: f32,
        off_y: f32,
        off_z: f32,
        vertex_tint_linear: Color128,
    ) -> f32;
    pub fn text_add_in_16(
        text_utf16: *const c_ushort,
        transform: *const Matrix,
        size: Vec2,
        fit: TextFit,
        style: TextStyle,
        position: TextAlign,
        align: TextAlign,
        off_x: f32,
        off_y: f32,
        off_z: f32,
        vertex_tint_linear: Color128,
    ) -> f32;
    pub fn text_size(text_utf8: *const c_char, style: TextStyle) -> Vec2;
    pub fn text_size_16(text_utf16: *const c_ushort, style: TextStyle) -> Vec2;
    pub fn text_char_at(
        text_utf8: *const c_char,
        style: TextStyle,
        char_index: i32,
        opt_size: *mut Vec2,
        fit: TextFit,
        position: TextAlign,
        align: TextAlign,
    ) -> Vec2;
    pub fn text_char_at_16(
        text_utf16: *const c_ushort,
        style: TextStyle,
        char_index: i32,
        opt_size: *mut Vec2,
        fit: TextFit,
        position: TextAlign,
        align: TextAlign,
    ) -> Vec2;
}

impl Text {
    /// Create a text style for use with other text functions! A text style is a font plus size/color/material
    /// parameters, and are used to keep text looking more consistent through the application by encouraging devs to
    /// re-use styles throughout the project.
    ///
    /// This fn will create an unique Material for this style based on Default.ShaderFont.
    /// <https://stereokit.net/Pages/StereoKit/Text/MakeStyle.html>
    ///
    /// see also [`crate::system::text_make_style`]
    pub fn make_style(
        font: impl AsRef<Font>,
        character_height_meters: f32,
        color_gamma: impl Into<Color128>,
    ) -> TextStyle {
        unsafe { text_make_style(font.as_ref().0.as_ptr(), character_height_meters, color_gamma.into()) }
    }

    /// Create a text style for use with other text functions! A text style is a font plus size/color/material
    /// parameters, and are used to keep text looking more consistent through the application by encouraging devs to
    /// re-use styles throughout the project.
    ///
    /// This function will create an unique Material for this style based on the provided Shader.
    /// <https://stereokit.net/Pages/StereoKit/Text/MakeStyle.html>
    ///
    /// see also [`crate::system::text_make_style_shader`]
    pub fn make_style_with_shader(
        font: impl AsRef<Font>,
        character_height_meters: f32,
        shader: impl AsRef<Shader>,
        color_gamma: impl Into<Color128>,
    ) -> TextStyle {
        unsafe {
            text_make_style_shader(
                font.as_ref().0.as_ptr(),
                character_height_meters,
                shader.as_ref().0.as_ptr(),
                color_gamma.into(),
            )
        }
    }

    /// Create a text style for use with other text functions! A text style is a font plus size/color/material
    /// parameters, and are used to keep text looking more consistent through the application by encouraging devs to
    /// re-use styles throughout the project.
    ///
    /// This overload allows you to set the specific Material that is used. This can be helpful if you’re keeping styles
    /// similar enough to re-use the material and save on draw calls. If you don’t know what that means, then prefer
    /// using the overload that takes a Shader, or takes neither a Shader nor a Material!
    /// <https://stereokit.net/Pages/StereoKit/Text/MakeStyle.html>
    ///
    /// see also [`crate::system::text_make_mat`]
    pub fn make_style_with_material(
        font: impl AsRef<Font>,
        character_height_meters: f32,
        material: impl AsRef<Material>,
        color_gamma: impl Into<Color128>,
    ) -> TextStyle {
        unsafe {
            text_make_style_mat(
                font.as_ref().0.as_ptr(),
                character_height_meters,
                material.as_ref().0.as_ptr(),
                color_gamma.into(),
            )
        }
    }

    /// Renders text at the given location! Must be called every frame you want this text to be visible.
    /// <https://stereokit.net/Pages/StereoKit/Text/Add.html>
    /// * text_style - if None will use the TextStyle::default()
    /// * vertex_tint_linear - if None will use Color128::WHITE
    /// * position - if None will use TextAlign::Center
    /// * align - if None will use TextAlign::Center
    /// * off_? - if None will use 0.0
    ///
    /// see also [`crate::system::text_add_at`]
    #[allow(clippy::too_many_arguments)]
    pub fn add_at(
        text: impl AsRef<str>,
        transform: impl Into<Matrix>,
        text_style: Option<TextStyle>,
        vertex_tint_linear: Option<Color128>,
        position: Option<TextAlign>,
        align: Option<TextAlign>,
        off_x: Option<f32>,
        off_y: Option<f32>,
        off_z: Option<f32>,
    ) {
        let c_str = CString::new(text.as_ref()).unwrap();
        let style = text_style.unwrap_or_default();
        let vertex_tint_linear = vertex_tint_linear.unwrap_or(Color128::WHITE);
        let position = position.unwrap_or(TextAlign::Center);
        let align = align.unwrap_or(TextAlign::Center);
        let off_x = off_x.unwrap_or(0.0);
        let off_y = off_y.unwrap_or(0.0);
        let off_z = off_z.unwrap_or(0.0);
        unsafe {
            text_add_at(
                c_str.as_ptr(),
                &transform.into(),
                style,
                position,
                align,
                off_x,
                off_y,
                off_z,
                vertex_tint_linear,
            )
        }
    }

    /// Renders text at the given location! Must be called every frame you want this text to be visible.
    /// <https://stereokit.net/Pages/StereoKit/Text/Add.html>
    /// * text_style - if None will use the TextStyle::default()
    /// * vertex_tint_linear - if None will use Color128::WHITE
    /// * position - if None will use TextAlign::Center
    /// * align - if None will use TextAlign::Center
    /// * off_? - if None will use 0.0
    /// Returns the vertical space used by this text.
    ///
    /// see also [`crate::system::text_add_in`]
    #[allow(clippy::too_many_arguments)]
    pub fn add_in(
        text: impl AsRef<str>,
        transform: impl Into<Matrix>,
        size: impl Into<Vec2>,
        fit: TextFit,
        text_style: Option<TextStyle>,
        vertex_tint_linear: Option<Color128>,
        position: Option<TextAlign>,
        align: Option<TextAlign>,
        off_x: Option<f32>,
        off_y: Option<f32>,
        off_z: Option<f32>,
    ) -> f32 {
        let c_str = CString::new(text.as_ref()).unwrap();
        let style = text_style.unwrap_or_default();
        let vertex_tint_linear = vertex_tint_linear.unwrap_or(Color128::WHITE);
        let position = position.unwrap_or(TextAlign::Center);
        let align = align.unwrap_or(TextAlign::Center);
        let off_x = off_x.unwrap_or(0.0);
        let off_y = off_y.unwrap_or(0.0);
        let off_z = off_z.unwrap_or(0.0);
        unsafe {
            text_add_in(
                c_str.as_ptr(),
                &transform.into(),
                size.into(),
                fit,
                style,
                position,
                align,
                off_x,
                off_y,
                off_z,
                vertex_tint_linear,
            )
        }
    }

    /// Sometimes you just need to know how much room some text takes up! This finds the size of the text in meters when
    /// using the indicated style!
    /// <https://stereokit.net/Pages/StereoKit/Text/Size.html>
    /// * text_style - if None will use the TextStyle::default()
    /// Returns size of the text in meters
    ///
    /// see also [`crate::system::text_size`]
    pub fn size(text: impl AsRef<str>, text_style: Option<TextStyle>) -> Vec2 {
        let c_str = CString::new(text.as_ref()).unwrap();
        let style = text_style.unwrap_or_default();
        unsafe { text_size(c_str.as_ptr(), style) }
    }
}

/// A settings flag that lets you describe the behavior of how StereoKit will refresh data about the world mesh, if
/// applicable. This is used with World.RefreshType.
/// <https://stereokit.net/Pages/StereoKit/WorldRefresh.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum WorldRefresh {
    /// Refreshing occurs when the user leaves the area that was most recently scanned. This area is a sphere that is
    /// 0.5 of the World::refresh_radius.
    Area = 0,
    /// Refreshing happens at timer intervals. If an update doesn’t happen in time, the next update will happen as soon
    /// as possible. The timer interval is configurable via World::refresh_nterval.
    Timer = 1,
}

/// For use with World::from_spatial_node, this indicates the type of
/// node that's being bridged with OpenXR.
/// <https://stereokit.net/Pages/StereoKit/SpatialNodeType.html>
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u32)]
pub enum SpatialNodeType {
    /// Static spatial nodes track the pose of a fixed location in
    /// the world relative to reference spaces. The tracking of static
    /// nodes may slowly adjust the pose over time for better accuracy but
    /// the pose is relatively stable in the short term, such as between
    /// rendering frames. For example, a QR code tracking library can use a
    /// static node to represent the location of the tracked QR code.
    Static = 0,
    /// Dynamic spatial nodes track the pose of a physical object
    /// that moves continuously relative to reference spaces. The pose of
    /// dynamic spatial nodes can be very different within the duration of
    /// a rendering frame. It is important for the application to use the
    /// correct timestamp to query the space location. For example, a color
    /// camera mounted in front of a HMD is also tracked by the HMD so a
    /// web camera library can use a dynamic node to represent the camera
    /// location.
    Dynamic = 1,
}

/// World contains information about the real world around the user. This includes things like play boundaries, scene
/// understanding, and other various things.
/// <https://stereokit.net/Pages/StereoKit/World.html>
pub struct World;

extern "C" {
    pub fn world_has_bounds() -> Bool32T;
    pub fn world_get_bounds_size() -> Vec2;
    pub fn world_get_bounds_pose() -> Pose;
    pub fn world_from_spatial_graph(spatial_graph_node_id: *mut u8, dynamic: SpatialNodeType, qpc_time: i64) -> Pose;
    pub fn world_from_perception_anchor(perception_spatial_anchor: *mut c_void) -> Pose;
    pub fn world_try_from_spatial_graph(
        spatial_graph_node_id: *mut u8,
        dynamic: SpatialNodeType,
        qpc_time: i64,
        out_pose: *mut Pose,
    ) -> Bool32T;
    pub fn world_try_from_perception_anchor(perception_spatial_anchor: *mut c_void, out_pose: *mut Pose) -> Bool32T;
    pub fn world_raycast(ray: Ray, out_intersection: *mut Ray) -> Bool32T;
    pub fn world_set_occlusion_enabled(enabled: Bool32T);
    pub fn world_get_occlusion_enabled() -> Bool32T;
    pub fn world_set_raycast_enabled(enabled: Bool32T);
    pub fn world_get_raycast_enabled() -> Bool32T;
    pub fn world_set_occlusion_material(material: MaterialT);
    pub fn world_get_occlusion_material() -> MaterialT;
    pub fn world_set_refresh_type(refresh_type: WorldRefresh);
    pub fn world_get_refresh_type() -> WorldRefresh;
    pub fn world_set_refresh_radius(radius_meters: f32);
    pub fn world_get_refresh_radius() -> f32;
    pub fn world_set_refresh_interval(every_seconds: f32);
    pub fn world_get_refresh_interval() -> f32;
    pub fn world_get_tracked() -> BtnState;
    pub fn world_get_origin_mode() -> OriginMode;
    pub fn world_get_origin_offset() -> Pose;
    pub fn world_set_origin_offset(offset: Pose);
}

impl World {
    /// Off by default. This tells StereoKit to load up and display an occlusion surface that allows the real world to
    /// occlude the application’s digital content! Most systems may allow you to customize the visual appearance of this
    /// occlusion surface via the World::occlusion_material. Check SK::System::world_occlusion_present to see if
    /// occlusion can be enabled. This will reset itself to false if occlusion isn’t possible. Loading occlusion data
    /// is asynchronous, so occlusion may not occur immediately after setting this flag.
    /// <https://stereokit.net/Pages/StereoKit/World/OcclusionEnabled.html>
    ///
    /// see also [crate::system::world_set_occlusion_enabled]
    pub fn occlusion_enabled(enabled: bool) {
        unsafe { world_set_occlusion_enabled(enabled as Bool32T) }
    }

    /// By default, this is a black(0,0,0,0) opaque unlit material that will occlude geometry, but won’t show up as
    /// visible anywhere. You can override this with whatever material you would like.
    /// <https://stereokit.net/Pages/StereoKit/World/OcclusionMaterial.html>
    ///
    /// see also [crate::system::world_set_occlusion_material]
    pub fn occlusion_material(material: impl AsRef<Material>) {
        unsafe { world_set_occlusion_material(material.as_ref().0.as_ptr()) }
    }

    /// This is relative to the base reference point and is NOT in world space! The origin StereoKit uses is actually a
    /// base reference point combined with an offset! You can use this to read or set the offset from the OriginMode
    /// reference point.
    /// <https://stereokit.net/Pages/StereoKit/World/OriginOffset.html>
    ///
    /// see also [crate::system::world_set_origin_offset]
    pub fn origin_offset(offset: impl Into<Pose>) {
        unsafe { world_set_origin_offset(offset.into()) }
    }

    /// Off by default. This tells StereoKit to load up collision meshes for the environment, for use with
    /// World::raycast. Check SK::System::world_raycast_present to see if raycasting can be enabled. This will reset
    /// itself to false if raycasting isn’t possible. Loading raycasting data is asynchronous, so collision surfaces may
    /// not be available immediately after setting this flag.
    /// <https://stereokit.net/Pages/StereoKit/World/RaycastEnabled.html>
    ///
    /// see also [crate::system::world_set_raycast_enabled]
    pub fn raycast_enabled(enabled: bool) {
        unsafe { world_set_raycast_enabled(enabled as Bool32T) }
    }

    /// The refresh interval speed, in seconds. This is only applicable when using WorldRefresh::Timer for the refresh
    /// type. Note that the system may not be able to refresh as fast as you wish, and in that case, StereoKit will
    /// always refresh as soon as the previous refresh finishes.
    /// <https://stereokit.net/Pages/StereoKit/World/RefreshInterval.html>
    ///
    /// see also [crate::system::world_set_refresh_interval]
    pub fn refresh_interval(speed: f32) {
        unsafe { world_set_refresh_interval(speed) }
    }

    /// Radius, in meters, of the area that StereoKit should scan for world data. Default is 4. When using the
    /// WorldRefresh::Area refresh type, the world data will refresh when the user has traveled half this radius from
    /// the center of where the most recent refresh occurred.
    /// <https://stereokit.net/Pages/StereoKit/World/RefreshRadius.html>
    ///
    /// see also [crate::system::world_set_refresh_radius]
    pub fn refresh_radius(distance: f32) {
        unsafe { world_set_refresh_radius(distance) }
    }

    /// What information should StereoKit use to determine when the next world data refresh happens? See the
    /// WorldRefresh enum for details.
    /// <https://stereokit.net/Pages/StereoKit/World/RefreshType.html>
    ///
    /// see also [crate::system::world_set_refresh_type]
    pub fn refresh_type(refresh_type: WorldRefresh) {
        unsafe { world_set_refresh_type(refresh_type) }
    }

    /// Converts a Windows.Perception.Spatial.SpatialAnchor’s pose into SteroKit’s coordinate system. This can be great
    /// for interacting with some of the UWP spatial APIs such as WorldAnchors.
    ///
    /// This method only works on UWP platforms, check Sk.System.perception_bridge_present to see if this is available.
    /// <https://stereokit.net/Pages/StereoKit/World/FromPerceptionAnchor.html>
    ///
    /// see also [crate::system::world_from_perception_anchor]
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn from_perception_anchor(perception_spatial_anchor: *mut c_void) -> Option<Pose> {
        let mut pose = Pose::IDENTITY;
        if unsafe { world_try_from_perception_anchor(perception_spatial_anchor, &mut pose) != 0 } {
            Some(pose)
        } else {
            None
        }
    }
    /// TODO : Ask for the non try version

    /// Converts a Windows Mirage spatial node GUID into a Pose based on its current position and rotation! Check
    /// Sk::System::spatial_bridge_present to see if this is available to use. Currently only on HoloLens, good for use
    /// with the Windows QR code package.
    /// <https://stereokit.net/Pages/StereoKit/World/FromSpatialNode.html>
    ///
    /// see also [crate::system::world_f]
    pub fn from_spatial_node(
        spatial_graph_node_id: impl AsRef<str>,
        spatial_node_type: SpatialNodeType,
        qpc_time: i64,
    ) -> Option<Pose> {
        let c_str = CString::new(spatial_graph_node_id.as_ref()).unwrap();
        let mut pose = Pose::IDENTITY;
        if unsafe {
            world_try_from_spatial_graph(c_str.as_ptr() as *mut u8, spatial_node_type, qpc_time, &mut pose) != 0
        } {
            Some(pose)
        } else {
            None
        }
    }

    /// World::raycast_enabled must be set to true first! Sk::System::world_raycast_present must also be true.
    /// This does a ray intersection with whatever represents the environment at the moment! In this case, it’s a
    /// watertight collection of low resolution meshes calculated by the Scene Understanding extension, which is only
    /// provided by the Microsoft HoloLens runtime.
    /// <https://stereokit.net/Pages/StereoKit/World/Raycast.html>
    ///
    /// see also [crate::system::world_raycast]
    pub fn raycast(ray: impl Into<Ray>) -> Option<Ray> {
        let mut intersection = Ray::default();
        if unsafe { world_raycast(ray.into(), &mut intersection) != 0 } {
            Some(intersection)
        } else {
            None
        }
    }

    /// This is the orientation and center point of the system’s boundary/guardian. This can be useful to find the floor
    /// height! Not all systems have a boundary, so be sure to check World::has_bounds() first.
    /// <https://stereokit.net/Pages/StereoKit/World/BoundsPose.html>
    ///
    /// see also [crate::system::world_get_bounds_pose]
    pub fn get_bounds_pose() -> Pose {
        unsafe { world_get_bounds_pose() }
    }

    /// This is the size of a rectangle within the play boundary/guardian’s space, in meters if one exists. Check
    /// World.BoundsPose for the center point and orientation of the boundary, and check World.HasBounds to see if it
    /// exists at all!
    /// <https://stereokit.net/Pages/StereoKit/World/BoundsSize.html>
    ///
    /// see also [crate::system::world_get_bounds_size]
    pub fn get_bounds_size() -> Vec2 {
        unsafe { world_get_bounds_size() }
    }

    /// This refers to the play boundary, or guardian system that the system may have! Not all systems have this, so
    /// it’s always a good idea to check this first!
    /// <https://stereokit.net/Pages/StereoKit/World/HasBounds.html>
    ///
    /// see also [crate::system::world_has_bounds]
    pub fn has_bounds() -> bool {
        unsafe { world_has_bounds() != 0 }
    }

    /// Off by default. This tells StereoKit to load up and display an occlusion surface that allows the real world to
    /// occlude the application’s digital content! Most systems may allow you to customize the visual appearance of this
    /// occlusion surface via the World::occlusion_material. Check SK::System::world_occlusion_present to see if
    /// occlusion can be enabled. This will reset itself to false if occlusion isn’t possible. Loading occlusion data
    /// is asynchronous, so occlusion may not occur immediately after setting this flag.
    /// <https://stereokit.net/Pages/StereoKit/World/OcclusionEnabled.html>
    ///
    /// see also [crate::system::world_get_occlusion_enabled]
    pub fn get_occlusion_enabled() -> bool {
        unsafe { world_get_occlusion_enabled() != 0 }
    }

    /// By default, this is a black(0,0,0,0) opaque unlit material that will occlude geometry, but won’t show up as
    /// visible anywhere. You can override this with whatever material you would like.
    /// <https://stereokit.net/Pages/StereoKit/World/OcclusionMaterial.html>
    ///
    /// see also [crate::system::world_get_occlusion_material]
    pub fn get_occlusion_material() -> Material {
        Material(NonNull::new(unsafe { world_get_occlusion_material() }).unwrap())
    }

    /// The mode or “reference space” that StereoKit uses for determining its base origin. This is determined by the
    /// initial value provided in SkSettings.origin, as well as by support from the underlying runtime. The mode
    /// reported here will not necessarily be the one requested in initialization, as fallbacks are implemented using
    /// different available modes.
    /// <https://stereokit.net/Pages/StereoKit/World/OriginMode.html>
    ///
    /// see also [crate::system::world_get_origin_mode]
    pub fn get_origin_mode() -> OriginMode {
        unsafe { world_get_origin_mode() }
    }

    /// This reports the status of the device's positional tracking. If the room is too dark, or a hand is covering
    /// tracking sensors, or some other similar 6dof tracking failure, this would report as not tracked.
    ///
    /// Note that this does not factor in the status of rotational tracking. Rotation is typically done via
    /// gyroscopes/accelerometers, which don't really fail the same way positional tracking system can.
    /// <https://stereokit.net/Pages/StereoKit/World/Tracked.html>
    ///
    /// see also [crate::system::world_get_tracked]
    pub fn get_tracked() -> BtnState {
        unsafe { world_get_tracked() }
    }

    /// This is relative to the base reference point and is NOT in world space! The origin StereoKit uses is actually a
    /// base reference point combined with an offset! You can use this to read or set the offset from the OriginMode
    /// reference point.
    /// <https://stereokit.net/Pages/StereoKit/World/OriginOffset.html>
    ///
    /// see also [crate::system::world_get_origin_offset]
    pub fn get_origin_offset() -> Pose {
        unsafe { world_get_origin_offset() }
    }

    /// Off by default. This tells StereoKit to load up collision meshes for the environment, for use with
    /// World::raycast. Check SK::System::world_raycast_present to see if raycasting can be enabled. This will reset
    /// itself to false if raycasting isn’t possible. Loading raycasting data is asynchronous, so collision surfaces may
    /// not be available immediately after setting this flag.
    /// <https://stereokit.net/Pages/StereoKit/World/RaycastEnabled.html>
    ///
    /// see also [crate::system::world_get_raycast_enabled]
    pub fn get_raycast_enabled() -> bool {
        unsafe { world_get_raycast_enabled() != 0 }
    }

    /// The refresh interval speed, in seconds. This is only applicable when using WorldRefresh::Timer for the refresh
    /// type. Note that the system may not be able to refresh as fast as you wish, and in that case, StereoKit will
    /// always refresh as soon as the previous refresh finishes.
    /// <https://stereokit.net/Pages/StereoKit/World/RefreshInterval.html>
    ///
    /// see also [crate::system::world_get_refresh_interval]
    pub fn get_refresh_interval() -> f32 {
        unsafe { world_get_refresh_interval() }
    }

    /// Radius, in meters, of the area that StereoKit should scan for world data. Default is 4. When using the
    /// WorldRefresh::Area refresh type, the world data will refresh when the user has traveled half this radius from
    /// the center of where the most recent refresh occurred.
    /// <https://stereokit.net/Pages/StereoKit/World/RefreshRadius.html>
    ///
    /// see also [crate::system::world_get_refresh_radius]
    pub fn get_refresh_radius() -> f32 {
        unsafe { world_get_refresh_radius() }
    }

    /// What information should StereoKit use to determine when the next world data refresh happens? See the
    /// WorldRefresh enum for details.
    /// <https://stereokit.net/Pages/StereoKit/World/RefreshType.html>
    ///
    /// see also [crate::system::world_get_refresh_type]
    pub fn get_refresh_type() -> WorldRefresh {
        unsafe { world_get_refresh_type() }
    }
}
