use proc_macro::TokenStream;
use quote::quote;

use crate::*;

pub fn encode_input(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    match &input.data {
        syn::Data::Struct(_) => {
            let struct_name = &input.ident;
            let generics = &input.generics;

            let struct_info = Arc::new(StructInfo::parse(&input));
            let _ = STRUCT_MAP
                .lock()
                .unwrap()
                .insert(struct_name.to_string(), struct_info.clone());

            let stmts_be = encode_struct(&input, ByteOrder::BE, &struct_info);
            let stmts_le = encode_struct(&input, ByteOrder::LE, &struct_info);

            quote! {
                impl #generics ::binserde::Encode for #struct_name #generics {
                    fn encode_be(&self, buf: &mut BytesMut) -> Result<(), ::binserde::error::EncodeError> {
                        #(#stmts_be)*;
                        Ok(())
                    }

                    fn encode_le(&self, buf: &mut BytesMut) -> Result<(), ::binserde::error::EncodeError> {
                        #(#stmts_le)*;
                        Ok(())
                    }
                }
            }
            .into()
        }
        syn::Data::Enum(_) => {
            let enum_name = &input.ident;
            let enum_info = Arc::new(EnumInfo::parse(&input));
            let _ = ENUM_MAP
                .lock()
                .unwrap()
                .insert(enum_name.to_string(), enum_info.clone());

            let stmts_be = encode_enum(&input, ByteOrder::BE, &enum_info);
            let stmts_le = encode_enum(&input, ByteOrder::LE, &enum_info);

            let generics = &input.generics;
            quote! {
                impl #generics ::binserde::Encode for #enum_name #generics {
                    fn encode_be(&self, buf: &mut BytesMut) -> Result<(), ::binserde::error::EncodeError> {
                        #(#stmts_be)*
                        Ok(())
                    }

                    fn encode_le(&self, buf: &mut BytesMut) -> Result<(), ::binserde::error::EncodeError> {
                        #(#stmts_le)*
                        Ok(())
                    }
                }
            }
            .into()
        }
        syn::Data::Union(_) => todo!(),
    }
}

fn encode_struct(
    input: &syn::DeriveInput,
    byte_order: ByteOrder,
    struct_info: &StructInfo,
) -> Vec<proc_macro2::TokenStream> {
    let mut encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();

    let syn::Data::Struct(data_struct) = &input.data else {
        return encode_stmts;
    };

    if let syn::Fields::Named(fields) = &data_struct.fields {
        let mut field_index = 0;
        let mut bitfield_section_index = 0;

        while field_index < fields.named.len() {
            let bitfield_section = &struct_info.bitfield_sections.get(bitfield_section_index);

            if bitfield_section
                .is_some_and(|(section, _)| field_index >= section[0] && field_index < section[1])
            {
                let bitfield_section = bitfield_section.unwrap();
                let section = bitfield_section.0;
                let byte_len = bitfield_section.1;
                let section_start = section[0];
                let section_end = section[1];

                encode_stmts.push(quote! {
                    let mut byte_list = [0u8; #byte_len];
                });

                let mut bit_offset = 0;

                for field_index in section_start..section_end {
                    let field = &fields.named[field_index];
                    let name = field.ident.clone().unwrap();
                    let name_str = name.to_string();
                    let ty = &field.ty;
                    let ty_str = ty.to_token_stream().to_string();
                    let field_info = &struct_info.fields[&name_str];

                    if ty_str == "bool" {
                        encode_stmts.push(quote! { let v = self.#name as u8; })
                    } else if is_enum_type(&ty_str) {
                        let enum_info = get_enum_info(&ty_str).unwrap();
                        let fixed_primitive_ty = str_to_type(&enum_info.repr_ty);
                        encode_stmts.push(quote! { let v = self.#name as #fixed_primitive_ty; })
                    } else {
                        encode_stmts.push(quote! { let v = self.#name; })
                    }

                    let bit_width = field_info.bit_width.unwrap();
                    let start_bit = bit_offset;

                    let max_value = 2u64.pow(bit_width as u32) - 1;

                    let int = syn::LitInt::new(&max_value.to_string(), Span::call_site());

                    encode_stmts.push(quote! {
                        if v > #int {
                            return Err(binserde::error::EncodeError::BitWidthLimit { field: #name_str, value: v.to_string() });
                        }
                    });

                    let stmt = match byte_order {
                        ByteOrder::BE => quote! {
                            for i in 0..#bit_width {
                                let global_bit = #start_bit + i;
                                let byte_idx = global_bit / 8;
                                let bit_idx = global_bit % 8; // LSB-first
                                let bit = (v >> (#bit_width - 1 - i)) & 1;
                                byte_list[byte_idx] |= (bit as u8) << bit_idx;
                            }
                        },
                        ByteOrder::LE => quote! {
                            for i in 0..#bit_width {
                                let global_bit = #start_bit + i;
                                let byte_idx = #byte_len - 1 - (global_bit / 8); // 小端字节翻转
                                let bit_idx = global_bit % 8; // LSB-first
                                let bit = (v >> i) & 1;
                                byte_list[byte_idx] |= (bit as u8) << bit_idx;
                            }
                        },
                    };

                    encode_stmts.push(stmt);

                    bit_offset += bit_width;
                }

                encode_stmts.push(quote! { buf.extend_from_slice(&byte_list);});

                field_index = section_end;
                bitfield_section_index += 1;
            } else {
                let field = &fields.named[field_index];
                let name = field.ident.clone().unwrap();
                let name_str = name.to_string();
                let ty_str = field.ty.to_token_stream().to_string();
                let field_info = &struct_info.fields[&name_str];

                let stmt = if is_str_type(&ty_str) {
                    match &field_info.str_delimiter {
                        Some(str_delimiter) => {
                            let str_delimiter =
                                proc_macro2::Literal::byte_string(str_delimiter.as_slice());
                            quote! { self.#name.encode_str(buf, Some(#str_delimiter)); }
                        }
                        None => quote! { self.#name.encode_str(buf, None); },
                    }
                } else {
                    match byte_order {
                        ByteOrder::BE => quote! { self.#name.encode_be(buf)?; },
                        ByteOrder::LE => quote! { self.#name.encode_le(buf)?; },
                    }
                };

                encode_stmts.push(stmt);

                field_index += 1;
            }
        }
    }

    encode_stmts
}

fn encode_enum(
    _input: &syn::DeriveInput,
    byte_order: ByteOrder,
    enum_info: &EnumInfo,
) -> Vec<proc_macro2::TokenStream> {
    let mut encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();

    let repr_ty = str_to_type(&enum_info.repr_ty);

    let stmt = match byte_order {
        ByteOrder::BE => quote! { v.encode_be(buf)?; },
        ByteOrder::LE => quote! { v.encode_le(buf)?; },
    };

    encode_stmts.push(quote! {
        let v = *self as #repr_ty;
        #stmt
    });

    encode_stmts
}
