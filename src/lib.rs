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

pub mod model;

pub mod prelude;

pub mod render_list;

/// Shader related structs and functions.
///
/// ## Examples
/// which are also unit tests:
///
/// [![Shader](https://raw.githubusercontent.com/mvvvv/StereoKit-rust/refs/heads/master/screenshots/shaders.jpeg)](shader::Shader)
pub mod shader;

pub mod sk;

pub mod sound;

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
