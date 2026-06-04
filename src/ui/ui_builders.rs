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

/// see Ui::label_builder
pub struct UiLabelBuilder {
    text: String,
    size: Vec2,
    use_padding: bool,
    text_align: Align,
}

impl UiLabelBuilder {
    pub fn new(text: impl AsRef<str>) -> Self {
        Self { text: text.as_ref().to_owned(), size: Vec2::ZERO, use_padding: true, text_align: Align::empty() }
    }

    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    pub fn use_padding(mut self, use_padding: bool) -> Self {
        self.use_padding = use_padding;
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn draw(self) {
        let cstr = CString::new(self.text).unwrap_or_default();
        unsafe { ui_label(cstr.as_ptr(), self.size, self.use_padding as Bool32T, self.text_align) }
    }
}

/// see Ui::button_builder
pub struct UiButtonBuilder {
    text: String,
    size: Vec2,
    text_align: Align,
}

impl UiButtonBuilder {
    pub fn new(text: impl AsRef<str>) -> Self {
        Self { text: text.as_ref().to_owned(), size: Vec2::ZERO, text_align: Align::empty() }
    }

    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn press(self) -> bool {
        let cstr = CString::new(self.text).unwrap_or_default();
        unsafe { ui_button(cstr.as_ptr(), self.size, self.text_align) != 0 }
    }
}

/// see Ui::button_at_builder
pub struct UiButtonAtBuilder {
    text: String,
    top_left_corner: Vec3,
    size: Vec2,
    text_align: Align,
}

impl UiButtonAtBuilder {
    pub fn new(text: impl AsRef<str>, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        Self {
            text: text.as_ref().to_owned(),
            top_left_corner: top_left_corner.into(),
            size: size.into(),
            text_align: Align::empty(),
        }
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn press(self) -> bool {
        let cstr = CString::new(self.text).unwrap_or_default();
        unsafe { ui_button_at(cstr.as_ptr(), self.top_left_corner, self.size, self.text_align) != 0 }
    }
}

/// see Ui::button_img_builder
pub struct UiButtonImgBuilder {
    text: String,
    image: SpriteT,
    image_layout: UiBtnLayout,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl UiButtonImgBuilder {
    pub fn new(text: impl AsRef<str>, image: impl AsRef<Sprite>) -> Self {
        Self {
            text: text.as_ref().to_owned(),
            image: image.as_ref().0.as_ptr(),
            image_layout: UiBtnLayout::Left,
            size: Vec2::ZERO,
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn press(self) -> bool {
        let cstr = CString::new(self.text).unwrap_or_default();
        unsafe {
            ui_button_img(cstr.as_ptr(), self.image, self.image_layout, self.size, self.image_tint, self.text_align)
                != 0
        }
    }
}

/// see Ui::button_img_at_builder
pub struct UiButtonImgAtBuilder {
    text: String,
    image: SpriteT,
    image_layout: UiBtnLayout,
    top_left_corner: Vec3,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl UiButtonImgAtBuilder {
    pub fn new(
        text: impl AsRef<str>,
        image: impl AsRef<Sprite>,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) -> Self {
        Self {
            text: text.as_ref().to_owned(),
            image: image.as_ref().0.as_ptr(),
            image_layout: UiBtnLayout::Left,
            top_left_corner: top_left_corner.into(),
            size: size.into(),
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn press(self) -> bool {
        let cstr = CString::new(self.text).unwrap_or_default();
        unsafe {
            ui_button_img_at(
                cstr.as_ptr(),
                self.image,
                self.image_layout,
                self.top_left_corner,
                self.size,
                self.image_tint,
                self.text_align,
            ) != 0
        }
    }
}

/// see Ui::toggle_builder
pub struct UiToggleBuilder<'a> {
    text: String,
    out_value: &'a mut bool,
    size: Vec2,
    text_align: Align,
}

impl<'a> UiToggleBuilder<'a> {
    pub fn new(text: impl AsRef<str>, out_value: &'a mut bool) -> Self {
        Self { text: text.as_ref().to_owned(), out_value, size: Vec2::ZERO, text_align: Align::empty() }
    }

    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn interact(self) -> Option<bool> {
        let cstr = CString::new(self.text).unwrap_or_default();
        let mut active: Bool32T = *self.out_value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = unsafe { ui_toggle(cstr.as_ptr(), active_ptr, self.size, self.text_align) != 0 };
        match change {
            true => {
                *self.out_value = active != 0;
                Some(*self.out_value)
            }
            false => None,
        }
    }
}

/// see Ui::toggle_img_builder
pub struct UiToggleImgBuilder<'a> {
    id: String,
    out_value: &'a mut bool,
    toggle_off: SpriteT,
    toggle_on: SpriteT,
    image_layout: UiBtnLayout,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl<'a> UiToggleImgBuilder<'a> {
    pub fn new(
        id: impl AsRef<str>,
        out_value: &'a mut bool,
        toggle_off: impl AsRef<Sprite>,
        toggle_on: impl AsRef<Sprite>,
    ) -> Self {
        Self {
            id: id.as_ref().to_owned(),
            out_value,
            toggle_off: toggle_off.as_ref().0.as_ptr(),
            toggle_on: toggle_on.as_ref().0.as_ptr(),
            image_layout: UiBtnLayout::Left,
            size: Vec2::ZERO,
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn interact(self) -> Option<bool> {
        let cstr = CString::new(self.id).unwrap_or_default();
        let mut active: Bool32T = *self.out_value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = unsafe {
            ui_toggle_img(
                cstr.as_ptr(),
                active_ptr,
                self.toggle_off,
                self.toggle_on,
                self.image_layout,
                self.size,
                self.image_tint,
                self.text_align,
            ) != 0
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

/// see Ui::toggle_at_builder
pub struct UiToggleAtBuilder<'a> {
    id: String,
    out_value: &'a mut bool,
    toggle_images: Option<(SpriteT, SpriteT)>,
    image_layout: UiBtnLayout,
    top_left_corner: Vec3,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl<'a> UiToggleAtBuilder<'a> {
    pub fn new(
        id: impl AsRef<str>,
        out_value: &'a mut bool,
        top_left_corner: impl Into<Vec3>,
        size: impl Into<Vec2>,
    ) -> Self {
        Self {
            id: id.as_ref().to_owned(),
            out_value,
            toggle_images: None,
            image_layout: UiBtnLayout::Left,
            top_left_corner: top_left_corner.into(),
            size: size.into(),
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    pub fn toggle_images(mut self, toggle_off: impl AsRef<Sprite>, toggle_on: impl AsRef<Sprite>) -> Self {
        self.toggle_images = Some((toggle_off.as_ref().0.as_ptr(), toggle_on.as_ref().0.as_ptr()));
        self
    }

    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn interact(self) -> Option<bool> {
        let cstr = CString::new(self.id).unwrap_or_default();
        let mut active: Bool32T = *self.out_value as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        let change = match self.toggle_images {
            Some((toggle_off, toggle_on)) => unsafe {
                ui_toggle_img_at(
                    cstr.as_ptr(),
                    active_ptr,
                    toggle_off,
                    toggle_on,
                    self.image_layout,
                    self.top_left_corner,
                    self.size,
                    self.image_tint,
                    self.text_align,
                ) != 0
            },
            None => unsafe {
                ui_toggle_at(cstr.as_ptr(), active_ptr, self.top_left_corner, self.size, self.text_align) != 0
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

/// see Ui::radio_builder
pub struct UiRadioBuilder {
    text: String,
    active: bool,
    image_off: SpriteT,
    image_on: SpriteT,
    image_layout: UiBtnLayout,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl UiRadioBuilder {
    pub fn new(text: impl AsRef<str>, active: bool) -> Self {
        Self {
            text: text.as_ref().to_owned(),
            active,
            image_off: Sprite::radio_off().0.as_ptr(),
            image_on: Sprite::radio_on().0.as_ptr(),
            image_layout: UiBtnLayout::Left,
            size: Vec2::ZERO,
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    pub fn images(mut self, image_off: impl AsRef<Sprite>, image_on: impl AsRef<Sprite>) -> Self {
        self.image_off = image_off.as_ref().0.as_ptr();
        self.image_on = image_on.as_ref().0.as_ptr();
        self
    }

    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn press(self) -> bool {
        let cstr = CString::new(self.text).unwrap_or_default();
        let mut active: Bool32T = self.active as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        unsafe {
            ui_toggle_img(
                cstr.as_ptr(),
                active_ptr,
                self.image_off,
                self.image_on,
                self.image_layout,
                self.size,
                self.image_tint,
                self.text_align,
            ) != 0
                && !self.active
        }
    }
}

/// see Ui::radio_at_builder
pub struct UiRadioAtBuilder {
    text: String,
    active: bool,
    image_off: SpriteT,
    image_on: SpriteT,
    image_layout: UiBtnLayout,
    top_left_corner: Vec3,
    size: Vec2,
    image_tint: Color128,
    text_align: Align,
}

impl UiRadioAtBuilder {
    pub fn new(text: impl AsRef<str>, active: bool, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        Self {
            text: text.as_ref().to_owned(),
            active,
            image_off: Sprite::radio_off().0.as_ptr(),
            image_on: Sprite::radio_on().0.as_ptr(),
            image_layout: UiBtnLayout::Left,
            top_left_corner: top_left_corner.into(),
            size: size.into(),
            image_tint: Color128::WHITE,
            text_align: Align::empty(),
        }
    }

    pub fn images(mut self, image_off: impl AsRef<Sprite>, image_on: impl AsRef<Sprite>) -> Self {
        self.image_off = image_off.as_ref().0.as_ptr();
        self.image_on = image_on.as_ref().0.as_ptr();
        self
    }

    pub fn image_layout(mut self, image_layout: UiBtnLayout) -> Self {
        self.image_layout = image_layout;
        self
    }

    pub fn image_tint(mut self, image_tint: impl Into<Color128>) -> Self {
        self.image_tint = image_tint.into();
        self
    }

    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    pub fn press(self) -> bool {
        let cstr = CString::new(self.text).unwrap_or_default();
        let mut active: Bool32T = self.active as Bool32T;
        let active_ptr: *mut Bool32T = &mut active;
        unsafe {
            ui_toggle_img_at(
                cstr.as_ptr(),
                active_ptr,
                self.image_off,
                self.image_on,
                self.image_layout,
                self.top_left_corner,
                self.size,
                self.image_tint,
                self.text_align,
            ) != 0
                && !self.active
        }
    }
}
