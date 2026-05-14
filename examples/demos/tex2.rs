use stereokit_rust::{
    font::Font,
    material::{Material, Transparency},
    maths::{Matrix, Quat, Vec3},
    mesh::Mesh,
    prelude::*,
    system::{Text, TextStyle},
    tex::{Tex, TexAddress, TexFormat, TexType},
    util::{Color32, named_colors::RED},
};

const SIZE: usize = 32;
const POS: Vec3 = Vec3::new(0.0, 1.0, -0.5);
const SZ: f32 = 0.4;

/// IStepper demo: volumetric 3D texture rendered via raymarching.
///
/// Copycat of `Test3DTex.cs` from <https://github.com/StereoKit/StereoKit/blob/develop/Examples/StereoKitTest/Tests/Test3DTex.cs>.
///
/// Requires `assets/shaders/texture3d.hlsl.sks` — run `cargo compile_sks` first.
#[derive(IStepper)]
pub struct Tex2 {
    id: StepperId,
    sk_info: Option<Rc<RefCell<SkInfo>>>,

    #[allow(dead_code)]
    volume: Tex,

    material: Material,
    world: Matrix,

    pub text: String,
    pub text_style: TextStyle,
    pub transform: Matrix,
}

unsafe impl Send for Tex2 {}

impl Tex2 {
    pub fn new() -> Self {
        Self { ..Default::default() }
    }
}

impl Default for Tex2 {
    fn default() -> Self {
        // Build a 32³ RGBA8 volume containing three overlapping colored spheres.
        let volume_data = build_sphere_volume(SIZE);
        let mut volume = Tex::new(TexType::ImageNomips | TexType::Volume, TexFormat::Rgba32Srgb, None);
        volume.set_colors_3d_32(SIZE, SIZE, SIZE, &volume_data);
        volume.address_mode(TexAddress::Clamp);

        // Build the cube transform; pass its inverse-transpose to the shader for ray-space transform.
        let world = Matrix::t_s(POS, Vec3::ONE * SZ);
        let world_inv = world.get_inverse().get_transposed();

        let mut material = Material::from_file("shaders/texture3d.hlsl.sks", None).unwrap_or_default();
        material.transparency(Transparency::Blend);
        material.get_all_param_info().set_texture("volume", &volume).set_matrix("world_inv", world_inv);

        Self {
            id: "Tex2".to_string(),
            sk_info: None,

            volume,
            material,
            world,

            text: "Tex2".to_string(),
            text_style: Text::make_style(Font::default(), 0.3, RED),
            transform: Matrix::t_r((Vec3::NEG_Z * 2.5) + Vec3::Y, Quat::from_angles(0.0, 180.0, 0.0)),
        }
    }
}

impl Tex2 {
    fn start(&mut self) -> bool {
        true
    }

    fn check_event(&mut self, _id: &StepperId, _key: &str, _value: &str) {}

    fn draw(&mut self, token: &MainThreadToken) {
        Mesh::cube().draw(token, &self.material, self.world, None, None);

        Text::add_at(token, &self.text, self.transform, Some(self.text_style), None, None, None, None, None, None);
    }
}

/// Builds a SIZE³ RGBA8 volume containing three overlapping colored spheres.
///
/// Layout matches StereoKit's expected order: `x + y*size + z*size*size`.
fn build_sphere_volume(size: usize) -> Vec<Color32> {
    let centers = [Vec3::new(0.3, 0.55, 0.5), Vec3::new(0.5, 0.3, 0.5), Vec3::new(0.7, 0.3, 0.7)];
    let radii = [0.3_f32, 0.2, 0.25];
    let colors = [
        Color32::new(255, 64, 64, 255), // red
        Color32::new(64, 255, 64, 255), // green
        Color32::new(64, 64, 255, 255), // blue
    ];

    let mut data = vec![Color32::new(0, 0, 0, 0); size * size * size];
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                let p = Vec3::new(
                    (x as f32 + 0.5) / size as f32,
                    (y as f32 + 0.5) / size as f32,
                    (z as f32 + 0.5) / size as f32,
                );
                for i in 0..3 {
                    if Vec3::distance(p, centers[i]) < radii[i] {
                        data[x + y * size + z * size * size] = colors[i];
                        break;
                    }
                }
            }
        }
    }
    data
}
