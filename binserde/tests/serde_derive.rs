use binserde::{Encoder, Decoder};
use bytes::BytesMut;

#[derive(Debug, Encoder, Decoder, Default, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum E {
    #[default]
    A = 1,
}

#[derive(Debug, Encoder, Decoder, PartialEq)]
pub struct P {
    #[binserde(bit_width = 2)]
    a: u8,
    #[binserde(bit_width = 2)]
    b: u8,
    c: u8,
    #[binserde(delimiter = b'\0')]
    str_: String,
    #[binserde(len_from = c)]
    vec: Vec<u8>,
}

#[test]
fn encode_decode_roundtrip() {
    let p = P {
        a: 4,
        b: 2,
        c: 2,
        str_: "123456".to_string(),
        vec: vec![123, 255],
    };

    let mut buf = BytesMut::with_capacity(64);

    // encode
    p.encode_be(&mut buf).expect("encode failed");

    // decode
    let mut offset = 0;
    let decoded = P::decode_be(&buf, &mut offset)
        .expect("decode failed");

    // assert
    assert_eq!(decoded, p);
}
