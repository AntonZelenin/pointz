use anyhow::*;
use iced_wgpu::wgpu;
use image::{GenericImageView, RgbaImage};
use std::path::Path;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub enum TextureType {
    Diffuse,
    Normal,
    Depth,
}

pub struct Texture {
    pub label: String,
    pub dimensions: (u32, u32),
    pub rgba_image: Option<RgbaImage>,
    pub type_: TextureType,
}

impl Texture {
    pub fn from_bytes(bytes: &[u8], label: &str, is_normal_map: bool) -> Result<Texture> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(&img, label, is_normal_map)
    }

    pub fn from_image(img: &image::DynamicImage, label: &str, is_normal_map: bool) -> Result<Self> {
        let dimensions = img.dimensions();
        // todo I can store rgba_image in a Texture
        let rgba_image = img.to_rgba8();
        let type_ = if is_normal_map {
            TextureType::Normal
        } else {
            TextureType::Diffuse
        };

        Ok(Texture {
            label: label.to_string(),
            dimensions,
            rgba_image: Some(rgba_image),
            type_,
        })
    }

    pub fn load<P: AsRef<Path>>(path: P, is_normal_map: bool) -> Result<Texture> {
        let path_copy = path.as_ref().to_path_buf();
        let label = match path_copy.to_str() {
            Some(l) => l,
            None => "no_name",
        };

        let img = image::open(path)?;
        // it will crash if label is longer then 64
        Self::from_image(&img, label, is_normal_map)
    }

    pub fn create_depth_texture(sc_desc: &wgpu::SwapChainDescriptor, label: &str) -> Self {
        Texture {
            label: label.to_string(),
            dimensions: (sc_desc.width, sc_desc.height),
            rgba_image: None,
            type_: TextureType::Depth,
        }
    }
}
