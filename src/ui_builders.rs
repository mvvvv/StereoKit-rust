use std::ffi::CString;

use crate::{
    maths::{Bool32T, Vec2, Vec3},
    sprite::{Sprite, SpriteT},
    system::Align,
    ui::{
        UiBtnLayout, ui_button, ui_button_at, ui_button_img, ui_button_img_at, ui_label, ui_toggle, ui_toggle_at,
        ui_toggle_img, ui_toggle_img_at,
    },
    util::Color128,
};

/// see [`Ui::label`](crate::ui::Ui::label)
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

/// see [`Ui::button`](crate::ui::Ui::button) and related button builders:`
///     [`Ui::button_at`](crate::ui::Ui::button_at)  
///     [`Ui::button_img`](crate::ui::Ui::button_img)  
///     [`Ui::button_img_at`](crate::ui::Ui::button_img_at)
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

    /// Switches this button to absolute-position mode (`button_at`).
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// Sets the image drawn with button text (`button_img`).
    pub fn image(mut self, image: impl AsRef<Sprite>) -> Self {
        self.image = Some(image.as_ref().0.as_ptr());
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

pub type UiButtonAtBuilder = UiButtonBuilder;
pub type UiButtonImgBuilder = UiButtonBuilder;
pub type UiButtonImgAtBuilder = UiButtonBuilder;

/// see [`Ui::toggle`](crate::ui::Ui::toggle) and related toggle builders:
///     [`Ui::toggle_at`](crate::ui::Ui::toggle_at)  
///     [`Ui::toggle_img`](crate::ui::Ui::toggle_img)  
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
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
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

    /// Sets the images used when toggle is off/on.
    ///
    /// This enables image-toggle behavior.
    pub fn toggle_images(mut self, toggle_off: impl AsRef<Sprite>, toggle_on: impl AsRef<Sprite>) -> Self {
        self.toggle_images = Some((toggle_off.as_ref().0.as_ptr(), toggle_on.as_ref().0.as_ptr()));
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

pub type UiToggleImgBuilder<'a> = UiToggleBuilder<'a>;
pub type UiToggleAtBuilder<'a> = UiToggleBuilder<'a>;

/// see [`Ui::radio`](crate::ui::Ui::radio) and related radio builders:
///     [`Ui::radio_at`](crate::ui::Ui::radio_at)
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
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// Sets the images used when radio is off/on.
    pub fn images(mut self, image_off: impl AsRef<Sprite>, image_on: impl AsRef<Sprite>) -> Self {
        self.image_off = image_off.as_ref().0.as_ptr();
        self.image_on = image_on.as_ref().0.as_ptr();
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

pub type UiRadioAtBuilder = UiRadioBuilder;
