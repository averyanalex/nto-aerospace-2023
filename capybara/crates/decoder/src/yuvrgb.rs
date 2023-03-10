use anyhow::Result;
use dcv_color_primitives as dcp;

use common::{VIDEO_HEIGHT, VIDEO_WIDTH};

use dcp::{ColorSpace, ImageFormat, PixelFormat};

const YUV_FORMAT: ImageFormat = ImageFormat {
    pixel_format: PixelFormat::I444,
    color_space: ColorSpace::Bt601,
    num_planes: 3,
};

const BGRA_FORMAT: ImageFormat = ImageFormat {
    pixel_format: PixelFormat::Bgra,
    color_space: ColorSpace::Rgb,
    num_planes: 1,
};

const RGB_FORMAT: ImageFormat = ImageFormat {
    pixel_format: PixelFormat::Rgb,
    color_space: ColorSpace::Rgb,
    num_planes: 1,
};

pub fn yuv_to_bgra(src_yuv_buf: &Vec<&[u8]>, yuv_strides: &[usize; 3]) -> Result<Vec<u8>> {
    dcp::initialize();

    let mut bgra_buf: Vec<_> = vec![0u8; VIDEO_WIDTH as usize * VIDEO_HEIGHT as usize* 4];
    let dst_bgra_buf = &mut [&mut bgra_buf[..]];
    let bgra_strides = &[0usize; 1];
    dcp::convert_image(
        VIDEO_WIDTH,
        VIDEO_HEIGHT,
        &YUV_FORMAT,
        Some(yuv_strides),
        src_yuv_buf,
        &BGRA_FORMAT,
        Some(bgra_strides),
        dst_bgra_buf,
    )?;

    let src_bgra_buf = &[&bgra_buf[..]];
    let mut rgb_buf: Vec<_> = vec![0u8; VIDEO_WIDTH  as usize * VIDEO_HEIGHT as usize * 3];
    let dst_rgb_buf = &mut [&mut rgb_buf[..]];
    dcp::convert_image(
        VIDEO_WIDTH,
        VIDEO_HEIGHT,
        &BGRA_FORMAT,
        Some(bgra_strides),
        src_bgra_buf,
        &RGB_FORMAT,
        None,
        dst_rgb_buf,
    )?;
    Ok(rgb_buf)
}
