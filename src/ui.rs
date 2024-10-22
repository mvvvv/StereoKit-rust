use std::{
    ffi::{c_char, c_ushort, CStr, CString},
    ptr::{null_mut, NonNull},
};

use crate::{
    material::{Material, MaterialT},
    maths::{Bool32T, Bounds, Pose, Vec2, Vec3},
    mesh::{Mesh, MeshT, Vertex},
    model::{Model, ModelT},
    sound::{Sound, SoundT},
    sprite::{Sprite, SpriteT},
    system::{BtnState, Handed, HierarchyParent, Log, TextAlign, TextContext, TextFit, TextStyle},
    util::{Color128, Color32},
    StereoKitError,
};

/// A description of what type of window to draw! This is a bit flag, so it can contain multiple elements.
/// <https://stereokit.net/Pages/StereoKit/UIWin.html>
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
/// The total lenght is [u32,u32] where the fist u32 is the enum and the second is the ExtraSlot value
/// native C function should convert this to UiColorT
/// <https://stereokit.net/Pages/StereoKit/UIColor.html>
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum UiPad {
    None = 0,
    Inside = 1,
    Outside = 2,
}
/// Used with StereoKit’s UI to indicate a particular type of UI element visual.
/// <https://stereokit.net/Pages/StereoKit/UIVisual.html>
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

#[derive(Debug, Copy, Clone)]
#[repr(C)]
/// Visual properties and spacing of the UI system.
/// <https://stereokit.net/Pages/StereoKit/UISettings.html>
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
pub struct UiSliderData {
    pub button_center: Vec2,
    pub finger_offset: f32,
    pub focus_state: BtnState,
    pub active_state: BtnState,
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
pub struct Ui;

extern "C" {
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
        step: Vec2,

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
        text_align: TextAlign,
    );
    pub fn ui_text_16(
        text: *const c_ushort,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        height: f32,
        text_align: TextAlign,
    );
    pub fn ui_text_sz(
        text: *const c_char,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        size: Vec2,
        text_align: TextAlign,
        fit: TextFit,
    );
    pub fn ui_text_sz_16(
        text: *const c_ushort,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        size: Vec2,
        text_align: TextAlign,
        fit: TextFit,
    );
    pub fn ui_text_at(
        text: *const c_char,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        text_align: TextAlign,
        fit: TextFit,
        window_relative_pos: Vec3,
        size: Vec2,
    );
    pub fn ui_text_at_16(
        text: *const c_ushort,
        scroll: *mut Vec2,
        scroll_direction: UiScroll,
        text_align: TextAlign,
        fit: TextFit,
        window_relative_pos: Vec3,
        size: Vec2,
    );
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
    /// see also [`crate::ui::ui_set_color`]
    pub fn color_scheme(color: impl Into<Color128>) {
        unsafe { ui_set_color(color.into()) };
    }

    /// Enables or disables the far ray grab interaction for Handle elements like the Windows. It can be enabled and
    /// disabled for individual UI elements, and if this remains disabled at the start of the next frame, then the
    /// hand ray indicators will not be visible. This is enabled by default.
    /// <https://stereokit.net/Pages/StereoKit/UI/EnableFarInteract.html>
    ///
    /// see also [`crate::ui::ui_enable_far_interact`]
    pub fn enable_far_interact(enable: bool) {
        unsafe { ui_enable_far_interact(enable as Bool32T) };
    }

    /// UI sizing and layout settings.
    /// <https://stereokit.net/Pages/StereoKit/UI/Settings.html>
    ///
    /// see also [`crate::ui::ui_settings`]
    pub fn settings(settings: UiSettings) {
        unsafe { ui_settings(settings) }
    }

    /// Shows or hides the collision volumes of the UI! This is for debug purposes, and can help identify visible and
    /// invisible collision issues.
    /// <https://stereokit.net/Pages/StereoKit/UI/ShowVolumes.html>
    ///
    /// see also [`crate::ui::ui_show_volumes`]
    pub fn show_volumes(show: bool) {
        unsafe { ui_show_volumes(show as Bool32T) };
    }

    /// This is the UIMove that is provided to UI windows that StereoKit itself manages, such as the fallback
    /// filepicker and soft keyboard.
    /// <https://stereokit.net/Pages/StereoKit/UI/SystemMoveType.html>
    ///
    /// see also [`crate::ui::ui_system_set_move_type`]
    pub fn system_move_type(move_type: UiMove) {
        unsafe { ui_system_set_move_type(move_type) };
    }

    /// A pressable button! A button will expand to fit the text provided to it, vertically and horizontally. Text is
    /// re-used as the id. Will return true only on the first frame it is pressed!
    /// <https://stereokit.net/Pages/StereoKit/UI/Button.html>
    /// * size - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///
    /// see also [`crate::ui::ui_button`] [`crate::ui::ui_button_sz`]
    pub fn button(id: impl AsRef<str>, size: Option<Vec2>) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
        match size {
            Some(size) => unsafe { ui_button_sz(cstr.as_ptr(), size) != 0 },
            None => unsafe { ui_button(cstr.as_ptr()) != 0 },
        }
    }

    /// A variant of Ui::button that doesn’t use the layout system, and instead goes exactly where you put it.
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonAt.html>
    /// * size - The layout size for this element in Hierarchy space.
    ///
    /// see also [`crate::ui::ui_button_at`]
    pub fn button_at(id: impl AsRef<str>, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();

        unsafe { ui_button_at(cstr.as_ptr(), top_left_corner.into(), size.into()) != 0 }
    }

    /// This is the core functionality of StereoKit’s buttons, without any of the rendering parts! If you’re trying to
    /// create your own pressable UI elements, or do more extreme customization of the look and feel of UI elements,
    /// then this function will provide a lot of complex pressing functionality for you!
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonBehavior.html>
    /// * out_hand - Id of the hand that interacted with the button. This will be -1 if no interaction has occurred.
    ///
    /// see also [`crate::ui::ui_button_behavior`]
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

    /// This is the core functionality of StereoKit's slider elements, without any of the rendering parts! If you're
    /// trying to create your own sliding UI elements, or do more extreme customization of the look and feel of slider
    /// UI elements, then this function will provide a lot of complex pressing functionality for you
    /// <https://stereokit.net/Pages/StereoKit/UI/SliderBehavior.html>
    /// * window_relative_pos - The layout position of the pressable area.
    /// * size - The size of the pressable area.
    /// * id - The id for this pressable element to track its state with.
    /// * value - The value that the slider will store slider state in.
    /// * min - The minimum value the slider can set, left side of the slider.
    /// * max - The maximum value the slider can set, right side of the slider.
    /// * step - Locks the value to increments of step. Starts at min, and increments by step. 0 is valid, and means
    ///   "don't lock to increments".
    /// * button_size_visual - This is the visual size of the element representing the touchable area of the slider.
    ///   This is used to calculate the center of the button's placement without going outside the provided bounds.
    /// * button_size_interact - The size of the interactive touch element of the slider. Set this to zero to use the
    ///   entire area as a touchable surface.
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * data - This is data about the slider interaction, you can use this for visualizing the slider behavior, or
    ///   reacting to its events.
    ///
    /// see also [`crate::ui::ui_slider_behavior`]
    #[allow(clippy::too_many_arguments)]
    pub fn slider_behavior(
        window_relative_pos: impl Into<Vec3>,
        size: impl Into<Vec2>,
        id: IdHashT,
        value: &mut Vec2,
        min: impl Into<Vec2>,
        max: impl Into<Vec2>,
        step: impl Into<Vec2>,
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
                step.into(),
                button_size_visual.into(),
                button_size_interact.into(),
                confirm_method,
                data,
            );
        }
    }
    /// This is the core functionality of StereoKit’s buttons, without any of the rendering parts! If you’re trying to
    /// create your own pressable UI elements, or do more extreme customization of the look and feel of UI elements,
    /// then this function will provide a lot of complex pressing functionality for you! This overload allows for
    /// customizing the depth of the button, which otherwise would use UISettings.depth for its values.
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonBehavior.html>
    /// * hand - Id of the hand that interacted with the button. This will be -1 if no interaction has occurred.
    ///
    /// see also [`crate::ui::ui_button_behavior_depth`]
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

    /// A pressable button accompanied by an image! The button will expand to fit the text provided to it, horizontally.
    /// Text is re-used as the id. Will return true only on the first frame it is pressed!
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonImg.html>
    /// * image_layout - If None will have default value of UiBtnLayout::Left
    /// * size - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///
    /// see also [`crate::ui::ui_button_img`] [`crate::ui::ui_button_img_sz`]
    pub fn button_img(
        id: impl AsRef<str>,
        image: impl AsRef<Sprite>,
        image_layout: Option<UiBtnLayout>,
        size: Option<Vec2>,
        color: Option<Color128>,
    ) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    /// * image_layout - If None will have default value of UiBtnLayout::Left
    /// * size - The layout size for this element in Hierarchy space.
    ///
    /// see also [`crate::ui::ui_button_img_at`]
    pub fn button_img_at(
        id: impl AsRef<str>,
        image: impl AsRef<Sprite>,
        image_layout: Option<UiBtnLayout>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        color: Option<Color128>,
    ) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    ///
    /// see also [`crate::ui::ui_button_round`]
    pub fn button_round(id: impl AsRef<str>, image: impl AsRef<Sprite>, diameter: f32) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { ui_button_round(cstr.as_ptr(), image.as_ref().0.as_ptr(), diameter) != 0 }
    }

    /// A variant of Ui::button_round that doesn’t use the layout system, and instead goes exactly where you put it.
    /// <https://stereokit.net/Pages/StereoKit/UI/ButtonRoundAt.html>
    ///
    /// see also [`crate::ui::ui_button_round_at`]
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
    /// * move_type - If None, has default value of UiMove::Exact
    /// * allower_gesture - If None, has default value of UiGesture::Pinch
    ///
    /// see also [`crate::ui::ui_handle_begin`] [`crate::ui::ui_handle_end`]
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
    /// * move_type - If None, has default value of UiMove::Exact
    /// * allower_gesture - If None, has default value of UiGesture::Pinch
    ///
    /// see also [`crate::ui::ui_handle_begin`]
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

    /// Finishes a handle! Must be called after UI::handle_begin() and all elements have been drawn. Pops the pose
    /// transform pushed by Ui::handle_begin() from the hierarchy stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/HandleEnd.html>
    ///
    /// see also [`crate::ui::ui_handle_end`]
    pub fn handle_end() {
        unsafe { ui_handle_end() };
    }

    /// This draws a line horizontally across the current layout. Makes a good separator between sections of UI!
    /// <https://stereokit.net/Pages/StereoKit/UI/HSeparator.html>
    ///
    /// see also [`crate::ui::ui_hseparator`]
    pub fn hseparator() {
        unsafe { ui_hseparator() };
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/HSlider.html>
    /// * step - Locks the value to increments of step. Starts at min, and increments by step. Default 0 is valid,
    ///   and means "don't lock to increments".
    /// * width - Physical width of the slider on the window. Default 0 will fill the remaining amount of window space.
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_vslider`]
    #[allow(clippy::too_many_arguments)]
    pub fn hslider(
        id: impl AsRef<str>,
        value: &mut f32,
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
        match unsafe { ui_hslider(cstr.as_ptr(), value, min, max, step, width, confirm_method, notify_on) != 0 } {
            true => Some(*value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/HSlider.html>
    /// * step - Locks the value to increments of step. Starts at min, and increments by step. Default 0 is valid,
    ///   and means "don't lock to increments".
    /// * width - Physical width of the slider on the window. Default 0 will fill the remaining amount of window space.
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_hslider_f64`]
    #[allow(clippy::too_many_arguments)]
    pub fn hslider_f64(
        id: impl AsRef<str>,
        value: &mut f64,
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
        match unsafe { ui_hslider_f64(cstr.as_ptr(), value, min, max, step, width, confirm_method, notify_on) != 0 } {
            true => Some(*value),
            false => None,
        }
    }

    /// A vertical slider element! You can stick your finger in it, and slide the value up and down.
    /// <https://stereokit.net/Pages/StereoKit/UI/HSliderAt.html>
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_hslider_at`]
    #[allow(clippy::too_many_arguments)]
    pub fn hslider_at(
        id: impl AsRef<str>,
        value: &mut f32,
        min: f32,
        max: f32,
        step: f32,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f32> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe {
            ui_hslider_at(
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
    /// <https://stereokit.net/Pages/StereoKit/UI/HSliderAt.html>
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_hslider_at_f64`]
    #[allow(clippy::too_many_arguments)]
    pub fn hslider_at_f64(
        id: impl AsRef<str>,
        value: &mut f64,
        min: f64,
        max: f64,
        step: f64,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f64> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let confirm_method = confirm_method.unwrap_or(UiConfirm::Push);
        let notify_on = notify_on.unwrap_or(UiNotify::Change);
        match unsafe {
            ui_hslider_at_f64(
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

    /// Adds an image to the UI!
    /// <https://stereokit.net/Pages/StereoKit/UI/Image.html>
    ///
    /// see also [`crate::ui::ui_image`]
    pub fn image(image: impl AsRef<Sprite>, size: impl Into<Vec2>) {
        unsafe { ui_image(image.as_ref().0.as_ptr(), size.into()) };
    }

    /// This is an input field where users can input text to the app! Selecting it will spawn a virtual keyboard, or act
    /// as the keyboard focus. Hitting escape or enter, or focusing another UI element will remove focus from this Input.
    /// <https://stereokit.net/Pages/StereoKit/UI/Input.html>
    /// * id - An id for tracking element state. MUST be unique within current hierarchy.
    /// * value - The string that will store the Input's content in.
    /// * size - The layout size for this element in Hierarchy space.
    /// * type_text - What category of text this Input represents. This may affect what kind of soft keyboard will
    ///   be displayed, if one is shown to the user.
    ///
    /// see also [`crate::ui::ui_input`]
    pub fn input(
        id: impl AsRef<str>,
        initial_text: impl AsRef<str>,
        size: Option<Vec2>,
        type_text: Option<TextContext>,
    ) -> Option<String> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let c_value = CString::new(initial_text.as_ref()).unwrap();
        let size = size.unwrap_or(Vec2::ZERO);
        let type_text = type_text.unwrap_or(TextContext::Text);
        if unsafe {
            ui_input(
                cstr.as_ptr(),
                c_value.as_ptr() as *mut c_char,
                initial_text.as_ref().len() as i32,
                size,
                type_text,
            ) != 0
        } {
            match unsafe { CStr::from_ptr(c_value.as_ptr()).to_str() } {
                Ok(result) => Some(result.to_owned()),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// This is an input field where users can input text to the app! Selecting it will spawn a virtual keyboard, or act
    ///  as the keyboard focus. Hitting escape or enter, or focusing another UI element will remove focus from this Input.
    /// <https://stereokit.net/Pages/StereoKit/UI/InputAt.html>
    /// * id - An id for tracking element state. MUST be unique within current hierarchy.
    /// * value - The string that will store the Input's content in.
    /// * top_left_corner - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * size - The layout size for this element in Hierarchy space.
    /// * type_text - What category of text this Input represents. This may affect what kind of soft keyboard will
    ///   be displayed, if one is shown to the user.
    ///
    /// see also [`crate::ui::ui_input_at`]
    pub fn input_at(
        id: impl AsRef<str>,
        initial_text: impl AsRef<str>,
        top_left_corner: impl Into<Vec3>,
        size: Option<Vec2>,
        type_text: Option<TextContext>,
    ) -> Option<String> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let c_value = CString::new(initial_text.as_ref()).unwrap();
        let size = size.unwrap_or(Vec2::ZERO);
        let type_text = type_text.unwrap_or(TextContext::Text);
        if unsafe {
            ui_input_at(
                cstr.as_ptr(),
                c_value.as_ptr() as *mut c_char,
                initial_text.as_ref().len() as i32,
                top_left_corner.into(),
                size,
                type_text,
            ) != 0
        } {
            match unsafe { CStr::from_ptr(c_value.as_ptr()).to_str() } {
                Ok(result) => Some(result.to_owned()),
                Err(_) => None,
            }
        } else {
            None
        }
    }

    /// Tells if the user is currently interacting with a UI element! This will be true if the hand has an active or
    /// focused UI element.
    /// <https://stereokit.net/Pages/StereoKit/UI/IsInteracting.html>
    ///
    /// see also [`crate::ui::ui_is_interacting`]
    pub fn is_interacting(hand: Handed) -> bool {
        unsafe { ui_is_interacting(hand) != 0 }
    }

    /// Adds some text to the layout! Text uses the UI’s current font settings, which can be changed with
    /// Ui::push/pop_text_style. Can contain newlines!
    /// <https://stereokit.net/Pages/StereoKit/UI/Label.html>
    /// * size - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///
    /// see also [`crate::ui::ui_label`] [`crate::ui::ui_label_sz`]
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
    ///
    /// see also [`crate::ui::ui_last_element_hand_active`]
    pub fn last_element_hand_active(hand: Handed) -> BtnState {
        unsafe { ui_last_element_hand_active(hand) }
    }

    /// Tells if the hand was involved in the focus state of the most recently called UI element using an id. Focus
    /// occurs when the hand is in or near an element, in such a way that indicates the user may be about to interact
    /// with it.
    /// <https://stereokit.net/Pages/StereoKit/UI/LastElementHandFocused.html>
    ///
    /// see also [`crate::ui::ui_last_element_hand_focused`]
    pub fn last_element_hand_focused(hand: Handed) -> BtnState {
        unsafe { ui_last_element_hand_focused(hand) }
    }

    /// Manually define what area is used for the UI layout. This is in the current Hierarchy’s coordinate space on the
    /// X/Y plane.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutArea.html>
    ///
    /// see also [`crate::ui::ui_layout_area`]
    pub fn layout_area(start: impl Into<Vec3>, dimensions: impl Into<Vec2>, add_margin: bool) {
        unsafe { ui_layout_area(start.into(), dimensions.into(), add_margin as Bool32T) };
    }

    /// This removes a layout from the layout stack that was previously added using Ui::layout_push, or
    /// Ui::layout_push_cut.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutPop.html>
    ///
    /// see also [`crate::ui::ui_layout_pop`]
    pub fn layout_pop() {
        unsafe { ui_layout_pop() };
    }

    /// This pushes a layout rect onto the layout stack. All UI elements using the layout system will now exist inside
    /// this layout area! Note that some UI elements such as Windows will already be managing a layout of their own on
    /// the stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutPush.html>
    ///
    /// see also [`crate::ui::ui_layout_push`]
    pub fn layout_push(start: impl Into<Vec3>, dimensions: impl Into<Vec2>, add_margin: bool) {
        unsafe { ui_layout_push(start.into(), dimensions.into(), add_margin as Bool32T) };
    }

    /// This cuts off a portion of the current layout area, and pushes that new area onto the layout stack. Left and Top
    /// cuts will always work, but Right and Bottom cuts can only exist inside of a parent layout with an explicit size,
    /// auto-resizing prevents these cuts. All UI elements using the layout system will now exist inside this layout
    /// area! Note that some UI elements such as Windows will already be managing a layout of their own on the stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutPushCut.html>
    ///
    /// see also [`crate::ui::ui_layout_push_cut`]
    pub fn layout_push_cut(cut_to: UiCut, size_meters: f32, add_margin: bool) {
        unsafe { ui_layout_push_cut(cut_to, size_meters, add_margin as Bool32T) };
    }

    /// Reserves a box of space for an item in the current UI layout! If either size axis is zero, it will be auto-sized
    /// to fill the current surface horizontally, and fill a single line_height vertically. Returns the Hierarchy local
    /// bounds of the space that was reserved, with a Z axis dimension of 0.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutReserve.html>
    /// * add_padding - If true, this will add the current padding value to the total final dimensions of the space that
    ///   is reserved.
    /// * depth - This allows you to quickly insert a depth into the Bounds you’re receiving. This will offset on the
    ///   Z axis in addition to increasing the dimensions, so that the bounds still remain sitting on the surface of the
    ///   UI. This depth value will not be reflected in the bounds provided by LayouLast.
    ///
    /// see also [`crate::ui::ui_layout_reserve`]
    pub fn layout_reserve(size: impl Into<Vec2>, add_padding: bool, depth: f32) -> Bounds {
        unsafe { ui_layout_reserve(size.into(), add_padding as Bool32T, depth) }
    }

    /// This adds a non-interactive Model to the UI panel layout, and allows you to specify its size.
    /// <https://stereokit.net/Pages/StereoKit/UI/Model.html>
    /// * size - The size this element should take from the layout.
    /// * model_scale - 0 will auto-scale the model to fit the layout space, but you can specify a different scale in
    ///   case you’d like a different size.
    ///
    /// see also [`crate::ui::ui_model`]
    pub fn model(model: impl AsRef<Model>, ui_size: Option<Vec2>, model_scale: Option<f32>) {
        let ui_size = ui_size.unwrap_or(Vec2::ZERO);
        let model_scale = model_scale.unwrap_or(0.0);
        unsafe { ui_model(model.as_ref().0.as_ptr(), ui_size, model_scale) };
    }

    /// This will advance the layout to the next line. If there’s nothing on the current line, it’ll advance to the
    /// start of the next on. But this won’t have any affect on an empty line, try Ui::hspace for that.
    /// <https://stereokit.net/Pages/StereoKit/UI/NextLine.html>
    ///
    /// see also [`crate::ui::ui_nextline`]
    pub fn next_line() {
        unsafe { ui_nextline() };
    }

    /// If you wish to manually draw a Panel, this function will let you draw one wherever you want!
    /// <https://stereokit.net/Pages/StereoKit/UI/PanelAt.html>
    /// * padding - If None the default value is UiPad::Outside
    ///
    /// see also [`crate::ui::ui_panel_at`]
    pub fn panel_at(start: impl Into<Vec3>, size: impl Into<Vec2>, padding: Option<UiPad>) {
        let padding = padding.unwrap_or(UiPad::Outside);
        unsafe { ui_panel_at(start.into(), size.into(), padding) };
    }

    /// This will begin a Panel element that will encompass all elements drawn between panel_begin and panel_end. This
    /// is an entirely visual element, and is great for visually grouping elements together. Every Begin must have a
    /// matching End.
    /// <https://stereokit.net/Pages/StereoKit/UI/PanelBegin.html>
    /// * padding - If None the default value is UiPad::Outside
    ///
    /// see also [`crate::ui::ui_panel_begin`]
    pub fn panel_begin(padding: Option<UiPad>) {
        let padding = padding.unwrap_or(UiPad::Outside);
        unsafe { ui_panel_begin(padding) };
    }

    /// This will finalize and draw a Panel element.
    /// <https://stereokit.net/Pages/StereoKit/UI/PanelEnd.html>
    ///
    /// see also [`crate::ui::ui_panel_end`]
    pub fn panel_end() {
        unsafe { ui_panel_end() };
    }

    /// Removes an ‘enabled’ state from the stack, and whatever was below will then be used as the primary enabled
    /// state.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopEnabled.html>
    ///
    /// see also [`crate::ui::ui_pop_enabled`]
    pub fn pop_enabled() {
        unsafe { ui_pop_enabled() };
    }

    /// Removes the last root id from the stack, and moves up to the one before it!
    /// <https://stereokit.net/Pages/StereoKit/UI/PopId.html>
    ///
    /// see also [`crate::ui::ui_pop_id`]
    pub fn pop_id() {
        unsafe { ui_pop_id() };
    }

    /// This pops the keyboard presentation state to what it was previously.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopPreserveKeyboard.html>
    ///
    /// see also [`crate::ui::ui_pop_preserve_keyboard`]
    pub fn pop_preserve_keyboard() {
        unsafe { ui_pop_preserve_keyboard() };
    }

    /// This removes an enabled status for grab auras from the stack, returning it to the state before the previous
    /// push_grab_aura call. Grab auras are an extra space and visual element that goes around Window elements to make
    /// them easier to grab.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopGrabAurahtml>
    ///
    /// see also [`crate::ui::ui_pop_grab_aura`]
    pub fn pop_grab_aura() {
        unsafe { ui_pop_grab_aura() };
    }

    /// This retreives the top of the grab aura enablement stack, in case you need to know if the current window will
    /// have an aura.
    /// <https://stereokit.net/Pages/StereoKit/UI/GrabAuraEnabled>
    ///
    /// see also [`crate::ui::ui_grab_aura_enabled`]
    pub fn grab_aura_enabled() -> bool {
        unsafe { ui_grab_aura_enabled() != 0 }
    }

    /// This will return to the previous UI layout on the stack. This must be called after a PushSurface call.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopSurface.html>
    ///
    /// see also [`crate::ui::ui_pop_surface`]
    pub fn pop_surface() {
        unsafe { ui_pop_surface() };
    }

    /// Removes a TextStyle from the stack, and whatever was below will then be used as the GUI’s primary font.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopTextStyle.html>
    ///
    /// see also [`crate::ui::ui_pop_text_style`]
    pub fn pop_text_style() {
        unsafe { ui_pop_text_style() };
    }

    /// Removes a Tint from the stack, and whatever was below will then be used as the primary tint.
    /// <https://stereokit.net/Pages/StereoKit/UI/PopTint.html>
    ///
    /// see also [`crate::ui::ui_pop_tint`]
    pub fn pop_tint() {
        unsafe { ui_pop_tint() };
    }

    /// This creates a Pose that is friendly towards UI popup windows, or windows that are created due to some type of
    /// user interaction. The fallback file picker and soft keyboard both use this function to position themselves!
    /// <https://stereokit.net/Pages/StereoKit/UI/PopupPose.html>
    ///
    /// see also [`crate::ui::ui_popup_pose`]
    pub fn popup_pose(shift: impl Into<Vec3>) -> Pose {
        unsafe { ui_popup_pose(shift.into()) }
    }

    /// This is a simple horizontal progress indicator bar. This is used by the hslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/ProgressBar.html>
    ///
    /// see also [`crate::ui::ui_hprogress_bar`]
    #[deprecated(since = "0.0.1", note = "Use HProgressBar instead")]
    pub fn progress_bar(percent: f32, width: f32) {
        unsafe { ui_hprogress_bar(percent, width, 0) }
    }

    /// This is a simple horizontal progress indicator bar. This is used by the hslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/HProgressBar.html>
    /// * percent - A value between 0 and 1 indicating progress from 0% to 100%.
    /// * width - Physical width of the slider on the window. 0 will fill the remaining amount of window space.
    /// * flip_fill_direction - By default, this fills from left to right. This allows you to flip the fill direction to
    ///   right to left.
    ///
    /// see also [`crate::ui::ui_hprogress_bar`]
    pub fn hprogress_bar(percent: f32, width: f32, flip_fill_direction: bool) {
        unsafe { ui_hprogress_bar(percent, width, flip_fill_direction as Bool32T) }
    }

    /// This is a simple vertical progress indicator bar. This is used by the vslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/VProgressBar.html>
    /// * percent - A value between 0 and 1 indicating progress from 0% to 100%.
    /// * width - Physical width of the slider on the window. 0 will fill the remaining amount of window space.
    /// * flip_fill_direction - By default, this fills from top to bottom. This allows you to flip the fill direction to
    ///   bottom to top.
    ///
    /// see also [`crate::ui::ui_vprogress_bar`]
    pub fn vprogress_bar(percent: f32, height: f32, flip_fill_direction: bool) {
        unsafe { ui_vprogress_bar(percent, height, flip_fill_direction as Bool32T) }
    }

    /// This is a simple horizontal progress indicator bar. This is used by the hslider to draw the slider bar beneath
    /// the interactive element. Does not include any text or label.
    /// <https://stereokit.net/Pages/StereoKit/UI/ProgressBarAt.html>
    ///
    /// see also [`crate::ui::ui_progress_bar_at`]
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
    /// * enabled - Should the following elements be enabled and interactable?
    /// * parent_behavior - Do we want to ignore or inherit the state of the current stack? Default should be false.
    ///   if None, has default value Inherit
    ///
    /// see also [`crate::ui::ui_push_enabled`]
    pub fn push_enabled(enabled: bool, parent_behavior: Option<HierarchyParent>) {
        let parent_behavior = parent_behavior.unwrap_or(HierarchyParent::Inherit);
        unsafe { ui_push_enabled(enabled as Bool32T, parent_behavior) }
    }

    /// Adds a root id to the stack for the following UI elements! This id is combined when hashing any following ids,
    /// to prevent id collisions in separate groups.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushId.html>
    ///
    /// see also [`crate::ui::ui_push_id`]
    pub fn push_id(root_id: impl AsRef<str>) {
        let cstr = CString::new(root_id.as_ref()).unwrap();
        unsafe { ui_push_id(cstr.as_ptr()) };
    }

    /// When a soft keyboard is visible, interacting with UI elements will cause the keyboard to close. This function
    /// allows you to change this behavior for certain UI elements, allowing the user to interact and still preserve the
    /// keyboard’s presence. Remember to Pop when you’re finished!
    /// <https://stereokit.net/Pages/StereoKit/UI/PushPreserveKeyboard.html>
    ///
    /// see also [`crate::ui::ui_push_preserve_keyboard`]
    pub fn push_preserve_keyboard(preserve_keyboard: bool) {
        unsafe { ui_push_preserve_keyboard(preserve_keyboard as Bool32T) }
    }

    /// This pushes an enabled status for grab auras onto the stack. Grab auras are an extra space and visual element
    /// that goes around Window elements to make them easier to grab. MUST be matched by a pop_grab_aura call.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushGrabAura.html>
    ///
    /// see also [`crate::ui::ui_push_grab_aura`]
    pub fn push_grab_aura(enabled: bool) {
        unsafe { ui_push_grab_aura(enabled as Bool32T) }
    }

    /// This will push a surface into SK’s UI layout system. The surface becomes part of the transform hierarchy, and SK
    /// creates a layout surface for UI content to be placed on and interacted with. Must be accompanied by a
    /// pop_surface call.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushSurface.html>
    ///
    /// see also [`crate::ui::ui_push_surface`]
    pub fn push_surface(pose: impl Into<Pose>, layout_start: impl Into<Vec3>, layout_dimension: impl Into<Vec2>) {
        unsafe { ui_push_surface(pose.into(), layout_start.into(), layout_dimension.into()) }
    }

    /// This pushes a Text Style onto the style stack! All text elements rendered by the GUI system will now use this
    /// styling.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushTextStyle.html>
    ///
    /// see also [`crate::ui::ui_push_text_style`]
    pub fn push_text_style(style: TextStyle) {
        unsafe { ui_push_text_style(style) }
    }

    /// All UI between push_tint and its matching pop_tint will be tinted with this color. This is implemented by
    /// multiplying this color with the current color of the UI element. The default is a White (1,1,1,1) identity tint.
    /// <https://stereokit.net/Pages/StereoKit/UI/PushTint.html>
    ///
    /// see also [`crate::ui::ui_push_tint`]
    pub fn push_tint(color_gamma: impl Into<Color128>) {
        unsafe { ui_push_tint(color_gamma.into()) }
    }

    /// This will reposition the Mesh’s vertices to work well with quadrant resizing shaders. The mesh should generally
    /// be centered around the origin, and face down the -Z axis. This will also overwrite any UV coordinates in the
    /// verts.
    ///
    /// You can read more about the technique here : <https://playdeck.net/blog/quadrant-sizing-efficient-ui-rendering>
    /// <https://stereokit.net/Pages/StereoKit/UI/QuadrantSizeMesh.html>
    ///
    /// see also [`crate::ui::ui_quadrant_size_mesh`]
    pub fn quadrant_size_mesh(mesh: impl AsRef<Mesh>, overflow_percent: f32) {
        unsafe { ui_quadrant_size_mesh(mesh.as_ref().0.as_ptr(), overflow_percent) }
    }

    /// This will reposition the vertices to work well with quadrant resizing shaders. The mesh should generally be
    /// centered around the origin, and face down the -Z axis. This will also overwrite any UV coordinates in the verts.
    ///
    /// You can read more about the technique here : <https://playdeck.net/blog/quadrant-sizing-efficient-ui-rendering>
    /// <https://stereokit.net/Pages/StereoKit/UI/QuadrantSizeVerts.html>
    ///
    /// see also [`crate::ui::ui_quadrant_size_verts`]
    pub fn quadrant_size_verts(verts: &[Vertex], overflow_percent: f32) {
        unsafe { ui_quadrant_size_verts(verts.as_ptr() as *mut Vertex, verts.len() as i32, overflow_percent) }
    }

    /// This generates a quadrantified mesh meant for UI buttons by sweeping a lathe over the rounded corners of a
    /// rectangle! Note that this mesh is quadrantified, so it requires special shaders to draw properly!
    /// <https://stereokit.net/Pages/StereoKit/UI/GenQuadrantMesh.html>
    /// * rounded_corners - A bit-flag indicating which corners should be rounded, and which should be sharp!
    /// * corner_radius - The radius of each rounded corner.
    /// * corner_resolution - How many slices/verts go into each corner? More is smoother, but more expensive to render.
    /// * delete_flat_sides - If two adjacent corners are sharp, should we skip connecting them with triangles? If this
    ///   edge will always be covered, then deleting these faces may save you some performance.
    /// * quadrantify - Does this generate a mesh compatible with StereoKit's quadrant shader system, or is this just a
    ///   traditional mesh? In most cases, this should be true, but UI elements such as the rounded button may be
    ///   exceptions.
    /// * lathe_pts" - The lathe points to sweep around the edge.
    ///
    /// Returns the final Mesh, ready for use in SK's theming system.
    ///
    /// see also [`crate::ui::ui_gen_quadrant_mesh`]
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
    /// * size - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///
    /// see also [`crate::ui::ui_radio`] [`crate::ui::ui_radio_sz`]
    #[deprecated(since = "0.0.1", note = "Performence issues, use radio_img instead")]
    pub fn radio(id: impl AsRef<str>, active: bool, size: Option<Vec2>) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    /// * size - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///
    /// see also [`crate::ui::ui_toggle_img`] [`crate::ui::ui_toggle_img_sz`]
    pub fn radio_img(
        id: impl AsRef<str>,
        active: bool,
        image_off: impl AsRef<Sprite>,
        image_on: impl AsRef<Sprite>,
        image_layout: UiBtnLayout,
        size: Option<Vec2>,
    ) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    ///
    /// see also [`crate::ui::ui_toggle_img_at`]
    pub fn radio_at(
        id: impl AsRef<str>,
        active: bool,
        image_off: impl AsRef<Sprite>,
        image_on: impl AsRef<Sprite>,
        image_layout: UiBtnLayout,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) -> bool {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    /// see also [`crate::ui::ui_sameline`]
    pub fn same_line() {
        unsafe { ui_sameline() }
    }

    /// Override the visual assets attached to a particular UI element.
    /// Note that StereoKit’s default UI assets use a type of quadrant sizing that is implemented in the Material and
    /// the Mesh. You don’t need to use quadrant sizing for your own visuals, but if you wish to know more, you can read
    /// more about the technique here : <https://playdeck.net/blog/quadrant-sizing-efficient-ui-rendering>
    /// You may also find Ui::quadrant_size_verts and Ui::quadrant_size_mesh to be helpful.
    /// <https://stereokit.net/Pages/StereoKit/UI/SetElementVisual.html>
    /// * visual - Which UI visual element to override. Use UiVisual::ExtraSlotXX if you need extra
    ///   UIVisual slots for your own custom UI elements.
    /// * mesh - The Mesh to use for the UI element's visual component. The Mesh will be scaled to match the dimensions
    ///   of the UI element.
    /// * material - The Material to use when rendering the UI element. The default Material is specifically designed
    ///   to work with quadrant sizing formatted meshes.
    /// * min_size - For some meshes, such as quadrant sized meshes, there's a minimum size where the mesh turns inside
    ///   out. This lets UI elements to accommodate for this minimum size, and behave somewhat more appropriately.
    ///
    /// see also [`crate::ui::ui_set_element_visual`]
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
    ///
    /// * visual - The UI element type to set the color category of.
    /// * color_category - The category of color to assign to this UI element. Use Ui::set_theme_color in combination
    ///   with this to assign a specific color. Use UiColor::ExtraSlotXX if you need extra UIColor slots
    ///   for your own custom UI elements.
    ///
    /// <https://stereokit.net/Pages/StereoKit/UI/SetElementColor.html>
    ///
    /// see also [`crate::ui::ui_set_element_color`]
    pub fn set_element_color(visual: UiVisual, color_category: UiColor) {
        unsafe { ui_set_element_color(visual, color_category) };
    }

    /// This sets the sound that a particulat UI element will make when you interact with it. One sound when the
    /// interaction starts, and one when it ends.
    ///
    /// * visual - The UI element to apply the sounds to. Use UiVisual::ExtraSlotXX if you need extra
    ///   UIVisual slots
    /// * activate - The sound made when the interaction begins. A null sound will fall back to the default sound.
    /// * deactivate - The sound made when the interaction ends. A null sound will fall back to the default sound.
    ///
    /// <https://stereokit.net/Pages/StereoKit/UI/SetElementSound.html>
    ///
    /// see also [`crate::ui::ui_set_element_sound`]
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
    /// * element_visual - The element type to draw. Use UiVisual::ExtraSlotXX to use extra UiVisual
    ///   slots for your own custom UI elements. If these slots are empty, SK will fall back to UiVisual::Default
    /// * element_color - If you wish to use the coloring from a different element, you can use this to override the
    ///   theme color used when drawing. Use UiVisual::ExtraSlotXX to use extra UiVisual slots for your
    ///   own custom UI elements. If these slots are empty, SK will fall back to UiVisual::Default.
    /// * start - This is the top left corner of the UI element relative to the current Hierarchy.
    /// * size - The layout size for this element in Hierarchy space.
    /// * focus - The amount of visual focus this element currently has, where 0 is unfocused, and 1 is active. You
    ///   can acquire a good focus value from `Ui::get_anim_focus`.
    ///
    /// see also [`crate::ui::ui_draw_element`] [`crate::ui::ui_draw_element_color`]
    pub fn draw_element(
        element_visual: UiVisual,
        element_color: Option<UiVisual>,
        start: Vec3,
        size: Vec3,
        focus: f32,
    ) {
        match element_color {
            Some(element_color) => unsafe { ui_draw_element_color(element_visual, element_color, start, size, focus) },
            None => unsafe { ui_draw_element(element_visual, start, size, focus) },
        }
    }

    /// This will get a final linear draw color for a particular UI element type with a particular focus value. This
    /// obeys the current hierarchy of tinting and enabled states.
    /// <https://stereokit.net/Pages/StereoKit/UI/GetElementColor.html>
    /// * element_visual - Get the color from this element type.  Use UiVisual::ExtraSlotXX to use extra
    ///   UiVisual slots for your own custom UI elements. If these slots are empty, SK will fall back to
    ///   UiVisual::Default.
    /// * focus - The amount of visual focus this element currently has, where 0 is unfocused, and 1 is active. You
    ///   can acquire a good focus value from `Ui::get_anim_focus`
    ///
    /// Returns a linear color good for tinting UI meshes.
    /// see also [`crate::ui::ui_get_element_color`]
    pub fn get_element_color(element_visual: UiVisual, focus: f32) -> Color128 {
        unsafe { ui_get_element_color(element_visual, focus) }
    }

    /// This resolves a UI element with an ID and its current
    /// states into a nicely animated focus value.
    /// <https://stereokit.net/Pages/StereoKit/UI/GetAnimFocus.html>
    /// * id - The hierarchical id of the UI element we're checking the focus of, this can be created with
    ///   `Ui::stack_hash`.
    /// * focus_state - The current focus state of the UI element.
    /// * activationState - The current activation status of the/ UI element.
    ///
    /// Returns a focus value in the realm of 0-1, where 0 is unfocused, and 1 is active.
    /// see also [`crate::ui::ui_get_anim_focus`]
    pub fn get_anim_focus(id: IdHashT, focus_state: BtnState, activation_state: BtnState) -> f32 {
        unsafe { ui_get_anim_focus(id, focus_state, activation_state) }
    }

    /// This allows you to explicitly set a theme color, for finer grained control over the UI appearance. Each theme
    /// type is still used by many different UI elements. This will automatically generate colors for different UI
    /// element states.
    /// <https://stereokit.net/Pages/StereoKit/UI/SetThemeColor.html>
    /// * color_category - The category of UI elements that are affected by this theme color. Use UiColor::ExtraSlotXX
    ///   if you need extra UiColor slots for your own custom UI elements.
    /// * color_state - The state of the UI element this color should apply to. If None has the value
    ///   UiColorState::Normal
    /// * color_gama : the gamma corrected color that should be applied to this theme color category in its normal
    ///   resting state. Active and disabled colors will be generated based on this color.
    ///
    /// see also [`crate::ui::ui_set_theme_color`] [`crate::ui::ui_set_theme_color_state`]
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
    /// * color_category - The category of UI elements that are affected by this theme color. Use UiColor::ExtraSlotXX
    ///   if you need extra UiColor slots for your own custom UI elements.
    ///   If the theme slot is empty, the color will be pulled from UiColor::None
    /// * color_state : The state of the UI element this color applies to. If None has the value UiColorState::Normal
    ///
    /// Returns the gamma space color for the theme color category in the indicated state.
    /// see also [`crate::ui::ui_get_theme_color`] [`crate::ui::ui_get_theme_color_state`]
    pub fn get_theme_color(color_category: UiColor, color_state: Option<UiColorState>) -> Color128 {
        match color_state {
            Some(color_state) => unsafe { ui_get_theme_color_state(color_category, color_state) },
            None => unsafe { ui_get_theme_color(color_category) },
        }
    }

    /// adds some vertical space to the current line! All UI following elements on this line will be offset.
    /// <https://stereokit.net/Pages/StereoKit/UI/VSpace.html>
    ///
    /// see also [`crate::ui::ui_vspace`]
    pub fn vspace(space: f32) {
        unsafe { ui_vspace(space) }
    }

    /// adds some horizontal space to the current line!
    /// <https://stereokit.net/Pages/StereoKit/UI/HSpace.html>
    ///
    /// see also [`crate::ui::ui_hspace`]
    pub fn hspace(space: f32) {
        unsafe { ui_hspace(space) }
    }

    /// This will hash the given text based id into a hash for use with certain StereoKit UI functions. This includes
    /// the hash of the current id stack.
    /// <https://stereokit.net/Pages/StereoKit/UI/StackHash.html>
    ///
    /// see also [`crate::ui::ui_stack_hash`]
    pub fn stack_hash(id: impl AsRef<str>) -> IdHashT {
        let cstr = CString::new(id.as_ref()).unwrap();
        unsafe { ui_stack_hash(cstr.as_ptr()) }
    }

    /// Displays a large chunk of text on the current layout. This can include new lines and spaces, and will properly
    /// wrap once it fills the entire layout! Text uses the UI’s current font settings, which can be changed with
    /// Ui::push/pop_text_style.
    /// <https://stereokit.net/Pages/StereoKit/UI/Text.html>
    /// * scroll - This is the current scroll value of the text, in meters, _not_ percent.
    /// * scrollDirection - What scroll bars are allowed to show on this text? Vertical, horizontal, both?
    /// * height - The vertical height of this Text element.
    /// * width - if not specified it will automatically take the remainder of the current layout.
    /// * text_align - Where should the text position itself within its bounds? Default is TextAlign::TopLeft is how most
    ///   european language are aligned.
    /// * fit - Describe how the text should behave when one of its size dimensions conflicts with the provided ‘size’
    ///   parameter. Ui::text uses TextFit::Wrap by default.
    ///
    /// see also [`crate::ui::ui_text`]
    pub fn text(
        text: impl AsRef<str>,
        scroll: Option<&mut Vec2>,
        scroll_direction: Option<UiScroll>,
        height: Option<f32>,
        width: Option<f32>,
        text_align: Option<TextAlign>,
        fit: Option<TextFit>,
    ) {
        let cstr = CString::new(text.as_ref()).unwrap();
        let scroll_direction = scroll_direction.unwrap_or(UiScroll::None);
        let height = height.unwrap_or(0.0);
        let text_align = text_align.unwrap_or(TextAlign::TopLeft);
        let fit = fit.unwrap_or(TextFit::Wrap);
        if let Some(width) = width {
            let size = Vec2::new(width, height);
            match scroll {
                Some(scroll) => unsafe { ui_text_sz(cstr.as_ptr(), scroll, scroll_direction, size, text_align, fit) },
                None => unsafe { ui_text_sz(cstr.as_ptr(), null_mut(), UiScroll::None, size, text_align, fit) },
            }
        } else {
            match scroll {
                Some(scroll) => unsafe { ui_text(cstr.as_ptr(), scroll, scroll_direction, height, text_align) },
                None => unsafe { ui_text(cstr.as_ptr(), null_mut(), UiScroll::None, 0.0, text_align) },
            }
        }
    }

    /// Displays a large chunk of text on the current layout. This can include new lines and spaces, and will properly
    /// wrap once it fills the entire layout! Text uses the UI’s current font settings, which can be changed with
    /// Ui::push/pop_text_style.
    /// <https://stereokit.net/Pages/StereoKit/UI/TextAt.html>
    /// * text_align - Where should the text position itself within its bounds? TextAlign::TopLeft is how most
    ///   european language are aligned.
    /// * fit - Describe how the text should behave when one of its size dimensions conflicts with the provided ‘size’
    ///   parameter. Ui::text uses TextFit::Wrap by default.
    /// * size - The layout size for this element in Hierarchy space.
    ///
    /// see also [`crate::ui::ui_text_at`]
    pub fn text_at(
        text: impl AsRef<str>,
        scroll: Option<&mut Vec2>,
        scroll_direction: Option<UiScroll>,
        text_align: TextAlign,
        fit: TextFit,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) {
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
                )
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
                )
            },
        }
    }

    /// A toggleable button! A button will expand to fit the text provided to it, vertically and horizontally. Text is
    /// re-used as the id. Will return the toggle value any time the toggle value changes or None if no change occurs
    /// <https://stereokit.net/Pages/StereoKit/UI/Toggle.html>
    /// * size - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///
    /// see also [`crate::ui::ui_toggle`] [`crate::ui::ui_toggle_sz`]
    pub fn toggle(id: impl AsRef<str>, value: bool, size: Option<Vec2>) -> Option<bool> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let mut active: Bool32T = value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = match size {
            Some(size) => unsafe { ui_toggle_sz(cstr.as_ptr(), active_ptr, size) != 0 },
            None => unsafe { ui_toggle(cstr.as_ptr(), active_ptr) != 0 },
        };

        match change {
            true => Some(active != 0),
            false => None,
        }
    }

    /// A toggleable button! A button will expand to fit the text provided to it, vertically and horizontally. Text is
    /// re-used as the id. Will return the toggle value any time the toggle value changes or None if no change occurs
    /// <https://stereokit.net/Pages/StereoKit/UI/Toggle.html>
    /// * image_layout - This enum specifies how the text and
    ///   image should be laid out on the button. Default `UiBtnLayout::Left`
    ///   will have the image on the left, and text on the right.
    /// * size - The layout size for this element in Hierarchy space. If an axis is left as zero, it will be
    ///   auto-calculated. For X this is the remaining width of the current layout, and for Y this is Ui::line_height.
    ///
    /// see also [`crate::ui::ui_toggle_img`] [`crate::ui::ui_toggle_img_sz`]
    pub fn toggle_img(
        id: impl AsRef<str>,
        value: bool,
        toggle_off: impl AsRef<Sprite>,
        toggle_on: impl AsRef<Sprite>,
        image_layout: Option<UiBtnLayout>,
        size: Option<Vec2>,
    ) -> Option<bool> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let mut active: Bool32T = value as Bool32T;
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
            true => Some(active != 0),
            false => None,
        }
    }

    /// A variant of Ui::toggle that doesn’t use the layout system, and instead goes exactly where you put it.
    /// <https://stereokit.net/Pages/StereoKit/UI/ToggleAt.html>
    /// * toggle-off- Image to use when the toggle value is false or when no toggle-on image is specified
    /// * toggle-on - Image to use when the toggle value is true and toggle-off has been specified.
    /// * imageLayout - This enum specifies how the text and image should be laid out on the button.
    ///   Default is UiBtnLayout::Left will have the image on the left, and text on the right.
    ///
    /// see also [`crate::ui::ui_toggle_img_at`] [`crate::ui::ui_toggle_at`]
    pub fn toggle_at(
        id: impl AsRef<str>,
        value: bool,
        image_off: Option<Sprite>,
        image_on: Option<Sprite>,
        image_layout: Option<UiBtnLayout>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) -> Option<bool> {
        let cstr = CString::new(id.as_ref()).unwrap();
        let mut active: Bool32T = value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = match image_off {
            Some(image_off) => {
                let image_layout = image_layout.unwrap_or(UiBtnLayout::Left);
                let sprite_off = image_off.as_ref().0.as_ptr();
                let image_on = image_on.unwrap_or(image_off);
                unsafe {
                    ui_toggle_img_at(
                        cstr.as_ptr(),
                        active_ptr as *mut Bool32T,
                        sprite_off,
                        image_on.as_ref().0.as_ptr(),
                        image_layout,
                        top_left_corner.into(),
                        size.into(),
                    ) != 0
                }
            }
            None => unsafe { ui_toggle_at(cstr.as_ptr(), active_ptr, top_left_corner.into(), size.into()) != 0 },
        };
        match change {
            true => Some(active != 0),
            false => None,
        }
    }

    /// A volume for helping to build one handed interactions. This checks for the presence of a hand inside the bounds,
    /// and if found, return that hand along with activation and focus information defined by the interactType.
    /// <https://stereokit.net/Pages/StereoKit/UI/VolumeAt.html>
    /// * out_hand - This will be the last unpreoccupied hand found inside the volume, and is the hand controlling the
    ///   interaction.
    /// * out_focusState - The focus state tells if the element has a hand inside of the volume that qualifies for focus.
    ///
    /// see also [`crate::ui::ui_volume_at`]
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
    /// * step - Locks the value to increments of step. Starts at min, and increments by step. Default 0 is valid,
    ///   and means "don't lock to increments".
    /// * height - Physical height of the slider on the window. Default 0 will fill the remaining amount of window space.
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_vslider`]
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
    /// * step - Locks the value to increments of step. Starts at min, and increments by step. Default 0 is valid,
    ///   and means "don't lock to increments".
    /// * height - Physical height of the slider on the window. Default 0 will fill the remaining amount of window space.
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_vslider_f64`]
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
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_vslider_at`]
    #[allow(clippy::too_many_arguments)]
    pub fn vslider_at(
        id: impl AsRef<str>,
        value: &mut f32,
        min: f32,
        max: f32,
        step: f32,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f32> {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    /// * confirm_method - How should the slider be activated? Default Push will be a push-button the user must press
    ///   first, and pinch will be a tab that the user must pinch and drag around.
    /// * notify_on - Allows you to modify the behavior of the return value. Default is UiNotify::Change
    ///
    /// see also [`crate::ui::ui_vslider_at_f64`]
    #[allow(clippy::too_many_arguments)]
    pub fn vslider_at_f64(
        id: impl AsRef<str>,
        value: &mut f64,
        min: f64,
        max: f64,
        step: f64,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
        confirm_method: Option<UiConfirm>,
        notify_on: Option<UiNotify>,
    ) -> Option<f64> {
        let cstr = CString::new(id.as_ref()).unwrap();
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
    /// * size - Physical size of the window! If None, then the size on that axis will be auto-
    ///   calculated based on the content provided during the previous frame.
    /// * windowType - Describes how the window should be drawn, use a header, a body, neither, or both? None is
    ///   UiWin::Normal
    /// * moveType - Describes how the window will move when dragged around. None is UiMove::FaceUser
    ///
    /// see also [`crate::ui::ui_window_begin`]
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
    /// see also [`crate::ui::ui_window_end`]
    pub fn window_end() {
        unsafe { ui_window_end() }
    }

    /// get the flag about the far ray grab interaction for Handle elements like the Windows. It can be enabled and
    /// disabled for individual UI elements, and if this remains disabled at the start of the next frame, then the
    /// hand ray indicators will not be visible. This is enabled by default.
    /// <https://stereokit.net/Pages/StereoKit/UI/EnableFarInteract.html>
    ///
    /// see also [`crate::ui::ui_far_interact_enabled`]
    pub fn get_enable_far_interact() -> bool {
        unsafe { ui_far_interact_enabled() != 0 }
    }

    /// Tells the Active state of the most recently called UI element that used an id.
    /// <https://stereokit.net/Pages/StereoKit/UI/LastElementActive.html>
    ///
    /// see also [`crate::ui::ui_last_element_active`]
    pub fn get_last_element_active() -> BtnState {
        unsafe { ui_last_element_active() }
    }

    /// Tells the Focused state of the most recently called UI element that used an id.
    /// <https://stereokit.net/Pages/StereoKit/UI/LastElementFocused.html>
    ///
    /// see also [`crate::ui::ui_last_element_focused`]
    pub fn get_last_element_focused() -> BtnState {
        unsafe { ui_last_element_focused() }
    }

    /// The hierarchy local position of the current UI layout position. The top left point of the next UI element will
    /// be start here!
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutAt.html>
    ///
    /// see also [`crate::ui::ui_layout_at`]
    pub fn get_layout_at() -> Vec3 {
        unsafe { ui_layout_at() }
    }

    /// These are the layout bounds of the most recently reserved layout space. The Z axis dimensions are always 0.
    /// Only UI elements that affect the surface’s layout will report their bounds here. You can reserve your own layout
    /// space via Ui::layout_reserve, and that call will also report here.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutLast.html>
    ///
    /// see also [`crate::ui::ui_layout_last`]
    pub fn get_layout_last() -> Bounds {
        unsafe { ui_layout_last() }
    }

    /// How much space is available on the current layout! This is based on the current layout position, so X will give
    /// you the amount remaining on the current line, and Y will give you distance to the bottom of the layout,
    /// including the current line. These values will be 0 if you’re using 0 for the layout size on that axis.
    /// <https://stereokit.net/Pages/StereoKit/UI/LayoutRemaining.html>
    ///
    /// see also [`crate::ui::ui_layout_remaining`]
    pub fn get_layout_remaining() -> Vec2 {
        unsafe { ui_layout_remaining() }
    }

    /// This is the height of a single line of text with padding in the UI’s layout system!
    /// <https://stereokit.net/Pages/StereoKit/UI/LineHeight.html>
    ///
    /// see also [`crate::ui::ui_line_height`]
    pub fn get_line_height() -> f32 {
        unsafe { ui_line_height() }
    }

    /// UI sizing and layout settings.
    /// <https://stereokit.net/Pages/StereoKit/UI/Settings.html>
    ///
    /// see also [`crate::ui::ui_get_settings`]
    pub fn get_settings() -> UiSettings {
        unsafe { ui_get_settings() }
    }

    /// This is the UiMove that is provided to UI windows that StereoKit itself manages, such as the fallback
    /// filepicker and soft keyboard.
    /// <https://stereokit.net/Pages/StereoKit/UI/SystemMoveType.html>
    ///
    /// see also [`crate::ui::ui_system_get_move_type`]
    pub fn get_system_move_type() -> UiMove {
        unsafe { ui_system_get_move_type() }
    }

    /// This returns the TextStyle that’s on top of the UI’s stack, according to Ui::(push/pop)_text_style.
    /// <https://stereokit.net/Pages/StereoKit/UI/TextStyle.html>
    ///
    /// see also [`crate::ui::ui_get_text_style`]
    pub fn get_text_style() -> TextStyle {
        unsafe { ui_get_text_style() }
    }

    /// This returns the current state of the UI's enabled status stack, set by `Ui::(push/pop)_enabled`.
    /// <https://stereokit.net/Pages/StereoKit/UI/Enabled.html>
    ///
    /// see also [`crate::ui::ui_is_enabled`]
    pub fn get_enabled() -> bool {
        unsafe { ui_is_enabled() != 0 }
    }
}
