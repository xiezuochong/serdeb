use crate::{Decode, DecodeStr};

impl Decode for bool {
    #[inline]
    fn decode_be(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
        let index = *offset;
        *offset += 1;
        if *offset < buf.len() {
            Ok(buf[index] == 1)
        } else {
            Err(())
        }
    }

    #[inline]
    fn decode_le(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
        let index = *offset;
        *offset += 1;
        if *offset < buf.len() {
            Ok(buf[index] == 1)
        } else {
            Err(())
        }
    }
}

macro_rules! impl_decode_for_fixed_primitive_data {
    ($($t:ty),+ $(,)?) => {
        $(
            impl Decode for $t {
                #[inline]
                fn decode_be(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
                    use std::convert::TryInto;
                    let size = std::mem::size_of::<$t>();
                    if *offset + size <= buf.len() {
                        let bytes: [u8; size_of::<$t>()] = buf[*offset..*offset + size].try_into().unwrap();
                        *offset += size;
                        Ok(<$t>::from_be_bytes(bytes))
                    } else {
                        Err(())
                    }
                }

                #[inline]
                fn decode_le(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
                    use std::convert::TryInto;
                    let size = std::mem::size_of::<$t>();
                    if *offset + size <= buf.len() {
                        let bytes: [u8; size_of::<$t>()] = buf[*offset..*offset + size].try_into().unwrap();
                        *offset += size;
                        Ok(<$t>::from_le_bytes(bytes))
                    } else {
                        Err(())
                    }
                }
            }
        )+
    };
}

// 使用宏批量实现
impl_decode_for_fixed_primitive_data!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, f32, f64);

impl<T: Decode + Default + Copy, const N: usize> Decode for [T; N] {
    #[inline]
    fn decode_be(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
        let mut list = [T::default(); N];
        let mut err_index = None;

        for i in 0..N {
            let res = T::decode_be(buf, offset);
            match res {
                Ok(v) => list[i] = v,
                Err(_) => {
                    err_index.get_or_insert(i);
                }
            }
        }

        if let Some(_err_index) = err_index {
            Err(())
        } else {
            Ok(list)
        }
    }

    #[inline]
    fn decode_le(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
        let mut list = [T::default(); N];
        let mut err_index = None;

        for i in 0..N {
            let res = T::decode_le(buf, offset);
            match res {
                Ok(v) => list[i] = v,
                Err(_) => {
                    err_index.get_or_insert(i);
                }
            }
        }

        if let Some(_err_index) = err_index {
            Err(())
        } else {
            Ok(list)
        }
    }
}

macro_rules! impl_decode_for_tuples {
    ($($name:ident),+) => {
        impl<$( $name: Decode ),+> Decode for ( $( $name, )+ ) {
            #[inline]
            fn decode_be(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
                Ok((
                    $( <$name as Decode>::decode_be(buf, offset)?, )+
                ))
            }

            #[inline]
            fn decode_le(buf: &[u8], offset: &mut usize) -> Result<Self, ()> {
                Ok((
                    $( <$name as Decode>::decode_le(buf, offset)?, )+
                ))
            }
        }
    };
}

impl_decode_for_tuples!(A);
impl_decode_for_tuples!(A, B);
impl_decode_for_tuples!(A, B, C);
impl_decode_for_tuples!(A, B, C, D);
impl_decode_for_tuples!(A, B, C, D, E);
impl_decode_for_tuples!(A, B, C, D, E, F);
impl_decode_for_tuples!(A, B, C, D, E, F, G);
impl_decode_for_tuples!(A, B, C, D, E, F, G, H);
impl_decode_for_tuples!(A, B, C, D, E, F, G, H, I);
impl_decode_for_tuples!(A, B, C, D, E, F, G, H, I, J);
impl_decode_for_tuples!(A, B, C, D, E, F, G, H, I, J, K);
impl_decode_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);


impl DecodeStr for String {
    fn decode_str(buf: &[u8], offset: &mut usize, delimiter: Option<&[u8]>) -> Result<Self, ()> {
        match delimiter {
            None => Ok(String::from_utf8_lossy(&buf[*offset..]).to_string()),
            Some(delimiter) => match memchr::memmem::find(&buf[*offset..], delimiter) {
                Some(pos) => {
                    let v = String::from_utf8_lossy(&buf[*offset..(*offset + pos)]).to_string();
                    *offset += pos + delimiter.len();
                    Ok(v)
                }
                None => Err(()),
            },
        }
    }
}
