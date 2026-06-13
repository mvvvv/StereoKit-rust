// SPDX-License-Identifier: MIT
//! This is a copycat of https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Demos/DemoPermissions.cs

use std::{cell::RefCell, rc::Rc};
use stereokit_rust::{
    font::Font,
    maths::{Matrix, Quat, Vec3},
    permission::{Permission, PermissionState, PermissionType},
    prelude::*,
    system::{Align, Text, TextStyle},
    tools::os_api::{SystemAction, system_deep_link},
    ui::{Ui, UiPad},
    util::named_colors::{CYAN, RED},
};

/// Demo that shows permission handling functionality
#[derive(IStepper)]
pub struct Permission1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    title: String,
    description: String,
    style_description: TextStyle,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Permission1 {}

impl Default for Permission1 {
    fn default() -> Self {
        let title = "Permissions".to_string();
        let description = "StereoKit comes with APIs for managing cross-platform 
permissions. While on Desktop, permissions are almost not an issue,
platforms like Android can get kinda complicated! Here, StereoKit 
provides an API that handles the more complicated permission issues 
of platforms like Android, and also can be a regular part of your 
desktop code!"
            .to_string();

        let style_description = TextStyle::from_font(Font::default(), 0.03, CYAN);
        Self {
            id: "Permission1".to_string(),
            sk_info: None,

            title,
            description,
            style_description,

            text: "Permission1".to_owned(), // Default text.
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r(
                (Vec3::NEG_Z * 2.5) + Vec3::Y, //
                Quat::from_angles(0.0, 180.0, 0.0),
            ),
        }
    }
}

impl Permission1 {
    /// Called by derive macro during IStepper::initialize
    fn start(&mut self) -> bool {
        true
    }

    /// Called by derive macro during IStepper::step
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::window_begin_auto(&self.title, Some([0.4, 0.0].into()), None, None);

        Ui::push_text_style(self.style_description);
        Ui::label(&self.description).size([0.395, 0.12]).use_padding(false).draw();
        Ui::pop_text_style();
        Ui::hseparator();

        // Iterate through all permission types
        for i in 0..(PermissionType::Max as u32) {
            let permission = unsafe { std::mem::transmute::<u32, PermissionType>(i) };
            let state = Permission::get_state(permission);

            Ui::push_id(format!("perm_{}", i));

            Ui::panel_begin(Some(UiPad::Inside));
            // Column 1: Permission name with fixed width
            Ui::text(format!("{}:", permission)).size([0.1, 0.03]).text_align(Align::CenterLeft).draw();

            // Column 2: State with fixed width on same line
            Ui::same_line();
            Ui::text(format!("{}", state)).size([0.08, 0.03]).text_align(Align::CenterLeft).draw();

            // Column 3: Interactive indicator with fixed width
            if Permission::is_interactive(permission) {
                Ui::same_line();
                Ui::text("(interactive)").size([0.1, 0.03]).text_align(Align::CenterLeft).draw();
            }

            // Column 4: Request button with fixed size
            if state == PermissionState::Capable {
                Ui::same_line();
                if Ui::button("Request").size([0.05, 0.0]).press() {
                    Permission::request(permission);
                }
            }

            Ui::panel_end();
            Ui::pop_id();
        }
        if cfg!(target_os = "android") && Ui::button("App Settings").press() {
            Ui::hseparator();
            let result = system_deep_link(SystemAction::Settings {
                setting: Some("/applications?package=com.stereokit.rust_binding.demos".to_string()),
            });
            Log::info(format!("Open VR Shell App Settings - Result: {:?}", result));
        }
        Ui::window_end();

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }

    /// Called by derive macro for event handling
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}
}
