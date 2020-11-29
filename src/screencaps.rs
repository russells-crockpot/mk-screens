use std::path::PathBuf;
use anyhow::Result;
use mktemp::Temp;

use crate::vidinfo::VidInfo;

pub fn generate(path: PathBuf) -> Result<(VidInfo, Vec<PathBuf>)>{
    println!("Generating screens for {}", path.file_name().unwrap().to_str().unwrap());
    let info = VidInfo::from(path)?;
    dbg!(info);
    todo!()
}
