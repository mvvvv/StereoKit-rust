use std::{
    ffi::{CStr, CString, c_char},
    ptr::null_mut,
};

use crate::{
    maths::{Bool32T, Bounds, Pose, Vec2, Vec3},
    sprite::{Sprite, SpriteT},
    system::{Align, TextContext, TextFit},
    ui::{
        UiBtnLayout, UiConfirm, UiGesture, UiMove, UiNotify, UiScroll, UiWin, ui_button, ui_button_at, ui_button_img,
        ui_button_img_at, ui_button_round, ui_button_round_at, ui_handle_begin, ui_handle_end, ui_hslider,
        ui_hslider_at, ui_input, ui_input_at, ui_label, ui_text, ui_text_at, ui_toggle, ui_toggle_at, ui_toggle_img,
        ui_toggle_img_at, ui_vslider, ui_vslider_at, ui_window_begin,
    },
    util::Color128,
};

/// see [`Ui::button`](crate::ui::Ui::button)
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
    pub fn update_text(&mut self, text: impl AsRef<str>) -> &mut Self {
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
    pub fn press(&mut self) -> bool {
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
    pub fn press(&mut self) -> bool {
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
/// [Handle](https://stereokit.net/Pages/StereoKit/UI/Handle.html)
/// [HandleBegin](https://stereokit.net/Pages/StereoKit/UI/HandleBegin.html)
#[must_use = "UiHandleBuilder does nothing until you call .grab() or begin_grab() on it"]
pub struct UiHandleBuilder<'a> {
    id: CString,
    pose: &'a mut Pose,
    handle: Bounds,
    scale: Option<&'a mut f32>,
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
            scale: None,
            draw_handle: false,
            move_type: UiMove::Exact,
            allower_gesture: UiGesture::Pinch,
        }
    }

    /// This additionally supports uniform scaling when two or more interactors grab the handle at the same time. With
    /// a single interactor the handle behaves exactly like the normal handle. With multiple interactors, their motion
    /// is combined into a translation, rotation, and a uniform scale. Interactors may freely join or leave the
    /// interaction without the handle jumping.
    /// Providing a scale here enables scaling; pass [`UIMove::ExactNoscale`] as the moveType if you want multi-
    /// interactor translate/rotate but no scaling.
    ///
    /// * `scale` - A uniform scale multiplier that gets accumulated as the user scales the handle with multiple
    ///   interactors. Seed this with 1 (or your starting scale). Since the Pose has no scale of its own, apply this
    ///   value to your content - the `handle` Bounds are scaled by it for you, so the grab volume and drawn handle stay
    ///   matched.
    pub fn scale(mut self, scale: &'a mut f32) -> Self {
        self.scale = Some(scale);
        self
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
    pub fn begin_grab(&mut self) -> bool {
        let scale = self.scale.as_deref_mut().map_or(null_mut(), |scale| scale as *mut f32);
        unsafe {
            ui_handle_begin(
                self.id.as_ptr(),
                self.pose,
                scale,
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
    pub fn grab(&mut self) -> bool {
        let scale = self.scale.as_deref_mut().map_or(null_mut(), |scale| scale as *mut f32);
        let change = unsafe {
            ui_handle_begin(
                self.id.as_ptr(),
                self.pose,
                scale,
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

/// see [`Ui::input`](crate::ui::Ui::input)
/// StereoKit original docs :
/// [Input](https://stereokit.net/Pages/StereoKit/UI/Input.html)
/// [InputAt](https://stereokit.net/Pages/StereoKit/UI/InputAt.html)
#[must_use = "UiInputBuilder does nothing until you call .edit() on it"]
pub struct UiInputBuilder<'a> {
    id: CString,
    out_value: &'a mut String,
    size: Vec2,
    top_left_corner: Option<Vec3>,
    type_text: TextContext,
}

impl<'a> UiInputBuilder<'a> {
    /// Creates a new input builder.
    pub fn new(id: impl AsRef<str>, out_value: &'a mut String) -> Self {
        Self {
            id: CString::new(id.as_ref()).unwrap_or_default(),
            out_value,
            top_left_corner: None,
            size: Vec2::ZERO,
            type_text: TextContext::Text,
        }
    }

    /// Switches this input to absolute-position mode.
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// The layout size for this element in Hierarchy space. Zero axes will auto-size. None is full auto-size.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// What category of text this Input represents. This may affect what kind of soft keyboard will be displayed, if
    /// one is shown to the user. None has default value of TextContext::Text.
    pub fn type_text(mut self, type_text: TextContext) -> Self {
        self.type_text = type_text;
        self
    }

    /// Executes input interaction.
    ///
    /// Returns the updated text in the input field if it has changed, otherwise `None`.
    pub fn edit(&mut self) -> Option<String> {
        let c_value = CString::new(self.out_value.as_str()).unwrap_or_default();
        let result = match self.top_left_corner {
            Some(top_left_corner) => unsafe {
                ui_input_at(
                    self.id.as_ptr(),
                    c_value.as_ptr() as *mut c_char,
                    self.out_value.capacity() as i32 + 16,
                    top_left_corner,
                    self.size,
                    self.type_text,
                ) != 0
            },
            None => unsafe {
                ui_input(
                    self.id.as_ptr(),
                    c_value.as_ptr() as *mut c_char,
                    self.out_value.capacity() as i32 + 16,
                    self.size,
                    self.type_text,
                ) != 0
            },
        };

        if result {
            match unsafe { CStr::from_ptr(c_value.as_ptr()).to_str() } {
                Ok(result) => {
                    self.out_value.clear();
                    self.out_value.push_str(result);
                    Some(result.to_owned())
                }
                Err(_) => None,
            }
        } else {
            None
        }
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
            text_align: Align::None,
        }
    }

    /// Changes the text of this label.
    pub fn update_text(&mut self, text: impl AsRef<str>) -> &mut Self {
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
    /// Sometimes you just want un-padded text. Default is true.
    pub fn use_padding(mut self, use_padding: bool) -> Self {
        self.use_padding = use_padding;
        self
    }

    /// Where should the text position itself within its bounds? Default is Align::None.
    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    /// Draws the label.
    pub fn draw(&mut self) {
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
    pub fn update_text(&mut self, text: impl AsRef<str>) -> &mut Self {
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
    pub fn press(&mut self) -> bool {
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

/// see [`Ui::hslider`](crate::ui::Ui::hslider) and [`Ui::vslider`](crate::ui::Ui::vslider)
/// StereoKit original docs :
/// [HSlider](https://stereokit.net/Pages/StereoKit/UI/HSlider.html)
/// [HSliderAt](https://stereokit.net/Pages/StereoKit/UI/HSlider.html)
/// [VSlider](https://stereokit.net/Pages/StereoKit/UI/VSlider.html)
/// [VSliderAt](https://stereokit.net/Pages/StereoKit/UI/VSliderAt.html)
#[must_use = "UiSliderBuilder does nothing until you call .interact() on it"]
pub struct UiSliderBuilder<'a> {
    id: CString,
    value: &'a mut f32,
    min: f32,
    max: f32,
    step: f32,
    space: f32,
    confirm_method: UiConfirm,
    notify_on: UiNotify,
    horizontal: bool,
    top_left_corner: Option<Vec3>,
    size: Vec2,
}

impl<'a> UiSliderBuilder<'a> {
    /// Creates a new horizontal slider builder.
    pub fn new_h(id: impl AsRef<str>, out_value: &'a mut f32, min: f32, max: f32) -> Self {
        Self {
            id: CString::new(id.as_ref()).unwrap_or_default(),
            value: out_value,
            min,
            max,
            step: 0.0,
            space: 0.0,
            confirm_method: UiConfirm::Push,
            notify_on: UiNotify::Change,
            horizontal: true,
            top_left_corner: None,
            size: Vec2::ZERO,
        }
    }

    /// Creates a new vertical slider builder.
    pub fn new_v(id: impl AsRef<str>, out_value: &'a mut f32, min: f32, max: f32) -> Self {
        Self {
            id: CString::new(id.as_ref()).unwrap_or_default(),
            value: out_value,
            min,
            max,
            step: 0.0,
            space: 0.0,
            confirm_method: UiConfirm::Push,
            notify_on: UiNotify::Change,
            horizontal: false,
            top_left_corner: None,
            size: Vec2::ZERO,
        }
    }

    /// Switches this slider to absolute-position mode that doesn’t use the layout system, and instead goes exactly
    /// where you put it.
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    /// `size` is the layout size for this element in Hierarchy space.
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// Locks the value to increments of step. Starts at min, and increments by step. Default is 0, which means "don't
    /// lock to increments".
    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Physical width of the slider if horizontal, height if vertical. Default is 0 will fill the remaining amount of
    /// window space. Doesn't mean nothing if the slider is in absolute-position mode.
    pub fn space(mut self, space: f32) -> Self {
        self.space = space;
        self
    }

    /// How should the slider be activated? Default is [`UiConfirm::Push`].
    ///
    /// Push will be a push-button the user must press first, and pinch will be a tab that the user must pinch and drag
    /// around
    pub fn confirm_method(mut self, confirm_method: UiConfirm) -> Self {
        self.confirm_method = confirm_method;
        self
    }

    /// Allows you to modify the behavior of the return value. Default is [`UiNotify::Change`].
    pub fn notify_on(mut self, notify_on: UiNotify) -> Self {
        self.notify_on = notify_on;
        self
    }

    /// Executes slider interaction.
    ///
    /// Returns the updated value when it changes, otherwise `None`.
    pub fn interact(&mut self) -> Option<f32> {
        let changed = if self.horizontal {
            match self.top_left_corner {
                Some(top_left_corner) => unsafe {
                    ui_hslider_at(
                        self.id.as_ptr(),
                        self.value,
                        self.min,
                        self.max,
                        self.step,
                        top_left_corner,
                        self.size,
                        self.confirm_method,
                        self.notify_on,
                    ) != 0
                },
                None => unsafe {
                    ui_hslider(
                        self.id.as_ptr(),
                        self.value,
                        self.min,
                        self.max,
                        self.step,
                        self.space,
                        self.confirm_method,
                        self.notify_on,
                    ) != 0
                },
            }
        } else {
            match self.top_left_corner {
                Some(top_left_corner) => unsafe {
                    ui_vslider_at(
                        self.id.as_ptr(),
                        self.value,
                        self.min,
                        self.max,
                        self.step,
                        top_left_corner,
                        self.size,
                        self.confirm_method,
                        self.notify_on,
                    ) != 0
                },
                None => unsafe {
                    ui_vslider(
                        self.id.as_ptr(),
                        self.value,
                        self.min,
                        self.max,
                        self.step,
                        self.space,
                        self.confirm_method,
                        self.notify_on,
                    ) != 0
                },
            }
        };

        if changed { Some(*self.value) } else { None }
    }
}

/// see [`Ui::text`](crate::ui::Ui::text)
/// StereoKit original docs :
/// [Text](https://stereokit.net/Pages/StereoKit/UI/Text.html)
/// [TextAt](https://stereokit.net/Pages/StereoKit/UI/TextAt.html)
#[must_use = "UiTextBuilder does nothing until you call .draw() on it"]
pub struct UiTextBuilder<'a> {
    text: CString,
    scroll: Option<&'a mut Vec2>,
    scroll_direction: UiScroll,
    top_left_corner: Option<Vec3>,
    size: Vec2,
    text_align: Align,
    fit: TextFit,
}

impl<'a> UiTextBuilder<'a> {
    /// Creates a new text builder.
    pub fn new(text: impl AsRef<str>) -> Self {
        Self {
            text: CString::new(text.as_ref()).unwrap_or_default(),
            scroll: None,
            scroll_direction: UiScroll::None,
            top_left_corner: None,
            size: Vec2::ZERO,
            text_align: Align::TopLeft,
            fit: TextFit::Wrap,
        }
    }

    /// Switches this text to absolute-position mode. Don't use with [`UiTextBuilder::size`] as it will override this
    /// size value.
    ///
    /// `top_left_corner` is relative to the current hierarchy.
    pub fn at(mut self, top_left_corner: impl Into<Vec3>, size: impl Into<Vec2>) -> Self {
        self.top_left_corner = Some(top_left_corner.into());
        self.size = size.into();
        self
    }

    /// What scroll bars are allowed to show on this text?
    ///
    /// Default is [`UiScroll::None`].
    pub fn scroll(mut self, scroll: &'a mut Vec2, scroll_direction: UiScroll) -> Self {
        self.scroll = Some(scroll);
        self.scroll_direction = scroll_direction;
        self
    }

    /// The layout size for this element in Hierarchy space. Don't use with [`UiTextBuilder::at`] as it will override
    /// this value.
    ///
    /// If x is 0.0, it will automatically take the remainder of the current layout. if y is 0.0, it will automatically
    /// size to fit the text.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// Where should the text position itself within its bounds?
    ///
    /// Default is Align::TopLeft.
    pub fn text_align(mut self, text_align: Align) -> Self {
        self.text_align = text_align;
        self
    }

    /// Describes how the text should behave when one of its size dimensions conflicts with the provided
    /// ['UiTextBuilder::size'] parameter.
    ///
    /// Default is [`TextFit::Wrap`] and this scrolling overload will always add [`TextFit::Clip`] internally.
    pub fn fit(mut self, fit: TextFit) -> Self {
        self.fit = fit;
        self
    }

    /// Draws the text.
    pub fn draw(&mut self) -> bool {
        let scroll = match &self.scroll {
            Some(scroll) => *scroll as *const Vec2 as *mut Vec2,
            None => std::ptr::null_mut(),
        };
        match self.top_left_corner {
            Some(top_left_corner) => unsafe {
                ui_text_at(
                    self.text.as_ptr(),
                    scroll,
                    self.scroll_direction,
                    self.text_align,
                    self.fit,
                    top_left_corner,
                    self.size,
                ) != 0
            },
            None => unsafe {
                ui_text(self.text.as_ptr(), scroll, self.scroll_direction, self.size, self.text_align, self.fit) != 0
            },
        }
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
    pub fn interact(&mut self) -> Option<bool> {
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

/// see [`Ui::window`](crate::ui::Ui::window) and [`Ui::window_end`](crate::ui::Ui::window_end)
/// StereoKit original docs :
/// [WindowBegin](https://stereokit.net/Pages/StereoKit/UI/WindowBegin.html)
#[must_use = "UiWindowBuilder does nothing until you call .begin() on it"]
pub struct UiWindowBuilder<'a> {
    text: CString,
    pose: Option<&'a mut Pose>,
    size: Vec2,
    window_type: UiWin,
    move_type: UiMove,
}

impl<'a> UiWindowBuilder<'a> {
    /// Creates a new window builder.
    /// If `pose` is None, it will use an automatically determined pose.
    pub fn new(text: impl AsRef<str>) -> Self {
        Self {
            text: CString::new(text.as_ref()).unwrap_or_default(),
            pose: None,
            size: Vec2::ZERO,
            window_type: UiWin::Normal,
            move_type: UiMove::FaceUser,
        }
    }

    /// Updates the text of this window in case you want to keep the Builder alive (ie: as a IStepper property)
    pub fn update_text(&mut self, text: impl AsRef<str>) -> &mut Self {
        self.text = CString::new(text.as_ref()).unwrap_or_default();
        self
    }

    /// The pose state for the window! With a Window-Head the user will be able to grab this header and move it around.
    ///
    /// Default will push an automatically determined pose onto the transform stack.
    pub fn pose(mut self, pose: &'a mut Pose) -> Self {
        self.pose = Some(pose);
        self
    }

    /// The physical size of the window!
    /// If either dimension is 0, then the size on that axis will be auto-calculated.
    ///
    /// Default is `Vec2::ZERO` to fill both dimensions automatically.
    pub fn size(mut self, size: impl Into<Vec2>) -> Self {
        self.size = size.into();
        self
    }

    /// Describes how the window should be drawn (header, body, neither, or both).
    ///
    /// Default is [`UiWin::Normal`].
    pub fn window_type(mut self, window_type: UiWin) -> Self {
        self.window_type = window_type;
        self
    }

    /// Describes how the window will move when dragged around.
    ///
    /// Default is [`UiMove::FaceUser`].
    pub fn move_type(mut self, move_type: UiMove) -> Self {
        self.move_type = move_type;
        self
    }

    /// Begins a new window! This will push a pose onto the transform stack.
    /// If `pose` is None, it will use an automatically determined pose.
    pub fn begin(&mut self) {
        let pose_ptr = match &self.pose {
            Some(pose) => *pose as *const _ as *mut _,
            None => std::ptr::null_mut(),
        };

        unsafe {
            ui_window_begin(self.text.as_ptr(), pose_ptr, self.size, self.window_type, self.move_type);
        }
    }
}
