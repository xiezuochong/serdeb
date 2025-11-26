use syn::Type;

use super::*;

pub struct BitFieldAccum {
    pub fields: Vec<(syn::Ident, syn::Type, usize)>,
    pub total_bits: usize,
    pub started: Option<syn::Ident>,
}

impl BitFieldAccum {
    pub fn new() -> Self {
        Self {
            fields: vec![],
            total_bits: 0,
            started: None,
        }
    }

    pub fn push(&mut self, name: syn::Ident, ty: syn::Type, bit_len: usize) {
        if self.started.is_none() {
            self.started = Some(name.clone());
        }

        self.fields.push((name, ty, bit_len));
        self.total_bits += bit_len;
    }

    pub fn is_active(&self) -> bool {
        self.started.is_some()
    }

    pub fn clear(&mut self) {
        self.fields.clear();
        self.total_bits = 0;
        self.started = None;
    }
}

#[derive(Debug)]
pub struct BitAttr {
    pub bit_len: usize,
    pub bit_end: bool,
}

impl BitAttr {
    pub fn from_field(field: &syn::Field) -> Option<Self> {
        let attrs = &field.attrs;

        let mut bit_len = None;
        let mut bit_end = false;

        for attr in attrs {
            if !attr.path().is_ident("bitfield") {
                continue;
            }

            if let Meta::List(list) = &attr.meta {
                let nested = syn::parse2::<MetaListParser>(list.tokens.clone())
                    .expect("Invalid bits(...) format")
                    .0;

                for meta in nested {
                    match meta {
                        Meta::NameValue(nv) => {
                            if nv.path.is_ident("len") {
                                bit_len = parse_int::<usize>(nv.value);
                            }
                        }
                        Meta::Path(path) => {
                            if path.is_ident("end") {
                                bit_end = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        bit_len.map(|len| BitAttr {
            bit_len: len,
            bit_end,
        })
    }
}

pub fn gen_bitfield_encode(acc: &BitFieldAccum, byte_order: ByteOrder) -> proc_macro2::TokenStream {
    let mut stmts = Vec::new();

    let byte_len = (acc.total_bits + 7) / 8;

    stmts.push(quote! {
        let mut byte_array = [0u8; #byte_len];
        let mut bits: u64 = 0;
        let mut bit_shift: u32 = 0;
    });

    let mut is_first = true;

    for (name, _, bit_len) in &acc.fields {
        if is_first {
            is_first = false
        }
        stmts.push(quote! {
            let mask: u64 = ::serde_lib::mask_for_bits(#bit_len as u32);
            let value = (self.#name as u64) & mask;

            bits |= value << bit_shift;
            bit_shift += #bit_len as u32;
        });
    }

    match byte_order {
        ByteOrder::LE => {
            stmts.push(quote! {
                for i in 0..#byte_len {
                    byte_array[i] = ((bits >> (i * 8)) & 0xFF) as u8;
                }
                buf.extend_from_slice(&byte_array);
            });
        }
        ByteOrder::BE => {
            stmts.push(quote! {
                for i in 0..#byte_len {
                    byte_array[#byte_len - 1 - i] = ((bits >> (i * 8)) & 0xFF) as u8;
                }
                buf.extend_from_slice(&byte_array);
            });
        }
    }

    quote! { #(#stmts)* }
}

pub fn gen_bitfield_decode(acc: &BitFieldAccum, byte_order: ByteOrder) -> proc_macro2::TokenStream {
    let mut stmts = Vec::new();

    let byte_len = (acc.total_bits + 7) / 8;

    // --- 字节序专用 decode 模板 ---
    let read_bits = match byte_order {
        ByteOrder::LE => quote! {
            let mut bits: u64 = 0;
            // BE：高字节在前
            for i in 0..#byte_len {
                bits |= (buf[*offset + i] as u64) << ((#byte_len - 1 - i) * 8);
            }
            let mut bit_shift: u32 = 0;
            *offset += #byte_len;
        },
        ByteOrder::BE => quote! {
            let mut bits: u64 = 0;
            // LE：低字节在前
            for i in 0..#byte_len {
                bits |= (buf[*offset + i] as u64) << (i * 8);
            }
            let mut bit_shift: u32 = 0;
            *offset += #byte_len;
        },
    };

    stmts.push(read_bits);

    for (name, ty, bit_len) in &acc.fields {
        let stmt = match ty {
            Type::Path(tp) => {
                let ident = &tp.path.segments.last().unwrap().ident;
                match ident.to_string().as_str() {
                    "bool" => quote! {
                        let mask: u64 = ::serde_lib::mask_for_bits(#bit_len as u32);
                        let #name: bool = ((bits >> bit_shift) & mask) != 0;
                        bit_shift += #bit_len as u32;
                    },
                    _ => quote! {
                        let mask: u64 = ::serde_lib::mask_for_bits(#bit_len as u32);
                        let #name: #ty = ((bits >> bit_shift) & mask) as #ty;
                        bit_shift += #bit_len as u32;
                    },
                }
            }
            _ => {
                quote! {}
            }
        };

        stmts.push(stmt);
    }

    quote! { #(#stmts)* }
}
