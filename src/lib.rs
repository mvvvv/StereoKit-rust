/// StereoKit-rust is a Rust binding for the StereoKit C API. It allows you to create VR applications with ease.
/// [![GitHub](https://github.com/mvvvv/StereoKit-rust/blob/master/StereoKit-rust.png)](https://github.com/mvvvv/StereoKit-rust/)
///
///
use std::{ffi::NulError, path::PathBuf};

#[cfg(feature = "event-loop")]
pub use stereokit_macros::IStepper;

pub use stereokit_macros::include_asset_tree;
pub use stereokit_macros::test_init_sk;
pub use stereokit_macros::test_screenshot;
pub use stereokit_macros::test_steps;

/// Some of the errors you might encounter when using StereoKit-rust.
use thiserror::Error;

/// Anchor related structs and functions.
///
/// With examples which are also unit tests.
pub mod anchor;

/// Font related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Font](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/font.jpeg)](font::Font)
pub mod font;

/// A namespace containing features exclusive to the rust bindings for StereoKit.
///
/// These are higher level pieces of functionality that do not necessarily adhere to the same goals and restrictions as
/// StereoKit’s core functionality does. This corresponds to the C# namespace:
/// <https://stereokit.net/Pages/StereoKit.Framework.html>
/// - An event loop manager based on Winit.
/// - HandMenuRadial related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![SkClosures](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sk_closures.jpeg)](framework::SkClosures)
/// [![IStepper](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/a_stepper.jpeg)](framework::IStepper)
/// [![StepperAction](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/stepper_actions.jpeg)](framework::StepperAction)
/// [![Steppers](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/steppers.jpeg)](framework::Steppers)
/// [![StepperClosures](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/stepper_closures.jpeg)](framework::StepperClosures)
pub mod framework;

/// Material related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Material](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/materials.jpeg)](material::Material)
/// [![Material Transparency](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/material_transparency.jpeg)](material::Material::transparency)
/// [![Material Face Cull](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/material_face_cull.jpeg)](material::Material::face_cull)
/// [![Material Parameter Info](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/param_infos.jpeg)](material::ParamInfos)
pub mod material;

/// Vec2, 3 and4, Quat and Matrix, Bounds, Plane and Ray related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Matrix](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/matrix.jpeg)](maths::Matrix)
/// [![Bounds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/bounds.jpeg)](maths::Bounds)
/// [![Plane](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/plane.jpeg)](maths::Plane)
/// [![Pose](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/pose.jpeg)](maths::Pose)
/// [![Sphere](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sphere.jpeg)](maths::Sphere)
/// [![Ray](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/ray.jpeg)](maths::Ray)
/// [![Intersect Meshes](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/intersect_meshes.jpeg)](maths::Ray::intersect_mesh)
/// [![Intersect Model](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/intersect_model.jpeg)](maths::Ray::intersect_model)
pub mod maths;

/// Mesh related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Mesh](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/meshes.jpeg)](mesh::Mesh)
/// [![Vertex](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/basic_mesh.jpeg)](mesh::Vertex)
/// [![Mesh bounds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_bounds.jpeg)](mesh::Mesh::bounds)
/// [![Mesh set_verts](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_verts.jpeg)](mesh::Mesh::set_verts)
/// [![Mesh set_inds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_set_inds.jpeg)](mesh::Mesh::set_inds)
/// [![Mesh draw](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_draw.jpeg)](mesh::Mesh::draw)
/// [![Mesh intersect](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/mesh_intersect.jpeg)](mesh::Mesh::intersect)
pub mod mesh;

/// Model related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Model](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model.jpeg)](model::Model)
/// [![Model from memory](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_memory.jpeg)](model::Model::from_memory)
/// [![Model from file](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_file.jpeg)](model::Model::from_file)
/// [![Model from mesh](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_from_mesh.jpeg)](model::Model::from_mesh)
/// [![Model bounds](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_bounds.jpeg)](model::Model::bounds)
/// [![Model draw](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_draw.jpeg)](model::Model::draw)
/// [![Model draw with material](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_draw_with_material.jpeg)](model::Model::draw_with_material)
/// [![Model intersect](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_intersect.jpeg)](model::Model::intersect)
/// [![Model Anims](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/anims.jpeg)](model::Anims)
/// [![Model Nodes](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_nodes.jpeg)](model::Nodes)
/// [![ModelNode](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/model_node.jpeg)](model::ModelNode)
pub mod model;

/// Prelude for StereoKit-rust. The basis for all StereoKit-rust programs.
pub mod prelude;

/// RenderList related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![RenderList](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list.jpeg)](render_list::RenderList)
/// [![RenderList add mesh](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_mesh.jpeg)](render_list::RenderList::add_mesh)
/// [![RenderList add model](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_add_model.jpeg)](render_list::RenderList::add_model)
/// [![RenderList draw now](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_draw_now.jpeg)](render_list::RenderList::draw_now)
/// [![RenderList push](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/render_list_push.jpeg)](render_list::RenderList::push)
pub mod render_list;

/// Shader related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Shader](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/shaders.jpeg)](shader::Shader)
pub mod shader;

/// StereoKit-rust specific structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Sk basic example](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sk_basic_example.jpeg)](sk::SkSettings::init_with_event_loop)
#[cfg(feature = "event-loop")]
pub mod sk;

/// StereoKit-rust specific structs and functions.
#[cfg(feature = "no-event-loop")]
pub mod sk;

/// Sound specific structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Sound](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sound.jpeg)](sound::Sound)
/// [![SoundInst](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sound_inst.jpeg)](sound::SoundInst)
/// [![Microphone](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/microphone.jpeg)](sound::Microphone)
pub mod sound;

/// Sprite specific structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Sprite](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite.jpeg)](sprite::Sprite)
/// [![Sprite from Tex](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_from_tex.jpeg)](sprite::Sprite::from_tex)
/// [![Sprite from File](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_from_file.jpeg)](sprite::Sprite::from_file)
/// [![Sprite draw](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_draw.jpeg)](sprite::Sprite::draw)
///
/// [![Sprite grid](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_grid.jpeg)](sprite::Sprite::grid)
/// [![Sprite list](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_list.jpeg)](sprite::Sprite::list)
/// [![Sprite arrow left](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_left.jpeg)](sprite::Sprite::arrow_left)
/// [![Sprite arrow right](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_right.jpeg)](sprite::Sprite::arrow_right)
/// [![Sprite arrow up](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_up.jpeg)](sprite::Sprite::arrow_up)
/// [![Sprite arrow down](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_arrow_down.jpeg)](sprite::Sprite::arrow_down)
/// [![Sprite radio off](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_radio_off.jpeg)](sprite::Sprite::radio_off)
/// [![Sprite radio on](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_radio_on.jpeg)](sprite::Sprite::radio_on)
/// [![Sprite toggle off](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_toggle_off.jpeg)](sprite::Sprite::toggle_off)
/// [![Sprite toggle on](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_toggle_on.jpeg)](sprite::Sprite::toggle_on)
/// [![Sprite backspace](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_backspace.jpeg)](sprite::Sprite::backspace)
/// [![Sprite close](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_close.jpeg)](sprite::Sprite::close)
/// [![Sprite shift](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/sprite_shift.jpeg)](sprite::Sprite::shift)
pub mod sprite;

pub mod system;

pub mod tex;

pub mod tools;

pub mod ui;

pub mod util;

/// Some of the errors you might encounter when using StereoKit-rust.
#[derive(Error, Debug)]
pub enum StereoKitError {
    #[error("unable to create model from file path {0}")]
    ModelFile(String),
    #[error("unable to find model with id {0}")]
    ModelFind(String),
    #[error("failed to create model {0} from memory for reason {1}")]
    ModelFromMem(String, String),
    #[error("failed to create model {0} from file for reason {1}")]
    ModelFromFile(PathBuf, String),
    #[error("failed to generate mesh {0}")]
    MeshGen(String),
    #[error("failed to find mesh {0}")]
    MeshFind(String),
    #[error("failed to convert to CString {0} in mesh_find")]
    MeshCString(String),
    #[error("failed to convert to CString {0} in tex_find")]
    TexCString(String),
    #[error("failed to find tex {0}")]
    TexFind(String),
    #[error("failed to copy tex {0}")]
    TexCopy(String),
    #[error("failed to create a tex from raw memory")]
    TexMemory,
    #[error("failed to create a tex from file {0} for reason {1}")]
    TexFile(PathBuf, String),
    #[error("failed to create a tex from multiple files {0} for reason {1}")]
    TexFiles(PathBuf, String),
    #[error("failed to create a tex from color {0} for reason {1}")]
    TexColor(String, String),
    #[error("failed to create a tex rendertarget {0} for reason {1}")]
    TexRenderTarget(String, String),
    #[error("failed to find font {0} for reason {1}")]
    FontFind(String, String),
    #[error("failed to create font from file {0} for reason {1}")]
    FontFile(PathBuf, String),
    #[error("failed to create font from multiple files {0} for reason {1}")]
    FontFiles(String, String),
    #[error("failed to create font family {0} for reason {1}")]
    FontFamily(String, String),
    #[error("failed to find shader {0} for reason {1}")]
    ShaderFind(String, String),
    #[error("failed to create shader from file {0} for reason {1}")]
    ShaderFile(PathBuf, String),
    #[error("failed to create shader from raw memory")]
    ShaderMem,
    #[error("failed to find material {0} for reason {1}")]
    MaterialFind(String, String),
    #[error("failed to create sprite from texture")]
    SpriteCreate,
    #[error("failed to create sprite from file {0}")]
    SpriteFile(PathBuf),
    #[error("failed to find sprite {0} for reason {1}")]
    SpriteFind(String, String),
    #[error("failed to find sound {0} for reason {1}")]
    SoundFind(String, String),
    #[error("failed to find render list {0} for reason {1}")]
    RenderListFind(String, String),
    #[error("failed to create sound from file {0}")]
    SoundFile(PathBuf),
    #[error("failed to create sound streaming {0}")]
    SoundCreate(String),
    #[error("failed to create anchor {0}")]
    AnchorCreate(String),
    #[error("failed to find anchor {0} for reason {1}")]
    AnchorFind(String, String),
    #[error("failed to init stereokit with settings {0}")]
    SkInit(String),
    #[cfg(feature = "event-loop")]
    #[error("failed to init stereokit event_loop")]
    SkInitEventLoop(#[from] winit::error::EventLoopError),
    #[error("failed to get a string from native C {0}")]
    CStrError(String),
    #[error("failed to read a file {0}")]
    ReadFileError(String),
    #[error("Directory {0} do not exist or is not a directory")]
    DirectoryError(String),
    #[error(transparent)]
    Other(#[from] NulError),
}
