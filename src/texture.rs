use anyhow::*;
use iced_wgpu::wgpu;
use image::{GenericImageView, RgbaImage};
use std::path::Path;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

// pub struct Texture {
//     pub texture: wgpu::Texture,
//     pub view: wgpu::TextureView,
//     pub sampler: wgpu::Sampler,
// }
//
// impl Texture {
//     pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
//
//     pub fn from_bytes(
//         device: &wgpu::Device,
//         queue: &wgpu::Queue,
//         bytes: &[u8],
//         label: &str,
//         is_normal_map: bool,
//     ) -> Result<Self> {
//         let img = image::load_from_memory(bytes)?;
//         Self::from_image(device, queue, &img, Some(label), is_normal_map)
//     }
//
//     pub fn from_image(
//         device: &wgpu::Device,
//         queue: &wgpu::Queue,
//         img: &image::DynamicImage,
//         label: Option<&str>,
//         is_normal_map: bool,
//     ) -> Result<Self> {
//         let dimensions = img.dimensions();
//         // todo it might break something, delete todo if all is working
//         // todo I can store rgba_image in a Texture
//         let rgba_image = img.to_rgba8();
//
//         let size = wgpu::Extent3d {
//             width: dimensions.0,
//             height: dimensions.1,
//             depth: 1,
//         };
//         let texture = device.create_texture(&wgpu::TextureDescriptor {
//             label,
//             size,
//             mip_level_count: 1,
//             sample_count: 1,
//             dimension: wgpu::TextureDimension::D2,
//             format: if is_normal_map {
//                 wgpu::TextureFormat::Rgba8Unorm
//             } else {
//                 wgpu::TextureFormat::Rgba8UnormSrgb
//             },
//             usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
//         });
//
//         queue.write_texture(
//             wgpu::TextureCopyView {
//                 texture: &texture,
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//             },
//             &rgba_image,
//             wgpu::TextureDataLayout {
//                 offset: 0,
//                 bytes_per_row: 4 * dimensions.0,
//                 rows_per_image: dimensions.1,
//             },
//             size,
//         );
//
//         let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
//         let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
//             address_mode_u: wgpu::AddressMode::ClampToEdge,
//             address_mode_v: wgpu::AddressMode::ClampToEdge,
//             address_mode_w: wgpu::AddressMode::ClampToEdge,
//             mag_filter: wgpu::FilterMode::Linear,
//             min_filter: wgpu::FilterMode::Nearest,
//             mipmap_filter: wgpu::FilterMode::Nearest,
//             ..Default::default()
//         });
//
//         Ok(Self {
//             texture,
//             view,
//             sampler,
//         })
//     }
//
//     pub fn create_depth_texture(
//         device: &wgpu::Device,
//         sc_desc: &wgpu::SwapChainDescriptor,
//         label: &str,
//     ) -> Self {
//         let size = wgpu::Extent3d {
//             width: sc_desc.width,
//             height: sc_desc.height,
//             depth: 1,
//         };
//         let desc = wgpu::TextureDescriptor {
//             label: Some(label),
//             size,
//             mip_level_count: 1,
//             sample_count: 1,
//             dimension: wgpu::TextureDimension::D2,
//             format: Self::DEPTH_FORMAT,
//             usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
//                 | wgpu::TextureUsage::SAMPLED
//                 | wgpu::TextureUsage::COPY_SRC,
//         };
//         let texture = device.create_texture(&desc);
//
//         let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
//         let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
//             address_mode_u: wgpu::AddressMode::ClampToEdge,
//             address_mode_v: wgpu::AddressMode::ClampToEdge,
//             address_mode_w: wgpu::AddressMode::ClampToEdge,
//             mag_filter: wgpu::FilterMode::Linear,
//             min_filter: wgpu::FilterMode::Nearest,
//             mipmap_filter: wgpu::FilterMode::Nearest,
//             lod_min_clamp: -100.0,
//             lod_max_clamp: 100.0,
//             compare: Some(wgpu::CompareFunction::LessEqual),
//             ..Default::default()
//         });
//
//         Self {
//             texture,
//             view,
//             sampler,
//         }
//     }
//
//     pub fn load<P: AsRef<Path>>(
//         device: &wgpu::Device,
//         queue: &wgpu::Queue,
//         path: P,
//         is_normal_map: bool,
//     ) -> Result<Self> {
//         let path_copy = path.as_ref().to_path_buf();
//         let label = path_copy.to_str();
//
//         let img = image::open(path)?;
//         // todo it will crash if label is longer then 64
//         Self::from_image(device, queue, &img, label, is_normal_map)
//     }
// }

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
        // todo it might break something, delete todo if all is working
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
        // todo it will crash if label is longer then 64
        Self::from_image(&img, label, is_normal_map)
    }

    pub fn create_depth_texture(sc_desc: &wgpu::SwapChainDescriptor, label: &str) -> Self {
        // let size = wgpu::Extent3d {
        //     width: sc_desc.width,
        //     height: sc_desc.height,
        //     depth: 1,
        // };
        // let desc = wgpu::TextureDescriptor {
        //     label: Some(label),
        //     size,
        //     mip_level_count: 1,
        //     sample_count: 1,
        //     dimension: wgpu::TextureDimension::D2,
        //     format: DEPTH_FORMAT,
        //     usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT
        //         | wgpu::TextureUsage::SAMPLED
        //         | wgpu::TextureUsage::COPY_SRC,
        // };
        // let texture = device.create_texture(&desc);

        // let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        // let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        //     address_mode_u: wgpu::AddressMode::ClampToEdge,
        //     address_mode_v: wgpu::AddressMode::ClampToEdge,
        //     address_mode_w: wgpu::AddressMode::ClampToEdge,
        //     mag_filter: wgpu::FilterMode::Linear,
        //     min_filter: wgpu::FilterMode::Nearest,
        //     mipmap_filter: wgpu::FilterMode::Nearest,
        //     lod_min_clamp: -100.0,
        //     lod_max_clamp: 100.0,
        //     compare: Some(wgpu::CompareFunction::LessEqual),
        //     ..Default::default()
        // });

        Texture {
            label: label.to_string(),
            dimensions: (sc_desc.width, sc_desc.height),
            rgba_image: None,
            type_: TextureType::Depth,
        }
    }
}
