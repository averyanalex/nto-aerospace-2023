pub mod camera;
pub mod decoder;
pub mod encoder;
pub mod muskrat;
pub mod photosaver;
pub mod phototaker;
pub mod radio;
pub mod ws;
pub mod yuvrgb;

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub enum PacketToSlave {
    TakePhoto,
}
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub enum PacketToMaster {
    VideoData(Vec<u8>),
    PhotoData(Vec<u8>),
}
