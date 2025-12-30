mod decode;
mod encode;
pub mod error;

#[cfg(feature = "derive")]
pub use serdeb_derive::{Decoder, Encoder};

pub use bytes::BytesMut;
pub use memchr;

use crate::error::EncodeError;

pub trait Encode {
    fn encode_be(&self, buf: &mut BytesMut) -> Result<(), EncodeError>;
    fn encode_le(&self, buf: &mut BytesMut) -> Result<(), EncodeError>;
}

pub trait EncodeStr {
    fn encode_str(&self, buf: &mut BytesMut, delimiter: Option<&[u8]>);
}

pub trait Decode: Sized {
    fn decode_be(buf: &[u8], offset: &mut usize) -> Result<Self, ()>;
    fn decode_le(buf: &[u8], offset: &mut usize) -> Result<Self, ()>;
}

pub trait DecodeStr: Sized {
    fn decode_str(buf: &[u8], offset: &mut usize, delimiter: Option<&[u8]>) -> Result<Self, ()>;
}