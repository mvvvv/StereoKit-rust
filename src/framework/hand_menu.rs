use crate::{
    material::Material,
    maths::{Matrix, Plane, Pose, Quat, Vec2, Vec3, lerp, units::CM},
    mesh::{Inds, Mesh, Vertex},
    prelude::*,
    sound::Sound,
    system::{
        Backend, BackendXRType, FingerId, Hand, Handed, Hierarchy, Input, JointId, Key, Lines, Text, TextAlign,
        TextStyle,
    },
    tex::Tex,
    ui::{Ui, UiColor},
    util::{
        Color128, Time,
        named_colors::{GREEN, WHITE},
    },
};
use std::{borrow::BorrowMut, collections::VecDeque};

/// StereoKit initialization settings! Setup SkSettings with your data before calling SkSetting.Init().
/// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuItem.html>
pub struct HandMenuItem {
    pub name: String,
    pub image: Option<Material>,
    pub action: RefCell<HandMenuAction>,
    pub callback: RefCell<Box<dyn FnMut()>>,
}

impl HandMenuItem {
    /// Makes a menu item!
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuItem/HandMenuItem.html>
    pub fn new<C: FnMut() + 'static>(
        name: impl AsRef<str>,
        image: Option<Material>,
        callback: C,
        action: HandMenuAction,
    ) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            image,
            callback: RefCell::new(Box::new(callback)),
            action: RefCell::<HandMenuAction>::new(action),
        }
    }

    /// This draws the menu item on the radial menu!
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuItem/Draw.html>
    pub fn draw_basic(&self, token: &MainThreadToken, at: Vec3, focused: bool) {
        let scale = match focused {
            true => Vec3::ONE * 0.6,
            false => Vec3::ONE * 0.5,
        };
        Text::add_at(
            token,
            &self.name,
            Matrix::ts(at, scale),
            None,
            None,
            None,
            Some(TextAlign::BottomCenter),
            None,
            None,
            None,
        );
    }
}

/// A Cell of the radial menu which can be a [layer][HandRadialLayer] or an [item][HandMenuItem].
pub enum HandRadial {
    Item(HandMenuItem),
    Layer(HandRadialLayer),
}

impl HandRadial {
    pub fn item<C: FnMut() + 'static>(
        name: impl AsRef<str>,
        image: Option<Material>,
        callback: C,
        action: HandMenuAction,
    ) -> Self {
        Self::Item(HandMenuItem::new(name, image, callback, action))
    }

    pub fn layer(
        name: impl AsRef<str>,
        image: Option<Material>,
        start_angle: Option<f32>,
        items: Vec<HandRadial>,
    ) -> Self {
        Self::Layer(HandRadialLayer::new(name, image, start_angle, items))
    }

    pub fn items_count(&self) -> usize {
        match self {
            HandRadial::Item(_) => 0,
            HandRadial::Layer(layer) => layer.items.len(),
        }
    }

    pub fn items(&self) -> &Vec<Rc<HandRadial>> {
        match self {
            HandRadial::Item(_) => todo!(),
            HandRadial::Layer(layer) => &layer.items,
        }
    }

    pub fn is_back_action(&self) -> bool {
        match self {
            HandRadial::Item(item) => {
                let value = item.action.borrow();
                *value == HandMenuAction::Back
            }
            HandRadial::Layer(_) => false,
        }
    }

    pub fn is_checked_action(&self) -> Option<u8> {
        match self {
            HandRadial::Item(item) => {
                let value = item.action.borrow();
                if let HandMenuAction::Checked(group) = *value { Some(group) } else { None }
            }
            HandRadial::Layer(_) => None,
        }
    }

    pub fn is_unchecked_action(&self) -> Option<u8> {
        match self {
            HandRadial::Item(item) => {
                let value = item.action.borrow();
                if let HandMenuAction::Unchecked(group) = *value { Some(group) } else { None }
            }
            HandRadial::Layer(_) => None,
        }
    }

    pub fn get_start_angle(&self) -> f32 {
        match self {
            HandRadial::Item(_) => 0.0,
            HandRadial::Layer(layer) => layer.start_angle,
        }
    }

    pub fn get_back_angle(&self) -> f32 {
        match self {
            HandRadial::Item(_) => 0.0,
            HandRadial::Layer(layer) => layer.back_angle,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            HandRadial::Item(item) => &item.name,
            HandRadial::Layer(layer) => &layer.layer_name,
        }
    }
}

/// This is a collection of display and behavior information for a single item on the hand menu.
/// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer.html>
pub struct HandRadialLayer {
    pub layer_name: String,
    pub items: Vec<Rc<HandRadial>>,
    pub start_angle: f32,
    pub back_angle: f32,
    parent: Option<String>,
    layer_item: HandMenuItem,
}

/// Creates a menu layer, this overload will calculate a back_angle if there are any back actions present in the item
/// list.
/// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/HandRadialLayer.html>
impl HandRadialLayer {
    pub fn new(
        name: impl AsRef<str>,
        image: Option<Material>,
        start_angle_opt: Option<f32>,
        items_in: Vec<HandRadial>,
    ) -> Self {
        let name = name.as_ref().to_owned();
        let mut items = vec![];
        for item in items_in {
            items.push(Rc::new(item));
        }

        let mut back_angle = 0.0;
        let mut start_angle = 0.0;
        match start_angle_opt {
            Some(value) => start_angle = value,
            None => {
                let mut i = 0.0;
                for item in items.iter() {
                    if item.is_back_action() {
                        let step = 360.0 / (items.len() as f32);
                        back_angle = (i + 0.5) * step;
                    }
                    i += 1.0;
                }
            }
        }

        Self {
            layer_name: name.clone(),
            items,
            start_angle,
            back_angle,
            parent: None,
            layer_item: HandMenuItem::new(name.clone(), image, || {}, HandMenuAction::Callback),
        }
    }

    /// This adds a menu layer as a child item of this layer.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/AddChild.html>
    pub fn add_child(&mut self, mut layer: HandRadialLayer) -> &mut Self {
        layer.parent = Some(self.layer_name.clone());
        self.items.push(Rc::new(HandRadial::Layer(layer)));
        self
    }

    /// Find a child menu layer by name. Recursive function
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/FindChild.html>
    pub fn find_child(&self, name: impl AsRef<str>) -> Option<&HandRadialLayer> {
        for line in self.items.iter() {
            let line = line.as_ref();
            match line {
                HandRadial::Layer(s) => {
                    if s.layer_name.eq(&name.as_ref().to_string()) {
                        return Some(s);
                    } else if let Some(sub_s) = s.find_child(&name) {
                        return Some(sub_s);
                    };
                }
                HandRadial::Item(_) => {}
            }
        }

        None
    }

    /// Finds the layer in the list of child layers, and removes it, if it exists.
    /// Not recursive. self must be the layer containing the one to delete
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/RemoveChild.html>
    pub fn remove_child(&mut self, name: impl AsRef<str>) -> bool {
        for (index, line) in self.items.iter().enumerate() {
            let line = line.as_ref();
            match line {
                HandRadial::Layer(s) => {
                    if s.layer_name.eq(&name.as_ref().to_string()) {
                        self.items.remove(index);
                        return true;
                    }
                }
                HandRadial::Item(_) => {}
            }
        }

        false
    }

    /// This appends a new menu item to the end of the menu’s list.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/AddItem.html>
    pub fn add_item(&mut self, menu_item: HandMenuItem) -> &mut Self {
        self.items.push(Rc::new(HandRadial::Item(menu_item)));
        self
    }

    /// Find a menu item by name.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/FindItem.html>
    pub fn find_item(&mut self, name: impl AsRef<str>) -> Option<&HandMenuItem> {
        for line in self.items.iter() {
            let line = line.as_ref();
            match line {
                HandRadial::Item(s) => {
                    if s.name.eq(name.as_ref()) {
                        return Some(s);
                    }
                }
                HandRadial::Layer(_) => {}
            }
        }

        None
    }

    /// Finds the item in the list, and removes it, if it exists.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/RemoveItem.html>
    pub fn remove_item(&mut self, name: impl AsRef<str>) -> bool {
        for (index, line) in self.items.iter().enumerate() {
            let line = line.as_ref();
            match line {
                HandRadial::Item(s) => {
                    if s.name.eq(name.as_ref()) {
                        self.items.remove(index);
                        return true;
                    }
                }
                HandRadial::Layer(_) => {}
            }
        }

        false
    }
}

/// This enum specifies how HandMenuItems should behave
/// when selected! This is in addition to the HandMenuItem's
/// callback function.
/// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuAction.html>

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum HandMenuAction {
    /// Execute the callback only and stay open (Warning ! this will send multiple time the callback)
    Callback,
    /// Go back to the previous layer.
    Back,
    /// Close the hand menu entirely! We're finished here.
    Close,
    /// Execute the callback only and stay open (Warning ! this will send multiple time the callback)
    /// Mark the Item as checked (could be changed to Unchecked)
    Checked(u8),
    /// Execute the callback only and stay open (Warning ! this will send multiple time the callback)
    /// Mark the Item as unchecked (could be changed to Checked)
    Unchecked(u8),
}

///The way to swap between more than one hand_menu_radial. Use this prefix for your ID to your HandMenuRadial menus
pub const HAND_MENU_RADIAL: &str = "hand_menu_radial_";
///If this menu is the one who takes the focus (true) or if he returns the focus on the menu previously active (false)
pub const HAND_MENU_RADIAL_FOCUS: &str = "hand_menu_radial_focus";

/// A menu that shows up in circle around the user’s hand! Selecting an item can perform an action, or even spawn a
/// sub-layer of menu items. This is an easy way to store actions out of the way, yet remain easily accessible to the
/// user.
///
/// The user faces their palm towards their head, and then makes a grip motion to spawn the menu. The user can then
/// perform actions by making fast, direction based motions that are easy to build muscle memory for.
/// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial.html>
///
/// ### Examples
/// ```
/// # stereokit_rust::test_init_sk!(); // !!!! Get a proper way to initialize sk !!!!
/// use stereokit_rust::{framework::*, material::Material, system::{Input, Key}};
///
/// // swapping a value
/// let mut swap_value = true;
///
/// // nice icon
/// let mut menu_ico = Material::pbr_clip()
///     .copy_for_tex("icons/hamburger.png", true, None).unwrap_or_default();
/// menu_ico.clip_cutoff(0.1);
///
/// //---Create then load hand menu radial
/// let mut hand_menu_stepper =
///     HandMenuRadial::new(HandRadialLayer::new("root", None, Some(100.0),
///     vec![
///         HandRadial::layer("Todo!", Some(menu_ico), None,
///             vec![
///                 HandRadial::item("Back", None, || {}, HandMenuAction::Back),
///                 HandRadial::item("Close", None, || {}, HandMenuAction::Close),
///             ],
///         ),
///         HandRadial::item("Swap", None,
///             move || {
///                 swap_value = !swap_value;
///             },
///             HandMenuAction::Checked(1),
///         ),
///         HandRadial::item("Close", None, || {}, HandMenuAction::Close),
///     ],
/// ));
/// let id = HandMenuRadial::build_id("1");
/// SkInfo::send_event(&Some(sk.get_sk_info_clone()),
///     StepperAction::add(id.clone(), hand_menu_stepper));
///
/// number_of_steps=10;
/// test_steps!(// !!!! Get a proper main loop !!!!
///     if iter == 1 {
///         SkInfo::send_event(&Some(sk.get_sk_info_clone()),
///             StepperAction::event(id.clone(), HAND_MENU_RADIAL_FOCUS, &true.to_string()));
///     }
///     if iter == 8 {
///         SkInfo::send_event(&Some(sk.get_sk_info_clone()),
///             StepperAction::remove(id.clone()));
///     }
/// );
/// ```

#[derive(IStepper)]
pub struct HandMenuRadial {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    enabled: bool,

    menu_stack: Vec<String>,
    menu_pose: Pose,
    dest_pose: Pose,
    root: Rc<HandRadial>,
    active_layer: Rc<HandRadial>,
    last_selected: Rc<HandRadial>,
    last_selected_time: f32,
    nav_stack: VecDeque<Rc<HandRadial>>,
    active_hand: Handed,
    activation: f32,
    menu_scale: f32,
    angle_offset: f32,

    background: Mesh,
    background_edge: Mesh,
    activation_button: Mesh,
    activation_hamburger: Mesh,
    activation_ring: Mesh,
    child_indicator: Mesh,
    img_frame: Mesh,
    pub checked_material: Material,
    pub on_checked_material: Material,
    pub text_style: TextStyle,
}

unsafe impl Send for HandMenuRadial {}

impl HandMenuRadial {
    pub fn build_id(id: &str) -> String {
        format!("{}{}", HAND_MENU_RADIAL, id)
    }

    /// Part of IStepper, you shouldn’t be calling this yourself.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/Initialize.html>
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, id: &StepperId, key: &str, value: &str) {
        if key == HAND_MENU_RADIAL_FOCUS {
            if value.parse().unwrap_or_default() {
                if *id == self.id {
                    self.enabled = true
                } else if self.enabled {
                    self.menu_stack.push(id.clone());
                    self.enabled = false;
                }
            } else if *id == self.id {
                self.enabled = false
            } else {
                if let Some(index) = self.menu_stack.iter().position(|x| *x == *id) {
                    self.menu_stack.remove(index);
                }
                if self.menu_stack.is_empty() {
                    self.enabled = true;
                }
            }
        }
    }

    /// Part of IStepper, you shouldn’t be calling this yourself.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/Step.html>
    fn draw(&mut self, token: &MainThreadToken) {
        if self.active_hand == Handed::Max {
            for hand in [Handed::Left, Handed::Right] {
                self.step_menu_indicator(token, hand);
            }
        } else {
            self.step_menu(token, Input::hand(self.active_hand));
        }
    }

    /// When using the Simulator, this key will activate the menu on the current hand, regardless of which direction it
    /// is facing.
    pub const SIMULATOR_KEY: Key = Key::F1;
    pub const MIN_DIST: f32 = 0.03;
    pub const MID_DIST: f32 = 0.065;
    pub const MAX_DIST: f32 = 0.1;
    pub const MIN_SCALE: f32 = 0.05;
    pub const SLICE_GAP: f32 = 0.002;
    pub const OUT_OF_VIEW_ANGLE: f32 = 0.866;
    pub const ACTIVATION_ANGLE: f32 = 0.978;

    /// Creates a hand menu from the provided array of menu layers! HandMenuRadial is an IStepper, so proper usage is to
    /// add it to the Stepper list via Sk.AddStepper. If no layers are provided to this constructor, a default
    /// root layer will be automatically added.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/HandMenuRadial.html>
    pub fn new(root_layer: HandRadialLayer) -> Self {
        let root = Rc::new(HandRadial::Layer(root_layer));
        let active_layer = root.clone();
        let last_selected = root.clone();
        let last_selected_time = Time::get_total_unscaledf();
        let activation_btn_radius = 1.0 * CM;
        let activation_button = generate_activation_button(activation_btn_radius);
        let activation_hamburger = generate_activation_hamburger(activation_btn_radius);
        let mut activation_ring = Mesh::new();
        generate_slice_mesh(360.0, activation_btn_radius, activation_btn_radius + 0.005, 0.0, &mut activation_ring);
        let child_indicator = generate_child_indicator(Self::MAX_DIST - 0.008, 0.004);
        let img_frame = generate_img_frame(Self::MIN_DIST + 0.013, 0.012);
        let tex_checked = Tex::from_file("icons/radio.png", true, None).unwrap_or_default();
        let mut checked_material = Material::pbr_clip().copy();
        checked_material.diffuse_tex(tex_checked).clip_cutoff(0.1);
        let tex_on_checked = Tex::from_file("icons/checked.png", true, None).unwrap_or_default();
        let mut on_checked_material = Material::pbr_clip().copy();
        on_checked_material.diffuse_tex(tex_on_checked).clip_cutoff(0.1).color_tint(GREEN);
        let mut text_style = TextStyle::default();
        text_style.layout_height(0.016);
        Self {
            id: "HandleMenuRadial_not_initialized".to_string(),
            sk_info: None,
            enabled: false,

            menu_stack: Vec::new(),
            menu_pose: Pose::default(),
            dest_pose: Pose::default(),
            root,
            active_layer,
            last_selected,
            last_selected_time,
            nav_stack: VecDeque::new(),
            active_hand: Handed::Max,
            activation: 0.0,
            menu_scale: 0.0,
            angle_offset: 0.0,
            background: Mesh::new(),
            background_edge: Mesh::new(),
            activation_button,
            activation_hamburger,
            activation_ring,
            child_indicator,
            img_frame,
            text_style,
            checked_material,
            on_checked_material,
        }
    }

    /// Force the hand menu to show at a specific location. This will close the hand menu if it was already open, and
    /// resets it to the root menu layer. Also plays an opening sound.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/Show.html>
    pub fn show(&mut self, at: impl Into<Vec3>, hand: Handed) {
        if self.active_hand != Handed::Max {
            self.close();
        }
        let at_pos = &at.into();
        Sound::click().play(*at_pos, None);
        self.dest_pose.position = *at_pos;
        self.dest_pose.orientation = Quat::look_at(*at_pos, Input::get_head().position, None);
        Log::diag(format!("dest_pose at show{}", self.dest_pose));
        self.active_layer = self.root.clone();
        self.active_hand = hand;

        generate_slice_mesh(
            360.0 / (self.root.items_count() as f32),
            Self::MIN_DIST,
            Self::MAX_DIST,
            Self::SLICE_GAP,
            &mut self.background,
        );
        generate_slice_mesh(
            360.0 / (self.root.items_count() as f32),
            Self::MAX_DIST,
            Self::MAX_DIST + 0.005,
            Self::SLICE_GAP,
            &mut self.background_edge,
        );
    }

    /// Closes the menu if it’s open! Plays a closing sound.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/Close.html>
    pub fn close(&mut self) {
        if self.active_hand != Handed::Max {
            Sound::unclick().play(self.menu_pose.position, None);
            self.menu_scale = Self::MIN_SCALE;
            self.active_hand = Handed::Max;
            self.angle_offset = 0.0;
            self.nav_stack.clear();
        }
    }

    fn step_menu_indicator(&mut self, token: &MainThreadToken, handed: Handed) {
        let hand = Input::hand(handed);
        if !hand.is_tracked() {
            return;
        };
        let mut show_menu = false;
        if Backend::xr_type() == BackendXRType::Simulator {
            if Input::key(Self::SIMULATOR_KEY).is_just_active() {
                show_menu = true
            }
        } else if (Input::get_controller_menu_button().is_just_active()) && handed == Handed::Left {
            show_menu = true;
        }
        if show_menu {
            self.menu_pose = hand.palm;
            let mut at = hand.get(FingerId::Index, JointId::Tip).position;
            if at == Vec3::ZERO {
                self.menu_pose = Input::controller(handed).aim;
                at = self.menu_pose.position
            }
            self.show(at, handed);
            return;
        }

        let head_fwd = Input::get_head().get_forward();
        let at = hand.palm.position;
        if at == Vec3::ZERO {
            //No way we get a palm pose equivalent, we do not show the hamburguer
            return;
        }
        let hand_dir = (at - Input::get_head().position).get_normalized();

        let in_view = Vec3::dot(head_fwd, hand_dir) > Self::OUT_OF_VIEW_ANGLE;

        if !in_view {
            return;
        }

        let palm_direction = hand.palm.get_forward();
        self.menu_pose = hand.palm;

        let direction_to_head = -hand_dir;
        let facing = Vec3::dot(palm_direction, direction_to_head);

        if facing < 0.0 {
            return;
        }

        let color_primary = Ui::get_theme_color(UiColor::Primary, None).to_linear();
        let color_common = Ui::get_theme_color(UiColor::Background, None).to_linear();
        let color_text = Ui::get_theme_color(UiColor::Text, None).to_linear();

        self.menu_pose.position += (1.0 - hand.grip_activation) * palm_direction * CM * 4.5;
        self.activation_button.draw(
            token,
            Material::ui(),
            self.menu_pose.to_matrix(None),
            Some(Color128::lerp(
                color_common,
                color_primary,
                ((facing - (Self::ACTIVATION_ANGLE - 0.01)) / 0.001).clamp(0.0, 1.0),
            )),
            None,
        );
        self.activation_hamburger
            .draw(token, Material::ui(), self.menu_pose.to_matrix(None), Some(color_text), None);
        self.menu_pose.position += (1.0 - hand.grip_activation) * palm_direction * CM * 2.0;
        self.activation_ring
            .draw(token, Material::ui(), self.menu_pose.to_matrix(None), Some(color_primary), None);

        if facing < Self::ACTIVATION_ANGLE {
            return;
        }

        if hand.is_just_gripped() {
            let mut at = hand.get(FingerId::Index, JointId::Tip).position;
            if at == Vec3::ZERO {
                at = Input::controller(hand.handed).aim.position
            }
            self.show(at, handed);
        }
    }

    fn step_menu(&mut self, token: &MainThreadToken, hand: Hand) {
        // animate the menu a bit
        let time = f32::min(1.0, Time::get_step_unscaledf() * 24.0);
        self.menu_pose.position = Vec3::lerp(self.menu_pose.position, self.dest_pose.position, time);
        self.menu_pose.orientation = Quat::slerp(self.menu_pose.orientation, self.dest_pose.orientation, time);
        self.activation = lerp(self.activation, 1.0, time);
        self.menu_scale = lerp(self.menu_scale, 1.0, time);

        // pre-calculate some circle traversal values
        let layer = self.active_layer.as_ref();
        let count = layer.items_count();
        let step = 360.0 / (count as f32);
        let half_step = step / 2.0;

        // Push the Menu's pose onto the stack, so we can draw, and work
        // in local space.
        Hierarchy::push(token, self.menu_pose.to_matrix(Some(self.menu_scale * Vec3::ONE)), None);

        // Calculate the status of the menu!
        let mut tip_world = hand.get(FingerId::Index, JointId::Tip).position;
        if tip_world == Vec3::ZERO {
            tip_world = Input::controller(hand.handed).aim.position
        }
        let tip_local = self.dest_pose.to_matrix(None).get_inverse().transform_point(tip_world);
        let mag_sq = tip_local.magnitude_squared();
        let on_menu = tip_local.z > -0.02 && tip_local.z < 0.02;
        let focused = on_menu && mag_sq > Self::MIN_DIST.powi(2);
        let selected = on_menu && mag_sq > Self::MID_DIST.powi(2);
        let cancel = mag_sq > Self::MAX_DIST.powi(2);
        //let arc_length = (Self::MIN_DIST * f32::min(90.0, step)).to_radians();

        // Find where our finger is pointing to, and draw that
        let mut finger_angle =
            tip_local.y.atan2(tip_local.x).to_degrees() - (layer.get_start_angle() + self.angle_offset);
        while finger_angle < 0.0 {
            finger_angle += 360.0;
        }
        let angle_id = (finger_angle / step).trunc() as usize;
        Lines::add(token, Vec3::new(0.0, 0.0, -0.008), Vec3::new(tip_local.x, tip_local.y, -0.008), WHITE, None, 0.006);

        // Now draw each of the menu items !
        let color_primary = Ui::get_theme_color(UiColor::Primary, None).to_linear();
        let color_common = Ui::get_theme_color(UiColor::Background, None).to_linear();
        for (i, line) in layer.items().iter().enumerate() {
            let curr_angle = (i as f32) * step + layer.get_start_angle() + self.angle_offset;
            let highlight = focused && angle_id == i && self.activation >= 0.99;
            let depth = if highlight { -0.005 } else { 0.0 };
            let mut at = Vec3::angle_xy(curr_angle + half_step, 0.0) * Self::MID_DIST;
            at.z = depth;

            let r = Matrix::tr(&Vec3::new(0.0, 0.0, depth), &Quat::from_angles(0.0, 0.0, curr_angle));
            self.background.draw(
                token,
                Material::ui(),
                r,
                Some(color_common * (if highlight { 2.0 } else { 1.0 })),
                None,
            );
            self.background_edge.draw(
                token,
                Material::ui(),
                r,
                Some(color_primary * (if highlight { 2.0 } else { 1.0 })),
                None,
            );
            let mut add_offset = 1.0;
            let item_to_draw: &HandMenuItem;

            match line.as_ref() {
                HandRadial::Item(item) => {
                    item_to_draw = item;
                    match *item.action.borrow() {
                        HandMenuAction::Back => self.child_indicator.draw(
                            token,
                            Material::ui(),
                            Matrix::tr(
                                &Vec3::new(0.0, 0.0, depth),
                                &Quat::from_angles(0.0, 0.0, curr_angle + half_step),
                            ),
                            None,
                            None,
                        ),
                        HandMenuAction::Close => (),
                        HandMenuAction::Callback => (),
                        HandMenuAction::Checked(_group) => {
                            let checked_material = if item_to_draw.image.is_none() {
                                &self.checked_material
                            } else {
                                &self.on_checked_material
                            };
                            self.img_frame.draw(
                                token,
                                checked_material,
                                Matrix::tr(
                                    &Vec3::new(0.0, 0.0, depth - 0.01),
                                    &Quat::from_angles(0.0, 0.0, curr_angle + half_step),
                                ),
                                None,
                                None,
                            );
                            add_offset = 1.2;
                        }
                        HandMenuAction::Unchecked(_group) => (),
                    };
                }
                HandRadial::Layer(layer) => {
                    item_to_draw = &layer.layer_item;
                    self.child_indicator.draw(
                        token,
                        Material::ui(),
                        Matrix::tr(&Vec3::new(0.0, 0.0, depth), &Quat::from_angles(0.0, 0.0, curr_angle + half_step)),
                        None,
                        None,
                    );
                }
            };
            if let Some(image_material) = &item_to_draw.image {
                self.img_frame.draw(
                    token,
                    image_material,
                    Matrix::tr(&Vec3::new(0.0, 0.0, depth), &Quat::from_angles(0.0, 0.0, curr_angle + half_step)),
                    None,
                    None,
                );
                add_offset = 1.2;
            }

            Ui::push_text_style(self.text_style);
            item_to_draw.draw_basic(token, at * add_offset, highlight);
            Ui::pop_text_style();
        }
        // Done with local work
        Hierarchy::pop(token);

        if self.activation < 0.99 {
            return;
        }
        if selected {
            if let Some(item_selected) = layer.items().get(angle_id) {
                if Rc::ptr_eq(item_selected, &self.last_selected)
                    && Time::get_total_unscaledf() - self.last_selected_time < 1.5
                {
                    return;
                };
                self.last_selected = item_selected.clone();
                self.last_selected_time = Time::get_total_unscaledf();

                if let Some(group_to_change) = item_selected.as_ref().is_unchecked_action() {
                    for line in layer.items().iter() {
                        if let Some(group) = line.as_ref().is_checked_action() {
                            if group == group_to_change {
                                let mut to_reverse = line.as_ref();
                                let to_to_reverse = to_reverse.borrow_mut();

                                if let HandRadial::Item(menu_item) = to_to_reverse {
                                    menu_item.action.replace(HandMenuAction::Unchecked(group));
                                }
                            }
                        }
                    }
                    let mut to_reverse = item_selected.as_ref();
                    let to_to_reverse = to_reverse.borrow_mut();
                    if let HandRadial::Item(menu_item) = to_to_reverse {
                        menu_item.action.replace(HandMenuAction::Checked(group_to_change));
                    }
                } else if let Some(group_to_change) = item_selected.as_ref().is_checked_action() {
                    // If there is only one of this group this is a toggle button
                    let mut cpt = 0;
                    for line in layer.items().iter() {
                        if let Some(group) = line.as_ref().is_checked_action() {
                            if group_to_change == group {
                                cpt += 1
                            }
                        } else if let Some(group) = line.as_ref().is_unchecked_action() {
                            if group_to_change == group {
                                cpt += 1
                            }
                        }
                    }
                    if cpt == 1 {
                        let mut to_reverse = item_selected.as_ref();
                        let to_to_reverse = to_reverse.borrow_mut();
                        if let HandRadial::Item(menu_item) = to_to_reverse {
                            menu_item.action.replace(HandMenuAction::Unchecked(group_to_change));
                        }
                    }
                }

                self.select_item(item_selected.clone(), tip_world, ((angle_id as f32) + 0.5) * step)
            } else {
                Log::err(format!("HandMenuRadial : Placement error for index {}", angle_id));
            }
        }
        if cancel {
            self.close()
        };
        let mut close_menu = false;
        if Backend::xr_type() == BackendXRType::Simulator {
            if Input::key(Self::SIMULATOR_KEY).is_just_active() {
                close_menu = true
            }
        } else if Input::get_controller_menu_button().is_just_active() {
            close_menu = true;
        }
        if close_menu {
            self.close()
        }
    }

    fn select_layer(&mut self, new_layer_rc: Rc<HandRadial>) {
        let new_layer = match new_layer_rc.as_ref() {
            HandRadial::Item(_) => {
                Log::err("HandMenuRadial : Item is not a valid layer");
                return;
            }
            HandRadial::Layer(layer) => layer,
        };
        Sound::click().play(self.menu_pose.position, None);
        self.nav_stack.push_back(self.active_layer.clone());
        self.active_layer = new_layer_rc.clone();
        let divisor = new_layer.items.len() as f32;
        generate_slice_mesh(360.0 / divisor, Self::MIN_DIST, Self::MAX_DIST, Self::SLICE_GAP, &mut self.background);
        generate_slice_mesh(
            360.0 / divisor,
            Self::MAX_DIST,
            Self::MAX_DIST + 0.005,
            Self::SLICE_GAP,
            &mut self.background_edge,
        );
        Log::diag(format!("HandRadialMenu : Layer {} opened", new_layer.layer_name));
    }

    fn back(&mut self) {
        Sound::unclick().play(self.menu_pose.position, None);
        if let Some(prev_layer) = self.nav_stack.pop_back() {
            self.active_layer = prev_layer.clone();
        } else {
            Log::err("HandMenuRadial : No back layer !!")
        }
        let divisor = self.active_layer.items_count() as f32;
        generate_slice_mesh(360.0 / divisor, Self::MIN_DIST, Self::MAX_DIST, Self::SLICE_GAP, &mut self.background);
        generate_slice_mesh(
            360.0 / divisor,
            Self::MAX_DIST,
            Self::MAX_DIST + 0.005,
            Self::SLICE_GAP,
            &mut self.background_edge,
        );
    }

    fn select_item(&mut self, line: Rc<HandRadial>, at: Vec3, from_angle: f32) {
        match line.as_ref() {
            HandRadial::Item(item) => {
                match *item.action.borrow() {
                    HandMenuAction::Close => self.close(),
                    HandMenuAction::Callback => {}
                    HandMenuAction::Checked(_) => {}
                    HandMenuAction::Unchecked(_) => {}
                    HandMenuAction::Back => {
                        self.back();
                        self.reposition(at, from_angle)
                    }
                };
                let mut callback = item.callback.borrow_mut();
                callback()
            }
            HandRadial::Layer(layer) => {
                Log::diag(format!("HandRadialMenu : open Layer {}", layer.layer_name));
                self.select_layer(line.clone());
                self.reposition(at, from_angle)
            }
        }
    }

    fn reposition(&mut self, at: Vec3, from_angle: f32) {
        let plane = Plane::from_point(self.menu_pose.position, self.menu_pose.get_forward());
        self.dest_pose.position = plane.closest(at);

        self.activation = 0.0;

        if self.active_layer.get_back_angle() != 0.0 {
            self.angle_offset = (from_angle - self.active_layer.get_back_angle()) + 180.0;
            while self.angle_offset < 0.0 {
                self.angle_offset += 360.0;
            }
            while self.angle_offset > 360.0 {
                self.angle_offset -= 360.0;
            }
        } else {
            self.angle_offset = 0.0
        };
    }
}

fn generate_slice_mesh(angle: f32, min_dist: f32, max_dist: f32, gap: f32, mesh: &mut Mesh) {
    let count = angle * 0.25;

    let inner_start_angle = gap / min_dist.to_radians();
    let inner_angle = angle - inner_start_angle * 2.0;
    let inner_step = inner_angle / (count - 1.0);

    let outer_start_angle = gap / max_dist.to_radians();
    let outer_angle = angle - outer_start_angle * 2.0;
    let outer_step = outer_angle / (count - 1.0);

    let mut verts: Vec<Vertex> = vec![];
    let mut inds: Vec<Inds> = vec![];

    let icount = count as u32;
    for i in 0..icount {
        let inner_dir = Vec3::angle_xy(inner_start_angle + (i as f32) * inner_step, 0.005);
        let outer_dir = Vec3::angle_xy(outer_start_angle + (i as f32) * outer_step, 0.005);
        verts.push(Vertex::new(inner_dir * min_dist, Vec3::FORWARD, None, None));
        verts.push(Vertex::new(outer_dir * max_dist, Vec3::FORWARD, None, None));

        if i != icount - 1 {
            inds.push((i + 1) * 2 + 1);
            inds.push(i * 2 + 1);
            inds.push(i * 2);

            inds.push((i + 1) * 2);
            inds.push((i + 1) * 2 + 1);
            inds.push(i * 2);
        }
    }

    mesh.set_verts(verts.as_slice(), true);
    mesh.set_inds(inds.as_slice());
}

fn generate_activation_button(radius: f32) -> Mesh {
    let spokes = 36;
    let mut verts: Vec<Vertex> = vec![];
    let mut inds: Vec<Inds> = vec![];

    for i in 0..spokes {
        verts.push(Vertex::new(
            Vec3::angle_xy((i as f32) * (360.0 / (spokes as f32)) * radius, 0.0),
            Vec3::FORWARD,
            None,
            None,
        ))
    }

    for i in 0..(spokes - 2) {
        let half = i / 2;

        if i % 2 == 0 {
            inds.push(spokes - 1 - half);
            inds.push(half + 1);
            inds.push((spokes - half) % spokes);
        } else {
            inds.push(half + 1);
            inds.push(spokes - (half + 1));
            inds.push(half + 2);
        }
    }

    let mut mesh = Mesh::new();

    mesh.set_inds(inds.as_slice());
    mesh.set_verts(verts.as_slice(), true);
    mesh
}

fn generate_activation_hamburger(radius: f32) -> Mesh {
    let mut verts: Vec<Vertex> = vec![];
    let mut inds: Vec<Inds> = vec![];

    let w = radius / 3.0;
    let h = radius / 16.0;
    let z = -0.003;

    for i in 0..3 {
        let y = -radius / 3.0 + (i as f32) * radius / 3.0;

        let a = i * 4;
        let b = i * 4 + 1;
        let c = i * 4 + 2;
        let d = i * 4 + 3;

        verts.push(Vertex::new(Vec3::new(-w, y - h, z), Vec3::FORWARD, None, None));
        verts.push(Vertex::new(Vec3::new(w, y - h, z), Vec3::FORWARD, None, None));
        verts.push(Vertex::new(Vec3::new(w, y + h, z), Vec3::FORWARD, None, None));
        verts.push(Vertex::new(Vec3::new(-w, y + h, z), Vec3::FORWARD, None, None));

        inds.push(c);
        inds.push(b);
        inds.push(a);

        inds.push(d);
        inds.push(c);
        inds.push(a);
    }

    let mut mesh = Mesh::new();
    mesh.set_inds(inds.as_slice());
    mesh.set_verts(verts.as_slice(), true);

    mesh
}

fn generate_child_indicator(distance: f32, radius: f32) -> Mesh {
    let mut verts: Vec<Vertex> = vec![];
    let mut inds: Vec<Inds> = vec![];

    verts.push(Vertex::new(Vec3::new(distance, radius * 2.0, 0.0), Vec3::FORWARD, None, None));
    verts.push(Vertex::new(Vec3::new(distance + radius, 0.0, 0.0), Vec3::FORWARD, None, None));
    verts.push(Vertex::new(Vec3::new(distance, -radius * 2.0, 0.0), Vec3::FORWARD, None, None));

    inds.push(0);
    inds.push(1);
    inds.push(2);

    let mut mesh = Mesh::new();
    mesh.set_inds(inds.as_slice());
    mesh.set_verts(verts.as_slice(), true);

    mesh
}

fn generate_img_frame(distance: f32, radius: f32) -> Mesh {
    let mut verts: Vec<Vertex> = vec![];
    let mut inds: Vec<Inds> = vec![];

    verts.push(Vertex::new(
        Vec3::new(distance + radius, -radius, 0.0),
        Vec3::FORWARD,
        Some(Vec2::new(0.0, 0.0)),
        None,
    ));
    verts.push(Vertex::new(
        Vec3::new(distance + radius, radius, 0.0),
        Vec3::FORWARD,
        Some(Vec2::new(1.0, 0.0)),
        None,
    ));
    verts.push(Vertex::new(
        Vec3::new(distance - radius, -radius, 0.0),
        Vec3::FORWARD,
        Some(Vec2::new(0.0, 1.0)),
        None,
    ));
    verts.push(Vertex::new(
        Vec3::new(distance - radius, radius, 0.0),
        Vec3::FORWARD,
        Some(Vec2::new(1.0, 1.0)),
        None,
    ));

    inds.push(0);
    inds.push(2);
    inds.push(1);
    inds.push(1);
    inds.push(2);
    inds.push(3);

    let mut mesh = Mesh::new();
    mesh.set_inds(inds.as_slice());
    mesh.set_verts(verts.as_slice(), true);

    mesh
}
