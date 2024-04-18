use std::{cell::RefCell, rc::Rc};

use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{units::M, Matrix, Pose, Quat, Vec3},
    mesh::Mesh,
    model::Model,
    sk::{IStepper, MainThreadToken, SkInfo, StepperId},
    sound::{Sound, SoundInst},
    system::{Log, Renderer, Text, TextStyle},
    util::{named_colors::RED, Time},
};

/// The plane1 stepper a flying plane
pub struct Plane1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    plane_pose: Pose,
    previous_target: Pose,
    next_target: Pose,
    speed_factor: f32,
    rotate_speed_factor: f32,
    rolling_speed_factor: f32,
    plane: Mesh,
    plane_sound: Sound,
    plane_sound_inst: Option<SoundInst>,
    material: Material,
    pub transform: Matrix,
    pub text: String,
    text_style: Option<TextStyle>,
}

unsafe impl Send for Plane1 {}

/// This code may be called in main thread
impl Default for Plane1 {
    fn default() -> Self {
        let model = Model::from_file("plane.glb", None).unwrap_or_default();
        let nodes = model.get_nodes();
        let plane = match nodes.find("Plane") {
            Some(plane) => match plane.get_mesh() {
                Some(mesh) => mesh,
                None => Mesh::cube(),
            },
            _ => Mesh::cube(),
        };
        let plane_pose = Pose::new(Vec3::NEG_Z + Vec3::Y * 1.5, None);
        let plane_sound = Sound::from_file("sounds/plane_engine.mp3").unwrap();

        Self {
            id: "Plane1".to_string(),
            sk_info: None,
            plane_pose,
            speed_factor: 0.5,
            rotate_speed_factor: 1.0,
            rolling_speed_factor: 5.0,
            previous_target: Pose::new(Vec3::ZERO, None),
            next_target: Pose::new(Vec3::ONE, None),
            plane,
            plane_sound,
            plane_sound_inst: None,
            material: Material::pbr(),
            transform: Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y), &Quat::from_angles(0.0, 180.0, 0.0)),
            text: "Plane1".to_owned(),
            text_style: None,
        }
    }
}

/// All the code here run in the main thread
impl IStepper for Plane1 {
    fn initialize(&mut self, id: StepperId, sk_info: Rc<RefCell<SkInfo>>) -> bool {
        self.id = id;
        self.sk_info = Some(sk_info);
        self.text_style = Some(Text::make_style(Font::default(), 0.3, RED));

        self.plane_sound_inst = Some(self.plane_sound.play(self.plane_pose.position, Some(1.0)));

        true
    }

    fn step(&mut self, token: &MainThreadToken) {
        self.draw(token)
    }

    fn shutdown(&mut self) {
        if let Some(sound_inst) = self.plane_sound_inst {
            sound_inst.stop();
        }
    }
}

impl Plane1 {
    fn draw(&mut self, token: &MainThreadToken) {
        self.animate_plane(token);

        Text::add_at(token, &self.text, self.transform, self.text_style, None, None, None, None, None, None);
    }

    fn animate_plane(&mut self, token: &MainThreadToken) {
        let forward = self.plane_pose.get_forward();
        let stepf = Time::get_stepf();
        let next_position = self.plane_pose.position + forward * stepf * self.speed_factor;
        let dir_to_go = Vec3::direction(self.next_target.position, next_position);
        let dot = (Vec3::dot(forward, dir_to_go) - 1.0) / -2.2;
        let up = Vec3::Y + dir_to_go * 2.0 + forward;
        let fast_rolling = Quat::look_at(self.plane_pose.position, next_position, Some(up)).get_normalized();
        self.plane_pose.orientation =
            Quat::slerp(self.plane_pose.orientation, fast_rolling, Time::get_stepf() * self.rolling_speed_factor)
                .get_normalized();
        let final_rotation = Quat::look_at(next_position, self.next_target.position, None).get_normalized();
        let next_rotation = Quat::slerp(
            self.plane_pose.orientation,
            final_rotation,
            Time::get_stepf() * (1.0 - dot) * self.rotate_speed_factor,
        )
        .get_normalized();
        if (next_position - self.next_target.position).length() < 0.6 * M {
            let x = stepf * 3000.0 % 3.0;
            let y = stepf * 3000.0 % 2.0 + 1.0;
            let z = Time::get_totalf() % 6.0 - 3.0;
            self.previous_target = self.next_target;
            self.next_target = Pose::new(Vec3::new(x, y, z), None);
            Log::diag(format!("Next dir is {}", self.next_target));
        } else if (self.previous_target.position - self.next_target.position).length() > 0.05 {
            self.previous_target.position +=
                Vec3::direction(self.next_target.position, self.previous_target.position) * Time::get_stepf() * 5.0;
        }
        self.plane_pose = Pose::new(next_position, Some(next_rotation.get_normalized()));
        Renderer::add_mesh(
            token,
            &self.plane,
            &self.material,
            self.plane_pose.to_matrix(Some(Vec3::ONE * 0.02)),
            Some(RED.into()),
            None,
        );

        if let Some(mut sound_inst) = self.plane_sound_inst {
            if !sound_inst.is_playing() {
                Log::diag("Play again!!");
                self.plane_sound_inst = Some(self.plane_sound.play(self.plane_pose.position, Some(100.0)));
            }
            sound_inst.position(self.plane_pose.position);
        }

        Mesh::sphere().draw(
            token,
            &self.material,
            self.previous_target.to_matrix(Some(Vec3::ONE * 0.1 * M)),
            None,
            None,
        );
    }
}
