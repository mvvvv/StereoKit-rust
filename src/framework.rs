use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use crate::{
    material::Material,
    maths::{lerp, units::CM, Matrix, Plane, Pose, Quat, Vec3},
    mesh::{Inds, Mesh, Vertex},
    sk::{IStepper, MainThreadToken, SkInfo, StepperId},
    sound::Sound,
    sprite::Sprite,
    system::{
        Backend, BackendXRType, FingerId, Hand, Handed, Hierarchy, Input, JointId, Key, Lines, Log, Text, TextAlign,
        TextStyle,
    },
    ui::{Ui, UiColor},
    util::{named_colors::WHITE, Color128, Time},
};

/// StereoKit initialization settings! Setup SkSettings with your data before calling SkSetting.Init().
/// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuItem.html
pub struct HandMenuItem {
    pub name: String,
    pub image: Option<Sprite>,
    pub action: HandMenuAction,
    pub callback: Box<RefCell<dyn FnMut()>>,
}

impl HandMenuItem {
    /// Makes a menu item!
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuItem/HandMenuItem.html
    pub fn new<C: FnMut() + 'static>(
        name: impl AsRef<str>,
        image: Option<Sprite>,
        callback: C,
        action: HandMenuAction,
    ) -> Self {
        Self { name: name.as_ref().to_owned(), image, callback: Box::new(RefCell::new(callback)), action }
    }

    /// This draws the menu item on the radial menu!
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuItem/Draw.html
    pub fn draw(&self, token: &MainThreadToken, at: Vec3, _arc_length: f32, _angle: f32, focused: bool) {
        match &self.image {
            Some(sprite) => {
                let height = TextStyle::default().get_char_height();
                let offset = Vec3::new(0.0, height * 0.75, 0.0);
                let scale = match focused {
                    true => Vec3::ONE * 1.2,
                    false => Vec3::ONE * 1.0,
                };
                Hierarchy::push(token, Matrix::ts(at, scale));
                sprite.draw(token, Matrix::ts(offset, height * Vec3::ONE), TextAlign::Center, None);
                Text::add_at(
                    token,
                    &self.name,
                    Matrix::ts(at, Vec3::ONE * 0.5),
                    None,
                    None,
                    None,
                    Some(TextAlign::BottomCenter),
                    None,
                    None,
                    None,
                );
                Hierarchy::pop(token);
            }
            None => {
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
    }
}

pub enum HandRadial {
    Item(HandMenuItem),
    Layer(HandRadialLayer),
}

impl HandRadial {
    pub fn item<C: FnMut() + 'static>(
        name: impl AsRef<str>,
        image: Option<Sprite>,
        callback: C,
        action: HandMenuAction,
    ) -> Self {
        Self::Item(HandMenuItem::new(name, image, callback, action))
    }

    pub fn layer(
        name: impl AsRef<str>,
        image: Option<Sprite>,
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
            HandRadial::Item(item) => item.action == HandMenuAction::Back,
            HandRadial::Layer(_) => false,
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
/// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer.html
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
/// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/HandRadialLayer.html
impl HandRadialLayer {
    pub fn new(
        name: impl AsRef<str>,
        image: Option<Sprite>,
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
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/AddChild.html
    pub fn add_child(&mut self, mut layer: HandRadialLayer) -> &mut Self {
        layer.parent = Some(self.layer_name.clone());
        self.items.push(Rc::new(HandRadial::Layer(layer)));
        self
    }

    /// Find a child menu layer by name. Recursive function
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/FindChild.html
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
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/RemoveChild.html
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
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/AddItem.html
    pub fn add_item(&mut self, menu_item: HandMenuItem) -> &mut Self {
        self.items.push(Rc::new(HandRadial::Item(menu_item)));
        self
    }

    /// Find a menu item by name.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/FindItem.html
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
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandRadialLayer/RemoveItem.html
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
}

/// A menu that shows up in circle around the user’s hand! Selecting an item can perform an action, or even spawn a
/// sub-layer of menu items. This is an easy way to store actions out of the way, yet remain easily accessible to the
/// user.
///
/// The user faces their palm towards their head, and then makes a grip motion to spawn the menu. The user can then
/// perform actions by making fast, direction based motions that are easy to build muscle memory for.
/// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial.html>
pub struct HandMenuRadial {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    menu_pose: Pose,
    dest_pose: Pose,
    root: Rc<HandRadial>,
    active_layer: Rc<HandRadial>,
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
}

unsafe impl Send for HandMenuRadial {}

impl IStepper for HandMenuRadial {
    /// Part of IStepper, you shouldn’t be calling this yourself.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/Initialize.html>
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        true
    }

    /// Part of IStepper, you shouldn’t be calling this yourself.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/Step.html>
    fn step(&mut self, token: &MainThreadToken) {
        if self.active_hand == Handed::Max {
            for hand in [Handed::Left, Handed::Right] {
                self.step_menu_indicator(token, hand);
            }
        } else {
            self.step_menu(token, Input::hand(self.active_hand));
        }
    }

    /// Part of IStepper, you shouldn’t be calling this yourself.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/Shutdown.html>
    fn shutdown(&mut self) {}
}

impl HandMenuRadial {
    /// When using the Simulator, this key will activate the menu on the current hand, regardless of which direction it
    /// is facing.
    pub const SIMULATOR_KEY: Key = Key::Backtick;
    pub const MIN_DIST: f32 = 0.03;
    pub const MID_DIST: f32 = 0.065;
    pub const MAX_DIST: f32 = 0.1;
    pub const MIN_SCALE: f32 = 0.05;
    pub const SLICE_GAP: f32 = 0.02;
    pub const OUT_OF_VIEW_ANGLE: f32 = 0.866;
    pub const ACTIVATION_ANGLE: f32 = 0.978;

    /// Creates a hand menu from the provided array of menu layers! HandMenuRadial is an IStepper, so proper usage is to
    /// add it to the Stepper list via Sk.AddStepper. If no layers are provided to this constructor, a default
    /// root layer will be automatically added.
    /// <https://stereokit.net/Pages/StereoKit.Framework/HandMenuRadial/HandMenuRadial.html>
    pub fn new(root_layer: HandRadialLayer) -> Self {
        let root = Rc::new(HandRadial::Layer(root_layer));
        let active_layer = root.clone();
        let activation_btn_radius = 1.0 * CM;
        let activation_button = generate_activation_button(activation_btn_radius);
        let activation_hamburger = generate_activation_hamburger(activation_btn_radius);
        let mut activation_ring = Mesh::new();
        generate_slice_mesh(360.0, activation_btn_radius, activation_btn_radius + 0.005, 0.0, &mut activation_ring);
        let child_indicator = generate_child_indicator(Self::MAX_DIST - 0.008, 0.004);
        Self {
            menu_pose: Pose::default(),
            dest_pose: Pose::default(),
            root,
            active_layer,
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
            id: "HandleMenuRadial".to_string(),
            sk_info: None,
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
        Log::info(format!("dest_pose at show{}", self.dest_pose));
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
            Self::MIN_DIST,
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

        if Backend::xr_type() == BackendXRType::Simulator {
            if Input::key(Self::SIMULATOR_KEY).is_just_active() {
                self.menu_pose = hand.palm;
                self.show(hand.get(FingerId::Index, JointId::Tip).position, handed);
            }
            return;
        }

        let head_fwd = Input::get_head().get_forward();
        let hand_dir = (hand.palm.position - Input::get_head().position).normalized();
        let in_view = Vec3::dot(head_fwd, hand_dir) > Self::OUT_OF_VIEW_ANGLE;

        if !in_view {
            return;
        }

        let palm_direction = hand.palm.get_forward();
        let direction_to_head = -hand_dir;
        let facing = Vec3::dot(palm_direction, direction_to_head);

        if facing < 0.0 {
            return;
        }

        let color_primary = Ui::get_theme_color(UiColor::Primary, None).to_linear();
        let color_common = Ui::get_theme_color(UiColor::Background, None).to_linear();
        let color_text = Ui::get_theme_color(UiColor::Text, None).to_linear();

        self.menu_pose = hand.palm;

        self.activation_ring
            .draw(token, Material::ui(), self.menu_pose.to_matrix(None), Some(color_primary), None);
        self.menu_pose.position += (1.0 - hand.grip_activation) * self.menu_pose.get_forward() * CM * 2.0;
        self.activation_button.draw(
            token,
            Material::ui(),
            self.menu_pose.to_matrix(None),
            Some(Color128::lerp(
                color_common,
                color_primary,
                f32::max(0.0, f32::min(1.0, (facing - (Self::ACTIVATION_ANGLE - 0.01)) / 0.001)),
            )),
            None,
        );
        self.activation_hamburger
            .draw(token, Material::ui(), self.menu_pose.to_matrix(None), Some(color_text), None);

        if facing < Self::ACTIVATION_ANGLE {
            return;
        }

        if hand.is_just_gripped() {
            self.show(hand.get(FingerId::Index, JointId::Tip).position, handed);
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
        Hierarchy::push(token, self.menu_pose.to_matrix(Some(self.menu_scale * Vec3::ONE)));

        // Calculate the status of the menu!
        let tip_world = hand.get(FingerId::Index, JointId::Tip).position;
        let tip_local = self.dest_pose.to_matrix(None).get_inverse().transform_point(tip_world);
        let mag_sq = tip_local.magnitude_squared();
        let on_menu = tip_local.z > -0.02 && tip_local.z < 0.02;
        let focused = on_menu && mag_sq > Self::MIN_DIST.powi(2);
        let selected = on_menu && mag_sq > Self::MID_DIST.powi(2);
        let cancel = mag_sq > Self::MAX_DIST.powi(2);
        let arc_length = (Self::MIN_DIST * f32::min(90.0, step)).to_radians();

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
            let item_to_draw: &HandMenuItem;
            let child_indicator = match line.as_ref() {
                HandRadial::Item(item) => {
                    item_to_draw = item;
                    match item.action {
                        HandMenuAction::Back => true,
                        HandMenuAction::Close => false,
                        HandMenuAction::Callback => false,
                    }
                }
                HandRadial::Layer(layer) => {
                    item_to_draw = &layer.layer_item;
                    true
                }
            };
            if child_indicator {
                self.child_indicator.draw(
                    token,
                    Material::ui(),
                    Matrix::tr(&Vec3::new(0.0, 0.0, depth), &Quat::from_angles(0.0, 0.0, curr_angle + half_step)),
                    None,
                    None,
                );
            }
            item_to_draw.draw(token, at, arc_length, curr_angle + half_step, highlight);
        }
        // Done with local work
        Hierarchy::pop(token);

        if self.activation < 0.99 {
            return;
        }
        if selected {
            if let Some(item_selected) = layer.items().get(angle_id) {
                self.select_item(item_selected.clone(), tip_world, ((angle_id as f32) + 0.5) * step)
            } else {
                Log::err(format!("HandMenuRadial : Placement error for index {}", angle_id));
            }
        }
        if cancel {
            self.close()
        };
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
                match item.action {
                    HandMenuAction::Close => self.close(),
                    HandMenuAction::Callback => {}
                    HandMenuAction::Back => {
                        self.back();
                        self.reposition(at, from_angle)
                    }
                };
                (item.callback.borrow_mut())();
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

    let inner_start_angle = gap / (min_dist.to_radians());
    let inner_angle = angle - inner_start_angle * 2.0;
    let inner_step = inner_angle / (count - 1.0);

    let outer_start_angle = gap / (max_dist.to_radians());
    let outer_angle = angle - outer_start_angle * 2.0;
    let outer_step = outer_angle / (count - 1.0);

    let mut verts: Vec<Vertex> = vec![];
    let mut inds: Vec<Inds> = vec![];

    let icount = count as u32;
    for i in 0..icount {
        let inner_dir = Vec3::angle_xy(inner_start_angle + (i as f32) * inner_step, 0.005);
        let outer_dir = Vec3::angle_xy(outer_start_angle + (i as f32) * outer_step, 0.005);
        verts.push(Vertex::new(inner_dir * min_dist, Vec3::FORWARD, None, None));
        verts.push(Vertex::new(outer_dir * min_dist, Vec3::FORWARD, None, None));

        if i != icount - 1 {
            inds.push((i + 1) * 2 + 1);
            inds.push(i * 2 + 1);
            inds.push(i * 2);

            inds.push((i + 1) * 2);
            inds.push((i + 1) * 2 + 1);
            inds.push(i * 2);
        }

        mesh.set_verts(verts.as_slice(), true);
        mesh.set_inds(inds.as_slice());
    }
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

    for i in 0..spokes - 2 {
        let half = i / 2;

        if i % 2 == 0 {
            inds.push(spokes - 1 - half);
            inds.push(half + 1);
            inds.push((spokes - half) % spokes);
        } else {
            inds.push(half + 2);
            inds.push(spokes - half + 1);
            inds.push(half + 1);
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
