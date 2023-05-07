#![allow(unused_must_use)]
use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::Path,
};

use rnacos_web_dist_wrap::get_embedded_file;

fn main() -> anyhow::Result<()> {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_dir_path = Path::new(&project_dir);
    let web_dir = project_dir_path.join("target").join("rnacos-web");
    load_web_resouce()?;
    if !web_dir.exists() {
        std::fs::create_dir_all(web_dir);
    }
    Ok(())
}

fn load_web_resouce() -> anyhow::Result<()> {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let project_dir_path = Path::new(&project_dir);
    let web_dir = project_dir_path.join("target").join("rnacos-web");
    let web_file_path = project_dir_path
        .join("target")
        .join(env::var("PROFILE").unwrap_or("debug".to_owned()))
        .join(format!("dist.zip"));
    let file_path_str = web_file_path.to_str().unwrap();
    if !web_file_path.exists() {
        println!("down embed file");
        match get_embedded_file("dist.zip") {
            Some(content) => {
                save_file(file_path_str,content.data.to_vec())?;
            },
            None => {},
        }
    }
    if web_file_path.exists() {
        println!("run unzip");
        unzip(file_path_str, web_dir.to_str().unwrap(), true)?;
    }
    Ok(())
}

fn save_file(file_path: &str, body: Vec<u8>) -> anyhow::Result<()> {
    let path = Path::new(file_path);
    if !path.exists() {
        if let Some(prefix) = path.parent() {
            if !prefix.exists() {
                std::fs::create_dir_all(prefix)?;
            }
        }
    }
    let mut file = OpenOptions::new().create(true).write(true).open(path)?;
    file.write_all(&body)?;
    Ok(())
}

fn unzip(from: &str, to: &str, pre_clear: bool) -> anyhow::Result<()> {
    let file = fs::File::open(from)?;
    let out_dir_path = std::path::Path::new(to);
    if pre_clear && out_dir_path.is_dir() {
        std::fs::remove_dir_all(out_dir_path)?;
    }

    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = match file.enclosed_name() {
            Some(path) => out_dir_path.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath).unwrap();
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
        // Get and Set permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    Ok(())
}
