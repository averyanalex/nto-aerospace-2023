use dcp::{ColorSpace, ImageFormat, PixelFormat};
use dcv_color_primitives as dcp;

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

// const RGB_FORMAT: ImageFormat = ImageFormat {
//     pixel_format: PixelFormat::Rgb,
//     color_space: ColorSpace::Rgb,
//     num_planes: 1,
// };

pub fn yuv_to_bgra(src_buf: &Vec<&[u8]>, strides: &[usize; 3]) -> [u8; 640 * 480 * 4] {
    dcp::initialize();

    let mut bgra_buf = [0u8; 640 * 480 * 4];
    let dst_bgra_buffers = &mut [&mut bgra_buf[..]];
    dcp::convert_image(
        640,
        480,
        &YUV_FORMAT,
        Some(strides),
        // None,
        src_buf,
        &BGRA_FORMAT,
        None,
        dst_bgra_buffers,
    )
    .unwrap();
    bgra_buf

    // let src_bgra_buffers = &[&bgra_buf[..]];
    // let mut rgb_buf = [0u8; 640 * 480 * 3];
    // let dst_rgb_buffers = &mut [&mut rgb_buf[..]];
    // dcp::convert_image(
    //     640,
    //     480,
    //     &BGRA_FORMAT,
    //     // Some(strides),
    //     None,
    //     // None,
    //     src_bgra_buffers,
    //     &RGB_FORMAT,
    //     None,
    //     dst_rgb_buffers,
    // )
    // .unwrap();
    // rgb_buf
}
