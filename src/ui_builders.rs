use std::ffi::CString;

use crate::{
    maths::{Bool32T, Bounds, Pose, Vec2, Vec3},
    sprite::{Sprite, SpriteT},
    system::Align,
    ui::{
        UiBtnLayout, UiGesture, UiMove, ui_button, ui_button_at, ui_button_img, ui_button_img_at, ui_button_round,
        ui_button_round_at, ui_handle_begin, ui_handle_end, ui_label, ui_toggle, ui_toggle_at, ui_toggle_img,
        ui_toggle_img_at,
    },
    util::Color128,
};

/// see [`Ui::button`](crate::ui::Ui::button)
///
/// StereoKit original docs :
/// [Button](https://stereokit.net/Pages/StereoKit/UI/Button.html)
/// [ButtonAt](https://stereokit.net/Pages/StereoKit/UI/ButtonAt.html)
/// [ButtonAtImg](https://stereokit.net/Pages/StereoKit/UI/ButtonAtImg.html)
/// [ButtonImg](https://stereokit.net/Pages/StereoKit/UI/ButtonImg.html)
#[must_use = "UiButtonBuilder does nothing until you call .press() on it"]
pub struct UiButtonBuilder {
    text: CString,
    image: Option<SpriteT>,
    image_layout: UiBtnLayout,
    top_left_corner: Option<Vec3>,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl UiButtonBuilder {
    /// Creates a new button builder.
    ///
    /// A button will expand to fit the text provided to it, vertically and horizontally.
    pub fn new(text: impl AsRef<str>) -> Self {
        Self {
            text: CString::new(text.as_ref()).unwrap_or_default(),
            image: None,
            image_layout: UiBtnLayout::Left,
            top_left_corner: None,
            size: Vec2::ZERO,
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    /// Switches this button to absolute-position mode.
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    /// [ButtonAt](https://stereokit.net/Pages/StereoKit/UI/ButtonAt.html)
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// Sets the image drawn with button text.
    ///
    /// [ButtonImg](https://stereokit.net/Pages/StereoKit/UI/ButtonImg.html)
    pub fn image(mut self, image: impl AsRef<Sprite>) -> Self {
        self.image = Some(image.as_ref().0.as_ptr());
        self
    }

    /// Change the text of this button in case you want to keep the Builder alive (ie: as a IStepper property)
    pub fn update_text(mut self, text: impl AsRef<str>) -> Self {
        self.text = CString::new(text.as_ref()).unwrap_or_default();
        self
    }

    /// Specifies how text and image are laid out on the button.
    ///
    /// If not set, default value is [`UiBtnLayout::Left`].
    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    /// The layout size for this element in Hierarchy space.
    ///
    /// If an axis is left as zero, it will be auto-calculated.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// The Sprite's color will be multiplied by this tint.
    ///
    /// If not set, default value is white.
    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    /// Where should the text position itself within its bounds?
    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    /// Executes the button interaction.
    ///
    /// Returns `true` only on the first frame it is pressed.
    pub fn press(self) -> bool {
        match (self.image, self.top_left_corner) {
            (None, None) => unsafe { ui_button(self.text.as_ptr(), self.size, self.text_align) != 0 },
            (None, Some(top_left_corner)) => unsafe {
                ui_button_at(self.text.as_ptr(), top_left_corner, self.size, self.text_align) != 0
            },
            (Some(image), None) => unsafe {
                ui_button_img(self.text.as_ptr(), image, self.image_layout, self.size, self.image_tint, self.text_align)
                    != 0
            },
            (Some(image), Some(top_left_corner)) => unsafe {
                ui_button_img_at(
                    self.text.as_ptr(),
                    image,
                    self.image_layout,
                    top_left_corner,
                    self.size,
                    self.image_tint,
                    self.text_align,
                ) != 0
            },
        }
    }
}

/// see [`Ui::button_round`](crate::ui::Ui::button_round)
/// StereoKit original docs :
/// [Button](https://stereokit.net/Pages/StereoKit/UI/ButtonRound.html)
/// [ButtonAt](https://stereokit.net/Pages/StereoKit/UI/ButtonRoundAt.html)
#[must_use = "UiButtonRoundBuilder does nothing until you call .press() on it"]
pub struct UiButtonRoundBuilder {
    id: CString,
    image: SpriteT,
    diameter: f32,
    top_left_corner: Option<Vec3>,
}

impl UiButtonRoundBuilder {
    /// Creates a new button round builder.
    pub fn new(id: impl AsRef<str>, image: impl AsRef<Sprite>, diameter: f32) -> Self {
        Self {
            id: CString::new(id.as_ref()).unwrap_or_default(),
            image: image.as_ref().0.as_ptr(),
            diameter,
            top_left_corner: None,
        }
    }

    /// Switches this button to absolute-position mode.
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    ///
    /// [ButtonAt](https://stereokit.net/Pages/StereoKit/UI/ButtonRoundAt.html)
    pub fn at(mut self, top_left_corner: impl Into<Vec3>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self
    }

    /// Executes the button interaction.
    ///
    /// Returns `true` only on the first frame it is pressed.
    pub fn press(self) -> bool {
        match self.top_left_corner {
            None => unsafe { ui_button_round(self.id.as_ptr(), self.image, self.diameter) != 0 },
            Some(top_left_corner) => unsafe {
                ui_button_round_at(self.id.as_ptr(), self.image, top_left_corner, self.diameter) != 0
            },
        }
    }
}

/// see [`Ui::handle`](crate::ui::Ui::handle) and [`Ui::handle_end`](crate::ui::Ui::handle_end)
/// StereoKit original docs :
/// [Handle](https://stereokit.net/Pages/StereoKit/UI/Handle.html
/// [HandleBegin](https://stereokit.net/Pages/StereoKit/UI/HandleBegin.html)
#[must_use = "UiHandleBuilder does nothing until you call .grab() or begin_grab() on it"]
pub struct UiHandleBuilder<'a> {
    id: CString,
    pose: &'a mut Pose,
    handle: Bounds,
    draw_handle: bool,
    move_type: UiMove,
    allower_gesture: UiGesture,
}

impl<'a> UiHandleBuilder<'a> {
    /// Creates a new handle builder.
    pub fn new(id: impl AsRef<str>, pose: &'a mut Pose, handle: Bounds) -> Self {
        Self {
            id: CString::new(id.as_ref()).unwrap_or_default(),
            pose,
            handle,
            draw_handle: false,
            move_type: UiMove::Exact,
            allower_gesture: UiGesture::Pinch,
        }
    }

    /// Should the handle's bounds be drawn? Default is false.
    pub fn draw_handle(mut self, draw_handle: bool) -> Self {
        self.draw_handle = draw_handle;
        self
    }

    /// What kind of movement should this handle allow? Default is [`UiMove::Exact`]
    pub fn move_type(mut self, move_type: UiMove) -> Self {
        self.move_type = move_type;
        self
    }

    /// What gesture should be used to interact with this handle? Default is [`UiGesture::Pinch`]
    pub fn allower_gesture(mut self, allower_gesture: UiGesture) -> Self {
        self.allower_gesture = allower_gesture;
        self
    }

    /// This begins a new UI group with its own layout! Much like a window, except with a more flexible handle, and no
    /// header. You can draw the handle, but it will have no text on it. The pose value is always relative to the
    /// current hierarchy stack. This call will also push the pose transform onto the hierarchy stack, so any objects
    /// drawn up to the corresponding [`Ui::handle_end()`](crate::ui::Ui::handle_end) will get transformed by the
    /// handle pose.
    ///
    /// Returns true for every frame the user is grabbing the handle and the pose has been changed.
    pub fn begin_grab(self) -> bool {
        unsafe {
            ui_handle_begin(
                self.id.as_ptr(),
                self.pose,
                self.handle,
                self.draw_handle as Bool32T,
                self.move_type,
                self.allower_gesture,
            ) != 0
        }
    }

    /// Executes the handle interaction.
    ///
    /// Returns true for every frame the user is grabbing the handle and the pose has been changed.
    pub fn grab(self) -> bool {
        let change = unsafe {
            ui_handle_begin(
                self.id.as_ptr(),
                self.pose,
                self.handle,
                self.draw_handle as Bool32T,
                self.move_type,
                self.allower_gesture,
            ) != 0
        };
        unsafe {
            ui_handle_end();
        }
        change
    }
}

/// see [`Ui::label`](crate::ui::Ui::label)
/// StereoKit original docs :
/// [Label](https://stereokit.net/Pages/StereoKit/UI/Label.html)
#[must_use = "UiLabelBuilder does nothing until you call .draw() on it"]
pub struct UiLabelBuilder {
    text: CString,
    size: Vec2,
    use_padding: bool,
    text_align: Align,
}

impl UiLabelBuilder {
    /// Creates a new label builder.
    pub fn new(text: impl AsRef<str>) -> Self {
        Self {
            text: CString::new(text.as_ref()).unwrap_or_default(),
            size: Vec2::ZERO,
            use_padding: true,
            text_align: Align::empty(),
        }
    }

    /// Changes the text of this label.
    pub fn update_text(mut self, text: impl AsRef<str>) -> Self {
        self.text = CString::new(text.as_ref()).unwrap_or_default();
        self
    }

    /// The layout size for this element in Hierarchy space.
    ///
    /// If an axis is left as zero, it will be auto-calculated.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// Should padding be included for positioning this text?
    ///
    /// Sometimes you just want un-padded text.
    pub fn use_padding(mut self, use_padding: bool) -> Self {
        self.use_padding = use_padding;
        self
    }

    /// Where should the text position itself within its bounds?
    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    /// Draws the label.
    pub fn draw(self) {
        unsafe { ui_label(self.text.as_ptr(), self.size, self.use_padding as Bool32T, self.text_align) }
    }
}

/// see [`Ui::radio`](crate::ui::Ui::radio)
/// StereoKit original docs :
/// [Radio](https://stereokit.net/Pages/StereoKit/UI/Radio.html)
/// [RadioAt](https://stereokit.net/Pages/StereoKit/UI/RadioAt.html)
#[must_use = "UiRadioBuilder does nothing until you call .press() on it"]
pub struct UiRadioBuilder {
    text: CString,
    active: bool,
    image_off: SpriteT,
    image_on: SpriteT,
    image_layout: UiBtnLayout,
    top_left_corner: Option<Vec3>,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl UiRadioBuilder {
    /// Creates a new radio builder.
    pub fn new(text: impl AsRef<str>, active: bool) -> Self {
        Self {
            text: CString::new(text.as_ref()).unwrap_or_default(),
            active,
            image_off: std::ptr::null_mut(),
            image_on: std::ptr::null_mut(),
            image_layout: UiBtnLayout::Left,
            top_left_corner: None,
            size: Vec2::ZERO,
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    /// Switches this radio to absolute-position mode (`radio_at`).
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    /// [RadioAt](https://stereokit.net/Pages/StereoKit/UI/RadioAt.html)
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// Sets the images used when radio is off/on. You have to set two different images if you want to see the state
    /// of the radio button.
    pub fn images(mut self, image_off: impl AsRef<Sprite>, image_on: impl AsRef<Sprite>) -> Self {
        self.image_off = image_off.as_ref().0.as_ptr();
        self.image_on = image_on.as_ref().0.as_ptr();
        self
    }

    /// Change the text of this radio in case you want to keep the Builder alive (ie: as a IStepper property)
    pub fn update_text(mut self, text: impl AsRef<str>) -> Self {
        self.text = CString::new(text.as_ref()).unwrap_or_default();
        self
    }

    /// Specifies how text and image are laid out on the radio.
    ///
    /// If not set, default value is [`UiBtnLayout::Left`].
    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    /// The layout size for this element in Hierarchy space.
    ///
    /// If an axis is left as zero, it will be auto-calculated.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// The Sprite's color will be multiplied by this tint.
    ///
    /// If not set, default value is white.
    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    /// Where should the text position itself within its bounds?
    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    /// Executes radio interaction.
    ///
    /// Returns `true` only when pressed and previously inactive.
    pub fn press(self) -> bool {
        let mut active: Bool32T = self.active as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let pressed = match self.top_left_corner {
            Some(top_left_corner) => unsafe {
                ui_toggle_img_at(
                    self.text.as_ptr(),
                    active_ptr,
                    self.image_off,
                    self.image_on,
                    self.image_layout,
                    top_left_corner,
                    self.size,
                    self.image_tint,
                    self.text_align,
                ) != 0
            },
            None => unsafe {
                ui_toggle_img(
                    self.text.as_ptr(),
                    active_ptr,
                    self.image_off,
                    self.image_on,
                    self.image_layout,
                    self.size,
                    self.image_tint,
                    self.text_align,
                ) != 0
            },
        };
        pressed && !self.active
    }
}

/// see [`Ui::toggle`](crate::ui::Ui::toggle)
/// StereoKit original docs :
/// [Toggle](https://stereokit.net/Pages/StereoKit/UI/Toggle.html)
/// [ToggleAt](https://stereokit.net/Pages/StereoKit/UI/ToggleAt.html)
/// [ToggleImg](https://stereokit.net/Pages/StereoKit/UI/ToggleImg.html)
#[must_use = "UiToggleBuilder does nothing until you call .interact() on it"]
pub struct UiToggleBuilder<'a> {
    id: CString,
    out_value: &'a mut bool,
    toggle_images: Option<(SpriteT, SpriteT)>,
    image_layout: UiBtnLayout,
    top_left_corner: Option<Vec3>,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl<'a> UiToggleBuilder<'a> {
    /// Creates a new toggle builder.
    pub fn new(text: impl AsRef<str>, out_value: &'a mut bool) -> Self {
        Self {
            id: CString::new(text.as_ref()).unwrap_or_default(),
            out_value,
            toggle_images: None,
            image_layout: UiBtnLayout::Left,
            top_left_corner: None,
            size: Vec2::ZERO,
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    /// Switches this toggle to absolute-position mode (`toggle_at`).
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    /// [ToggleAt](https://stereokit.net/Pages/StereoKit/UI/ToggleAt.html)
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// Sets the images used when toggle is off/on.
    ///
    /// This change the default image
    /// [ToggleImg](https://stereokit.net/Pages/StereoKit/UI/ToggleImg.html)
    pub fn images(mut self, toggle_off: impl AsRef<Sprite>, toggle_on: impl AsRef<Sprite>) -> Self {
        self.toggle_images = Some((toggle_off.as_ref().0.as_ptr(), toggle_on.as_ref().0.as_ptr()));
        self
    }

    /// Change the id/text of this toggle in case you want to keep the Builder alive (ie: as a IStepper property)
    pub fn update_id(mut self, text: impl AsRef<str>) -> Self {
        self.id = CString::new(text.as_ref()).unwrap_or_default();
        self
    }

    /// The layout size for this element in Hierarchy space.
    ///
    /// If an axis is left as zero, it will be auto-calculated.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// Specifies how text and image are laid out on the toggle.
    ///
    /// If not set, default value is [`UiBtnLayout::Left`].
    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    /// The Sprite's color will be multiplied by this tint.
    ///
    /// If not set, default value is white.
    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    /// Where should the text position itself within its bounds?
    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    /// Executes toggle interaction.
    ///
    /// Returns the updated value when it changes, otherwise `None`.
    pub fn interact(self) -> Option<bool> {
        let mut active: Bool32T = *self.out_value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = match (self.toggle_images, self.top_left_corner) {
            (None, None) => unsafe { ui_toggle(self.id.as_ptr(), active_ptr, self.size, self.text_align) != 0 },
            (Some((toggle_off, toggle_on)), None) => unsafe {
                ui_toggle_img(
                    self.id.as_ptr(),
                    active_ptr,
                    toggle_off,
                    toggle_on,
                    self.image_layout,
                    self.size,
                    self.image_tint,
                    self.text_align,
                ) != 0
            },
            (None, Some(top_left_corner)) => unsafe {
                ui_toggle_at(self.id.as_ptr(), active_ptr, top_left_corner, self.size, self.text_align) != 0
            },
            (Some((toggle_off, toggle_on)), Some(top_left_corner)) => unsafe {
                ui_toggle_img_at(
                    self.id.as_ptr(),
                    active_ptr,
                    toggle_off,
                    toggle_on,
                    self.image_layout,
                    top_left_corner,
                    self.size,
                    self.image_tint,
                    self.text_align,
                ) != 0
            },
        };
        match change {
            true => {
                *self.out_value = active != 0;
                Some(*self.out_value)
            }
            false => None,
        }
    }
}
