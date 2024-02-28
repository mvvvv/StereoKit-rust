use stereokit_rust::{
    font::Font,
    material::{Cull, Material},
    maths::{Bounds, Matrix, Plane, Pose, Quat, Ray, Sphere, Vec3},
    mesh::Mesh,
    model::Model,
    sk::{IStepper, StepperAction, StepperId},
    system::{Handed, Input, Lines, Log, Text, TextStyle},
    ui::Ui,
    util::named_colors::{BLACK, BLUE, GREEN, RED, WHITE, YELLOW_GREEN},
};
use winit::event_loop::EventLoopProxy;

pub const SPHERE_RADIUS: f32 = 0.4;

pub struct Math1 {
    id: StepperId,
    event_loop_proxy: Option<EventLoopProxy<StepperAction>>,
    pub transform_model: Matrix,
    pub transform_ico_sphere: Matrix,
    model_pose: Pose,
    model: Model,
    little_sphere: Mesh,
    ico_sphere: Mesh,
    material: Material,
    pub transform_text: Matrix,
    text: String,
    text_style: TextStyle,
}

impl Default for Math1 {
    fn default() -> Self {
        let transform_model = Matrix::tr(&((Vec3::NEG_Z * 1.0) + Vec3::Y * 1.0), &Quat::from_angles(90.0, 0.0, 0.0));
        let transform_ico_sphere = Matrix::ts(Vec3::NEG_Z * 0.5 + Vec3::X + Vec3::Y * 1.5, Vec3::ONE * 0.3);
        let model_pose = transform_model.get_pose();
        let transform_text = Matrix::tr(&((Vec3::NEG_Z * 2.5) + Vec3::Y * 2.0), &Quat::from_angles(0.0, 180.0, 0.0));
        let material = Material::pbr();
        let model = Model::from_mesh(Mesh::generate_sphere(SPHERE_RADIUS * 2.0, Some(16)), &material);
        let little_sphere = Mesh::generate_sphere(0.02, None);
        let ico_sphere = Mesh::find("mobiles.gltf/mesh/0_0_Icosphere").unwrap();
        Self {
            id: "Math1".to_string(),
            event_loop_proxy: None,
            transform_model,
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

impl IStepper for Math1 {
    fn initialize(&mut self, id: StepperId, event_loop_proxy: EventLoopProxy<StepperAction>) -> bool {
        self.id = id;
        self.event_loop_proxy = Some(event_loop_proxy);
        //--If default transform_model as been changed then new model_pose
        self.model_pose = self.transform_model.get_pose();
        true
    }

    fn step(&mut self, _event_report: &[StepperAction]) {
        self.draw()
    }
}

impl Math1 {
    fn draw(&mut self) {
        Ui::handle("Math1_Cube", &mut self.model_pose, self.model.get_bounds(), false, None, None);

        let right_hand = Input::hand(Handed::Right);

        let hand_pose = right_hand.palm;
        let ray = Ray::new(hand_pose.position, hand_pose.get_up());

        if right_hand.is_just_pinched() {
            Log::diag(format!("{:?}", ray));
        }

        // Draw a line for the ray
        Lines::add(ray.position, ray.position + ray.direction * 0.5, WHITE, None, 0.01);

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
            self.little_sphere.draw(&self.material, sphere_transform, Some(GREEN.into()), None);

            let sphere_form = Sphere::new(self.model_pose.position, SPHERE_RADIUS);
            if let Some(out_sphere) = ray.intersect_sphere(sphere_form) {
                let sphere_transform = Matrix::t(out_sphere);
                self.little_sphere.draw(&self.material, sphere_transform, Some(BLUE.into()), None);

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
            self.little_sphere.draw(&self.material, sphere_transform, Some(WHITE.into()), None);
        }

        // Add little_sphere to ico_sphere if pointed by the ray
        let transform_ico = self.transform_ico_sphere;
        // Reduce the ray to mesh space
        let inverse_transform_ico = transform_ico.get_inverse();
        let ray_to_bounds = Ray::new(
            inverse_transform_ico.transform_point(ray.position),
            inverse_transform_ico.transform_normal(ray.direction),
        );
        // add blue little sphere only
        if let Some(out_mesh) = ray_to_bounds.intersect_mesh(&self.ico_sphere, Cull::Back) {
            let sphere_transform = Matrix::t(transform_ico.transform_point(out_mesh));
            self.little_sphere.draw(&self.material, sphere_transform, Some(BLUE.into()), None);
        }

        self.model.draw(transform, Some(color.into()), None);
        self.ico_sphere.draw(&self.material, transform_ico, Some(YELLOW_GREEN.into()), None);
        Text::add_at(&self.text, self.transform_text, Some(self.text_style), None, None, None, None, None, None);
    }
}