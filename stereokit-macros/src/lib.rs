use std::{
    fs::read_dir,
    path::{Path, PathBuf},
    str::FromStr,
};

//use proc_macro::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use proc_macro::{TokenStream, TokenTree};

/// Embed the tree of the assets sub-directories in your crate.
/// useful if you want to browse some assets
#[proc_macro]
pub fn include_asset_tree(body: TokenStream) -> TokenStream {
    let mut vec_path = vec![];
    let cargo_dir = std::env::var("CARGO_MANIFEST_DIR").ok().unwrap();
    let path_cargo = Path::new(&cargo_dir);
    if let Some(TokenTree::Literal(dir)) = body.into_iter().next() {
        let mut sub_dir = dir.to_string();
        sub_dir.remove(0);
        sub_dir.pop();
        let path_assets = path_cargo.join(&sub_dir);
        if path_assets.is_dir() {
            let sub_path = Path::new(&sub_dir).to_owned();
            vec_path.append(&mut get_sub_dirs(path_assets, &sub_path))
        } else {
            vec_path.push("!!No asset dir tree!!".to_string());
        }
    }
    let stringified = format!("&{:?}", vec_path);
    TokenStream::from_str(&stringified).unwrap()

    // let body = [TokenTree::Literal(Literal::string("/assets"))].into_iter().collect();
    // [
    //     TokenTree::Punct(Punct::new('&', Spacing::Alone)),
    //     TokenTree::Group(Group::new(Delimiter::Bracket, body)),
    // ]
    // .into_iter()
    // .collect()
}

fn get_sub_dirs(path_assets: PathBuf, sub_path: &Path) -> Vec<String> {
    let mut vec_path = vec![];
    if path_assets.exists() && path_assets.is_dir() {
        vec_path.push(sub_path.to_string_lossy().to_string().replace("\\", "/"));
        if let Ok(read_dir) = read_dir(path_assets) {
            for file in read_dir.flatten() {
                let path_sub_assets = file.path();

                if path_sub_assets.is_dir() {
                    let sub_sub_path = &sub_path.join(file.file_name());
                    vec_path.append(&mut get_sub_dirs(path_sub_assets, sub_sub_path));
                }
            }
        }
    } else {
        vec_path.push("!!No asset sub dir tree!!".to_string())
    }
    vec_path
}
