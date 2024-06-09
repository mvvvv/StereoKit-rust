pub use macros::include_asset_tree;
use std::{ffi::NulError, path::PathBuf};
use thiserror::Error;
pub mod anchor;
#[cfg(feature = "event-loop")]
pub mod event_loop;
pub mod font;
#[cfg(feature = "event-loop")]
pub mod framework;
pub mod material;
pub mod maths;
pub mod mesh;
pub mod model;
pub mod render_list;
pub mod shader;
pub mod sk;
pub mod sound;
pub mod sprite;
pub mod system;
pub mod tex;
#[cfg(feature = "event-loop")]
pub mod tools;
pub mod ui;
pub mod util;

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
    #[error("failed to create a tex from raw memory")]
    TexMemory,
    #[error("failed to create a tex from file {0} for reason {1}")]
    TexFile(PathBuf, String),
    #[error("failed to create a tex from multiple files {0} for reason {1}")]
    TexFiles(PathBuf, String),
    #[error("failed to create a tex from color {0} for reason {1}")]
    TexColor(String, String),
    #[error("failed to find font {0} for reason {1}")]
    FontFind(String, String),
    #[error("failed to create font from file {0} for reason {1}")]
    FontFile(PathBuf, String),
    #[error("failed to create font from multiple files {0} for reason {1}")]
    FontFiles(String, String),
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
    #[error("failed to find sound {0}")]
    SoundFind(String),
    #[error("failed to create sound from file {0}")]
    SoundFile(PathBuf),
    #[error("failed to create sound streaming {0}")]
    SoundCreate(String),
    #[error("failed to find anchor {0} for reason {1}")]
    AnchorFind(String, String),
    #[error("failed to init stereokit with settings {0}")]
    SkInit(String),
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
