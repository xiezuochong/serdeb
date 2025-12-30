use serdeb::{DecodeStr, EncodeStr};

#[test]
fn test() {
    use bytes::BytesMut;

    let mut buf = BytesMut::new();

    let str = "123456";

    str.encode_str(&mut buf, Some(&[b'\0']));

    let res = String::decode_str(&buf, &mut 0, Some(&[b'\0'])).unwrap();

    assert_eq!(str, res);

    buf.clear();

}
