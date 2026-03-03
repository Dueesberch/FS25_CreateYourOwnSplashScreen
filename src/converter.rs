use ddsfile::{D3DFormat, Dds, NewD3dParams};
use image::imageops::FilterType;
use image::{Rgba, RgbaImage};
use squish::{Format, Params};
use std::fs::File;
use std::io;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use winreg::{RegKey, enums::*};

pub fn find_fs25_install_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) =
            hklm.open_subkey("SOFTWARE\\WOW6432Node\\GIANTS Software\\FarmingSimulator2025")
        {
            if let Ok(dir) = key.get_value::<String, _>("InstallDir") {
                return Some(PathBuf::from(dir));
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let steam_path = PathBuf::from(home)
                .join(".local/share/Steam/steamapps/common/Farming Simulator 25");
            if steam_path.exists() {
                return Some(steam_path);
            }
        }
    }

    let default_steam = if cfg!(windows) {
        PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common\Farming Simulator 25")
    } else {
        PathBuf::from("/usr/share/steam/steamapps/common/Farming Simulator 25")
    };

    if default_steam.exists() {
        Some(default_steam)
    } else {
        None
    }
}

pub fn install_to_game(source: PathBuf, install_dir: PathBuf) -> io::Result<()> {
    let target_dir = install_dir.join("shared");
    if !target_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "shared-Ordner nicht gefunden",
        ));
    }

    let target_path = target_dir.join("splash.dds");
    let target_path_2 = target_dir.join("splash_highlandsFishing.dds");
    let backup_path = target_dir.join("splash.dds.bak");
    let backup_path_2 = target_dir.join("splash_highlandsFishing.dds.bak");

    if target_path.exists() && !backup_path.exists() {
        std::fs::copy(&target_path, &backup_path)?;
    }
    if target_path_2.exists() && !backup_path_2.exists() {
        std::fs::copy(&target_path_2, &backup_path_2)?;
    }

    std::fs::copy(&source, target_path)?;
    std::fs::copy(source, target_path_2)?;
    Ok(())
}

pub fn convert_to_dds(
    input_path: PathBuf,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let final_size = 4096;
    let target_center_y = 3048;

    let img = image::open(&input_path)?.to_rgba8();
    let (width, height) = img.dimensions();

    let new_w = 4096;
    let aspect_ratio = height as f32 / width as f32;
    let new_h = (new_w as f32 * aspect_ratio) as u32;

    let resized = image::imageops::resize(&img, new_w, new_h, FilterType::Lanczos3);

    let mut canvas = RgbaImage::from_pixel(final_size, final_size, Rgba([0, 0, 0, 255]));

    let x_offset = 0;

    let y_offset = (target_center_y as i64) - (new_h as i64 / 2);

    image::imageops::overlay(&mut canvas, &resized, x_offset, y_offset);

    let mut compressed =
        vec![0u8; Format::Bc1.compressed_size(final_size as usize, final_size as usize)];
    Format::Bc1.compress(
        canvas.as_raw(),
        final_size as usize,
        final_size as usize,
        Params {
            algorithm: squish::Algorithm::IterativeClusterFit,
            weights: [0.2126, 0.7152, 0.0722],
            weigh_colour_by_alpha: false,
        },
        &mut compressed,
    );

    let mut dds = Dds::new_d3d(NewD3dParams {
        height: final_size,
        width: final_size,
        depth: None,
        format: D3DFormat::DXT1,
        mipmap_levels: None,
        caps2: None,
    })
    .map_err(|e| format!("DDS Header Fehler: {:?}", e))?;

    dds.data = compressed;

    let parent = input_path.parent().ok_or("Pfad ungültig")?;
    let primary_path = parent.join("splash.dds");

    let mut file = File::create(&primary_path)?;
    dds.write(&mut file)?;

    Ok(primary_path)
}
