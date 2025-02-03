use stereokit_rust::{
    font::Font,
    material::Material,
    maths::{Bounds, Matrix, Plane, Pose, Quat, Ray, Sphere, Vec3},
    mesh::Mesh,
    model::Model,
    prelude::*,
    system::{Handed, Input, Lines, Text, TextStyle},
    ui::Ui,
    util::{
        named_colors::{BLACK, BLUE, GREEN, RED, WHITE, YELLOW_GREEN},
        Time,
    },
};

pub const SPHERE_RADIUS: f32 = 0.4;
pub const ROTATION_SPEED: f32 = 30.0;

/// The Math1 stepper
#[derive(IStepper)]
pub struct Math1 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,
    pub transform_ico_sphere: Matrix,
    pub model_pose: Pose,
    model: Model,
    little_sphere: Mesh,
    ico_sphere: Mesh,
    material: Material,
    pub transform_text: Matrix,
    text: String,
    text_style: TextStyle,
}

unsafe impl Send for Math1 {}

impl Default for Math1 {
    /// Creates a new instance of Math1 with default values.
    fn default() -> Self {
        let transform_ico_sphere = Matrix::ts(Vec3::NEG_Z * 0.5 + Vec3::X + Vec3::Y * 1.5, Vec3::ONE * 0.3);
        let model_pose = Pose::new(Vec3::NEG_Z + Vec3::Y * 1.0, None);
        let transform_text = Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y * 2.0), &Quat::from_angles(0.0, 180.0, 0.0));
        let material = Material::pbr();
        let model = Model::from_mesh(Mesh::generate_sphere(SPHERE_RADIUS * 2.0, Some(16)), &material);
        let little_sphere = Mesh::generate_sphere(0.02, None);
        let ico_sphere = Mesh::find("mobiles.gltf/mesh/0_0_Icosphere").unwrap_or_default();
        Self {
            id: "Math1".to_string(),
            sk_info: None,
            transform_ico_sphere,
            model_pose,
            model,
            little_sphere,
            ico_sphere,
            material,
            transform_text,
            text: "Math1".to_owned(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
        }
    }
}

impl Math1 {
    /// Called from IStepper::initialize here you can abort the initialization by returning false
    fn start(&mut self) -> bool {
        true
    }

    /// Called from IStepper::step, here you can check the event report
    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    /// Called from IStepper::step after check_event, here you can draw the scene
    fn draw(&mut self, token: &MainThreadToken) {
        Ui::handle("Math1_Cube", &mut self.model_pose, self.model.get_bounds(), false, None, None);

        let right_hand = Input::hand(Handed::Right);

        let mut hand_pose = right_hand.palm;
        let ray;
        if hand_pose.position == Vec3::ZERO {
            hand_pose = Input::controller(Handed::Right).pose;
            ray = Ray::new(hand_pose.position, hand_pose.get_up() * -1.0);
        } else {
            ray = Ray::new(hand_pose.position, hand_pose.get_up());
        }
        if right_hand.is_just_pinched() {
            Log::diag(format!("{:?}", ray));
        }

        // Draw a line for the ray
        Lines::add(token, ray.position, ray.position + ray.direction * 0.5, WHITE, None, 0.01);

        let transform = self.model_pose.to_matrix(None);

        // Reduce the ray to bound space
        let inverse_transform = transform.get_inverse();
        let ray_to_bounds = Ray::new(
            inverse_transform.transform_point(ray.position),
            inverse_transform.transform_normal(ray.direction),
        );

        // Add little_spheres and change color of the big sphere if the ray hit bounds then sphere surface.
        let mut color = WHITE;
        if let Some(out_bounds_inverse) = ray_to_bounds.intersect_bound(self.model.get_bounds()) {
            color = RED; // changed from WHITE
            let out_bounds = transform.transform_point(out_bounds_inverse);
            let sphere_transform = Matrix::t(out_bounds);
            self.little_sphere.draw(token, &self.material, sphere_transform, Some(GREEN.into()), None);

            let sphere_form = Sphere::new(self.model_pose.position, SPHERE_RADIUS);
            if let Some(out_sphere) = ray.intersect_sphere(sphere_form) {
                let sphere_transform = Matrix::t(out_sphere);
                self.little_sphere.draw(token, &self.material, sphere_transform, Some(BLUE.into()), None);

                let sphere_ctrl = Sphere::new(out_sphere, 0.01);
                if sphere_ctrl.contains(out_bounds) {
                    color = GREEN; // changed from Black
                } else {
                    let bound_ctrl = Bounds::new(out_sphere, Vec3::ONE * 0.05);
                    if bound_ctrl.contains_point(out_bounds) {
                        color = BLACK; // changed from RED
                    }
                }
            }
        }

        // Add little_sphere to the floor if pointed by the ray
        let plane = Plane::new(Vec3::Y, 0.0);
        if let Some(out_plane) = ray.intersect(plane) {
            let sphere_transform = Matrix::ts(out_plane, Vec3::ONE * 8.0);
            self.little_sphere.draw(token, &self.material, sphere_transform, Some(WHITE.into()), None);
        }

        // Add little_sphere to ico_sphere if pointed by the ray
        let center = Vec3::NEG_Z * 0.5 + Vec3::X;
        Lines::add(token, center, center + Vec3::Y * 2.5, RED, None, 0.1);
        let rotation = Quat::from_angles(0.0, ROTATION_SPEED * Time::get_step_unscaledf(), 0.0);
        let mut transform_ico = self.transform_ico_sphere;
        let radius_circle = 0.9;

        let mut pose = transform_ico.get_pose();
        let circle_pose = Pose::new(
            pose.get_forward() * radius_circle + pose.position,
            Some(Quat::look_dir(-pose.get_forward()) * rotation),
        );
        let forward = circle_pose.get_forward() * radius_circle;
        pose.position = forward + circle_pose.position;
        pose.orientation *= rotation;

        transform_ico = pose.to_matrix(Some(Vec3::ONE * 0.3));

        self.transform_ico_sphere = transform_ico;

        // Reduce the ray to mesh space
        let inverse_transform_ico = transform_ico.get_inverse();
        let ray_to_bounds = Ray::new(
            inverse_transform_ico.transform_point(ray.position),
            inverse_transform_ico.transform_normal(ray.direction),
        );
        // add blue little sphere only
        if let Some(out_mesh) = ray_to_bounds.intersect_mesh(&self.ico_sphere, None) {
            let sphere_transform = Matrix::t(transform_ico.transform_point(out_mesh.0));
            self.little_sphere.draw(token, &self.material, sphere_transform, Some(BLUE.into()), None);
        }

        self.model.draw(token, transform, Some(color.into()), None);
        self.ico_sphere.draw(token, &self.material, transform_ico, Some(YELLOW_GREEN.into()), None);
        Text::add_at(token, &self.text, self.transform_text, Some(self.text_style), None, None, None, None, None, None);
    }
}
