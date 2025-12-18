use bytes::BufMut;

use crate::{Encode, EncodeStr, error::EncodeError};

impl Encode for bool {
    #[inline]
    fn encode_be(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
        buf.put_u8(*self as u8);
        Ok(())
    }

    #[inline]
    fn encode_le(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
        buf.put_u8(*self as u8);
        Ok(())
    }
}

macro_rules! impl_encode_for_fixed_primitive_data {
    ($($t:ty),+ $(,)?) => {
        $(
            impl Encode for $t {
                #[inline]
                fn encode_be(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
                    buf.extend_from_slice(&self.to_be_bytes());
                    Ok(())
                }

                #[inline]
                fn encode_le(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
                    buf.extend_from_slice(&self.to_le_bytes());
                    Ok(())
                }
            }
        )+
    };
}

impl_encode_for_fixed_primitive_data!(u8, i8, u16, i16, u32, i32, u64, i64, u128, i128, f32, f64);

impl<T: Encode> Encode for [T] {
    #[inline]
    fn encode_be(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
        for x in self {
            x.encode_be(buf)?;
        }
        Ok(())
    }

    #[inline]
    fn encode_le(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
        for x in self {
            x.encode_le(buf)?;
        }
        Ok(())
    }
}

macro_rules! impl_encode_for_tuples {
    ($($name:ident),+) => {
        impl<$( $name: Encode ),+> Encode for ( $( $name, )+ ) {
            #[inline]
            fn encode_be(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
                #[allow(non_snake_case)]
                let ( $( $name, )+ ) = self;
                $( $name.encode_be(buf)?; )+
                Ok(())
            }

            #[inline]
            fn encode_le(&self, buf: &mut bytes::BytesMut) -> Result<(), EncodeError> {
                #[allow(non_snake_case)]
                let ( $( $name, )+ ) = self;
                $( $name.encode_le(buf)?; )+
                Ok(())
            }
        }
    };
}

impl_encode_for_tuples!(A);
impl_encode_for_tuples!(A, B);
impl_encode_for_tuples!(A, B, C);
impl_encode_for_tuples!(A, B, C, D);
impl_encode_for_tuples!(A, B, C, D, E);
impl_encode_for_tuples!(A, B, C, D, E, F);
impl_encode_for_tuples!(A, B, C, D, E, F, G);
impl_encode_for_tuples!(A, B, C, D, E, F, G, H);
impl_encode_for_tuples!(A, B, C, D, E, F, G, H, I);
impl_encode_for_tuples!(A, B, C, D, E, F, G, H, I, J);
impl_encode_for_tuples!(A, B, C, D, E, F, G, H, I, J, K);
impl_encode_for_tuples!(A, B, C, D, E, F, G, H, I, J, K, L);

impl EncodeStr for str {
    #[inline]
    fn encode_str(&self, buf: &mut bytes::BytesMut, delimiter: Option<&[u8]>) {
        buf.extend_from_slice(self.as_bytes());
        if let Some(delimiter) = delimiter {
            buf.extend_from_slice(delimiter);
        }
    }
}
