use crate::{
    StereoKitError,
    material::{Material, MaterialT},
    maths::{Bool32T, Bounds, Pose, Vec2, Vec3},
    mesh::{Mesh, MeshT, Vertex},
    model::{Model, ModelT},
    sound::{Sound, SoundT},
    sprite::{Sprite, SpriteT},
    system::{Align, BtnState, Handed, HierarchyParent, Log, TextContext, TextFit, TextStyle},
    util::{Color32, Color128},
};
use std::{
    ffi::{CStr, CString, c_char, c_ushort},
    ptr::{NonNull, null_mut},
};

/// A description of what type of window to draw! This is a bit flag, so it can contain multiple elements.
/// <https://stereokit.net/Pages/StereoKit/UIWin.html>
///
/// see [`Ui::window_begin`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiWin {
    /// No body, no head. Not really a flag, just set to this value. The Window will still be grab/movable. To prevent
    /// it from being grabbable, combine with the UIMove.None option, or switch to Ui::(push/pop)_surface.
    Empty = 1,
    /// Flag to include a head on the window.
    Head = 2,
    /// Flag to include a body on the window.
    Body = 4,
    /// A normal window has a head and a body to it. Both can be grabbed.
    Normal = 6,
}

/// This describes how a UI element moves when being dragged around by a user!
/// <https://stereokit.net/Pages/StereoKit/UIMove.html>
///
/// see [`Ui::window_begin`] [`Ui::handle_begin`] [`Ui::handle`] [`Ui::system_move_type`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiMove {
    /// The element follows the position and orientation of the user’s hand exactly.
    Exact = 0,
    /// The element follows the position of the user’s hand, but orients to face the user’s head instead of just using
    /// the hand’s rotation.
    FaceUser = 1,
    /// This element follows the hand’s position only, completely discarding any rotation information.
    PosOnly = 2,
    /// Do not allow user input to change the element’s pose at all! You may also be interested in Ui::(push/pop)_surface.
    None = 3,
}

/// This describes how a layout should be cut up! Used with Ui::layout_push_cut.
/// <https://stereokit.net/Pages/StereoKit/UICut.html>
///
/// see [`Ui::layout_push_cut`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiCut {
    /// This cuts a chunk from the left side of the current layout. This will work for layouts that are auto-sizing, and
    /// fixed sized.
    Left = 0,
    /// This cuts a chunk from the right side of the current layout. This will work for layouts that are fixed sized,
    /// but not layouts that auto-size on the X axis!
    Right = 1,
    /// This cuts a chunk from the top side of the current layout. This will work for layouts that are auto-sizing, and
    /// fixed sized.
    Top = 2,
    /// This cuts a chunk from the bottom side of the current layout. This will work for layouts that are fixed sized,
    /// but not layouts that auto-size on the Y axis!
    Bottom = 3,
}

/// Theme color categories to pair with Ui::set_theme_color.
/// The total lenght is `[u32,u32]` where the fist u32 is the enum and the second is the ExtraSlot value
/// native C function should convert this to UiColorT
/// <https://stereokit.net/Pages/StereoKit/UIColor.html>
///
/// see [`Ui::set_theme_color`] [`Ui::set_element_color`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiColor {
    /// he default category, used to indicate that no category has been selected.
    None = 0,
    /// This is the main accent color used by window headers, separators, etc.
    Primary = 1,
    /// This is a background sort of color that should generally be dark. Used by window bodies and backgrounds of
    /// certain elements.
    Background = 2,
    /// A normal UI element color, for elements like buttons and sliders.
    Common = 3,
    /// Not really used anywhere at the moment, maybe for the Ui::panel.
    Complement = 4,
    /// Text color! This should generally be really bright, and at the very least contrast-ey.
    Text = 5,
    /// A maximum enum value to allow for iterating through enum values.
    Max = 6,
    /// All the extra color slots
    ExtraSlot01,
    ExtraSlot02,
    ExtraSlot03,
    ExtraSlot04,
    ExtraSlot05,
    ExtraSlot06,
    ExtraSlot07,
    ExtraSlot08,
    ExtraSlot09,
    ExtraSlot10,
    ExtraSlot11,
    ExtraSlot12,
    ExtraSlot13,
    ExtraSlot14,
    ExtraSlot15,
    ExtraSlot16,
}

/// Indicates the state of a UI theme color.
/// <https://stereokit.net/Pages/StereoKit/UIColorState.html>
///
/// see [`Ui::set_theme_color`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiColorState {
    /// The UI element is in its normal resting state.
    Normal = 0,
    /// The UI element has been activated fully by some type of interaction.
    Active = 1,
    /// The UI element is currently disabled, and cannot be used.
    Disabled = 2,
}

/// Used with StereoKit’s UI, and determines the interaction confirmation behavior for certain elements, such as the
/// Ui::hslider!
/// <https://stereokit.net/Pages/StereoKit/UIConfirm.html>
///
/// see [`Ui::hslider`]  [`Ui::vslider`]  [`Ui::slider_behavior`] [`Ui::volume_at`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiConfirm {
    /// The user must push a button with their finger to confirm interaction with this element. This is simpler to
    /// activate as it requires no learned gestures, but may result in more false positives.
    Push = 0,
    /// The user must use a pinch gesture to interact with this element. This is much harder to activate by accident,
    /// but does require the user to make a precise pinch gesture. You can pretty much be sure that’s what the user
    /// meant to do!
    Pinch = 1,
    /// HSlider specific. Same as Pinch, but pulling out from the slider creates a scaled slider that lets you adjust
    /// the slider at a more granular resolution.
    VariablePinch = 2,
}

/// Describes the layout of a button with image/text contents! You can think of the naming here as being the location of
/// the image, with the text filling the remaining space.
/// <https://stereokit.net/Pages/StereoKit/UIBtnLayout.html>
///
/// see [`Ui::button_img`]  [`Ui::radio_img`]  [`Ui::toggle_img`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiBtnLayout {
    /// Hide the image, and only show text.
    None = 0,
    /// Image to the left, text to the right. Image will take up no more than half the width.
    Left = 1,
    /// Image to the right, text to the left. Image will take up no more than half the width.
    Right = 2,
    /// Image will be centered in the button, and fill up the button as though it was the only element. Text will cram
    /// itself under the padding below the image.
    Center = 3,
    /// Same as Center, but omitting the text.
    CenterNoText = 4,
}

/// Determines when this UI function returns true.
/// <https://stereokit.net/Pages/StereoKit/UINotify.html>
///
/// see [`Ui::vslider`]  [`Ui::hslider`]  
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiNotify {
    /// This function returns true any time the values has changed!
    Change = 0,
    /// This function returns true when the user has finished interacting with it. This does not guarantee the value has
    /// changed.
    Finalize = 1,
}

/// This is a bit flag that describes different types and combinations of gestures used within the UI system.
/// <https://stereokit.net/Pages/StereoKit/UIGesture.html>
///
/// see [`Ui::handle`]  [`Ui::handle_begin`]  
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiGesture {
    /// Default zero state, no gesture at all.
    None = 0,
    /// A pinching action, calculated by taking the distance between the tip of the thumb and the index finger.
    Pinch = 1,
    /// A gripping or grasping motion meant to represent a full hand grab. This is calculated using the distance between
    /// the root and the tip of the ring finger.
    Grip = 2,
    /// This is a bit flag combination of both Pinch and Grip.
    PinchGrip = 3,
}

/// <https://stereokit.net/Pages/StereoKit/UIPad.html>
///
/// see [`Ui::panel_begin`]  [`Ui::panel_at`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiPad {
    None = 0,
    Inside = 1,
    Outside = 2,
}

/// Used with StereoKit’s UI to indicate a particular type of UI element visual.
/// <https://stereokit.net/Pages/StereoKit/UIVisual.html>
///
/// see [`Ui::set_element_visual`]  [`Ui::set_element_color`] [`Ui::draw_element`] [`Ui::set_element_sound`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiVisual {
    /// Default state, no UI element at all.
    None = 0,
    /// A default root UI element. Not a particular element, but other elements may refer to this if there is nothing
    /// more specific present.
    Default = 1,
    /// Refers to Ui::button elements.
    Button = 2,
    /// Refers to Ui::toggle elements.
    Toggle = 3,
    /// Refers to Ui::input elements.
    Input = 4,
    /// Refers to Ui::handle/handle_begin elements.
    Handle = 5,
    /// Refers to UI::window/window_begin body panel element, this element is used when a Window head is also present.
    WindowBody = 6,
    /// Refers to Ui::window/window_begin body element, this element is used when a Window only has the body panel,
    /// without a head.
    WindowBodyOnly = 7,
    /// Refers to Ui::window/window_begin head panel element, this element is used when a Window body is also present.
    WindowHead = 8,
    /// Refers to Ui::window/window_begin head element, this element is used when a Window only has the head panel,
    /// without a body.
    WindowHeadOnly = 9,
    /// Refers to Ui::hseparator element.
    Separator = 10,
    /// Refers to the back line component of the Ui::hslider element for full lines.
    SliderLine = 11,
    /// Refers to the back line component of the Ui::hslider element for the active or “full” half of the line.
    SliderLineActive = 12,
    /// Refers to the back line component of the Ui::hslider element for the inactive or “empty” half of the line.
    SliderLineInactive = 13,
    /// Refers to the push button component of the Ui::hslider element when using UiConfirm::Push.
    SliderPush = 14,
    /// Refers to the pinch button component of the Ui::hslider element when using UiConfirm::Pinch.
    SliderPinch = 15,
    /// Refers to Ui::button_round elements.
    ButtonRound = 16,
    /// Refers to Ui::panel_(begin/end) elements.
    Panel = 17,
    /// Refers to the text position indicator carat on text input elements.
    Carat = 18,
    /// An aura ...
    Aura = 19,
    /// A maximum enum value to allow for iterating through enum values.
    Max = 20,
    /// All the extra color slots
    ExtraSlot01,
    ExtraSlot02,
    ExtraSlot03,
    ExtraSlot04,
    ExtraSlot05,
    ExtraSlot06,
    ExtraSlot07,
    ExtraSlot08,
    ExtraSlot09,
    ExtraSlot10,
    ExtraSlot11,
    ExtraSlot12,
    ExtraSlot13,
    ExtraSlot14,
    ExtraSlot15,
    ExtraSlot16,
}

/// For UI elements that can be oriented horizontally or vertically, this specifies that orientation.
/// <https://stereokit.net/Pages/StereoKit/UIDir.html>
///
/// see [`Ui::progress_bar_at`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiDir {
    /// The element should be layed out along the horizontal axis.
    Horizontal,
    /// The element should be layed out along the vertical axis.
    Vertical,
}

bitflags::bitflags! {
/// For elements that contain corners, this bit flag allows you to specify which corners.
/// <https://stereokit.net/Pages/StereoKit/UICorner.html>
///
/// see [`Ui::gen_quadrant_mesh`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct UiCorner : u32
{
    /// No corners at all.
    const None        = 0;
    /// The top right corner.
    const TopRight    = 1 << 1;
    /// The top left corner.
    const TopLeft     = 1 << 0;
    /// The bottom left corner.
    const BottomLeft  = 1 << 3;
    /// The bottom right corner.
    const BottomRight = 1 << 2;
    /// All corners.
    const All    = Self::TopLeft.bits()    | Self::TopRight.bits() | Self::BottomLeft.bits() | Self::BottomRight.bits();
    /// The top left and top right corners.
    const Top    = Self::TopLeft.bits()    | Self::TopRight.bits();
    /// The bottom left and bottom right corners.
    const Bottom = Self::BottomLeft.bits() | Self::BottomRight.bits();
    /// The top left and bottom left corners.
    const Left   = Self::TopLeft.bits()    | Self::BottomLeft.bits();
    /// The top right and bottom right corners.
    const Right  = Self::TopRight.bits()   | Self::BottomRight.bits();
}
}

bitflags::bitflags! {
/// This describes how UI elements with scrollable regions scroll
/// around or use scroll bars! This allows you to enable or disable
/// vertical and horizontal scrolling.
/// <https://stereokit.net/Pages/StereoKit/UIScroll.html>
///
/// see [`Ui::text`] [`Ui::text_at`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct UiScroll : u32
{
    /// No scroll bars or scrolling.
    const None       = 0;
    /// This will enable vertical scroll bars or scrolling.
    const Vertical   = 1 << 0;
    /// This will enable horizontal scroll bars or scrolling.
    const Horizontal = 1 << 1;
    /// This will enable both vertical and horizontal scroll bars
    /// or scrolling.
    const Both = Self::Vertical.bits() | Self::Horizontal.bits();
}
}

/// A point on a lathe for a mesh generation algorithm. This is the 'silhouette' of the mesh, or the shape the mesh
/// would take if you spun this line of points in a cylinder.
/// <https://stereokit.net/Pages/StereoKit/UILathePt.html>
///
/// see [`Ui::gen_quadrant_mesh`]
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct UiLathePt {
    /// Lathe point 'location', where 'x' is a percentage of the lathe radius alnong the current surface normal, and Y
    /// is the absolute Z axis value.
    pub pt: Vec2,
    /// The lathe normal point, which will be rotated along the surface of the mesh.
    pub normal: Vec2,
    /// Vertex color of the current lathe vertex.
    pub color: Color32,
    /// Will there be triangles connecting this lathe point to the next in the list, or is this a jump without
    /// triangles?
    pub connect_next: Bool32T,
    /// Should the triangles attaching this point to the next be ordered backwards?
    pub flip_face: Bool32T,
}

impl UiLathePt {
    pub fn new(
        pt: impl Into<Vec2>,
        normal: impl Into<Vec2>,
        color: Color32,
        connect_next: bool,
        flip_face: bool,
    ) -> Self {
        Self {
            pt: pt.into(),
            normal: normal.into(),
            color,
            connect_next: connect_next.into(),
            flip_face: flip_face.into(),
        }
    }
    /// This is a default lathe for a button!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    ///
    /// see also [`ui_gen_quadrant_mesh`]
    pub fn button() -> [UiLathePt; 6] {
        let white = Color32::WHITE;
        let black = Color32::BLACK;
        let shadow_center = Color32::rgba(0, 0, 0, 200);
        [
            UiLathePt::new([0.00, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([0.95, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([1.0, -0.45], [1.0, 0.0], white, true, false),
            UiLathePt::new([1.00, -0.1], [1.0, 0.0], white, false, false),
            UiLathePt::new([1.20, 0.49], [0.0, 1.0], black, true, true),
            UiLathePt::new([0.00, 0.49], [0.0, 1.0], shadow_center, true, true),
        ]
    }
    /// This is a default lathe for an input!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    ///
    /// see also [`ui_gen_quadrant_mesh`]
    pub fn input() -> [UiLathePt; 10] {
        let gray = Color32::rgba(200, 200, 200, 255);
        let white = Color32::WHITE;
        let black = Color32::BLACK;
        let shadow_center = Color32::rgba(0, 0, 0, 200);
        [
            UiLathePt::new([0.00, -0.1], [0.0, 1.0], gray, true, false),
            UiLathePt::new([0.80, -0.1], [0.0, 1.0], gray, false, false),
            UiLathePt::new([0.80, -0.1], [-1.0, 0.0], gray, true, false),
            UiLathePt::new([0.80, -0.5], [-1.0, 0.0], white, false, false),
            UiLathePt::new([0.80, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([0.95, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([1.0, -0.45], [1.0, 0.0], white, true, false),
            UiLathePt::new([1.00, -0.1], [1.0, 0.0], white, false, false),
            UiLathePt::new([1.20, 0.49], [0.0, 1.0], black, true, true),
            UiLathePt::new([0.00, 0.49], [0.0, 1.0], shadow_center, true, true),
        ]
    }

    /// This is a default lathe for a plane!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    ///
    /// see also [`ui_gen_quadrant_mesh`]
    pub fn plane() -> [UiLathePt; 2] {
        let white = Color32::WHITE;
        [
            UiLathePt::new([0.0, 0.0], [0.0, 1.0], white, true, false),
            UiLathePt::new([1.0, 0.0], [0.0, 1.0], white, false, false),
        ]
    }

    /// This is a default lathe for a panel!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    ///
    /// see also [`ui_gen_quadrant_mesh`]
    pub fn panel() -> [UiLathePt; 6] {
        let white = Color32::WHITE;
        [
            UiLathePt::new([0.0, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([1.0, -0.5], [0.0, 1.0], white, false, false),
            UiLathePt::new([1.0, -0.5], [1.0, 0.0], white, true, false),
            UiLathePt::new([1.0, 0.5], [1.0, 0.0], white, false, false),
            UiLathePt::new([1.0, 0.5], [0.0, -1.0], white, true, false),
            UiLathePt::new([0.0, 0.5], [0.0, -1.0], white, true, false),
        ]
    }
    /// This is a default lathe for a slider!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    ///
    /// see also [`ui_gen_quadrant_mesh`]
    pub fn slider() -> [UiLathePt; 6] {
        let white = Color32::WHITE;
        let black = Color32::BLACK;
        let shadow_edge = Color32::rgba(0, 0, 0, 100);
        [
            UiLathePt::new([0.0, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([1.0, -0.5], [0.0, 1.0], white, false, false),
            UiLathePt::new([1.0, -0.5], [1.0, 0.0], white, true, false),
            UiLathePt::new([1.0, 0.5], [1.0, 0.0], white, false, false),
            UiLathePt::new([1.0, 0.49], [0.0, 1.0], shadow_edge, true, false),
            UiLathePt::new([2.0, 0.49], [0.0, 1.0], black, false, false),
        ]
    }

    /// This is a default lathe for a slider button!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    ///
    /// see also [`ui_gen_quadrant_mesh`]
    pub fn slider_btn() -> [UiLathePt; 6] {
        let white = Color32::WHITE;
        let black = Color32::BLACK;
        let shadow_edge = Color32::rgba(0, 0, 0, 100);
        [
            UiLathePt::new([0.0, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([0.8, -0.5], [0.0, 1.0], white, true, false),
            UiLathePt::new([1.0, -0.4], [1.0, 0.0], white, true, false),
            UiLathePt::new([1.0, 0.5], [1.0, 0.0], white, false, false),
            UiLathePt::new([1.0, 0.49], [0.0, 1.0], shadow_edge, true, false),
            UiLathePt::new([2.0, 0.49], [0.0, 1.0], black, false, false),
        ]
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
/// Visual properties and spacing of the UI system.
/// <https://stereokit.net/Pages/StereoKit/UISettings.html>
///
/// see [`Ui::settings`]
pub struct UiSettings {
    /// The margin is the space between a window and its contents. In meters.
    pub margin: f32,
    /// Spacing between an item and its parent, in meters.
    pub padding: f32,
    /// Spacing between sibling items, in meters.
    pub gutter: f32,
    /// The Z depth of 3D UI elements, in meters.
    pub depth: f32,
    /// Radius of the UI element corners, in meters.
    pub rounding: f32,
    /// How far up does the white back-border go on UI elements? This is a 0-1 percentage of the depth value.
    pub backplate_depth: f32,
    // How wide is the back-border around the UI elements? In meters.
    pub backplate_border: f32,
}

pub type IdHashT = u64;

#[derive(Default, Debug, Copy, Clone)]
#[repr(C)]
/// Visual properties of a slider behavior.
/// <https://stereokit.net/Pages/StereoKit/UISliderData.html>
///
/// see [`Ui::slider_behavior`]
pub struct UiSliderData {
    /// The center location of where the slider's interactionelement is.
    pub button_center: Vec2,
    /// The current distance of the finger, within the pressable volume of the slider, from the bottom of the slider
    pub finger_offset: f32,
    /// This is the current frame's "focus" state for the button.
    pub focus_state: BtnState,
    /// This is the current frame's "active" state for the button.
    pub active_state: BtnState,
    /// The interactor that is currently driving the activity or focus of the slider. Or -1 if there is no interaction.
    pub interactor: i32,
}

/// This class is a collection of user interface and interaction methods! StereoKit uses an Immediate Mode GUI system,
/// which can be very easy to work with and modify during runtime.
///
/// You must call the UI method every frame you wish it to be available, and if you no longer want it to be present, you
/// simply stop calling it! The id of the element is used to track its state from frame to frame, so for elements with
/// state, you’ll want to avoid changing the id during runtime! Ids are also scoped per-window, so different windows can
/// re-use the same id, but a window cannot use the same id twice.
/// <https://stereokit.net/Pages/StereoKit/UI.html>
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{ui::{Ui,UiBtnLayout}, maths::{Vec2, Vec3, Pose}, sprite::Sprite};
///
/// let mut window_pose = Pose::new(
///     [0.01, 0.09, 0.88], Some([0.0, 185.0, 0.0].into()));
///
/// let (mut choice, mut doubt, mut scaling) = ("C", false, 0.5);
/// let (on, off) = (Sprite::radio_on(), Sprite::radio_off());
/// let exit_sprite = Sprite::from_file("textures/exit.jpeg", None, None)
///                               .expect("exit.jpeg should be ok");
///
/// filename_scr = "screenshots/ui.jpeg";
/// test_screenshot!( // !!!! Get a proper main loop !!!!
///     Ui::window_begin("Question", &mut window_pose, None, None, None);
///     Ui::text("Are you a robot ?", None, None, None, Some(0.13), None, None);
///     Ui::hseparator();
///     Ui::label("Respond wisely", Some([0.08, 0.03].into()), false);
///     if Ui::radio_img("yes", choice == "A", &off, &on, UiBtnLayout::Left, None) {
///         choice = "A"; scaling = 0.0;
///     }
///     Ui::same_line();
///     if Ui::radio_img("no", choice == "B", &off, &on, UiBtnLayout::Left, None){
///         choice = "B"; scaling = 1.0;
///     }
///     Ui::same_line();
///     if Ui::radio_img("maybe", choice == "C", &off, &on, UiBtnLayout::Left, None) {
///         choice = "C"; scaling = 0.5;
///     }
///     Ui::panel_begin(None);
///     Ui::toggle("Doubt value:", &mut doubt, None);
///     Ui::push_enabled(doubt, None);
///     Ui::hslider("scaling", &mut scaling, 0.0, 1.0,Some(0.05), Some(0.14), None, None);
///     Ui::pop_enabled();
///     Ui::panel_end();
///     Ui::same_line();
///     if Ui::button_img("Exit", &exit_sprite, Some(UiBtnLayout::CenterNoText),
///                       Some(Vec2::new(0.08, 0.08)), None) {
///        sk.quit(None);
///     }
///     Ui::window_end();
/// );
/// ```
/// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui.jpeg" alt="screenshot" width="200">
pub struct Ui;

unsafe extern "C" {
    pub fn ui_quadrant_size_verts(ref_vertices: *mut Vertex, vertex_count: i32, overflow_percent: f32);
    pub fn ui_quadrant_size_mesh(ref_mesh: MeshT, overflow_percent: f32);
    pub fn ui_gen_quadrant_mesh(
        rounded_corners: UiCorner,
        corner_radius: f32,
        corner_resolution: u32,
        delete_flat_sides: Bool32T,
        quadrantify: Bool32T,
        lathe_pts: *const UiLathePt,
        lathe_pt_count: i32,
    ) -> MeshT;
    pub fn ui_show_volumes(show: Bool32T);
    pub fn ui_enable_far_interact(enable: Bool32T);
    pub fn ui_far_interact_enabled() -> Bool32T;
    pub fn ui_system_get_move_type() -> UiMove;
    pub fn ui_system_set_move_type(move_type: UiMove);
    pub fn ui_settings(settings: UiSettings);
    pub fn ui_get_settings() -> UiSettings;
    pub fn ui_get_margin() -> f32;
    pub fn ui_get_padding() -> f32;
    pub fn ui_get_gutter() -> f32;
    pub fn ui_set_color(color: Color128);
    pub fn ui_set_theme_color(color_type: UiColor, color_gamma: Color128);
    pub fn ui_get_theme_color(color_type: UiColor) -> Color128;
    pub fn ui_set_theme_color_state(color_type: UiColor, state: UiColorState, color_gamma: Color128);
    pub fn ui_get_theme_color_state(color_type: UiColor, state: UiColorState) -> Color128;
    pub fn ui_set_element_visual(element_visual: UiVisual, mesh: MeshT, material: MaterialT, min_size: Vec2);
    pub fn ui_set_element_color(element_visual: UiVisual, color_category: UiColor);
    pub fn ui_set_element_sound(element_visual: UiVisual, activate: SoundT, deactivate: SoundT);
    pub fn ui_has_keyboard_focus() -> Bool32T;
    pub fn ui_popup_pose(shift: Vec3) -> Pose;

    pub fn ui_draw_element(element_visual: UiVisual, start: Vec3, size: Vec3, focus: f32);
    pub fn ui_draw_element_color(
        element_visual: UiVisual,
        element_color: UiVisual,
        start: Vec3,
        size: Vec3,
        focus: f32,
    );
    pub fn ui_get_element_color(element_visual: UiVisual, focus: f32) -> Color128;
    pub fn ui_get_anim_focus(id: IdHashT, focus_state: BtnState, activation_state: BtnState) -> f32;

    pub fn ui_push_grab_aura(enabled: Bool32T);
    pub fn ui_pop_grab_aura();
    pub fn ui_grab_aura_enabled() -> Bool32T;
    pub fn ui_push_text_style(style: TextStyle);
    pub fn ui_pop_text_style();
    pub fn ui_get_text_style() -> TextStyle;
    pub fn ui_is_enabled() -> Bool32T;
    pub fn ui_push_tint(tint_gamma: Color128);
    pub fn ui_pop_tint();
    pub fn ui_push_enabled(enabled: Bool32T, parent_behavior: HierarchyParent);
    pub fn ui_pop_enabled();
    pub fn ui_push_preserve_keyboard(preserve_keyboard: Bool32T);
    pub fn ui_pop_preserve_keyboard();
    pub fn ui_push_surface(surface_pose: Pose, layout_start: Vec3, layout_dimensions: Vec2);
    pub fn ui_pop_surface();
    pub fn ui_push_id(id: *const c_char) -> IdHashT;
    pub fn ui_push_id_16(id: *const c_ushort) -> IdHashT;
    pub fn ui_push_idi(id: i32) -> IdHashT;
    pub fn ui_pop_id();
    pub fn ui_stack_hash(string: *const c_char) -> IdHashT;
    pub fn ui_stack_hash_16(string: *const c_ushort) -> IdHashT;
    pub fn ui_layout_area(start: Vec3, dimensions: Vec2, add_margin: Bool32T);
    pub fn ui_layout_remaining() -> Vec2;
    pub fn ui_layout_at() -> Vec3;
    pub fn ui_layout_last() -> Bounds;
    pub fn ui_layout_reserve(size: Vec2, add_padding: Bool32T, depth: f32) -> Bounds;
    pub fn ui_layout_push(start: Vec3, dimensions: Vec2, add_margin: Bool32T);
    pub fn ui_layout_push_cut(cut_to: UiCut, size: f32, add_margin: Bool32T);
    pub fn ui_layout_pop();
    /// Deprecaded: pub fn ui_last_element_hand_used(hand: Handed) -> BtnState;
    pub fn ui_last_element_hand_active(hand: Handed) -> BtnState;
    pub fn ui_last_element_hand_focused(hand: Handed) -> BtnState;
    pub fn ui_last_element_active() -> BtnState;
    pub fn ui_last_element_focused() -> BtnState;
    // Deprecated: pub fn ui_area_remaining() -> Vec2;
    pub fn ui_nextline();
    pub fn ui_sameline();
    pub fn ui_line_height() -> f32;
    pub fn ui_is_interacting(hand: Handed) -> Bool32T;
    pub fn ui_button_behavior(
        window_relative_pos: Vec3,
        size: Vec2,
        id: IdHashT,
        out_finger_offset: *mut f32,
        out_button_state: *mut BtnState,
        out_focus_state: *mut BtnState,
        out_opt_hand: *mut i32,
    );
    pub fn ui_button_behavior_depth(
        window_relative_pos: Vec3,
        size: Vec2,
        id: IdHashT,
        button_depth: f32,
        button_activation_depth: f32,
        out_finger_offset: *mut f32,
        out_button_state: *mut BtnState,
        out_focus_state: *mut BtnState,
        out_opt_hand: *mut i32,
    );
    pub fn ui_slider_behavior(
        window_relative_pos: Vec3,
        size: Vec2,
        id: IdHashT,
        value: *mut Vec2,
        min: Vec2,
        max: Vec2,
        button_size_visual: Vec2,
        button_size_interact: Vec2,
        confirm_method: UiConfirm,
        data: *mut UiSliderData,
    );
    pub fn ui_volume_at(
        id: *const c_char,
        bounds: Bounds,
        interact_type: UiConfirm,
        out_opt_hand: *mut Handed,
        out_opt_focus_state: *mut BtnState,
    ) -> BtnState;
    pub fn ui_volume_at_16(
        id: *const c_ushort,
        bounds: Bounds,
        interact_type: UiConfirm,
        out_opt_hand: *mut Handed,
        out_opt_focus_state: *mut BtnState,
    ) -> BtnState;
    // Deprecated : pub fn ui_volume_at(id: *const c_char, bounds: Bounds) -> Bool32T;
    // Deprecated : pub fn ui_volume_at_16(id: *const c_ushort, bounds: Bounds) -> Bool32T;
    // Deprecated : pub fn ui_interact_volume_at(bounds: Bounds, out_hand: *mut Handed) -> BtnState;
    pub fn ui_label(text: *const c_char, use_padding: Bool32T);
    pub fn ui_label_16(text: *const c_ushort, use_padding: Bool32T);
    pub fn ui_label_sz(text: *const c_char, size: Vec2, use_padding: Bool32T);
    pub fn ui_label_sz_16(text: *const c_ushort, size: Vec2, use_padding: Bool32T);
    pub fn ui_text(
        text: *const c_char,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        height: f32,
        text_align: Align,
    ) -> Bool32T;
    pub fn ui_text_16(
        text: *const c_ushort,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        height: f32,
        text_align: Align,
    ) -> Bool32T;
    pub fn ui_text_sz(
        text: *const c_char,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        size: Vec2,
        text_align: Align,
        fit: TextFit,
    ) -> Bool32T;
    pub fn ui_text_sz_16(
        text: *const c_ushort,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        size: Vec2,
        text_align: Align,
        fit: TextFit,
    ) -> Bool32T;
    pub fn ui_text_at(
        text: *const c_char,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        text_align: Align,
        fit: TextFit,
        window_relative_pos: Vec3,
        size: Vec2,
    ) -> Bool32T;
    pub fn ui_text_at_16(
        text: *const c_ushort,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        text_align: Align,
        fit: TextFit,
        window_relative_pos: Vec3,
        size: Vec2,
    ) -> Bool32T;
    pub fn ui_button(text: *const c_char) -> Bool32T;
    pub fn ui_button_16(text: *const c_ushort) -> Bool32T;
    pub fn ui_button_sz(text: *const c_char, size: Vec2) -> Bool32T;
    pub fn ui_button_sz_16(text: *const c_ushort, size: Vec2) -> Bool32T;
    pub fn ui_button_at(text: *const c_char, window_relative_pos: Vec3, size: Vec2) -> Bool32T;
    pub fn ui_button_at_16(text: *const c_ushort, window_relative_pos: Vec3, size: Vec2) -> Bool32T;
    pub fn ui_button_img(text: *const c_char, image: SpriteT, image_layout: UiBtnLayout, color: Color128) -> Bool32T;
    pub fn ui_button_img_16(
        text: *const c_ushort,
        image: SpriteT,
        image_layout: UiBtnLayout,
        color: Color128,
    ) -> Bool32T;
    pub fn ui_button_img_sz(
        text: *const c_char,
        image: SpriteT,
        image_layout: UiBtnLayout,
        size: Vec2,
        color: Color128,
    ) -> Bool32T;
    pub fn ui_button_img_sz_16(
        text: *const c_ushort,
        image: SpriteT,
        image_layout: UiBtnLayout,
        size: Vec2,
        color: Color128,
    ) -> Bool32T;
    pub fn ui_button_img_at(
        text: *const c_char,
        image: SpriteT,
        image_layout: UiBtnLayout,
        window_relative_pos: Vec3,
        size: Vec2,
        color: Color128,
    ) -> Bool32T;
    pub fn ui_button_img_at_16(
        text: *const c_ushort,
        image: SpriteT,
        image_layout: UiBtnLayout,
        window_relative_pos: Vec3,
        size: Vec2,
        color: Color128,
    ) -> Bool32T;
    pub fn ui_button_round(id: *const c_char, image: SpriteT, diameter: f32) -> Bool32T;
    pub fn ui_button_round_16(id: *const c_ushort, image: SpriteT, diameter: f32) -> Bool32T;
    pub fn ui_button_round_at(id: *const c_char, image: SpriteT, window_relative_pos: Vec3, diameter: f32) -> Bool32T;
    pub fn ui_button_round_at_16(
        id: *const c_ushort,
        image: SpriteT,
        window_relative_pos: Vec3,
        diameter: f32,
    ) -> Bool32T;
    pub fn ui_toggle(text: *const c_char, pressed: *mut Bool32T) -> Bool32T;
    pub fn ui_toggle_16(text: *const c_ushort, pressed: *mut Bool32T) -> Bool32T;
    pub fn ui_toggle_sz(text: *const c_char, pressed: *mut Bool32T, size: Vec2) -> Bool32T;
    pub fn ui_toggle_sz_16(text: *const c_ushort, pressed: *mut Bool32T, size: Vec2) -> Bool32T;
    pub fn ui_toggle_at(text: *const c_char, pressed: *mut Bool32T, window_relative_pos: Vec3, size: Vec2) -> Bool32T;
    pub fn ui_toggle_at_16(
        text: *const c_ushort,
        pressed: *mut Bool32T,
        window_relative_pos: Vec3,
        size: Vec2,
    ) -> Bool32T;
    pub fn ui_toggle_img(
        text: *const c_char,
        pressed: *mut Bool32T,
        toggle_off: SpriteT,
        toggle_on: SpriteT,
        image_layout: UiBtnLayout,
    ) -> Bool32T;
    pub fn ui_toggle_img_16(
        text: *const c_ushort,
        pressed: *mut Bool32T,
        toggle_off: SpriteT,
        toggle_on: SpriteT,
        image_layout: UiBtnLayout,
    ) -> Bool32T;
    pub fn ui_toggle_img_sz(
        text: *const c_char,
        pressed: *mut Bool32T,
        toggle_off: SpriteT,
        toggle_on: SpriteT,
        image_layout: UiBtnLayout,
        size: Vec2,
    ) -> Bool32T;
    pub fn ui_toggle_img_sz_16(
        text: *const c_ushort,
        pressed: *mut Bool32T,
        toggle_off: SpriteT,
        toggle_on: SpriteT,
        image_layout: UiBtnLayout,
        size: Vec2,
    ) -> Bool32T;
    pub fn ui_toggle_img_at(
        text: *const c_char,
        pressed: *mut Bool32T,
        toggle_off: SpriteT,
        toggle_on: SpriteT,
        image_layout: UiBtnLayout,
        window_relative_pos: Vec3,
        size: Vec2,
    ) -> Bool32T;
    pub fn ui_toggle_img_at_16(
        text: *const c_ushort,
        pressed: *mut Bool32T,
        toggle_off: SpriteT,
        toggle_on: SpriteT,
        image_layout: UiBtnLayout,
        window_relative_pos: Vec3,
        size: Vec2,
    ) -> Bool32T;
    pub fn ui_hslider(
        id: *const c_char,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        width: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_hslider_16(
        id: *const c_ushort,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        width: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_hslider_f64(
        id: *const c_char,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        width: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_hslider_f64_16(
        id: *const c_ushort,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        width: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_hslider_at(
        id: *const c_char,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_hslider_at_16(
        id: *const c_ushort,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_hslider_at_f64(
        id: *const c_char,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_hslider_at_f64_16(
        id: *const c_ushort,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider(
        id: *const c_char,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        height: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider_16(
        id: *const c_ushort,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        height: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider_f64(
        id: *const c_char,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        height: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider_f64_16(
        id: *const c_ushort,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        height: f32,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider_at(
        id: *const c_char,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider_at_16(
        id: *const c_ushort,
        value: *mut f32,
        min: f32,
        max: f32,
        step: f32,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider_at_f64(
        id: *const c_char,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_vslider_at_f64_16(
        id: *const c_ushort,
        value: *mut f64,
        min: f64,
        max: f64,
        step: f64,
        window_relative_pos: Vec3,
        size: Vec2,
        confirm_method: UiConfirm,
        notify_on: UiNotify,
    ) -> Bool32T;
    pub fn ui_input(
        id: *const c_char,
        buffer: *mut c_char,
        buffer_size: i32,
        size: Vec2,
        type_: TextContext,
    ) -> Bool32T;
    pub fn ui_input_at(
        id: *const c_char,
        buffer: *mut c_char,
        buffer_size: i32,
        window_relative_pos: Vec3,
        size: Vec2,
        type_: TextContext,
    ) -> Bool32T;
    pub fn ui_input_16(
        id: *const c_ushort,
        buffer: *mut c_ushort,
        buffer_size: i32,
        size: Vec2,
        type_: TextContext,
    ) -> Bool32T;
    pub fn ui_input_at_16(
        id: *const c_ushort,
        buffer: *mut c_ushort,
        buffer_size: i32,
        window_relative_pos: Vec3,
        size: Vec2,
        type_: TextContext,
    ) -> Bool32T;
    pub fn ui_image(image: SpriteT, size: Vec2);
    pub fn ui_model(model: ModelT, ui_size: Vec2, model_scale: f32);
    pub fn ui_model_at(model: ModelT, start: Vec3, size: Vec3, color: Color128);
    pub fn ui_hprogress_bar(percent: f32, width: f32, flip_fil_dir: Bool32T);
    pub fn ui_vprogress_bar(percent: f32, height: f32, flip_fil_dir: Bool32T);
    pub fn ui_progress_bar_at(
        percent: f32,
        window_relative_pos: Vec3,
        size: Vec2,
        bar_direction: UiDir,
        flip_fil_dir: Bool32T,
    );
    pub fn ui_hseparator();
    // Deprecated : pub fn ui_space(space: f32);
    pub fn ui_hspace(horizontal_space: f32);
    pub fn ui_vspace(vertical_space: f32);
    pub fn ui_handle_begin(
        text: *const c_char,
        movement: *mut Pose,
        handle: Bounds,
        draw: Bool32T,
        move_type: UiMove,
        allowed_gestures: UiGesture,
    ) -> Bool32T;
    pub fn ui_handle_begin_16(
        text: *const c_ushort,
        movement: *mut Pose,
        handle: Bounds,
        draw: Bool32T,
        move_type: UiMove,
        allowed_gestures: UiGesture,
    ) -> Bool32T;
    pub fn ui_handle_end();
    pub fn ui_window_begin(text: *const c_char, pose: *mut Pose, size: Vec2, window_type: UiWin, move_type: UiMove);
    pub fn ui_window_begin_16(
        text: *const c_ushort,
        pose: *mut Pose,
        size: Vec2,
        window_type: UiWin,
        move_type: UiMove,
    );
    pub fn ui_window_end();
    pub fn ui_panel_at(start: Vec3, size: Vec2, padding: UiPad);
    pub fn ui_panel_begin(padding: UiPad);
    pub fn ui_panel_end();
}

impl Ui {
    /// StereoKit will generate a color palette from this gamma space color, and use it to skin the UI! To explicitly
    /// adjust individual theme colors, see Ui::set_theme_color.
    ///  <https://stereokit.net/Pages/StereoKit/UI/ColorScheme.html>
    ///
    /// see also [`ui_set_color`] [`Ui::set_theme_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// Ui::color_scheme(named_colors::GREEN);
    ///
    /// let mut agree = true;
    /// let mut scaling = 0.75;
    /// filename_scr = "screenshots/ui_color_scheme.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Give a value", &mut window_pose, None, None, None);
    ///     Ui::panel_begin(None);
    ///     Ui::hslider("scaling", &mut scaling, 0.0, 1.0, Some(0.05), Some(0.14), None, None);
    ///     Ui::panel_end();
    ///     Ui::toggle("I agree!",&mut agree, None);
    ///     Ui::same_line();
    ///     if Ui::button("Exit", None) {
    ///        sk.quit(None);
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_color_scheme.jpeg" alt="screenshot" width="200">
    pub fn color_scheme(color: impl Into<Color128>) {
        unsafe { ui_set_color(color.into()) };
    }

    /// Enables or disables the far ray grab interaction for Handle elements like the Windows. It can be enabled and
    /// disabled for individual UI elements, and if this remains disabled at the start of the next frame, then the
    /// hand ray indicators will not be visible. This is enabled by default.
    /// <https://stereokit.net/Pages/StereoKit/UI/EnableFarInteract.html>
    ///
    /// see also [`ui_enable_far_interact`] [`Ui::get_enable_far_interact`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::ui::Ui;
    ///
    /// assert_eq!(Ui::get_enable_far_interact(), true);
    ///
    /// Ui::enable_far_interact(false);
    /// assert_eq!(Ui::get_enable_far_interact(), false);
    /// ```
    pub fn enable_far_interact(enable: bool) {
        unsafe { ui_enable_far_interact(enable as Bool32T) };
    }

    /// UI sizing and layout settings.
    /// <https://stereokit.net/Pages/StereoKit/UI/Settings.html>
    ///
    /// see also [`ui_settings`] [`Ui::get_settings`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::ui::{Ui, UiSettings};
    ///
    /// let settings = Ui::get_settings();
    /// assert_eq!(settings.margin, 0.010000001);
    /// assert_eq!(settings.padding, 0.010000001);
    /// assert_eq!(settings.gutter, 0.010000001);
    /// assert_eq!(settings.depth, 0.010000001);
    /// assert_eq!(settings.rounding, 0.0075000003);
    /// assert_eq!(settings.backplate_depth, 0.4);
    /// assert_eq!(settings.backplate_border, 0.0005);
    ///
    /// let new_settings = UiSettings {
    ///     margin: 0.005,
    ///     padding: 0.005,
    ///     gutter: 0.005,
    ///     depth: 0.015,
    ///     rounding: 0.004,
    ///     backplate_depth: 0.6,
    ///     backplate_border: 0.002,
    /// };
    /// Ui::settings(new_settings);
    /// let settings = Ui::get_settings();
    /// assert_eq!(settings.margin, 0.005);
    /// assert_eq!(settings.padding, 0.005);
    /// assert_eq!(settings.gutter, 0.005);
    /// assert_eq!(settings.depth, 0.015);
    /// assert_eq!(settings.rounding, 0.004);
    /// assert_eq!(settings.backplate_depth, 0.6);
    /// assert_eq!(settings.backplate_border, 0.002);
    /// ```
    pub fn settings(settings: UiSettings) {
        unsafe { ui_settings(settings) }
    }

    /// Shows or hides the collision volumes of the UI! This is for debug purposes, and can help identify visible and
    /// invisible collision issues.
    /// <https://stereokit.net/Pages/StereoKit/UI/ui_show_volumes.html>
    ///
    /// see also [`ui_show_volumes`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::ui::Ui;
    ///
    /// Ui::show_volumes(true);
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_show_volumes.jpeg" alt="screenshot" width="200">
    pub fn show_volumes(show: bool) {
        unsafe { ui_show_volumes(show as Bool32T) };
    }

    /// This is the UiMove that is provided to UI windows that StereoKit itself manages, such as the fallback
    /// filepicker and soft keyboard.
    /// <https://stereokit.net/Pages/StereoKit/UI/SystemMoveType.html>
    ///
    /// see also [`ui_system_set_move_type`] [`Ui::get_system_move_type`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::ui::{Ui, UiMove};
    ///
    /// assert_eq!(Ui::get_system_move_type(), UiMove::FaceUser);
    ///
    /// Ui::system_move_type(UiMove::Exact);
    /// assert_eq!(Ui::get_system_move_type(), UiMove::Exact);
    /// ```
    pub fn system_move_type(move_type: UiMove) {
        unsafe { ui_system_set_move_type(move_type) };
    }

    /// A pressable button! A button will expand to fit the text provided to it, vertically and horizontally. Text is
    /// re-used as the id. Will return true only on the first frame it is pressed!
    /// <https://stereokit.net/Pages/StereoKit/UI/Button.html>
    /// * `text` - Text to display on the button and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::get_line_height.
    ///
    /// Returns true if the button was pressed this frame.
    /// see also [`ui_button`] [`ui_button_sz`] [`Ui::button_at`]  [`Ui::button_behavior`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut button = 0;
    /// filename_scr = "screenshots/ui_button.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Press a button", &mut window_pose, None, None, None);
    ///     if Ui::button("1", None) {button = 1}
    ///     Ui::same_line();
    ///     if Ui::button("2", Some([0.025, 0.025].into())) {button = 2}
    ///     if Ui::button("3", Some([0.04, 0.04].into())) {button = 3}
    ///     if Ui::button_at("4", [-0.01, -0.01, 0.005],[0.05, 0.05]) {button = 4}
    ///     if Ui::button_at("5", [-0.04, -0.08, 0.005],[0.03, 0.03]) {button = 5}
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_button.jpeg" alt="screenshot" width="200">
    pub fn button(text: impl AsRef<str>, size: Option<Vec2>) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();
        match size {
            Some(size) => unsafe { ui_button_sz(cstr.as_ptr(), size) != 0 },
            None => unsafe { ui_button(cstr.as_ptr()) != 0 },
        }
    }

    /// A variant of Ui::button that doesn’t use the layout system, and instead goes exactly where you put it.
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonAt.html>
    /// * `text` - Text to display on the button and id for tracking element state. MUST be unique within current
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    ///   hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    ///
    /// Returns true if the button was pressed this frame.
    /// see also [`ui_button_at`] [`Ui::button_behavior`]
    /// see example in [`Ui::button`]
    pub fn button_at(text: impl AsRef<str>, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();

        unsafe { ui_button_at(cstr.as_ptr(), top_left_corner.into(), size.into()) != 0 }
    }

    /// This is the core functionality of StereoKit’s buttons, without any of the rendering parts! If you’re trying to
    /// create your own pressable UI elements, or do more extreme customization of the look and feel of UI elements,
    /// then this function will provide a lot of complex pressing functionality for you!
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonBehavior.html>
    /// * `window_relative_pos` - The layout position of the pressable area.
    /// * `size` - The size of the pressable area.
    /// * `id` - The id for this pressable element to track its state with.
    /// * `out_finger_offset` - This is the current distance of the finger, within the pressable volume, from the
    ///   bottom of the button.
    /// * `out_button_state` - This is the current frame’s “active” state for the button.
    /// * `out_focus_state` - This is the current frame’s “focus” state for the button.
    /// * `out_hand` - Id of the hand that interacted with the button. This will be -1 if no interaction has occurred.
    ///
    /// see also [`ui_button_behavior`] [`Ui::button_behavior_depth`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, system::BtnState};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut out_button_state = BtnState::empty();
    /// let mut out_focus_state = BtnState::empty();
    /// let mut out_finger_offset = 0.0;
    /// filename_scr = "screenshots/ui_button_behavior.jpeg";
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("I'm a button", &mut window_pose, None, None, None);
    ///     Ui::button_behavior([0.0, 0.0, 0.005],[0.05, 0.05], "Button1",
    ///                         &mut out_finger_offset, &mut out_button_state,
    ///                         &mut out_focus_state, None);
    ///     if out_button_state.is_just_inactive() {
    ///        println!("Button1 pressed");
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn button_behavior(
        window_relative_pos: impl Into<Vec3>,
        size: impl Into<Vec2>,
        id: impl AsRef<str>,
        out_finger_offset: &mut f32,
        out_button_state: &mut BtnState,
        out_focus_state: &mut BtnState,
        out_hand: Option<&mut i32>,
    ) {
        let id_hash = Ui::stack_hash(id);
        let mut nevermind = 0;
        let out_opt_hand = out_hand.unwrap_or(&mut nevermind);

        unsafe {
            ui_button_behavior(
                window_relative_pos.into(),
                size.into(),
                id_hash,
                out_finger_offset,
                out_button_state,
                out_focus_state,
                out_opt_hand,
            )
        }
    }

    /// This is the core functionality of StereoKit’s buttons, without any of the rendering parts! If you’re trying to
    /// create your own pressable UI elements, or do more extreme customization of the look and feel of UI elements,
    /// then this function will provide a lot of complex pressing functionality for you! This overload allows for
    /// customizing the depth of the button, which otherwise would use UiSettings.depth for its values.
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonBehavior.html>
    /// * hand - Id of the hand that interacted with the button. This will be -1 if no interaction has occurred.
    ///
    /// see also [`ui_button_behavior_depth`]
    /// * `window_relative_pos` - The layout position of the pressable area.
    /// * `size` - The size of the pressable area.
    /// * `id` - The id for this pressable element to track its state with.
    /// * `button_depth` - This is the z axis depth of the pressable area.
    /// * `button_activation_depth` - This is the current distance of the finger, within the pressable volume, from the
    ///   bottom of the button.
    /// * `out_finger_offset` - This is the current distance of the finger, within the pressable volume, from the
    ///   bottom of the button.
    /// * `out_button_state` - This is the current frame’s “active” state for the button.
    /// * `out_focus_state` - This is the current frame’s “focus” state for the button.
    ///
    /// see also [`ui_button_behavior_depth`] [`Ui::button_behavior`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, system::BtnState};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut out_button_state = BtnState::empty();
    /// let mut out_focus_state = BtnState::empty();
    /// let mut out_finger_offset = 0.0;
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("I'm a button", &mut window_pose, None, None, None);
    ///     Ui::button_behavior_depth([0.0, 0.0, 0.005],[0.05, 0.05], "Button1", 0.01, 0.005,
    ///                         &mut out_finger_offset, &mut out_button_state,
    ///                         &mut out_focus_state, None);
    ///     if out_button_state.is_just_inactive() {
    ///        println!("Button1 pressed");
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn button_behavior_depth(
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        id: impl AsRef<str>,
        button_depth: f32,
        button_activation_depth: f32,
        out_finger_offset: &mut f32,
        out_button_state: &mut BtnState,
        out_focus_state: &mut BtnState,
        out_opt_hand: Option<&mut i32>,
    ) {
        let id_hash = Ui::stack_hash(id);
        let mut nevermind = 0;
        let out_opt_hand = out_opt_hand.unwrap_or(&mut nevermind);

        unsafe {
            ui_button_behavior_depth(
                top_left_corner.into(),
                size.into(),
                id_hash,
                button_depth,
                button_activation_depth,
                out_finger_offset,
                out_button_state,
                out_focus_state,
                out_opt_hand,
            )
        }
    }

    /// This is the core functionality of StereoKit's slider elements, without any of the rendering parts! If you're
    /// trying to create your own sliding UI elements, or do more extreme customization of the look and feel of slider
    /// UI elements, then this function will provide a lot of complex pressing functionality for you
    /// <https://stereokit.net/Pages/StereoKit/UI/SliderBehavior.html>
    /// * `window_relative_pos` - The layout position of the pressable area.
    /// * `size` - The size of the pressable area.
    /// * `id` - The id for this pressable element to track its state with.
    /// * `value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `button_size_visual` - This is the visual size of the element representing the touchable area of the slider.
    ///   This is used to calculate the center of the button's placement without going outside the provided bounds.
    /// * `button_size_interact` - The size of the interactive touch element of the slider. Set this to zero to use the
    ///   entire area as a touchable surface.
    /// * `confirm_method` - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * `data` - This is data about the slider interaction, you can use this for visualizing the slider behavior, or
    ///   reacting to its events.
    ///
    /// see also [`ui_slider_behavior`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiSliderData, UiVisual}, maths::{Vec2, Vec3, Pose},
    ///                      system::BtnState};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.07, 0.90], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let depth = Ui::get_settings().depth;
    /// let size = Vec2::new(0.18, 0.11);
    /// let btn_height = Ui::get_line_height() * 0.5;
    /// let btn_size = Vec3::new(btn_height, btn_height, depth);
    ///
    /// let mut slider_pt = Vec2::new(0.25, 0.65);
    /// let id_slider = "touch panel";
    /// let id_slider_hash = Ui::stack_hash(&id_slider);
    ///
    /// filename_scr = "screenshots/ui_slider_behavior.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     let prev = slider_pt;
    ///     let mut slider = UiSliderData::default();
    ///
    ///     Ui::window_begin("I'm a slider", &mut window_pose, None, None, None);
    ///     let bounds = Ui::layout_reserve(size, false, depth);
    ///     let tlb = bounds.tlb();
    ///     Ui::slider_behavior(tlb , bounds.dimensions.xy(), id_slider_hash, &mut slider_pt,
    ///                         Vec2::ZERO, Vec2::ONE, Vec2::ZERO, btn_size.xy(), None, &mut slider);
    ///     let focus = Ui::get_anim_focus(id_slider_hash, slider.focus_state, slider.active_state);
    ///     Ui::draw_element(UiVisual::SliderLine, None,tlb,
    ///                      Vec3::new(bounds.dimensions.x, bounds.dimensions.y, depth * 0.1),
    ///                      if slider.focus_state.is_active() { 0.5 } else { 0.0 });
    ///     Ui::draw_element(UiVisual::SliderPush, None,
    ///                      slider.button_center.xy0() + btn_size.xy0() / 2.0, btn_size, focus);
    ///     if slider.active_state.is_just_inactive() {
    ///        println!("Slider1 moved");
    ///     }
    ///     Ui::label(format!("x: {:.2}          y: {:.2}", slider_pt.x, slider_pt.y), None, true);
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_slider_behavior.jpeg" alt="screenshot" width="200">
    #[allow(clippy::too_many_arguments)]
    pub fn slider_behavior(
        window_relative_pos: impl Into<Vec3>,
        size: impl Into<Vec2>,
        id: IdHashT,
        value: &mut Vec2,
        min: impl Into<Vec2>,
        max: impl Into<Vec2>,
        button_size_visual: impl Into<Vec2>,
        button_size_interact: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        data: &mut UiSliderData,
    ) {
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        unsafe {
            ui_slider_behavior(
                window_relative_pos.into(),
                size.into(),
                id,
                value,
                min.into(),
                max.into(),
                button_size_visual.into(),
                button_size_interact.into(),
                confirm_method,
                data,
            );
        }
    }
    /// A pressable button accompanied by an image! The button will expand to fit the text provided to it, horizontally.
    /// Text is re-used as the id. Will return true only on the first frame it is pressed!
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonImg.html>
    /// * `text` - Text to display on the button and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `image` - This is the image that will be drawn along with the text. See imageLayout for where the image gets
    ///   drawn!
    /// * `image_layout` - This enum specifies how the text and image should be laid out on the button. For example,
    ///   UiBtnLayout::Left will have the image on the left, and text on the right. If None will have default value of
    ///   UiBtnLayout::Left
    /// * `size` - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::get_line_height.
    /// * `color` - The Sprite’s color will be multiplied by this tint. None will have default value of white.
    ///
    /// Returns true only on the first frame it is pressed!
    /// see also [`ui_button_img`] [`ui_button_img_sz`] [`Ui::button_img_at`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui,UiBtnLayout}, maths::{Vec2, Vec3, Pose},
    ///                      sprite::Sprite, util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [-0.01, 0.095, 0.88], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut choice = "C";
    /// let log_sprite = Sprite::from_file("icons/log_viewer.png", None, None)
    ///                               .expect("log_viewer.jpeg should be ok");
    /// let scr_sprite = Sprite::from_file("icons/screenshot.png", None, None)
    ///                               .expect("screenshot.jpeg should be ok");
    /// let app_sprite = Sprite::grid();
    ///
    /// let fly_sprite = Sprite::from_file("icons/fly_over.png", None, None)
    ///                               .expect("fly_over.jpeg should be ok");
    /// let close_sprite = Sprite::close();
    ///
    /// filename_scr = "screenshots/ui_button_img.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Choose a pretty image", &mut window_pose, None, None, None);
    ///     if Ui::button_img("Log", &log_sprite, Some(UiBtnLayout::Center),
    ///                       Some([0.07, 0.07].into()), Some(named_colors::GOLD.into())) {
    ///         choice = "A";
    ///     }
    ///     if Ui::button_img("screenshot", &scr_sprite, Some(UiBtnLayout::CenterNoText),
    ///                       Some([0.07, 0.07].into()), None){
    ///         choice = "B";
    ///     }
    ///     if Ui::button_img("Applications", &app_sprite, Some(UiBtnLayout::Right),
    ///                       Some([0.17, 0.04].into()), None) {
    ///         choice = "C";
    ///     }
    ///     if Ui::button_img_at("fly", &fly_sprite, Some(UiBtnLayout::CenterNoText),
    ///                          [-0.01, -0.04, 0.0], [0.12, 0.12], Some(named_colors::CYAN.into())) {
    ///         choice = "D";
    ///     }
    ///     if Ui::button_img_at("close", &close_sprite, Some(UiBtnLayout::CenterNoText),
    ///                         [-0.08, 0.03, 0.0], [0.05, 0.05], None) {
    ///         sk.quit(None);
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_button_img.jpeg" alt="screenshot" width="200">
    pub fn button_img(
        text: impl AsRef<str>,
        image: impl AsRef<Sprite>,
        image_layout: Option<UiBtnLayout>,
        size: Option<Vec2>,
        color: Option<Color128>,
    ) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();
        let image_layout = image_layout.unwrap_or(UiBtnLayout::Left);
        let color = color.unwrap_or(Color128::WHITE);
        match size {
            Some(size) => unsafe {
                ui_button_img_sz(cstr.as_ptr(), image.as_ref().0.as_ptr(), image_layout, size, color) != 0
            },
            None => unsafe { ui_button_img(cstr.as_ptr(), image.as_ref().0.as_ptr(), image_layout, color) != 0 },
        }
    }

    /// A variant of UI::button_img that doesn’t use the layout system, and instead goes exactly where you put it.
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonImgAt.html>
    /// * `text` - Text to display on the button and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `image` - This is the image that will be drawn along with the text. See imageLayout for where the image gets
    ///   drawn!
    /// * `image_layout` - This enum specifies how the text and image should be laid out on the button. For example,
    ///   UiBtnLayout::Left will have the image on the left, and text on the right. If None will have default value of
    ///   UiBtnLayout::Left
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    /// * `color` - The Sprite’s color will be multiplied by this tint. None will have default value of white.
    ///
    /// Returns true only on the first frame it is pressed!
    /// see also [`ui_button_img_at`]
    /// see example in [`Ui::button_img`]
    pub fn button_img_at(
        text: impl AsRef<str>,
        image: impl AsRef<Sprite>,
        image_layout: Option<UiBtnLayout>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        color: Option<Color128>,
    ) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();
        let image_layout = image_layout.unwrap_or(UiBtnLayout::Left);
        let color = color.unwrap_or(Color128::WHITE);
        unsafe {
            ui_button_img_at(
                cstr.as_ptr(),
                image.as_ref().0.as_ptr(),
                image_layout,
                top_left_corner.into(),
                size.into(),
                color,
            ) != 0
        }
    }

    /// A pressable button! A button will expand to fit the text provided to it, vertically and horizontally. Text is
    /// re-used as the id. Will return true only on the first frame it is pressed!
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonRound.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    ///   hierarchy.
    /// * `image` - An image to display as the face of the button.
    /// * `diameter` - The diameter of the button’s visual.
    ///
    /// Returns true only on the first frame it is pressed!
    /// see also [`ui_button_round`] [`Ui::button_round_at`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, sprite::Sprite};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut button = 0;
    /// let close_sprite = Sprite::close();
    /// let shift_sprite = Sprite::shift();
    /// let list_sprite = Sprite::list();
    /// let backspace_sprite = Sprite::backspace();
    ///
    /// filename_scr = "screenshots/ui_button_round.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Press a round button", &mut window_pose, None, None, None);
    ///     if Ui::button_round("1", &close_sprite, 0.07) {button = 1}
    ///     Ui::same_line();
    ///     if Ui::button_round("2", &shift_sprite, 0.05) {button = 2}
    ///     if Ui::button_round_at("3", &list_sprite, [-0.04, 0.04, 0.005], 0.03) {button = 3}
    ///     if Ui::button_round_at("4", &backspace_sprite, [-0.04, -0.08, 0.005], 0.04) {button = 4}
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_button_round.jpeg" alt="screenshot" width="200">
    pub fn button_round(id: impl AsRef<str>, image: impl AsRef<Sprite>, diameter: f32) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { ui_button_round(cstr.as_ptr(), image.as_ref().0.as_ptr(), diameter) != 0 }
    }

    /// A variant of Ui::button_round that doesn’t use the layout system, and instead goes exactly where you put it.
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonRoundAt.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    ///   hierarchy.
    /// * `image` - An image to display as the face of the button.
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `diameter` - The diameter of the button’s visual.
    ///
    /// Returns true only on the first frame it is pressed!
    /// see also [`ui_button_round_at`]
    /// see example in [`Ui::button_round`]
    pub fn button_round_at(
        id: impl AsRef<str>,
        image: impl AsRef<Sprite>,
        top_left_corner: impl Into<Vec3>,
        diameter: f32,
    ) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { ui_button_round_at(cstr.as_ptr(), image.as_ref().0.as_ptr(), top_left_corner.into(), diameter) != 0 }
    }

    /// This begins and ends a handle so you can just use its grabbable/moveable functionality! Behaves much like a
    /// window, except with a more flexible handle, and no header. You can draw the handle, but it will have no text on
    /// it. Returns true for every frame the user is grabbing the handle.
    /// <https://stereokit.net/Pages/StereoKit/UI/Handle.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `pose` - The pose state for the handle! The user will be able to grab this handle and move it around.
    ///   The pose is relative to the current hierarchy stack.
    /// * `handle` - Size and location of the handle, relative to the pose.
    /// * `draw_handle` - Should this function draw the handle for you, or will you draw that yourself?
    /// * `move_type` - Describes how the handle will move when dragged around. If None, has default value of UiMove::Exact
    /// * `allower_gesture` - Which hand gestures are used for interacting with this Handle? If None, has default value of
    ///   UiGesture::Pinch
    ///
    /// Returns true for every frame the user is grabbing the handle.
    /// see also [`ui_handle_begin`] [`ui_handle_end`] [`Ui::handle_begin`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiMove, UiGesture}, maths::{Vec2, Vec3, Pose, Bounds},
    ///                      material::Material, mesh::Mesh, util::named_colors};
    ///
    /// // lets create two handles of the same size:
    /// let mut handle_pose1 = Pose::new(
    ///     [-0.02, -0.035, 0.92], Some([0.0, 145.0, 0.0].into()));
    /// let mut handle_pose2 = Pose::new(
    ///     [0.02, 0.035, 0.92], Some([0.0, 145.0, 0.0].into()));
    /// let handle_bounds = Bounds::new([0.0, 0.0, 0.0], [0.045, 0.045, 0.045]);
    ///
    /// let mut material_bound = Material::ui_box();
    /// material_bound  .color_tint(named_colors::GOLD)
    ///                 .get_all_param_info().set_float("border_size", 0.0025);
    /// let cube_bounds  = Mesh::cube();
    ///
    /// filename_scr = "screenshots/ui_handle.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     // Handles are drawn
    ///     Ui::handle("Handle1", &mut handle_pose1, handle_bounds, true,
    ///                Some(UiMove::FaceUser), Some(UiGesture::Pinch));
    ///
    ///     // Handles aren't drawn so we draw a cube_bound to show where they are.
    ///     Ui::handle("Handle2", &mut handle_pose2, handle_bounds, false,
    ///                Some(UiMove::PosOnly), Some(UiGesture::PinchGrip));
    ///     cube_bounds.draw(token, &material_bound,
    ///                      handle_pose2.to_matrix(Some(handle_bounds.dimensions)), None, None);
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_handle.jpeg" alt="screenshot" width="200">
    pub fn handle(
        id: impl AsRef<str>,
        pose: &mut Pose,
        handle: Bounds,
        draw_handle: bool,
        move_type: Option<UiMove>,
        allower_gesture: Option<UiGesture>,
    ) -> bool {
        let move_type = move_type.unwrap_or(UiMove::Exact);
        let allower_gesture = allower_gesture.unwrap_or(UiGesture::Pinch);
        let cstr = CString::new(id.as_ref()).unwrap();
        let result = unsafe {
            ui_handle_begin(cstr.as_ptr(), pose, handle, draw_handle as Bool32T, move_type, allower_gesture) != 0
        };
        unsafe { ui_handle_end() }
        result
    }

    /// This begins a new UI group with its own layout! Much like a window, except with a more flexible handle, and no
    /// header. You can draw the handle, but it will have no text on it. The pose value is always relative to the
    /// current hierarchy stack. This call will also push the pose transform onto the hierarchy stack, so any objects
    /// drawn up to the corresponding Ui::handle_end() will get transformed by the handle pose. Returns true for every
    /// frame the user is grabbing the handle.
    /// <https://stereokit.net/Pages/StereoKit/UI/HandleBegin.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `pose` - The pose state for the handle! The user will be able to grab this handle and move it around.
    ///   The pose is relative to the current hierarchy stack.
    /// * `handle` - Size and location of the handle, relative to the pose.
    /// * `draw_handle` - Should this function draw the handle for you, or will you draw that yourself?
    /// * `move_type` - Describes how the handle will move when dragged around. If None, has default value of UiMove::Exact
    /// * `allower_gesture` - Which hand gestures are used for interacting with this Handle? If None, has default value of
    ///   UiGesture::Pinch
    ///
    /// Returns true for every frame the user is grabbing the handle.
    /// see also [`ui_handle_begin`] [`Ui::handle`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiMove, UiGesture},
    ///                      maths::{Vec2, Vec3, Pose, Bounds, Matrix},
    ///                      material::Material, mesh::Mesh, util::named_colors, sprite::Sprite};
    ///
    /// // lets create two handles of the same size:
    /// let mut handle_pose1 = Pose::new(
    ///     [-0.02, -0.035, 0.92], Some([0.0, 145.0, 0.0].into()));
    /// let mut handle_pose2 = Pose::new(
    ///     [0.02, 0.035, 0.92], Some([0.0, 145.0, 0.0].into()));
    /// let handle_bounds = Bounds::new([0.0, 0.0, 0.0], [0.045, 0.045, 0.045]);
    ///
    /// let mut material_bound = Material::ui_box();
    /// material_bound  .color_tint(named_colors::GOLD)
    ///                 .get_all_param_info().set_float("border_size", 0.0025);
    /// let cube_bounds  = Mesh::cube();
    ///
    /// let sphere = Mesh::generate_sphere(0.045, None);
    /// let material_sphere = Material::pbr();
    ///
    /// filename_scr = "screenshots/ui_handle_begin.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     // Handles aren't drawn so we draw som cube_bounds to show where they are.
    ///     Ui::handle_begin("Handle1", &mut handle_pose1, handle_bounds, false,
    ///                Some(UiMove::FaceUser), Some(UiGesture::Pinch));
    ///     sphere.draw(token, &material_sphere, Matrix::IDENTITY, None, None);
    ///     cube_bounds.draw(token, &material_bound, Matrix::s(handle_bounds.dimensions), None, None);
    ///     Ui::handle_end();
    ///
    ///     Ui::handle_begin("Handle2", &mut handle_pose2, handle_bounds, false,
    ///                Some(UiMove::PosOnly), Some(UiGesture::PinchGrip));
    ///     sphere.draw(token, &material_sphere, Matrix::IDENTITY, None, None);
    ///     cube_bounds.draw(token, &material_bound, Matrix::s(handle_bounds.dimensions), None, None);
    ///     Ui::handle_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_handle_begin.jpeg" alt="screenshot" width="200">
    pub fn handle_begin(
        id: impl AsRef<str>,
        pose: &mut Pose,
        handle: Bounds,
        draw_handle: bool,
        move_type: Option<UiMove>,
        allower_gesture: Option<UiGesture>,
    ) -> bool {
        let move_type = move_type.unwrap_or(UiMove::Exact);
        let allower_gesture = allower_gesture.unwrap_or(UiGesture::Pinch);
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { ui_handle_begin(cstr.as_ptr(), pose, handle, draw_handle as Bool32T, move_type, allower_gesture) != 0 }
    }

    /// Finishes a handle! Must be called after [`Ui::handle_begin`] and all elements have been drawn. Pops the pose
    /// transform pushed by Ui::handle_begin() from the hierarchy stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/HandleEnd.html>
    ///
    /// see also [`ui_handle_end`]
    /// see example in [`Ui::handle_begin`]
    pub fn handle_end() {
        unsafe { ui_handle_end() };
    }

    /// This draws a line horizontally across the current layout. Makes a good separator between sections of UI!
    /// <https://stereokit.net/Pages/StereoKit/UI/HSeparator.html>
    ///
    /// see also [`ui_hseparator`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2,  Pose}, system::TextStyle,
    ///                      util::named_colors, font::Font};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let font = Font::default();
    /// let mut style_big    = TextStyle::from_font(&font, 0.002, named_colors::CYAN);
    /// style_big.layout_height(0.018);
    /// let mut style_medium = TextStyle::from_font(&font, 0.002, named_colors::RED);
    /// style_medium.layout_height(0.010);
    /// let mut style_small  = TextStyle::from_font(&font, 0.002, named_colors::GOLD);
    /// style_small.layout_height(0.006);
    ///
    /// filename_scr = "screenshots/ui_hseparator.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Separators", &mut window_pose, Some([0.17, 0.0].into()), None, None);
    ///     Ui::push_text_style(style_big);
    ///     Ui::text("The first part", None, None, None, None, None, None);
    ///     Ui::hseparator();
    ///     Ui::pop_text_style();
    ///     Ui::push_text_style(style_medium);
    ///     Ui::text("The second part", None, None, None, None, None, None);
    ///     Ui::hseparator();
    ///     Ui::pop_text_style();
    ///     Ui::push_text_style(style_small);
    ///     Ui::text("The third part", None, None, None, None, None, None);
    ///     Ui::hseparator();
    ///     Ui::pop_text_style();
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_hseparator.jpeg" alt="screenshot" width="200">
    pub fn hseparator() {
        unsafe { ui_hseparator() };
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/HSlider.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `width` - Physical width of the slider on the window. None is default 0 will fill the remaining amount of
    ///   window space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_hslider`] [`Ui::hslider_f64`] [`Ui::hslider_at`] [`Ui::hslider_at_f64`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiConfirm, UiNotify}, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut scaling1 = 0.15;
    /// let mut scaling2 = 0.50f64;
    /// let mut scaling3 = 0.0;
    /// let mut scaling4 = 0.85;
    ///
    /// filename_scr = "screenshots/ui_hslider.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("VSlider", &mut window_pose, None, None, None);
    ///     Ui::hslider(    "scaling1", &mut scaling1, 0.0, 1.0, Some(0.05), Some(0.10),
    ///                     None, None);
    ///     Ui::hslider_f64("scaling2", &mut scaling2, 0.0, 1.0, None, Some(0.12),
    ///                     Some(UiConfirm::Pinch), None);
    ///     Ui::hslider_at( "scaling3", &mut scaling3, 0.0, 1.0, None,
    ///                     [0.0, 0.0, 0.0], [0.08, 0.02],
    ///                     None, Some(UiNotify::Finalize));
    ///     if let Some(new_value) = Ui::hslider_at_f64(
    ///                     "scaling4", &mut scaling4, 0.0, 1.0, None,
    ///                     [0.07, -0.085, 0.0], [0.15, 0.036],
    ///                     Some(UiConfirm::VariablePinch), None) {
    ///         if new_value == 1.0 {
    ///             Log::info("scaling4 is at max");
    ///         }
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_hslider.jpeg" alt="screenshot" width="200">
    #[allow(clippy::too_many_arguments)]
    pub fn hslider(
        id: impl AsRef<str>,
        out_value: &mut f32,
        min: f32,
        max: f32,
        step: Option<f32>,
        width: Option<f32>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f32> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let width = width.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe { ui_hslider(cstr.as_ptr(), out_value, min, max, step, width, confirm_method, notify_on) != 0 } {
            true => Some(*out_value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/HSlider.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `width` - Physical width of the slider on the window. None is default 0 will fill the remaining amount of
    ///   window space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_hslider_f64`] [`Ui::hslider`] [`Ui::hslider_at`] [`Ui::hslider_at_f64`]
    /// see example in [`Ui::hslider`]
    #[allow(clippy::too_many_arguments)]
    pub fn hslider_f64(
        id: impl AsRef<str>,
        out_value: &mut f64,
        min: f64,
        max: f64,
        step: Option<f64>,
        width: Option<f32>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f64> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let width = width.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe { ui_hslider_f64(cstr.as_ptr(), out_value, min, max, step, width, confirm_method, notify_on) != 0 }
        {
            true => Some(*out_value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/HSliderAt.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_hslider_at`] [`Ui::hslider_f64`] [`Ui::hslider`] [`Ui::hslider_at_f64`]
    /// see example in [`Ui::hslider`]
    #[allow(clippy::too_many_arguments)]
    pub fn hslider_at(
        id: impl AsRef<str>,
        out_value: &mut f32,
        min: f32,
        max: f32,
        step: Option<f32>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f32> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe {
            ui_hslider_at(
                cstr.as_ptr(),
                out_value,
                min,
                max,
                step,
                top_left_corner.into(),
                size.into(),
                confirm_method,
                notify_on,
            ) != 0
        } {
            true => Some(*out_value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/HSliderAt.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_hslider_at_f64`] [`Ui::hslider_f64`] [`Ui::hslider`] [`Ui::hslider_at`]
    /// see example in [`Ui::hslider`]
    #[allow(clippy::too_many_arguments)]
    pub fn hslider_at_f64(
        id: impl AsRef<str>,
        out_value: &mut f64,
        min: f64,
        max: f64,
        step: Option<f64>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f64> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe {
            ui_hslider_at_f64(
                cstr.as_ptr(),
                out_value,
                min,
                max,
                step,
                top_left_corner.into(),
                size.into(),
                confirm_method,
                notify_on,
            ) != 0
        } {
            true => Some(*out_value),
            false => None,
        }
    }

    /// Adds an image to the UI!
    /// <https://stereokit.net/Pages/StereoKit/UI/Image.html>
    /// * `sprite` - A valid sprite.
    /// * `size` - Size in Hierarchy local meters. If one of the components is 0, it’ll be automatically determined from
    ///   the other component and the image’s aspect ratio.
    ///
    /// see also [`ui_image`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, sprite::Sprite};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let log_sprite = Sprite::from_file("icons/log_viewer.png", None, None)
    ///                               .expect("log_viewer.jpeg should be ok");
    /// let scr_sprite = Sprite::from_file("icons/screenshot.png", None, None)
    ///                               .expect("screenshot.jpeg should be ok");
    /// let app_sprite = Sprite::grid();
    ///
    /// filename_scr = "screenshots/ui_image.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Images", &mut window_pose, None, None, None);
    ///     Ui::image(&log_sprite, [0.03, 0.03]);
    ///     Ui::same_line();
    ///     Ui::image(&app_sprite, [0.06, 0.06]);
    ///     Ui::image(&scr_sprite, [0.05, 0.05]);
    ///     Ui::same_line();
    ///     Ui::image(&log_sprite, [0.05, 0.05]);
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_image.jpeg" alt="screenshot" width="200">
    pub fn image(image: impl AsRef<Sprite>, size: impl Into<Vec2>) {
        unsafe { ui_image(image.as_ref().0.as_ptr(), size.into()) };
    }

    /// This is an input field where users can input text to the app! Selecting it will spawn a virtual keyboard, or act
    /// as the keyboard focus. Hitting escape or enter, or focusing another UI element will remove focus from this Input.
    /// <https://stereokit.net/Pages/StereoKit/UI/Input.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The string that will store the Input’s content in.
    /// * `size` - The layout size for this element in Hierarchy space.  Zero axes will auto-size. None is full auto-size.
    /// * `type_text` - What category of text this Input represents. This may affect what kind of soft keyboard will
    ///   be displayed, if one is shown to the user. None has default value of TextContext::Text.
    ///
    /// Returns the current text in the input field if it has changed, otherwise `None`.
    /// see also [`ui_input`] [`Ui::input_at`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose},
    ///                      system::TextContext};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.05, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut username = String::from("user");
    /// let mut password = String::from("password");
    /// let mut zip_code = String::from("97400");
    /// let mut pin_code = String::from("123456");
    /// filename_scr = "screenshots/ui_input.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Input fields", &mut window_pose, None, None, None);
    ///     Ui::input("username1", &mut username, Some([0.15, 0.03].into()), None);
    ///     Ui::input("password1", &mut password, None, Some(TextContext::Password));
    ///     Ui::input_at("zip code1", &mut zip_code, [0.08, -0.09, 0.0], [0.06, 0.03],
    ///                  Some(TextContext::Number));
    ///
    ///     if let Some(new_value) =
    ///         Ui::input_at("pin_code1", &mut pin_code, [0.0, -0.09, 0.0], [0.05, 0.03],
    ///                      Some(TextContext::Number)) {
    ///         if new_value.is_empty() {
    ///             Log::warn("pin_code should not be empty");
    ///         }
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_input.jpeg" alt="screenshot" width="200">
    pub fn input(
        id: impl AsRef<str>,
        out_value: &mut String,
        size: Option<Vec2>,
        type_text: Option<TextContext>,
    ) -> Option<String> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let c_value = CString::new(out_value.as_str()).unwrap();
        let size = size.unwrap_or(Vec2::ZERO);
        let type_text = type_text.unwrap_or(TextContext::Text);
        if unsafe {
            ui_input(cstr.as_ptr(), c_value.as_ptr() as *mut c_char, out_value.capacity() as i32 + 16, size, type_text)
                != 0
        } {
            match unsafe { CStr::from_ptr(c_value.as_ptr()).to_str() } {
                Ok(result) => {
                    out_value.clear();
                    out_value.push_str(result);
                    Some(result.to_owned())
                }
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// This is an input field where users can input text to the app! Selecting it will spawn a virtual keyboard, or act
    ///  as the keyboard focus. Hitting escape or enter, or focusing another UI element will remove focus from this Input.
    /// <https://stereokit.net/Pages/StereoKit/UI/InputAt.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The string that will store the Input's content in.
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    /// * `type_text` - What category of text this Input represents. This may affect what kind of soft keyboard will
    ///   be displayed, if one is shown to the user. None has default value of TextContext::Text.
    ///
    /// Returns the current text in the input field if it has changed during this step, otherwise `None`.
    /// see also [`ui_input_at`]
    /// see example in [`Ui::input`]
    pub fn input_at(
        id: impl AsRef<str>,
        out_value: &mut String,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        type_text: Option<TextContext>,
    ) -> Option<String> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let c_value = CString::new(out_value.as_str()).unwrap();
        let size = size.into();
        let type_text = type_text.unwrap_or(TextContext::Text);
        if unsafe {
            ui_input_at(
                cstr.as_ptr(),
                c_value.as_ptr() as *mut c_char,
                out_value.capacity() as i32 + 16,
                top_left_corner.into(),
                size,
                type_text,
            ) != 0
        } {
            match unsafe { CStr::from_ptr(c_value.as_ptr()).to_str() } {
                Ok(result) => {
                    out_value.clear();
                    out_value.push_str(result);
                    Some(result.to_owned())
                }
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// Tells if the user is currently interacting with a UI element! This will be true if the hand has an active or
    /// focused UI element.
    /// <https://stereokit.net/Pages/StereoKit/UI/IsInteracting.html>
    /// * `hand` - The hand to check for interaction.
    ///
    /// Returns true if the hand has an active or focused UI element. False otherwise.
    /// see also [`ui_is_interacting`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, system::Handed};
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     // These are unit tests. no arms, no chocolate.
    ///     assert_eq!(Ui::is_interacting(Handed::Right), false);
    ///     assert_eq!(Ui::is_interacting(Handed::Left), false);
    ///     assert_eq!(Ui::is_interacting(Handed::Max), false);
    /// );
    /// ```
    pub fn is_interacting(hand: Handed) -> bool {
        unsafe { ui_is_interacting(hand) != 0 }
    }

    /// Adds some text to the layout! Text uses the UI’s current font settings, which can be changed with
    /// Ui::push/pop_text_style. Can contain newlines!
    /// <https://stereokit.net/Pages/StereoKit/UI/Label.html>
    /// * `text` - Label text to display. Can contain newlines! Doesn’t use text as id, so it can be non-unique.
    /// * `size` - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is
    ///   [`Ui::get_line_height`]. If None, both axis will be auto-calculated.
    /// * `use_padding` - Should padding be included for positioning this text? Sometimes you just want un-padded text!
    ///
    /// see also [`ui_label`] [`ui_label_sz`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.93], Some([0.0, 185.0, 0.0].into()));
    ///
    /// filename_scr = "screenshots/ui_label.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Labels", &mut window_pose, None, None, None);
    ///     Ui::label("Label 1", None, false);
    ///     Ui::same_line();
    ///     Ui::label("Label 2", Some([0.025, 0.0].into()), false);
    ///     Ui::label("Label 3", Some([0.1,   0.01].into()), true);
    ///     Ui::label("Label 4", Some([0.0,   0.0045].into()), false);
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_label.jpeg" alt="screenshot" width="200">
    pub fn label(text: impl AsRef<str>, size: Option<Vec2>, use_padding: bool) {
        let cstr = CString::new(text.as_ref()).unwrap();
        match size {
            Some(size) => unsafe { ui_label_sz(cstr.as_ptr(), size, use_padding as Bool32T) },
            None => unsafe { ui_label(cstr.as_ptr(), use_padding as Bool32T) },
        }
    }

    /// Tells if the hand was involved in the active state of the most recently called UI element using an id. Active
    /// state is frequently a single frame in the case of Buttons, but could be many in the case of Sliders or Handles.
    /// <https://stereokit.net/Pages/StereoKit/UI/LastElementHandActive.html>
    /// * `hand` - Which hand we’re checking.
    ///
    /// Returns a BtnState that indicated the hand was “just active” this frame, is currently “active” or if it “just
    /// became inactive” this frame.
    /// see also [`ui_last_element_hand_active`] [`Ui::get_last_element_active`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, system::{Handed, BtnState}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Last Element Hand Active", &mut window_pose, None, None, None);
    ///     Ui::button("Button1", None);
    ///     let state_hand = Ui::last_element_hand_active(Handed::Right);
    ///     let state_element = Ui::get_last_element_active();
    ///
    ///     assert_eq!( state_hand.is_just_active(), false);
    ///     assert_eq!( state_element.is_just_active(), false);
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn last_element_hand_active(hand: Handed) -> BtnState {
        unsafe { ui_last_element_hand_active(hand) }
    }

    /// Tells if the hand was involved in the focus state of the most recently called UI element using an id. Focus
    /// occurs when the hand is in or near an element, in such a way that indicates the user may be about to interact
    /// with it.
    /// <https://stereokit.net/Pages/StereoKit/UI/LastElementHandFocused.html>
    /// * `hand` - Which hand we’re checking.
    ///
    /// Returns a BtnState that indicated the hand was “just focused” this frame, is currently “focused” or if it “just
    /// became focused” this frame.
    /// see also [`ui_last_element_hand_focused`] [`Ui::get_last_element_focused`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, system::{Handed, BtnState}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Last Element Hand Focuse", &mut window_pose, None, None, None);
    ///     Ui::button("Button1", None);
    ///     let state_hand = Ui::last_element_hand_focused(Handed::Right);
    ///     let state_element = Ui::get_last_element_focused();
    ///     assert_eq!( state_hand.is_just_active(), false);
    ///     assert_eq!( state_element.is_just_active(), false);
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn last_element_hand_focused(hand: Handed) -> BtnState {
        unsafe { ui_last_element_hand_focused(hand) }
    }

    /// Manually define what area is used for the UI layout. This is in the current Hierarchy’s coordinate space on the
    /// X/Y plane.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutArea.html>
    /// * `start` - The top left of the layout area, relative to the current Hierarchy in local meters.
    /// * `dimensions` - The size of the layout area from the top left, in local meters.
    /// * `add_margin` - If true, the layout area will have a margin added to it.
    ///
    /// see also [`ui_layout_area`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, sprite::Sprite};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let sprite = Sprite::from_file("icons/log_viewer.png", None, None)
    ///                       .expect("open_gltf.jpeg should be ok");
    ///
    /// filename_scr = "screenshots/ui_layout_area.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Layout Area", &mut window_pose, Some([0.15, 0.13].into()), None, None);
    ///     Ui::layout_area([0.1, -0.04, 0.0], [0.01, 0.01], true);
    ///     Ui::image(&sprite, [0.07, 0.07]);
    ///     Ui::layout_area([0.00, -0.01, 0.0], [0.01, 0.01], true);
    ///     Ui::label("Text and more", None, false);
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_layout_area.jpeg" alt="screenshot" width="200">
    pub fn layout_area(start: impl Into<Vec3>, dimensions: impl Into<Vec2>, add_margin: bool) {
        unsafe { ui_layout_area(start.into(), dimensions.into(), add_margin as Bool32T) };
    }

    /// This pushes a layout rect onto the layout stack. All UI elements using the layout system will now exist inside
    /// this layout area! Note that some UI elements such as Windows will already be managing a layout of their own on
    /// the stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutPush.html>
    /// * `start` - The top left position of the layout. Note that Windows have their origin at the top center, the
    ///   left side of a window is X+, and content advances to the X- direction.
    /// * `dimensions` - The total size of the layout area. A value of zero means the layout will expand in that axis,
    ///   but may prevent certain types of layout “Cuts”.
    /// * `add_margin` - Adds a spacing margin to the interior of the layout. Most of the time you won’t need this,
    ///   but may be useful when working without a Window.
    ///
    /// see also [`ui_layout_push`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiCut}, maths::{Vec2, Vec3, Pose}, sprite::Sprite};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let sprite = Sprite::from_file("icons/screenshot.png", None, None)
    ///                       .expect("open_gltf.jpeg should be ok");
    ///
    /// filename_scr = "screenshots/ui_layout_push.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Layout Push", &mut window_pose, Some([0.15, 0.13].into()), None, None);
    ///     Ui::layout_push([0.1, -0.04, 0.0], [0.01, 0.01], true);
    ///     Ui::image(&sprite, [0.07, 0.07]);
    ///     Ui::layout_pop();
    ///     Ui::layout_push_cut( UiCut::Right, 0.1, true);
    ///     Ui::label("Text and more ...", None, false);
    ///     Ui::layout_push_cut( UiCut::Bottom, 0.02, true);
    ///     Ui::label("And again and again ...", None, false);
    ///     Ui::layout_pop();
    ///     Ui::layout_pop();
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_layout_push.jpeg" alt="screenshot" width="200">
    pub fn layout_push(start: impl Into<Vec3>, dimensions: impl Into<Vec2>, add_margin: bool) {
        unsafe { ui_layout_push(start.into(), dimensions.into(), add_margin as Bool32T) };
    }

    /// This cuts off a portion of the current layout area, and pushes that new area onto the layout stack. Left and Top
    /// cuts will always work, but Right and Bottom cuts can only exist inside of a parent layout with an explicit size,
    /// auto-resizing prevents these cuts. All UI elements using the layout system will now exist inside this layout
    /// area! Note that some UI elements such as Windows will already be managing a layout of their own on the stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutPushCut.html>
    /// * `cut_to` - Which side of the current layout should the cut happen to? Note that Right and Bottom will require
    ///   explicit sizes in the parent layout, not auto-sizes.   
    /// * `size_meters` - The size of the layout cut, in meters.
    /// * `add_margin` - Adds a spacing margin to the interior of the layout. Most of the time you won’t need this,
    ///   but may be useful when working without a Window.
    ///
    /// see also [`ui_layout_push_cut`]
    /// see example in [`Ui::layout_push`]
    pub fn layout_push_cut(cut_to: UiCut, size_meters: f32, add_margin: bool) {
        unsafe { ui_layout_push_cut(cut_to, size_meters, add_margin as Bool32T) };
    }

    /// This removes a layout from the layout stack that was previously added using Ui::layout_push, or
    /// Ui::layout_push_cut.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutPop.html>
    ///
    /// see also [`ui_layout_pop`]
    /// see example in [`Ui::layout_push`]
    pub fn layout_pop() {
        unsafe { ui_layout_pop() };
    }

    /// Reserves a box of space for an item in the current UI layout! If either size axis is zero, it will be auto-sized
    /// to fill the current surface horizontally, and fill a single line_height vertically. Returns the Hierarchy local
    /// bounds of the space that was reserved, with a Z axis dimension of 0.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutReserve.html>
    /// * `size` - Size of the layout box in Hierarchy local meters.
    /// * `add_padding` - If true, this will add the current padding value to the total final dimensions of the space that
    ///   is reserved.
    /// * `depth` - This allows you to quickly insert a depth into the Bounds you’re receiving. This will offset on the
    ///   Z axis in addition to increasing the dimensions, so that the bounds still remain sitting on the surface of the
    ///   UI. This depth value will not be reflected in the bounds provided by LayouLast.
    ///
    /// Returns the Hierarchy local bounds of the space that was reserved, with a Z axis dimension of 0.
    /// see also [`ui_layout_reserve`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose, Bounds}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Layout Reserve", &mut window_pose, Some([0.2, 0.2].into()), None, None);
    ///
    ///     let bounds = Ui::layout_reserve([0.05, 0.05], true, 0.005);
    ///
    ///     let bounds_no_pad = Ui::layout_reserve([0.05, 0.05], false, 0.005);
    ///
    ///     assert_eq!(bounds, Bounds::new([0.055, -0.045, -0.0025], [0.07, 0.07, 0.005]));
    ///     assert_eq!(bounds_no_pad, Bounds::new([0.065, -0.115, -0.0025], [0.05, 0.05, 0.005]));
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn layout_reserve(size: impl Into<Vec2>, add_padding: bool, depth: f32) -> Bounds {
        unsafe { ui_layout_reserve(size.into(), add_padding as Bool32T, depth) }
    }

    /// This adds a non-interactive Model to the UI panel layout, and allows you to specify its size.
    /// <https://stereokit.net/Pages/StereoKit/UI/Model.html>
    /// * `model` - The model to use
    /// * `ui_size` - The size this element should take from the layout.
    /// * `model_scale` - 0 will auto-scale the model to fit the layout space, but you can specify a different scale in
    ///   case you’d like a different size. None will auto-scale the model.
    ///
    /// see also [`ui_model`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, model::Model,
    ///                      mesh::Mesh, material::Material};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.92], Some([0.0, 215.0, 0.0].into()));
    ///
    /// let model = Model::from_mesh(Mesh::sphere(), Material::pbr());
    ///
    /// filename_scr = "screenshots/ui_model.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Model", &mut window_pose, None, None, None);
    ///     Ui::model(&model, Some([0.03, 0.03].into()), None);
    ///     Ui::model(&model, Some([0.04, 0.04].into()), Some(0.05));
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_model.jpeg" alt="screenshot" width="200">
    pub fn model(model: impl AsRef<Model>, ui_size: Option<Vec2>, model_scale: Option<f32>) {
        let ui_size = ui_size.unwrap_or(Vec2::ZERO);
        let model_scale = model_scale.unwrap_or(0.0);
        unsafe { ui_model(model.as_ref().0.as_ptr(), ui_size, model_scale) };
    }

    /// This will advance the layout to the next line. If there’s nothing on the current line, it’ll advance to the
    /// start of the next on. But this won’t have any affect on an empty line, try Ui::hspace for that.
    /// <https://stereokit.net/Pages/StereoKit/UI/NextLine.html>
    ///
    /// see also [`ui_nextline`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.93], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Next line", &mut window_pose, None, None, None);
    ///     Ui::label("Line 1", None, false);
    ///     Ui::next_line();
    ///     Ui::next_line();
    ///     Ui::next_line();
    ///     Ui::label("Line 5", None, false);
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn next_line() {
        unsafe { ui_nextline() };
    }

    /// If you wish to manually draw a Panel, this function will let you draw one wherever you want!
    /// <https://stereokit.net/Pages/StereoKit/UI/PanelAt.html>
    /// * `start` - The top left corner of the Panel element.
    /// * `size` - The size of the Panel element, in hierarchy local meters.
    /// * `padding` - Only UiPad::Outsize has any effect here. UiPad::Inside will behave the same as UiPad::None.
    ///
    /// see also [`ui_panel_at`] [`Ui::panel_begin`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiPad, UiCut}, maths::{Vec2, Vec3, Pose, Bounds}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.90], Some([0.0, 185.0, 0.0].into()));
    ///
    /// filename_scr = "screenshots/ui_panel_at.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Panel at", &mut window_pose, Some([0.2, 0.15].into()), None, None);
    ///     Ui::panel_at([0.11, -0.01, 0.0], [0.08, 0.03], Some(UiPad::None));
    ///     Ui::label("panel 1", None, false);
    ///
    ///     Ui::layout_push_cut( UiCut::Right, 0.1, true);
    ///     Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
    ///     Ui::label("panel 2", None, false);
    ///     Ui::layout_pop();
    ///
    ///     Ui::layout_push_cut( UiCut::Bottom, 0.08, false);
    ///     Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
    ///     Ui::label("panel 3", None, false);
    ///     Ui::layout_pop();
    ///
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_panel_at.jpeg" alt="screenshot" width="200">
    pub fn panel_at(start: impl Into<Vec3>, size: impl Into<Vec2>, padding: Option<UiPad>) {
        let padding = padding.unwrap_or(UiPad::Outside);
        unsafe { ui_panel_at(start.into(), size.into(), padding) };
    }

    /// This will begin a Panel element that will encompass all elements drawn between panel_begin and panel_end. This
    /// is an entirely visual element, and is great for visually grouping elements together. Every Begin must have a
    /// matching End.
    /// <https://stereokit.net/Pages/StereoKit/UI/PanelBegin.html>
    /// * `padding` - Describes how padding is applied to the visual element of the Panel. If None the default value is
    ///   UiPad::Outside
    ///
    /// see also [`ui_panel_begin`] [`Ui::panel_at`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiPad, UiCut}, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.90], Some([0.0, 185.0, 0.0].into()));
    ///
    /// filename_scr = "screenshots/ui_panel_begin.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Panel begin", &mut window_pose, Some([0.2, 0.15].into()), None, None);
    ///     Ui::panel_begin(Some(UiPad::None));
    ///     Ui::label("panel 1", None, false);
    ///     Ui::panel_end();
    ///
    ///     Ui::layout_push_cut( UiCut::Right, 0.1, true);
    ///     Ui::panel_begin(None);
    ///     Ui::label("panel 2", None, false);
    ///     Ui::panel_end();
    ///     Ui::layout_pop();
    ///
    ///     Ui::layout_push_cut( UiCut::Bottom, 0.08, false);
    ///     Ui::panel_begin(Some(UiPad::Inside));
    ///     Ui::label("panel 3\nwith CRLF", None, false);
    ///     Ui::panel_end();
    ///     Ui::layout_pop();
    ///
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_panel_begin.jpeg" alt="screenshot" width="200">
    pub fn panel_begin(padding: Option<UiPad>) {
        let padding = padding.unwrap_or(UiPad::Outside);
        unsafe { ui_panel_begin(padding) };
    }

    /// This will finalize and draw a Panel element.
    /// <https://stereokit.net/Pages/StereoKit/UI/PanelEnd.html>
    ///
    /// see also [`ui_panel_end`]
    /// see example in [`Ui::panel_begin`]
    pub fn panel_end() {
        unsafe { ui_panel_end() };
    }

    /// Removes an ‘enabled’ state from the stack, and whatever was below will then be used as the primary enabled
    /// state.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopEnabled.html>
    ///
    /// see also [`ui_pop_enabled`]
    /// see example in [`Ui::push_enabled`]
    pub fn pop_enabled() {
        unsafe { ui_pop_enabled() };
    }

    /// Removes the last root id from the stack, and moves up to the one before it!
    /// <https://stereokit.net/Pages/StereoKit/UI/PopId.html>
    ///
    /// see also [`ui_pop_id`]
    /// see example in [`Ui::push_id`]
    pub fn pop_id() {
        unsafe { ui_pop_id() };
    }

    /// This pops the keyboard presentation state to what it was previously.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopPreserveKeyboard.html>
    ///
    /// see also [`ui_pop_preserve_keyboard`]
    /// see example in [`Ui::push_preserve_keyboard`]
    pub fn pop_preserve_keyboard() {
        unsafe { ui_pop_preserve_keyboard() };
    }

    /// This removes an enabled status for grab auras from the stack, returning it to the state before the previous
    /// push_grab_aura call. Grab auras are an extra space and visual element that goes around Window elements to make
    /// them easier to grab.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopGrabAurahtml>
    ///
    /// see also [`ui_pop_grab_aura`]
    /// see example in [`Ui::push_grab_aura`]
    pub fn pop_grab_aura() {
        unsafe { ui_pop_grab_aura() };
    }

    /// This retreives the top of the grab aura enablement stack, in case you need to know if the current window will
    /// have an aura.
    /// <https://stereokit.net/Pages/StereoKit/UI/GrabAuraEnabled>
    ///
    /// see also [`ui_grab_aura_enabled`]
    /// see example in [`Ui::push_grab_aura`]
    pub fn grab_aura_enabled() -> bool {
        unsafe { ui_grab_aura_enabled() != 0 }
    }

    /// This will return to the previous UI layout on the stack. This must be called after a PushSurface call.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopSurface.html>
    ///
    /// see also [`ui_pop_surface`]
    /// see example in [`Ui::push_surface`]
    pub fn pop_surface() {
        unsafe { ui_pop_surface() };
    }

    /// Removes a TextStyle from the stack, and whatever was below will then be used as the GUI’s primary font.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopTextStyle.html>
    ///
    /// see also [`ui_pop_text_style`]
    /// see example in [`Ui::push_text_style`]
    pub fn pop_text_style() {
        unsafe { ui_pop_text_style() };
    }

    /// Removes a Tint from the stack, and whatever was below will then be used as the primary tint.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopTint.html>
    ///
    /// see also [`ui_pop_tint`]
    /// see example in [`Ui::push_tint`]
    pub fn pop_tint() {
        unsafe { ui_pop_tint() };
    }

    /// This creates a Pose that is friendly towards UI popup windows, or windows that are created due to some type of
    /// user interaction. The fallback file picker and soft keyboard both use this function to position themselves!
    /// <https://stereokit.net/Pages/StereoKit/UI/PopupPose.html>
    /// * `shift` - A positional shift from the default location, this is useful to account for the height of the window,
    ///   and center or offset this pose. A value of [0.0, -0.1, 0.0] may be a good starting point.
    ///
    /// Returns a pose between the UI or hand that is currently active, and the user’s head. Good for popup windows.
    /// see also [`ui_popup_pose`]
    pub fn popup_pose(shift: impl Into<Vec3>) -> Pose {
        unsafe { ui_popup_pose(shift.into()) }
    }

    /// This is a simple horizontal progress indicator bar. This is used by the hslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/ProgressBar.html>
    ///
    /// see also [`ui_hprogress_bar`]
    #[deprecated(since = "0.0.1", note = "Use HProgressBar instead")]
    pub fn progress_bar(percent: f32, width: f32) {
        unsafe { ui_hprogress_bar(percent, width, 0) }
    }

    /// This is a simple horizontal progress indicator bar. This is used by the hslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/HProgressBar.html>
    /// * `percent` - A value between 0 and 1 indicating progress from 0% to 100%.
    /// * `width` - Physical width of the slider on the window. 0 will fill the remaining amount of window space.
    /// * `flip_fill_direction` - By default, this fills from left to right. This allows you to flip the fill direction to
    ///   right to left.
    ///
    /// see also [`ui_hprogress_bar`]
    /// see example in [`Ui::progress_bar_at`]
    pub fn hprogress_bar(percent: f32, width: f32, flip_fill_direction: bool) {
        unsafe { ui_hprogress_bar(percent, width, flip_fill_direction as Bool32T) }
    }

    /// This is a simple vertical progress indicator bar. This is used by the vslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/VProgressBar.html>
    /// * `percent` - A value between 0 and 1 indicating progress from 0% to 100%.
    /// * `width` - Physical width of the slider on the window. 0 will fill the remaining amount of window space.
    /// * `flip_fill_direction` - By default, this fills from top to bottom. This allows you to flip the fill direction to
    ///   bottom to top.
    ///
    /// see also [`ui_vprogress_bar`]
    /// see example in [`Ui::progress_bar_at`]
    pub fn vprogress_bar(percent: f32, height: f32, flip_fill_direction: bool) {
        unsafe { ui_vprogress_bar(percent, height, flip_fill_direction as Bool32T) }
    }

    /// This is a simple horizontal progress indicator bar. This is used by the hslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/ProgressBarAt.html>
    /// * `percent` - A value between 0 and 1 indicating progress from 0% to 100%.
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    ///
    /// see also [`ui_progress_bar_at`] [`Ui::vprogress_bar`] [`Ui::hprogress_bar`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiDir}, maths::{Vec2, Vec3, Pose}, font::Font, system::TextStyle,
    ///                      util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// filename_scr = "screenshots/ui_progress_bar_at.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Progress Bar", &mut window_pose, None, None, None);
    ///
    ///     Ui::vprogress_bar(0.20, 0.08, false);
    ///     Ui::progress_bar_at(0.55, [ 0.02,  -0.01, 0.0], [0.01, 0.08], UiDir::Vertical, false);
    ///     Ui::progress_bar_at(0.75, [-0.005, -0.01, 0.0], [0.01, 0.08], UiDir::Vertical, false);
    ///     Ui::progress_bar_at(0.95, [-0.03,  -0.01, 0.0], [0.01, 0.08], UiDir::Vertical, false);
    ///
    ///     Ui::hprogress_bar(0.25, 0.1, true);
    ///     Ui::progress_bar_at(0.75, [0.05, -0.13, 0.0], [0.1, 0.01], UiDir::Horizontal, true);
    ///
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_progress_bar_at.jpeg" alt="screenshot" width="200">
    pub fn progress_bar_at(
        percent: f32,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        bar_direction: UiDir,
        flip_fill_direction: bool,
    ) {
        unsafe {
            ui_progress_bar_at(
                percent,
                top_left_corner.into(),
                size.into(),
                bar_direction,
                flip_fill_direction as Bool32T,
            )
        }
    }

    /// All UI between push_enabled and its matching pop_enabled will set the UI to an enabled or disabled state,
    /// allowing or preventing interaction with specific elements. The default state is true.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushEnabled.html>
    /// * `enabled` - Should the following elements be enabled and interactable?
    /// * `parent_behavior` - Do we want to ignore or inherit the state of the current stack?
    ///   if None, has default value HierarchyParent::Inherit
    ///
    /// see also [`ui_push_enabled`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, system::HierarchyParent};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    /// let mut enabled_value = false;
    /// let mut toggle_value = false;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Push Enabled", &mut window_pose, None, None, None);
    ///     assert_eq!(Ui::get_enabled(), true);
    ///     Ui::toggle("Enabled", &mut enabled_value, None);
    ///     Ui::push_enabled(enabled_value, None);
    ///
    ///     Ui::push_enabled(true, None);
    ///     assert_eq!(Ui::get_enabled(), false);
    ///     Ui::hprogress_bar(0.20, 0.08, false);
    ///     Ui::pop_enabled();
    ///
    ///     let bt2 = Ui::button("Button", None);
    ///
    ///     Ui::push_enabled(true, Some(HierarchyParent::Ignore));
    ///     assert_eq!(Ui::get_enabled(), true);
    ///     Ui::toggle("I'm a robot!",&mut toggle_value, None);
    ///     Ui::pop_enabled();
    ///
    ///     Ui::pop_enabled();
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn push_enabled(enabled: bool, parent_behavior: Option<HierarchyParent>) {
        let parent_behavior = parent_behavior.unwrap_or(HierarchyParent::Inherit);
        unsafe { ui_push_enabled(enabled as Bool32T, parent_behavior) }
    }

    /// Adds a root id to the stack for the following UI elements! This id is combined when hashing any following ids,
    /// to prevent id collisions in separate groups.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushId.html>
    /// * `root_id` - The root id to use until the following PopId call. MUST be unique within current hierarchy.
    ///
    /// see also [`ui_push_id`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.92], Some([0.0, 185.0, 0.0].into()));
    /// let mut toggle_value1 = false;
    /// let mut toggle_value1b = true;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Push Id", &mut window_pose, None, None, None);
    ///     Ui::push_id("group1");
    ///     Ui::toggle("Choice 1",&mut toggle_value1, None);
    ///     Ui::pop_id();
    ///     Ui::push_id("group2");
    ///     Ui::toggle("Choice 1",&mut toggle_value1b, None);
    ///     Ui::pop_id();
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn push_id(root_id: impl AsRef<str>) {
        let cstr = CString::new(root_id.as_ref()).unwrap();
        unsafe { ui_push_id(cstr.as_ptr()) };
    }

    /// When a soft keyboard is visible, interacting with UI elements will cause the keyboard to close. This function
    /// allows you to change this behavior for certain UI elements, allowing the user to interact and still preserve the
    /// keyboard’s presence. Remember to Pop when you’re finished!
    /// <https://stereokit.net/Pages/StereoKit/UI/PushPreserveKeyboard.html>
    /// * `preserve_keyboard` - If true, interacting with elements will NOT hide the keyboard. If false, interaction
    ///   will hide the keyboard.
    ///
    /// see also [`ui_push_preserve_keyboard`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    /// let mut title = String::from("title");
    /// let mut author = String::from("author");
    /// let mut volume = 0.5;
    /// let mut mute = false;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Sound track", &mut window_pose, None, None, None);
    ///     Ui::push_preserve_keyboard(true);
    ///     Ui::input("Title", &mut title, Some([0.15, 0.03].into()), None);
    ///     Ui::input("Author", &mut author, Some([0.15, 0.03].into()), None);
    ///     Ui::hslider("volume", &mut volume, 0.0, 1.0, Some(0.05), None, None, None);
    ///     Ui::toggle("mute", &mut mute, None);
    ///     Ui::pop_preserve_keyboard();
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn push_preserve_keyboard(preserve_keyboard: bool) {
        unsafe { ui_push_preserve_keyboard(preserve_keyboard as Bool32T) }
    }

    /// This pushes an enabled status for grab auras onto the stack. Grab auras are an extra space and visual element
    /// that goes around Window elements to make them easier to grab. MUST be matched by a pop_grab_aura call.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushGrabAura.html>
    /// * `enabled` - Is the grab aura enabled or not?
    ///
    /// see also [`ui_push_grab_aura`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    /// let mut title = String::from("");
    ///
    /// assert_eq!(Ui::grab_aura_enabled(), true);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::push_grab_aura(false);
    ///     assert_eq!(Ui::grab_aura_enabled(), false);
    ///     Ui::window_begin("Write a title", &mut window_pose, None, None, None);
    ///     Ui::label("Title:", None, false);
    ///     Ui::input("Title", &mut title, Some([0.15, 0.03].into()), None);
    ///     Ui::window_end();
    ///     Ui::pop_grab_aura();
    ///     assert_eq!(Ui::grab_aura_enabled(), true);
    /// );
    /// ```
    pub fn push_grab_aura(enabled: bool) {
        unsafe { ui_push_grab_aura(enabled as Bool32T) }
    }

    /// This will push a surface into SK’s UI layout system. The surface becomes part of the transform hierarchy, and SK
    /// creates a layout surface for UI content to be placed on and interacted with. Must be accompanied by a
    /// pop_surface call.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushSurface.html>
    /// * `pose` - The Pose of the UI surface, where the surface forward direction is the same as the Pose’s.
    /// * `layout_start` - This is an offset from the center of the coordinate space created by the surfacePose.
    ///   Vec3.Zero would mean that content starts at the center of the surfacePose.
    /// * `layout_dimension` - The size of the surface area to use during layout. Like other UI layout sizes, an axis
    ///   set to zero means it will auto-expand in that direction.
    ///
    /// see also [`ui_push_surface`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiPad}, maths::{Vec2, Vec3, Pose}, util::named_colors,
    ///                      font::Font, system::TextStyle};
    /// let mut title = String::from("");
    /// let style = TextStyle::from_font(Font::default(), 0.05, named_colors::BLUE);
    ///
    /// let mut surface_pose = Pose::new(
    ///     [-0.09, 0.075, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// filename_scr = "screenshots/ui_push_surface.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::push_surface(surface_pose, [0.0, 0.0, 0.0], [0.1, 0.1]);
    ///     Ui::push_text_style(style);
    ///     Ui::label("Surface", Some([0.25, 0.03].into()), false);
    ///     Ui::pop_text_style();
    ///     Ui::panel_begin(Some(UiPad::Inside));
    ///     Ui::label("Give a title:", None, false);
    ///     Ui::input("Title", &mut title, Some([0.15, 0.03].into()), None);
    ///     Ui::panel_end();
    ///     Ui::pop_surface();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_push_surface.jpeg" alt="screenshot" width="200">
    pub fn push_surface(pose: impl Into<Pose>, layout_start: impl Into<Vec3>, layout_dimension: impl Into<Vec2>) {
        unsafe { ui_push_surface(pose.into(), layout_start.into(), layout_dimension.into()) }
    }

    /// This pushes a Text Style onto the style stack! All text elements rendered by the GUI system will now use this
    /// styling.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushTextStyle.html>
    ///
    /// see also [`ui_push_text_style`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, font::Font, system::TextStyle,
    ///                      util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let font = Font::default();
    /// let style_big    = TextStyle::from_font(&font, 0.015, named_colors::CYAN);
    /// let style_medium = TextStyle::from_font(&font, 0.012, named_colors::FUCHSIA);
    /// let style_small  = TextStyle::from_font(&font, 0.009, named_colors::GOLD);
    /// let style_mini   = TextStyle::from_font(&font, 0.006, named_colors::WHITE);
    ///
    /// filename_scr = "screenshots/ui_push_text_style.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Push TextStyle", &mut window_pose, Some([0.16, 0.0].into()), None, None);
    ///
    ///     Ui::push_text_style(style_big);
    ///     Ui::text("The first part", None, None, None, None, None, None);
    ///     assert_eq!(Ui::get_text_style(), style_big);
    ///     Ui::pop_text_style();
    ///
    ///     Ui::push_text_style(style_medium);
    ///     Ui::text("The second part", None, None, None, None, None, None);
    ///     Ui::pop_text_style();
    ///
    ///     Ui::push_text_style(style_small);
    ///     Ui::text("The third part", None, None, None, None, None, None);
    ///     Ui::push_text_style(style_mini);
    ///     Ui::text("The Inside part", None, None, None, None, None, None);
    ///     assert_eq!(Ui::get_text_style(), style_mini);
    ///     Ui::pop_text_style();
    ///     Ui::text("----////", None, None, None, None, None, None);
    ///     Ui::pop_text_style();
    ///
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_push_text_style.jpeg" alt="screenshot" width="200">
    pub fn push_text_style(style: TextStyle) {
        unsafe { ui_push_text_style(style) }
    }

    /// All UI between push_tint and its matching pop_tint will be tinted with this color. This is implemented by
    /// multiplying this color with the current color of the UI element. The default is a White (1,1,1,1) identity tint.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushTint.html>
    /// * `color_gamma` - A normal (gamma corrected) color value. This is internally converted to linear, so tint
    ///   multiplication happens on linear color values.
    ///
    /// see also [`ui_push_tint`] [`Ui::color_scheme`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}, util::Color128};
    /// let mut title = String::from("Push Tint");
    /// let mut volume = 0.5;
    /// let mut mute = true;
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.05, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let blue  = Color128::rgb(0.0, 0.0, 1.0);
    /// let red   = Color128::rgb(1.0, 0.0, 0.0);
    /// let green = Color128::rgb(0.0, 1.0, 0.0);
    ///
    /// filename_scr = "screenshots/ui_push_tint.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Push Tint", &mut window_pose, None, None, None);
    ///     Ui::push_tint(blue.to_gamma());
    ///     Ui::input("Title", &mut title, Some([0.15, 0.03].into()), None);
    ///     Ui::pop_tint();
    ///     Ui::push_tint(red.to_gamma());
    ///     Ui::hslider("volume", &mut volume, 0.0, 1.0, Some(0.05), None, None, None);
    ///     Ui::pop_tint();
    ///     Ui::push_tint(green.to_gamma());
    ///     Ui::toggle("mute", &mut mute, None);
    ///     Ui::pop_tint();
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_push_tint.jpeg" alt="screenshot" width="200">
    pub fn push_tint(color_gamma: impl Into<Color128>) {
        unsafe { ui_push_tint(color_gamma.into()) }
    }

    /// This will reposition the Mesh’s vertices to work well with quadrant resizing shaders. The mesh should generally
    /// be centered around the origin, and face down the -Z axis. This will also overwrite any UV coordinates in the
    /// verts.
    ///
    /// You can read more about the technique here : <https://playdeck.net/blog/quadrant-sizing-efficient-ui-rendering>
    /// <https://stereokit.net/Pages/StereoKit/UI/QuadrantSizeMesh.html>
    /// * `mesh` - The vertices of this Mesh will be retrieved, modified, and overwritten.
    /// * `overflow_percent` - When scaled, should the geometry stick out past the “box” represented by the scale, or
    ///   edge up against it? A value of 0 will mean the geometry will fit entirely inside the “box”, and a value of 1
    ///   means the geometry will start at the boundary of the box and continue outside it.
    ///
    /// see also [`ui_quadrant_size_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiCorner, UiVisual, UiLathePt}, maths::{Vec2, Vec3, Pose, Matrix},
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.025, 0.948], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let material = Material::pbr();
    /// let transform1 = Matrix::t_r_s([-0.1, 0.0, 0.74], [0.0, 130.0, 0.0], [3.0, 1.0, 0.05]);
    ///
    /// let mut mesh = Mesh::generate_cube([1.0, 1.0, 1.0], None);
    /// Ui::quadrant_size_mesh(&mut mesh, 0.20);
    ///
    /// let bounds = mesh.get_bounds();
    /// assert_eq!(bounds.center, Vec3 { x: 0.0, y: 0.0, z: 0.0 });
    /// //TODO:
    /// assert_eq!(bounds.dimensions, Vec3 { x: 0.0, y: 0.0, z: 1.0 });
    ///
    /// Ui::set_element_visual(UiVisual::Separator, mesh, None, None);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Push Tint", &mut window_pose, None, None, None);
    ///     Ui::hseparator();
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::hseparator();
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn quadrant_size_mesh(mesh: impl AsRef<Mesh>, overflow_percent: f32) {
        unsafe { ui_quadrant_size_mesh(mesh.as_ref().0.as_ptr(), overflow_percent) }
    }

    /// This will reposition the vertices to work well with quadrant resizing shaders. The mesh should generally be
    /// centered around the origin, and face down the -Z axis. This will also overwrite any UV coordinates in the verts.
    ///
    /// You can read more about the technique here : <https://playdeck.net/blog/quadrant-sizing-efficient-ui-rendering>
    /// <https://stereokit.net/Pages/StereoKit/UI/QuadrantSizeVerts.html>
    /// * `verts` - A list of vertices to be modified to fit the sizing shader.
    /// * `overflow_percent` - When scaled, should the geometry stick out past the “box” represented by the scale, or
    ///   edge up against it? A value of 0 will mean the geometry will fit entirely inside the “box”, and a value of 1
    ///   means the geometry will start at the boundary of the box and continue outside it.
    ///
    /// see also [`ui_quadrant_size_verts`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiCorner, UiVisual, UiLathePt},
    ///                      maths::{Vec2, Vec3, Pose, Matrix},
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.025, 0.948], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let material = Material::pbr();
    /// let transform1 = Matrix::t_r_s([-0.1, 0.0, 0.74], [0.0, 130.0, 0.0], [3.0, 1.0, 0.05]);
    ///
    /// let mut mesh = Mesh::generate_cube([1.0, 1.0, 1.0], None);
    /// let mut verts = mesh.get_verts();
    /// Ui::quadrant_size_verts(&verts, 0.0);
    /// let mut remesh = mesh.clone_ref();
    /// remesh.set_verts(&verts, true);
    ///
    /// let bounds = mesh.get_bounds();
    /// assert_eq!(bounds.center, Vec3 { x: 0.0, y: 0.0, z: 0.0 });
    /// //TODO:
    /// assert_eq!(bounds.dimensions, Vec3 { x: 0.0, y: 0.0, z: 1.0 });
    ///
    /// Ui::set_element_visual(UiVisual::Separator, mesh, None, None);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Push Tint", &mut window_pose, None, None, None);
    ///     Ui::hseparator();
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::hseparator();
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn quadrant_size_verts(verts: &[Vertex], overflow_percent: f32) {
        unsafe { ui_quadrant_size_verts(verts.as_ptr() as *mut Vertex, verts.len() as i32, overflow_percent) }
    }

    /// This generates a quadrantified mesh meant for UI buttons by sweeping a lathe over the rounded corners of a
    /// rectangle! Note that this mesh is quadrantified, so it requires special shaders to draw properly!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    /// * `rounded_corners` - A bit-flag indicating which corners should be rounded, and which should be sharp!
    /// * `corner_radius` - The radius of each rounded corner.
    /// * `corner_resolution` - How many slices/verts go into each corner? More is smoother, but more expensive to render.
    /// * `delete_flat_sides` - If two adjacent corners are sharp, should we skip connecting them with triangles? If this
    ///   edge will always be covered, then deleting these faces may save you some performance.
    /// * `quadrantify` - Does this generate a mesh compatible with StereoKit's quadrant shader system, or is this just a
    ///   traditional mesh? In most cases, this should be true, but UI elements such as the rounded button may be
    ///   exceptions.
    /// * `lathe_pts`" - The lathe points to sweep around the edge.
    ///
    /// Returns the final Mesh, ready for use in SK's theming system.
    /// see also [`ui_gen_quadrant_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiCorner, UiVisual, UiLathePt}, maths::{Vec2, Vec3, Pose, Matrix},
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.028, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let material = Material::pbr();
    /// let mut mesh_button = Ui::gen_quadrant_mesh(
    ///     UiCorner::All, 0.002, 8, false, true, &UiLathePt::button())
    ///                        .expect("mesh should be created");
    /// let mut mesh_input = Ui::gen_quadrant_mesh(
    ///     UiCorner::All, 0.018, 8, false, true, &UiLathePt::input())
    ///                        .expect("mesh should be created");
    ///
    /// let bounds = mesh_button.get_bounds();
    /// assert_eq!(bounds.center, Vec3 { x: 0.0, y: 0.0, z: -0.005 });
    /// assert_eq!(bounds.dimensions, Vec3 { x: 0.004, y: 0.004, z: 0.99 });
    ///
    /// Ui::set_element_visual(UiVisual::Button, mesh_button, None, None);
    /// Ui::set_element_visual(UiVisual::Input, mesh_input, None, None);
    ///
    /// let mut text = String::from("Text");
    ///
    /// filename_scr = "screenshots/ui_gen_quadrant_mesh.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("gen_quadrant_mesh", &mut window_pose, None, None, None);
    ///     Ui::input("input", &mut text, None, None );
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_gen_quadrant_mesh.jpeg" alt="screenshot" width="200">
    pub fn gen_quadrant_mesh(
        rounded_corners: UiCorner,
        corner_radius: f32,
        corner_resolution: u32,
        delete_flat_sides: bool,
        quadrantify: bool,
        lathe_pts: &[UiLathePt],
    ) -> Result<Mesh, StereoKitError> {
        match NonNull::new(unsafe {
            ui_gen_quadrant_mesh(
                rounded_corners,
                corner_radius,
                corner_resolution,
                delete_flat_sides as Bool32T,
                quadrantify as Bool32T,
                lathe_pts.as_ptr(),
                lathe_pts.len() as i32,
            )
        }) {
            Some(mesh_t) => Ok(Mesh(mesh_t)),
            None => Err(StereoKitError::MeshGen("gen_quadrant_mesh failed !".to_owned())),
        }
    }

    /// A Radio is similar to a button, except you can specify if it looks pressed or not regardless of interaction.
    /// This can be useful for radio-like behavior! Check an enum for a value, and use that as the ‘active’ state, Then
    /// switch to that enum value if Radio returns true.
    /// <https://stereokit.net/Pages/StereoKit/UI/Radio.html>
    /// * `text` - Text to display on the Radio and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `active` - Does this button look like it’s pressed?
    /// * `size` - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is
    ///   [`Ui::get_line_height`].
    ///
    /// Returns true only on the first frame it is pressed.
    /// see also [`ui_toggle_img`] [`ui_toggle_img_sz`]
    #[deprecated(since = "0.0.1", note = "Performence issues, use radio_img instead")]
    pub fn radio(text: impl AsRef<str>, active: bool, size: Option<Vec2>) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();
        let mut active: Bool32T = active as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        match size {
            Some(size) => unsafe {
                ui_toggle_img_sz(
                    cstr.as_ptr(),
                    active_ptr,
                    Sprite::radio_off().0.as_ptr(),
                    Sprite::radio_on().0.as_ptr(),
                    UiBtnLayout::Left,
                    size,
                ) != 0
            },
            None => unsafe {
                ui_toggle_img(
                    cstr.as_ptr(),
                    active_ptr,
                    Sprite::radio_off().0.as_ptr(),
                    Sprite::radio_on().0.as_ptr(),
                    UiBtnLayout::Left,
                ) != 0
            },
        }
    }

    /// A Radio is similar to a button, except you can specify if it looks pressed or not regardless of interaction.
    /// This can be useful for radio-like behavior! Check an enum for a value, and use that as the ‘active’ state, Then
    /// switch to that enum value if Radio returns true.
    /// This version allows you to override the images used by the Radio.
    /// <https://stereokit.net/Pages/StereoKit/UI/Radio.html>
    /// * `text` - Text to display on the Radio and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `active` - Does this button look like it’s pressed?
    /// * `image_off` - Image to use when the radio value is false.
    /// * `image_on` - Image to use when the radio value is true.
    /// * `image_layout` - This enum specifies how the text and image should be laid out on the radio. For example,
    ///   UiBtnLayout::Left will have the image on the left, and text on the right.
    /// * `size` - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is
    ///   [`Ui::get_line_height`].
    ///
    /// Returns true only on the first frame it is pressed.
    /// see also [`ui_toggle_img`] [`ui_toggle_img_sz`] [Ui::radio_at]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiBtnLayout}, maths::{Vec2, Vec3, Pose}, sprite::Sprite};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.035, 0.91], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let (on, off) = (Sprite::radio_on(), Sprite::radio_off());
    ///
    /// let mut choice = "A";
    ///
    /// filename_scr = "screenshots/ui_radio.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Radio", &mut window_pose, None, None, None);
    ///     if Ui::radio_img("A", choice == "A", &off, &on, UiBtnLayout::Right,
    ///                      Some([0.06, 0.05].into())) {
    ///         choice = "A";
    ///     }
    ///     Ui::same_line();
    ///     if Ui::radio_img("B", choice == "B", &off, &on, UiBtnLayout::Center,
    ///                      Some([0.03, 0.05].into())){
    ///         choice = "B";
    ///     }
    ///     Ui::same_line();
    ///     if Ui::radio_img("C", choice == "C", &off, &on, UiBtnLayout::Left, None) {
    ///         choice = "C";
    ///     }
    ///     if Ui::radio_at("D", choice == "D", &off, &on, UiBtnLayout::Right,
    ///                     [0.06, -0.07, 0.0], [0.06, 0.03]) {
    ///         choice = "D";
    ///     }    
    ///     if Ui::radio_at("E", choice == "E", &off, &on, UiBtnLayout::Left,
    ///                     [-0.01, -0.07, 0.0], [0.06, 0.03]) {
    ///         choice = "E";
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_radio.jpeg" alt="screenshot" width="200">
    pub fn radio_img(
        text: impl AsRef<str>,
        active: bool,
        image_off: impl AsRef<Sprite>,
        image_on: impl AsRef<Sprite>,
        image_layout: UiBtnLayout,
        size: Option<Vec2>,
    ) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();
        let mut active: Bool32T = active as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        match size {
            Some(size) => unsafe {
                ui_toggle_img_sz(
                    cstr.as_ptr(),
                    active_ptr,
                    image_off.as_ref().0.as_ptr(),
                    image_on.as_ref().0.as_ptr(),
                    image_layout,
                    size,
                ) != 0
            },
            None => unsafe {
                ui_toggle_img(
                    cstr.as_ptr(),
                    active_ptr,
                    image_off.as_ref().0.as_ptr(),
                    image_on.as_ref().0.as_ptr(),
                    image_layout,
                ) != 0
            },
        }
    }

    /// A Radio is similar to a button, except you can specify if it looks pressed or not regardless of interaction.
    /// This can be useful for radio-like behavior! Check an enum for a value, and use that as the ‘active’ state, Then
    /// switch to that enum value if Radio returns true. This version allows you to override the images used by
    /// the Radio.
    /// <https://stereokit.net/Pages/StereoKit/UI/RadioAt.html>
    /// * `text` - Text to display on the Radio and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `active` - Does this button look like it’s pressed?
    /// * `image_off` - Image to use when the radio value is false.
    /// * `image_on` - Image to use when the radio value is true.
    /// * `image_layout` - This enum specifies how the text and image should be laid out on the radio. For example,
    ///   UiBtnLayout::Left will have the image on the left, and text on the right.
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * size - The layout size for this element in Hierarchy space.
    ///
    /// Returns true only on the first frame it is pressed.
    /// see also [`ui_toggle_img_at`]
    /// see example in [`Ui::radio_img`]
    pub fn radio_at(
        text: impl AsRef<str>,
        active: bool,
        image_off: impl AsRef<Sprite>,
        image_on: impl AsRef<Sprite>,
        image_layout: UiBtnLayout,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();
        let mut active: Bool32T = active as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        unsafe {
            ui_toggle_img_at(
                cstr.as_ptr(),
                active_ptr,
                image_off.as_ref().0.as_ptr(),
                image_on.as_ref().0.as_ptr(),
                image_layout,
                top_left_corner.into(),
                size.into(),
            ) != 0
        }
    }

    /// Moves the current layout position back to the end of the line that just finished, so it can continue on the same
    /// line as the last element!
    /// <https://stereokit.net/Pages/StereoKit/UI/SameLine.html>
    ///
    /// see also [`ui_sameline`]
    pub fn same_line() {
        unsafe { ui_sameline() }
    }

    /// Override the visual assets attached to a particular UI element.
    /// Note that StereoKit’s default UI assets use a type of quadrant sizing that is implemented in the Material and
    /// the Mesh. You don’t need to use quadrant sizing for your own visuals, but if you wish to know more, you can read
    /// more about the technique here : <https://playdeck.net/blog/quadrant-sizing-efficient-ui-rendering>
    /// You may also find Ui::quadrant_size_verts and Ui::quadrant_size_mesh to be helpful.
    /// <https://stereokit.net/Pages/StereoKit/UI/SetElementVisual.html>
    /// * `visual` - Which UI visual element to override. Use UiVisual::ExtraSlotXX if you need extra
    ///   UIVisual slots for your own custom UI elements.
    /// * `mesh` - The Mesh to use for the UI element's visual component. The Mesh will be scaled to match the dimensions
    ///   of the UI element.
    /// * `material` - The Material to use when rendering the UI element. None is for the default Material specifically
    ///   designed to work with quadrant sizing formatted meshes.
    /// * `min_size` - For some meshes, such as quadrant sized meshes, there's a minimum size where the mesh turns inside
    ///   out. This lets UI elements to accommodate for this minimum size, and behave somewhat more appropriately. None
    ///   is Vec2::ZERO.
    ///
    /// see also [`ui_set_element_visual`]
    /// see example in [`Ui::gen_quadrant_mesh`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiColor, UiVisual, UiLathePt, UiCorner}, maths::{Vec2, Vec3, Pose},
    ///                      mesh::Mesh, material::Material, util::named_colors};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.025, 0.92], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let material = Material::pbr();
    /// let mut mesh = Ui::gen_quadrant_mesh(
    ///     UiCorner::All, 0.005, 8, false, true, &UiLathePt::plane())
    ///                        .expect("mesh should be created");
    ///
    /// Ui::set_element_visual(UiVisual::Separator, mesh, None, None);
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("set element visual", &mut window_pose, None, None, None);
    ///     Ui::hseparator();
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::hseparator();
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn set_element_visual(
        visual: UiVisual,
        mesh: impl AsRef<Mesh>,
        material: Option<Material>,
        min_size: Option<Vec2>,
    ) {
        let material = match material {
            Some(mat) => mat.0.as_ptr(),
            None => null_mut(),
        };
        let min_size = min_size.unwrap_or_default();
        unsafe { ui_set_element_visual(visual, mesh.as_ref().0.as_ptr(), material, min_size) };
    }

    /// This allows you to override the color category that a UI element is assigned to.
    /// <https://stereokit.net/Pages/StereoKit/UI/SetElementColor.html>
    /// * `visual` - The UI element type to set the color category of.
    /// * `color_category` - The category of color to assign to this UI element. Use Ui::set_theme_color in combination
    ///   with this to assign a specific color. Use UiColor::ExtraSlotXX if you need extra UIColor slots
    ///   for your own custom UI elements.
    ///
    /// see also [`ui_set_element_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiColor, UiVisual}, maths::{Vec2, Vec3, Pose},
    ///                      util::{named_colors, Color128}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.025, 0.93], Some([0.0, 185.0, 0.0].into()));
    ///
    /// assert_eq!(Ui::get_element_color(UiVisual::Separator, 1.0),
    ///            Color128 { r: 1.0620984, g: 0.49995762, b: 0.2311526, a: 1.0 });
    ///
    /// Ui::set_element_color(UiVisual::Separator, UiColor::Complement);
    /// assert_eq!(Ui::get_element_color(UiVisual::Separator, 1.0),
    ///            Color128 { r: 0.10546647, g: 0.092475444, b: 0.08364652, a: 1.0 });
    ///
    /// assert_eq!(Ui::get_element_color(UiVisual::Button, 0.0),
    ///            Color128 { r: 0.2058468, g: 0.1961254, b: 0.18924558, a: 0.0 });
    /// Ui::set_element_color(UiVisual::Button, UiColor::Background);
    /// assert_eq!(Ui::get_element_color(UiVisual::Button, 0.0),
    ///            Color128 { r: 0.091664724, g: 0.08037374, b: 0.072700225, a: 0.0 });
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("set_element_color", &mut window_pose, None, None, None);
    ///     Ui::hseparator();
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::hseparator();
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn set_element_color(visual: UiVisual, color_category: UiColor) {
        unsafe { ui_set_element_color(visual, color_category) };
    }

    /// This sets the sound that a particulat UI element will make when you interact with it. One sound when the
    /// interaction starts, and one when it ends.
    /// <https://stereokit.net/Pages/StereoKit/UI/SetElementSound.html>
    /// * `visual` - The UI element to apply the sounds to. Use UiVisual::ExtraSlotXX if you need extra
    ///   UIVisual slots
    /// * `activate` - The sound made when the interaction begins. None will fall back to the default sound.
    /// * `deactivate` - The sound made when the interaction ends. None will fall back to the default sound.
    ///
    /// see also [`ui_set_element_sound`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiVisual}, maths::{Vec2, Vec3, Pose}, sound::Sound};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let sound_activate = Sound::click();
    /// let sound_deactivate = Sound::unclick();
    ///
    /// Ui::set_element_sound(UiVisual::Button, Some(sound_activate), Some(sound_deactivate));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Set Element Sound", &mut window_pose, None, None, None);
    ///     if Ui::button("Button1", None) {todo!();}
    ///     if Ui::button("Button2", None) {todo!();}
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn set_element_sound(visual: UiVisual, activate: Option<Sound>, deactivate: Option<Sound>) {
        let activate = match activate {
            Some(sound) => sound.0.as_ptr(),
            None => null_mut(),
        };
        let deactivate = match deactivate {
            Some(sound) => sound.0.as_ptr(),
            None => null_mut(),
        };
        unsafe { ui_set_element_sound(visual, activate, deactivate) };
    }

    /// This will draw a visual element from StereoKit's theming system, while paying attention to certain factors
    /// such as enabled/disabled, tinting and more.
    /// <https://stereokit.net/Pages/StereoKit/UI/DrawElement.html>
    /// * `element_visual` - The element type to draw. Use UiVisual::ExtraSlotXX to use extra UiVisual
    ///   slots for your own custom UI elements. If these slots are empty, SK will fall back to UiVisual::Default
    /// * `element_color` - If you wish to use the coloring from a different element, you can use this to override the
    ///   theme color used when drawing. Use UiVisual::ExtraSlotXX to use extra UiVisual slots for your
    ///   own custom UI elements. If these slots are empty, SK will fall back to UiVisual::Default.
    /// * `start` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    /// * `focus` - The amount of visual focus this element currently has, where 0 is unfocused, and 1 is active. You
    ///   can acquire a good focus value from `Ui::get_anim_focus`.
    ///
    /// see also [`ui_draw_element`] [`ui_draw_element_color`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiVisual}, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    ///
    /// filename_scr = "screenshots/ui_draw_element.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Draw Element", &mut window_pose, Some([0.22, 0.18].into()), None, None);
    ///     Ui::draw_element(UiVisual::Button, None, [0.1, -0.01, 0.0], [0.1, 0.025, 0.005], 1.0);
    ///     Ui::draw_element(UiVisual::Input, None, [0.0, -0.01, 0.0], [0.1, 0.025, 0.005], 1.0);
    ///     Ui::draw_element(UiVisual::Handle, None, [0.1, -0.05, 0.0], [0.1, 0.025, 0.005], 1.0);
    ///     Ui::draw_element(UiVisual::Toggle, None, [0.0, -0.05, 0.0], [0.1, 0.025, 0.005], 1.0);
    ///     Ui::draw_element(UiVisual::Separator, None, [0.1, -0.08, 0.0], [0.2, 0.005, 0.005], 1.0);
    ///     Ui::draw_element(UiVisual::Aura, None, [0.1, -0.1, 0.0], [0.08, 0.08, 0.005], 0.5);
    ///     Ui::draw_element(UiVisual::Default, None, [0.0, -0.1, 0.0], [0.1, 0.025, 0.005], 0.0);
    ///     Ui::draw_element(UiVisual::Carat, None, [0.0, -0.14, 0.0], [0.025, 0.025, 0.005], 1.0);
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_draw_element.jpeg" alt="screenshot" width="200">
    pub fn draw_element(
        element_visual: UiVisual,
        element_color: Option<UiVisual>,
        start: impl Into<Vec3>,
        size: impl Into<Vec3>,
        focus: f32,
    ) {
        match element_color {
            Some(element_color) => unsafe {
                ui_draw_element_color(element_visual, element_color, start.into(), size.into(), focus)
            },
            None => unsafe { ui_draw_element(element_visual, start.into(), size.into(), focus) },
        }
    }

    /// This will get a final linear draw color for a particular UI element type with a particular focus value. This
    /// obeys the current hierarchy of tinting and enabled states.
    /// <https://stereokit.net/Pages/StereoKit/UI/GetElementColor.html>
    /// * `element_visual` - Get the color from this element type.  Use UiVisual::ExtraSlotXX to use extra
    ///   UiVisual slots for your own custom UI elements. If these slots are empty, SK will fall back to
    ///   UiVisual::Default.
    /// * `focus` - The amount of visual focus this element currently has, where 0 is unfocused, and 1 is active. You
    ///   can acquire a good focus value from `Ui::get_anim_focus`
    ///
    /// Returns a linear color good for tinting UI meshes.
    /// see also [`ui_get_element_color`]
    /// see example in [`Ui::set_element_color`]
    pub fn get_element_color(element_visual: UiVisual, focus: f32) -> Color128 {
        unsafe { ui_get_element_color(element_visual, focus) }
    }

    /// This resolves a UI element with an ID and its current states into a nicely animated focus value.
    /// <https://stereokit.net/Pages/StereoKit/UI/GetAnimFocus.html>
    /// * `id` - The hierarchical id of the UI element we're checking the focus of, this can be created with
    ///   `Ui::stack_hash`.
    /// * `focus_state` - The current focus state of the UI element.
    /// * `activationState` - The current activation status of the/ UI element.
    ///
    /// Returns a focus value in the realm of 0-1, where 0 is unfocused, and 1 is active.
    /// see also [`ui_get_anim_focus`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, system::BtnState, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Get Anim Focus", &mut window_pose, None, None, None);
    ///     if Ui::button("button1", None) {todo!()}
    ///     let id = Ui::stack_hash("button1");
    ///     let focus = Ui::get_anim_focus(id, BtnState::Inactive, BtnState::Inactive);
    ///     assert_eq!(focus, 0.0);
    ///     let focus = Ui::get_anim_focus(id, BtnState::Active, BtnState::Inactive);
    ///     assert_eq!(focus, 0.5);
    ///     let focus = Ui::get_anim_focus(id, BtnState::Active, BtnState::Active);
    ///     assert_eq!(focus, 1.0);
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn get_anim_focus(id: IdHashT, focus_state: BtnState, activation_state: BtnState) -> f32 {
        unsafe { ui_get_anim_focus(id, focus_state, activation_state) }
    }

    /// This allows you to explicitly set a theme color, for finer grained control over the UI appearance. Each theme
    /// type is still used by many different UI elements. This will automatically generate colors for different UI
    /// element states.
    /// <https://stereokit.net/Pages/StereoKit/UI/SetThemeColor.html>
    /// * `color_category` - The category of UI elements that are affected by this theme color. Use UiColor::ExtraSlotXX
    ///   if you need extra UiColor slots for your own custom UI elements.
    /// * `color_state` - The state of the UI element this color should apply to. If None has the value
    ///   UiColorState::Normal
    /// * `color_gama` : the gamma corrected color that should be applied to this theme color category in its normal
    ///   resting state. Active and disabled colors will be generated based on this color.
    ///
    /// see also [`ui_set_theme_color`] [`ui_set_theme_color_state`] [`Ui::color_scheme`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiColor, UiColorState}, maths::{Vec2, Vec3, Pose},
    ///                      util::{named_colors, Color128}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.04, 0.93], Some([0.0, 185.0, 0.0].into()));
    ///
    /// assert_eq!(Ui::get_theme_color(UiColor::Primary, None),
    ///            Color128 { r: 0.75, g: 0.5325, b: 0.375, a: 1.0 });
    ///
    /// let red: Color128 = named_colors::RED.into();
    /// Ui::set_theme_color(UiColor::Common, Some(UiColorState::Disabled), red.to_gamma());
    /// assert_eq!(Ui::get_theme_color(UiColor::Common,  Some(UiColorState::Disabled)),
    ///             red.to_gamma());
    ///
    /// let green: Color128 = named_colors::GREEN.into();
    /// Ui::set_theme_color(UiColor::Primary, None, green.to_gamma());
    /// assert_eq!(Ui::get_theme_color(UiColor::Primary, None),
    ///            green.to_gamma());
    ///
    /// let blue: Color128 = named_colors::BLUE.into();
    /// Ui::set_theme_color(UiColor::Background, None, blue.to_gamma());
    /// assert_eq!(Ui::get_theme_color(UiColor::Background, None),
    ///            blue.to_gamma());
    ///
    /// filename_scr = "screenshots/ui_set_theme_color.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("set_theme_color", &mut window_pose, None, None, None);
    ///     Ui::push_enabled(false, None);
    ///     if Ui::button("Button", None) { todo!() };
    ///     Ui::pop_enabled();
    ///     Ui::hseparator();
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_set_theme_color.jpeg" alt="screenshot" width="200">
    pub fn set_theme_color(
        color_category: UiColor,
        color_state: Option<UiColorState>,
        color_gamma: impl Into<Color128>,
    ) {
        Log::diag(format!("set_theme_color for category: {:?}", unsafe {
            std::mem::transmute::<UiColor, u32>(color_category)
        }));
        match color_state {
            Some(color_state) => unsafe { ui_set_theme_color_state(color_category, color_state, color_gamma.into()) },
            None => unsafe { ui_set_theme_color(color_category, color_gamma.into()) },
        }
    }

    /// This allows you to inspect the current color of the theme color category in a specific state! If you set the
    /// color with Ui::color_scheme, or without specifying a state, this may be a generated color, and not necessarily
    /// the color that was provided there.
    /// <https://stereokit.net/Pages/StereoKit/UI/GetThemeColor.html>
    /// * `color_category` - The category of UI elements that are affected by this theme color. Use UiColor::ExtraSlotXX
    ///   if you need extra UiColor slots for your own custom UI elements.
    ///   If the theme slot is empty, the color will be pulled from UiColor::None
    /// * `color_state` : The state of the UI element this color applies to. If None has the value UiColorState::Normal
    ///
    /// Returns the gamma space color for the theme color category in the indicated state.
    /// see also [`ui_get_theme_color`] [`ui_get_theme_color_state`]
    pub fn get_theme_color(color_category: UiColor, color_state: Option<UiColorState>) -> Color128 {
        match color_state {
            Some(color_state) => unsafe { ui_get_theme_color_state(color_category, color_state) },
            None => unsafe { ui_get_theme_color(color_category) },
        }
    }

    /// adds some vertical space to the current line! All UI following elements on this line will be offset.
    /// <https://stereokit.net/Pages/StereoKit/UI/VSpace.html>
    /// * `space` - Space in meters to shift the layout by.
    ///
    /// see also [`ui_vspace`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.88], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("VSpace", &mut window_pose, None, None, None);
    ///     Ui::label("Line 1", None, false);
    ///     Ui::vspace(0.02);
    ///     Ui::label("Line 2", None, false);
    ///     Ui::vspace(0.04);
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn vspace(space: f32) {
        unsafe { ui_vspace(space) }
    }

    /// adds some horizontal space to the current line!
    /// <https://stereokit.net/Pages/StereoKit/UI/HSpace.html>
    /// * `space` - Space in meters to shift the layout by.
    ///
    /// see also [`ui_hspace`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.88], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("HSpace", &mut window_pose, None, None, None);
    ///     Ui::label("Bla bla ...", None, false);
    ///     Ui::same_line();
    ///     Ui::hspace(0.08);
    ///     if Ui::button("Exit", None) {sk.quit(None);}
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn hspace(space: f32) {
        unsafe { ui_hspace(space) }
    }

    /// This will hash the given text based id into a hash for use with certain StereoKit UI functions. This includes
    /// the hash of the current id stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/StackHash.html>
    /// * `id` - Text to hash along with the current id stack.
    ///
    /// Returns an integer based hash id for use with SK UI.
    /// see also [`ui_stack_hash`]
    /// ### Examples
    /// ```
    /// use stereokit_rust::ui::Ui;
    ///
    /// let hash1 = Ui::stack_hash("button1");
    /// let hash2 = Ui::stack_hash("input2");
    ///
    /// assert_eq!(hash1, 17108023170974569920);
    /// assert_eq!(hash2, 5305247587935581291);
    /// ```
    pub fn stack_hash(id: impl AsRef<str>) -> IdHashT {
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { ui_stack_hash(cstr.as_ptr()) }
    }

    /// A scrolling text element! This is for reading large chunks of text that may be too long to fit in the available
    /// space when scroll is Some(size). It requires a height, as well as a place to store the current scroll value.
    /// Text uses the UI's current font settings, which can be changed with UI.Push/PopTextStyle.
    /// <https://stereokit.net/Pages/StereoKit/UI/Text.html>
    /// * `text` - The text you wish to display, there's no additional parsing done to this text, so put it in as you want
    ///   to see it!
    /// * `scroll` - This is the current scroll value of the text, in meters, _not_ percent.
    /// * `scrollDirection` - What scroll bars are allowed to show on this text? Vertical, horizontal, both? None is
    ///   UiScroll::None.
    /// * `height` - The vertical height of this Text element. None is 0.0.
    /// * `width` - if None it will automatically take the remainder of the current layout.
    /// * `text_align` - Where should the text position itself within its bounds? None is Align::TopLeft is how most
    ///   european language are aligned.
    /// * `fit` - Describe how the text should behave when one of its size dimensions conflicts with the provided ‘size’
    ///   parameter. None will use TextFit::Wrap by default and this scrolling overload will always add `TextFit.Clip`
    ///   internally.
    ///
    /// Returns true if any of the scroll bars have changed this frame.
    /// see also [`ui_text`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiScroll}, system::{Align, TextFit}, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut scroll_value = Vec2::new(0.05, 0.0);
    /// let mut scroll_value_at = Vec2::new(0.00, 0.165);
    /// let text = r#"Lorem ipsum dolor sit amet, consectetur
    /// adipiscing elit, sed do eiusmod tempor incididunt ut
    /// labore et dolore magna aliqua. Ut enim ad minim veniam,
    /// quis nostrud exercitation ullamco laboris nisi ut ... "#;
    ///
    /// filename_scr = "screenshots/ui_text.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Text", &mut window_pose, Some([0.22, 0.14].into()), None, None);
    ///     Ui::text(text, Some(&mut scroll_value), Some(UiScroll::Both), Some(0.07), Some(0.21),
    ///              Some(Align::TopCenter), Some(TextFit::Clip));
    ///     Ui::text(text, None, None, Some(0.04), Some(1.8),
    ///              None, Some(TextFit::Exact));
    ///     Ui::text_at(text, Some(&mut scroll_value_at), Some(UiScroll::Both), Align::TopRight,
    ///                 TextFit::Wrap, [0.10, -0.14, 0.0], [0.21, 0.04]);
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_text.jpeg" alt="screenshot" width="200">
    pub fn text(
        text: impl AsRef<str>,
        scroll: Option<&mut Vec2>,
        scroll_direction: Option<UiScroll>,
        height: Option<f32>,
        width: Option<f32>,
        text_align: Option<Align>,
        fit: Option<TextFit>,
    ) -> bool {
        let cstr = CString::new(text.as_ref()).unwrap();
        let scroll_direction = scroll_direction.unwrap_or(UiScroll::None);
        let height = height.unwrap_or(0.0);
        let text_align = text_align.unwrap_or(Align::TopLeft);
        let fit = fit.unwrap_or(TextFit::Wrap);
        if let Some(width) = width {
            let size = Vec2::new(width, height);
            match scroll {
                Some(scroll) => unsafe {
                    ui_text_sz(cstr.as_ptr(), scroll, scroll_direction, size, text_align, fit) != 0
                },
                None => unsafe { ui_text_sz(cstr.as_ptr(), null_mut(), UiScroll::None, size, text_align, fit) != 0 },
            }
        } else {
            match scroll {
                Some(scroll) => unsafe { ui_text(cstr.as_ptr(), scroll, scroll_direction, height, text_align) != 0 },
                None => unsafe { ui_text(cstr.as_ptr(), null_mut(), UiScroll::None, 0.0, text_align) != 0 },
            }
        }
    }

    /// Displays a large chunk of text on the current layout. This can include new lines and spaces, and will properly
    /// wrap once it fills the entire layout! Text uses the UI’s current font settings, which can be changed with
    /// Ui::push/pop_text_style.
    /// <https://stereokit.net/Pages/StereoKit/UI/TextAt.html>
    /// * `text` - The text you wish to display, there's no additional parsing done to this text, so put it in as you want
    ///   to see it!
    /// * `scroll` - This is the current scroll value of the text, in meters, _not_ percent.
    /// * `scrollDirection` - What scroll bars are allowed to show on this text? Vertical, horizontal, both?
    /// * `text_align` - Where should the text position itself within its bounds?
    /// * `fit` - Describe how the text should behave when one of its size dimensions conflicts with the provided ‘size’
    ///   parameter.
    /// * `size` - The layout size for this element in Hierarchy space.
    ///
    /// Returns true if any of the scroll bars have changed this frame.
    /// see also [`ui_text_at`]
    /// see example in [`Ui::text`]
    pub fn text_at(
        text: impl AsRef<str>,
        scroll: Option<&mut Vec2>,
        scroll_direction: Option<UiScroll>,
        text_align: Align,
        fit: TextFit,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) -> bool {
        let scroll_direction = scroll_direction.unwrap_or(UiScroll::None);
        let cstr = CString::new(text.as_ref()).unwrap();
        match scroll {
            Some(scroll) => unsafe {
                ui_text_at(
                    cstr.as_ptr(),
                    scroll,
                    scroll_direction,
                    text_align,
                    fit,
                    top_left_corner.into(),
                    size.into(),
                ) != 0
            },
            None => unsafe {
                ui_text_at(
                    cstr.as_ptr(),
                    null_mut(),
                    UiScroll::None,
                    text_align,
                    fit,
                    top_left_corner.into(),
                    size.into(),
                ) != 0
            },
        }
    }

    /// A toggleable button! A button will expand to fit the text provided to it, vertically and horizontally. Text is
    /// re-used as the id. Will return the toggle value any time the toggle value changes or None if no change occurs
    /// <https://stereokit.net/Pages/StereoKit/UI/Toggle.html>
    /// * `text` - Text to display on the Toggle and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `out_value` - The current state of the toggle button! True means it’s toggled on, and false means it’s
    ///   toggled off.
    /// * `size` - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::get_line_height.
    ///   None is for auto-calculated.
    ///
    /// Will return the new value (same as `out_value`) any time the toggle value changes.
    /// see also [`ui_toggle`] [`ui_toggle_sz`] [`Ui::toggle_img`] [`Ui::toggle_at`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiBtnLayout}, maths::{Vec2, Vec3, Pose}, sprite::Sprite};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.065, 0.91], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let (on, off) = (Sprite::arrow_up(), Sprite::arrow_down());
    ///
    /// let mut choiceA = false; let mut choiceB = true;
    /// let mut choiceC = false; let mut choiceD = true;
    /// let mut choiceE = false; let mut choiceF = true;
    ///
    /// filename_scr = "screenshots/ui_toggle.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Toggle button", &mut window_pose, None, None, None);
    ///     Ui::toggle_img("A", &mut choiceA, &off, &on, Some(UiBtnLayout::Right),
    ///                    Some([0.06, 0.05].into()));
    ///     Ui::same_line();
    ///     if let Some(bool) = Ui::toggle_img("B", &mut choiceB, &off, &on,
    ///                                        Some(UiBtnLayout::Center),
    ///                                        Some([0.06, 0.05].into())) {todo!()}
    ///
    ///     Ui::toggle("C", &mut choiceC, None);
    ///     Ui::same_line();
    ///     Ui::toggle("D", &mut choiceD, Some([0.06, 0.04].into()));
    ///
    ///     Ui::toggle_at("E", &mut choiceE, Some(&off), None, Some(UiBtnLayout::Right),
    ///                     [0.06, -0.12, 0.0], [0.06, 0.03]);
    ///     if let Some(bool) = Ui::toggle_at("F", &mut choiceF, None, None, None,
    ///                                       [-0.01, -0.12, 0.0], [0.06, 0.03]) {todo!()}
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_toggle.jpeg" alt="screenshot" width="200">
    pub fn toggle(text: impl AsRef<str>, out_value: &mut bool, size: Option<Vec2>) -> Option<bool> {
        let cstr = CString::new(text.as_ref()).unwrap();
        let mut active: Bool32T = *out_value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = match size {
            Some(size) => unsafe { ui_toggle_sz(cstr.as_ptr(), active_ptr, size) != 0 },
            None => unsafe { ui_toggle(cstr.as_ptr(), active_ptr) != 0 },
        };

        match change {
            true => {
                *out_value = active != 0;
                Some(*out_value)
            }
            false => None,
        }
    }

    /// A toggleable button! A button will expand to fit the text provided to it, vertically and horizontally. Text is
    /// re-used as the id. Will return the toggle value any time the toggle value changes or None if no change occurs
    /// <https://stereokit.net/Pages/StereoKit/UI/Toggle.html>
    /// * `text` - Text to display on the Toggle and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `out_value` - The current state of the toggle button! True means it’s toggled on, and false means it’s
    ///   toggled off.
    /// * `toggle_off` - Image to use when the toggle value is false.
    /// * `toggle_on` - Image to use when the toggle value is true.
    /// * `image_layout` - This enum specifies how the text and image should be laid out on the button. Default
    ///   [`UiBtnLayout::Left`]
    ///   will have the image on the left, and text on the right.
    /// * `size` - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///   None is for auto-calculated.
    ///
    /// Will return the new value (same as `out_value`) any time the toggle value changes.
    /// see also [`ui_toggle_img`] [`ui_toggle_img_sz`] [`Ui::toggle`] [`Ui::toggle_at`]
    /// see example in [`Ui::toggle`]
    pub fn toggle_img(
        id: impl AsRef<str>,
        out_value: &mut bool,
        toggle_off: impl AsRef<Sprite>,
        toggle_on: impl AsRef<Sprite>,
        image_layout: Option<UiBtnLayout>,
        size: Option<Vec2>,
    ) -> Option<bool> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let mut active: Bool32T = *out_value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let image_layout = image_layout.unwrap_or(UiBtnLayout::Left);
        let change = match size {
            Some(size) => unsafe {
                ui_toggle_img_sz(
                    cstr.as_ptr(),
                    active_ptr,
                    toggle_off.as_ref().0.as_ptr(),
                    toggle_on.as_ref().0.as_ptr(),
                    image_layout,
                    size,
                ) != 0
            },
            None => unsafe {
                ui_toggle_img(
                    cstr.as_ptr(),
                    active_ptr,
                    toggle_off.as_ref().0.as_ptr(),
                    toggle_on.as_ref().0.as_ptr(),
                    image_layout,
                ) != 0
            },
        };
        match change {
            true => {
                *out_value = active != 0;
                Some(*out_value)
            }
            false => None,
        }
    }

    /// A variant of Ui::toggle that doesn’t use the layout system, and instead goes exactly where you put it.
    /// <https://stereokit.net/Pages/StereoKit/UI/ToggleAt.html>
    /// * `text` - Text to display on the Toggle and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `out_value` - The current state of the toggle button! True means it’s toggled on, and false means it’s
    ///   toggled off.
    /// * `toggle_off`- Image to use when the toggle value is false or when no toggle-on image is specified.
    /// * `toggle_on` - Image to use when the toggle value is true and toggle-off has been specified. None will use
    ///   `toggle_off` image if it has been specified.
    /// * `imageLayout` - This enum specifies how the text and image should be laid out on the button.
    ///   None is [`UiBtnLayout::Left`] will have the image on the left, and text on the right.
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    ///
    /// Will return the new value (same as `out_value`) any time the toggle value changes.
    /// see also [`ui_toggle_img_at`] [`ui_toggle_at`] [`Ui::toggle_img`] [`Ui::toggle`]
    /// see example in [`Ui::toggle`]
    pub fn toggle_at(
        id: impl AsRef<str>,
        out_value: &mut bool,
        toggle_off: Option<&Sprite>,
        toggle_on: Option<&Sprite>,
        image_layout: Option<UiBtnLayout>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) -> Option<bool> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let mut active: Bool32T = *out_value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = match toggle_off {
            Some(image_off) => {
                let image_layout = image_layout.unwrap_or(UiBtnLayout::Left);
                let sprite_off = image_off.0.as_ptr();
                let image_on = toggle_on.unwrap_or(image_off);
                unsafe {
                    ui_toggle_img_at(
                        cstr.as_ptr(),
                        active_ptr as *mut Bool32T,
                        sprite_off,
                        image_on.0.as_ptr(),
                        image_layout,
                        top_left_corner.into(),
                        size.into(),
                    ) != 0
                }
            }
            None => unsafe { ui_toggle_at(cstr.as_ptr(), active_ptr, top_left_corner.into(), size.into()) != 0 },
        };
        match change {
            true => {
                *out_value = active != 0;
                Some(*out_value)
            }
            false => None,
        }
    }

    /// A volume for helping to build one handed interactions. This checks for the presence of a hand inside the bounds,
    /// and if found, return that hand along with activation and focus information defined by the interactType.
    /// <https://stereokit.net/Pages/StereoKit/UI/VolumeAt.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `bounds` - Size and position of the volume, relative to the current Hierarchy.
    /// * `interact_type` - `UiConfirm::Pinch` will activate when the hand performs a ‘pinch’ gesture. `UiConfirm::Push`
    ///   will activate when the hand enters the volume, and behave the same as element’s focusState.
    /// * `out_hand` - This will be the last unpreoccupied hand found inside the volume, and is the hand controlling the
    ///   interaction.
    /// * `out_focusState` - The focus state tells if the element has a hand inside of the volume that qualifies for focus.
    ///
    /// see also [`ui_volume_at`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiConfirm}, maths::{Vec2, Vec3, Pose, Bounds},
    ///                      system::{Hand, Handed, BtnState}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.075, 0.9], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let bounds = Bounds::new([0.0, -0.05, 0.0], [0.05, 0.05, 0.05]);
    ///
    /// let mut hand_volume = Handed::Max;
    /// let mut focus_state = BtnState::Inactive;
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Volume At", &mut window_pose, None, None, None);
    ///     let is_active = Ui::volume_at("volume", bounds, UiConfirm::Push,
    ///                                   Some(&mut hand_volume), Some(&mut focus_state));
    ///     assert_eq!(is_active, BtnState::Inactive);
    ///
    ///     let is_active = Ui::volume_at("volume", bounds, UiConfirm::Pinch,
    ///                                   None, None);
    ///     assert_eq!(is_active, BtnState::Inactive);
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn volume_at(
        id: impl AsRef<str>,
        bounds: impl Into<Bounds>,
        interact_type: UiConfirm,
        out_hand: Option<*mut Handed>,
        out_focus_state: Option<*mut BtnState>,
    ) -> BtnState {
        let cstr = CString::new(id.as_ref()).unwrap();
        let hand = out_hand.unwrap_or(null_mut());
        let focus_state = out_focus_state.unwrap_or(null_mut());
        unsafe { ui_volume_at(cstr.as_ptr(), bounds.into(), interact_type, hand, focus_state) }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/VSlider.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `height` - Physical height of the slider on the window. None is default 0 will fill the remaining amount of
    ///   window space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_vslider`] [`Ui::vslider_f64`] [`Ui::vslider_at`]  [`Ui::vslider_at_f64`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiConfirm, UiNotify}, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.91], Some([0.0, 185.0, 0.0].into()));
    ///
    /// let mut scaling1 = 0.15;
    /// let mut scaling2 = 0.50f64;
    /// let mut scaling3 = 0.0;
    /// let mut scaling4 = 0.85;
    ///
    /// filename_scr = "screenshots/ui_vslider.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("HSlider", &mut window_pose, Some([0.18, 0.14].into()), None, None);
    ///     Ui::vslider(    "scaling1", &mut scaling1, 0.0, 1.0, Some(0.05), Some(0.10),
    ///                     None, None);
    ///     Ui::same_line();
    ///     Ui::vslider_f64("scaling2", &mut scaling2, 0.0, 1.0, None, Some(0.12),
    ///                     Some(UiConfirm::Pinch), None);
    ///
    ///     Ui::vslider_at( "scaling3", &mut scaling3, 0.0, 1.0, None,
    ///                     [-0.01, -0.01, 0.0], [0.02, 0.08],
    ///                     None, Some(UiNotify::Finalize));
    ///     if let Some(new_value) = Ui::vslider_at_f64(
    ///                     "scaling4", &mut scaling4, 0.0, 1.0, None,
    ///                     [-0.05, -0.01, 0.0], [0.036, 0.15],
    ///                     Some(UiConfirm::VariablePinch), None) {
    ///         if new_value == 1.0 {
    ///             Log::info("scaling4 is at max");
    ///         }
    ///     }
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_vslider.jpeg" alt="screenshot" width="200">
    #[allow(clippy::too_many_arguments)]
    pub fn vslider(
        id: impl AsRef<str>,
        value: &mut f32,
        min: f32,
        max: f32,
        step: Option<f32>,
        height: Option<f32>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f32> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let height = height.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe { ui_vslider(cstr.as_ptr(), value, min, max, step, height, confirm_method, notify_on) != 0 } {
            true => Some(*value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/VSlider.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `height` - Physical height of the slider on the window. None is default 0 will fill the remaining amount of
    ///   window space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_vslider_f64`] [`Ui::vslider`] [`Ui::vslider_at`]  [`Ui::vslider_at_f64`]
    /// see example in [`Ui::vslider`]
    #[allow(clippy::too_many_arguments)]
    pub fn vslider_f64(
        id: impl AsRef<str>,
        value: &mut f64,
        min: f64,
        max: f64,
        step: Option<f64>,
        height: Option<f32>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f64> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let height = height.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe { ui_vslider_f64(cstr.as_ptr(), value, min, max, step, height, confirm_method, notify_on) != 0 } {
            true => Some(*value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/VSliderAt.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_vslider_at`] [`Ui::vslider`] [`Ui::vslider_f64`]  [`Ui::vslider_at_f64`]
    /// see example in [`Ui::vslider`]
    #[allow(clippy::too_many_arguments)]
    pub fn vslider_at(
        id: impl AsRef<str>,
        value: &mut f32,
        min: f32,
        max: f32,
        step: Option<f32>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f32> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe {
            ui_vslider_at(
                cstr.as_ptr(),
                value,
                min,
                max,
                step,
                top_left_corner.into(),
                size.into(),
                confirm_method,
                notify_on,
            ) != 0
        } {
            true => Some(*value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/VSliderAt.html>
    /// * `id` - An id for tracking element state. MUST be unique within current hierarchy.
    /// * `out_value` - The value that the slider will store slider state in.
    /// * `min` - The minimum value the slider can set, left side of the slider.
    /// * `max` - The maximum value the slider can set, right side of the slider.
    /// * `step` - Locks the value to increments of step. Starts at min, and increments by step. None is default 0
    ///   and means "don't lock to increments".
    /// * `top_left_corner` - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * `size` - The layout size for this element in Hierarchy space.
    /// * `confirm_method` - How should the slider be activated? None is default Push will be a push-button the user
    ///   must press first, and pinch will be a tab that the user must pinch and drag around.
    /// * `notify_on` - Allows you to modify the behavior of the return value. None is default UiNotify::Change.
    ///
    /// Returns new value of the slider if it has changed during this step.
    /// see also [`ui_vslider_at_f64`] [`Ui::vslider`] [`Ui::vslider_at`]  [`Ui::vslider_f64`]
    /// see example in [`Ui::vslider`]
    #[allow(clippy::too_many_arguments)]
    pub fn vslider_at_f64(
        id: impl AsRef<str>,
        value: &mut f64,
        min: f64,
        max: f64,
        step: Option<f64>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f64> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let step = step.unwrap_or(0.0);
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe {
            ui_vslider_at_f64(
                cstr.as_ptr(),
                value,
                min,
                max,
                step,
                top_left_corner.into(),
                size.into(),
                confirm_method,
                notify_on,
            ) != 0
        } {
            true => Some(*value),
            false => None,
        }
    }

    /// Begins a new window! This will push a pose onto the transform stack, and all UI elements will be relative to
    /// that new pose. The pose is actually the top-center of the window. Must be finished with a call to
    /// Ui::window_end().
    /// If size is None the size will be auto-calculated based on the content provided during the previous frame.
    /// <https://stereokit.net/Pages/StereoKit/UI/WindowBegin.html>
    /// * `text` - Text to display on the window title and id for tracking element state. MUST be unique within current
    ///   hierarchy.
    /// * `pose` - The pose state for the window! If showHeader is true, the user will be able to grab this header and
    ///   move it around.
    /// * `size` - Physical size of the window! If None, then the size on that axis will be auto-
    ///   calculated based on the content provided during the previous frame.
    /// * `windowType` - Describes how the window should be drawn, use a header, a body, neither, or both? None is
    ///   UiWin::Normal
    /// * `moveType` - Describes how the window will move when dragged around. None is UiMove::FaceUser
    ///
    /// see also [`ui_window_begin`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiMove, UiWin}, maths::{Vec2, Vec3, Pose}};
    ///
    /// let mut window_pose1 = Pose::new(
    ///     [-0.07, 0.115, 0.89], Some([0.0, 185.0, 0.0].into()));
    /// let mut window_pose2 = Pose::new(
    ///     [-0.05, 0.02, 0.89], Some([0.0, 180.0, 0.0].into()));
    /// let mut window_pose3 = Pose::new(
    ///     [0.09, -0.075, 0.89], Some([0.0, 175.0, 0.0].into()));
    ///
    /// filename_scr = "screenshots/ui_window.jpeg";
    /// test_screenshot!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Window", &mut window_pose1, None, Some(UiWin::Body), None);
    ///     Ui::label("Hello", None, true);
    ///     Ui::window_end();
    ///
    ///     Ui::window_begin("Window", &mut window_pose2, Some([0.19, 0.05].into()), None, None);
    ///     Ui::label("World", None, true);
    ///     Ui::window_end();
    ///
    ///     Ui::window_begin("Window", &mut window_pose3, None, None, Some(UiMove::Exact));
    ///     Ui::label("!!", None, true);
    ///     Ui::window_end();
    /// );
    /// ```
    /// <img src="https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ui_window.jpeg" alt="screenshot" width="200">
    pub fn window_begin(
        text: impl AsRef<str>,
        pose: &mut Pose,
        size: Option<Vec2>,
        window_type: Option<UiWin>,
        move_type: Option<UiMove>,
    ) {
        let cstr = CString::new(text.as_ref()).unwrap();
        let window_type = window_type.unwrap_or(UiWin::Normal);
        let move_type = move_type.unwrap_or(UiMove::FaceUser);
        let size = size.unwrap_or(Vec2::ZERO);
        unsafe { ui_window_begin(cstr.as_ptr(), pose, size, window_type, move_type) }
    }

    /// Finishes a window! Must be called after Ui::window_begin() and all elements have been drawn.
    /// <https://stereokit.net/Pages/StereoKit/UI/WindowEnd.html>
    ///
    /// see also [`ui_window_end`]
    /// see example in [`Ui::window_begin`]
    pub fn window_end() {
        unsafe { ui_window_end() }
    }

    /// get the flag about the far ray grab interaction for Handle elements like the Windows. It can be enabled and
    /// disabled for individual UI elements, and if this remains disabled at the start of the next frame, then the
    /// hand ray indicators will not be visible. This is enabled by default.
    /// <https://stereokit.net/Pages/StereoKit/UI/EnableFarInteract.html>
    ///
    /// see also [`ui_far_interact_enabled`]
    /// see example in [`Ui::enable_far_interact`]
    pub fn get_enable_far_interact() -> bool {
        unsafe { ui_far_interact_enabled() != 0 }
    }

    /// Tells the Active state of the most recently called UI element that used an id.
    /// <https://stereokit.net/Pages/StereoKit/UI/LastElementActive.html>
    ///
    /// see also [`ui_last_element_active`]
    /// see example in [`Ui::last_element_hand_active`]
    pub fn get_last_element_active() -> BtnState {
        unsafe { ui_last_element_active() }
    }

    /// Tells the Focused state of the most recently called UI element that used an id.
    /// <https://stereokit.net/Pages/StereoKit/UI/LastElementFocused.html>
    ///
    /// see also [`ui_last_element_focused`]
    /// see example in [`Ui::last_element_hand_focused`]
    pub fn get_last_element_focused() -> BtnState {
        unsafe { ui_last_element_focused() }
    }

    /// The hierarchy local position of the current UI layout position. The top left point of the next UI element will
    /// be start here!
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutAt.html>
    ///
    /// see also [`ui_layout_at`]
    /// see example in [`Ui::panel_at`]
    pub fn get_layout_at() -> Vec3 {
        unsafe { ui_layout_at() }
    }

    /// These are the layout bounds of the most recently reserved layout space. The Z axis dimensions are always 0.
    /// Only UI elements that affect the surface’s layout will report their bounds here. You can reserve your own layout
    /// space via Ui::layout_reserve, and that call will also report here.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutLast.html>
    ///
    /// see also [`ui_layout_last`]
    /// ### Examples TODO: Very very slow under Windows
    /// ```no_run
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::{Ui, UiPad, UiCut}, maths::{Vec2, Vec3, Pose, Bounds}};
    ///
    /// let mut window_pose = Pose::new(
    ///     [0.01, 0.055, 0.90], Some([0.0, 185.0, 0.0].into()));
    ///
    /// test_steps!( // !!!! Get a proper main loop !!!!
    ///     Ui::window_begin("Panel at", &mut window_pose, Some([0.2, 0.15].into()), None, None);
    ///     Ui::panel_at([0.11, -0.01, 0.0], [0.08, 0.03], Some(UiPad::None));
    ///     Ui::label("panel 1", None, false);
    ///
    ///     Ui::layout_push_cut( UiCut::Right, 0.1, true);
    ///     Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
    ///     Ui::label("panel 2", None, false);
    ///     Ui::layout_pop();
    ///     assert_eq!(Ui::get_layout_last(),
    ///         Bounds { center: Vec3 { x: -0.02382, y: -0.035, z: 0.0 },
    ///                  dimensions: Vec3 { x: 0.06765, y: 0.05, z: 0.0 } });
    ///
    ///     Ui::layout_push_cut( UiCut::Bottom, 0.08, false);
    ///     Ui::panel_at(Ui::get_layout_at(), Ui::get_layout_remaining(), None);
    ///     Ui::label("panel 3", None, false);
    ///     Ui::layout_pop();
    ///     assert_eq!(Ui::get_layout_last(),
    ///         Bounds { center: Vec3 { x: 0.0661, y: -0.075, z: 0.0 },
    ///                  dimensions: Vec3 { x: 0.0476, y: 0.03, z: 0.0 } });    
    ///
    ///     Ui::window_end();
    /// );
    /// ```
    pub fn get_layout_last() -> Bounds {
        unsafe { ui_layout_last() }
    }

    /// How much space is available on the current layout! This is based on the current layout position, so X will give
    /// you the amount remaining on the current line, and Y will give you distance to the bottom of the layout,
    /// including the current line. These values will be 0 if you’re using 0 for the layout size on that axis.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutRemaining.html>
    ///
    /// see also [`ui_layout_remaining`]
    /// see example in [`Ui::panel_at`]
    pub fn get_layout_remaining() -> Vec2 {
        unsafe { ui_layout_remaining() }
    }

    /// This is the height of a single line of text with padding in the UI’s layout system!
    /// <https://stereokit.net/Pages/StereoKit/UI/LineHeight.html>
    ///
    /// see also [`ui_line_height`]
    /// ### Examples
    /// ```
    /// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
    /// use stereokit_rust::{ui::Ui, maths::{Vec2, Vec3, Pose}};
    ///
    /// let line_height = Ui::get_line_height();
    /// assert_eq!(line_height, 0.030000001);
    /// ```
    pub fn get_line_height() -> f32 {
        unsafe { ui_line_height() }
    }

    /// UI sizing and layout settings.
    /// <https://stereokit.net/Pages/StereoKit/UI/Settings.html>
    ///
    /// see also [`ui_get_settings`]
    /// see example in [`Ui::settings`]
    pub fn get_settings() -> UiSettings {
        unsafe { ui_get_settings() }
    }

    /// This is the UiMove that is provided to UI windows that StereoKit itself manages, such as the fallback
    /// filepicker and soft keyboard.
    /// <https://stereokit.net/Pages/StereoKit/UI/SystemMoveType.html>
    ///
    /// see also [`ui_system_get_move_type`]
    /// see example in [`Ui::system_move_type`]
    pub fn get_system_move_type() -> UiMove {
        unsafe { ui_system_get_move_type() }
    }

    /// This returns the TextStyle that’s on top of the UI’s stack, according to Ui::(push/pop)_text_style.
    /// <https://stereokit.net/Pages/StereoKit/UI/TextStyle.html>
    ///
    /// see also [`ui_get_text_style`]
    /// see example in [`Ui::push_text_style`]
    pub fn get_text_style() -> TextStyle {
        unsafe { ui_get_text_style() }
    }

    /// This returns the current state of the UI's enabled status stack, set by `Ui::(push/pop)_enabled`.
    /// <https://stereokit.net/Pages/StereoKit/UI/Enabled.html>
    ///
    /// see also [`ui_is_enabled`]
    /// see example in [`Ui::push_enabled`]
    pub fn get_enabled() -> bool {
        unsafe { ui_is_enabled() != 0 }
    }
}
