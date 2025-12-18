extern crate proc_macro;

mod decoder;
mod encoder;

use std::{
    collections::{BTreeMap, HashMap},
    str::FromStr,
    sync::{Arc, LazyLock, Mutex},
};

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    Ident, Meta, Path, PathSegment, Result, Token, Type, TypePath,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

struct MetaListParser(Punctuated<Meta, Token![,]>);

impl Parse for MetaListParser {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(MetaListParser(Punctuated::parse_terminated(input)?))
    }
}

fn parse_int<T: std::str::FromStr>(expr: &syn::Expr) -> Option<T>
where
    <T as FromStr>::Err: std::fmt::Display,
{
    if let syn::Expr::Lit(expr_lit) = expr {
        if let syn::Lit::Int(lit) = &expr_lit.lit {
            return lit.base10_parse::<T>().ok();
        }
    }
    None
}

fn is_fixed_primitive_type_str(ty_str: &str) -> bool {
    matches!(
        ty_str,
        "bool" | "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "f32" | "f64"
    )
}

fn is_fixed_primitive_type(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => {
            let ident = &tp.path.segments.last().unwrap().ident.to_string();
            is_fixed_primitive_type_str(ident.as_str())
        }
        _ => false,
    }
}

fn is_str_type(ty: &str) -> bool {
    matches!(ty, "String" | "str")
}

fn is_dynamic_list_type(ty: &str) -> bool {
    matches!(ty, "Vec")
}

fn is_enum_type(enum_name: &str) -> bool {
    ENUM_MAP.lock().unwrap().contains_key(enum_name)
}

fn get_enum_info(enum_name: &str) -> Option<Arc<EnumInfo>> {
    ENUM_MAP.lock().unwrap().get(enum_name).cloned()
}

fn is_struct_type(struct_name: &str) -> bool {
    STRUCT_MAP.lock().unwrap().contains_key(struct_name)
}

#[derive(Debug, Clone, Copy)]
enum ByteOrder {
    BE,
    LE,
}

#[proc_macro_derive(Encoder, attributes(byte_order, binserde))]
pub fn encoder_derive(input: TokenStream) -> TokenStream {
    encoder::encode_input(input)
}

#[proc_macro_derive(Decoder, attributes(byte_order, binserde))]
pub fn decoder_derive(input: TokenStream) -> TokenStream {
    decoder::decode_input(input)
}

/// 将 Ident 转成 syn::Type
fn str_to_type(str: &str) -> Type {
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: vec![PathSegment::from(Ident::new(str, Span::call_site()))]
                .into_iter()
                .collect(),
        },
    })
}

/// 将 Ident 转成 syn::Type
// fn ident_to_type(ident: Ident) -> Type {
//     Type::Path(TypePath {
//         qself: None,
//         path: Path {
//             leading_colon: None,
//             segments: vec![PathSegment::from(ident.clone())].into_iter().collect(),
//         },
//     })
// }

static ENUM_MAP: LazyLock<Mutex<HashMap<String, Arc<EnumInfo>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static STRUCT_MAP: LazyLock<Mutex<HashMap<String, Arc<StructInfo>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Default, Clone)]
struct EnumInfo {
    repr_ty: String,
    variants: BTreeMap<String, Option<String>>,
}

impl EnumInfo {
    fn parse(input: &syn::DeriveInput) -> Self {
        let mut enum_info = Self::default();

        let mut repr_ty = None;

        for attr in &input.attrs {
            if attr.path().is_ident("repr") {
                if let Meta::List(meta_list) = &attr.meta {
                    let Ok(meta_list_parser) =
                        syn::parse2::<MetaListParser>(meta_list.tokens.clone())
                    else {
                        continue;
                    };

                    if let Some(meta) = meta_list_parser.0.first() {
                        repr_ty = Some(meta.path().get_ident().unwrap().to_string())
                    }
                }
            }
        }

        if let syn::Data::Enum(data_enum) = &input.data {
            for var in data_enum.variants.iter() {
                let ident = &var.ident;
                let discr = var
                    .discriminant
                    .as_ref()
                    .map(|(_, expr)| expr.to_token_stream().to_string());

                enum_info.variants.insert(ident.to_string(), discr);
            }
        }

        match repr_ty {
            Some(repr_ty) => enum_info.repr_ty = repr_ty,
            None => panic!("Enum need repr(u8/u16....)"),
        }

        enum_info
    }
}

#[derive(Debug, Clone, Default)]
struct FieldInfo {
    use_default: bool,
    default_value: Option<String>,
    bit_width: Option<usize>,
    len_from: Option<String>,
    str_delimiter: Option<Vec<u8>>,
}

#[allow(unused)]
#[derive(Debug, Default, Clone)]
struct StructInfo {
    fields: HashMap<String, FieldInfo>,
    bitfield_sections: Vec<([usize; 2], usize)>,
}

impl StructInfo {
    fn parse(input: &syn::DeriveInput) -> Self {
        let mut struct_info = Self::default();

        if let syn::Data::Struct(date_struct) = &input.data {
            if let syn::Fields::Named(fields) = &date_struct.fields {
                let mut bit_width = 0;
                let mut bitfield_start = None;
                for (i, field) in fields.named.iter().enumerate() {
                    let ident = field.ident.clone().unwrap().to_string();

                    let field_info = FieldInfo::parse(field);

                    if let Some(v) = field_info.bit_width {
                        if bitfield_start.is_none() {
                            bitfield_start = Some(i);
                        }
                        bit_width += v;
                    } else {
                        if let Some(bitfield_start) = bitfield_start.take() {
                            struct_info
                                .bitfield_sections
                                .push(([bitfield_start, i], (bit_width + 7) / 8));
                            bit_width = 0;
                        }
                    }

                    struct_info.fields.insert(ident, field_info);
                }

                if let Some(bitfield_start) = bitfield_start.take() {
                    struct_info
                        .bitfield_sections
                        .push(([bitfield_start, fields.named.len()], (bit_width + 7) / 8));
                }
            }
        }

        struct_info
    }
}

impl FieldInfo {
    fn parse(field: &syn::Field) -> Self {
        let ty_str = field.ty.to_token_stream().to_string();
        let mut info = Self::default();
        for attr in &field.attrs {
            let Some(ident) = attr.path().get_ident() else {
                continue;
            };

            let ident_str = ident.to_string();

            if ident_str.ne("binserde") {
                continue;
            }

            let Meta::List(meta_list) = &attr.meta else {
                continue;
            };

            let Ok(meta_list_parser) = syn::parse2::<MetaListParser>(meta_list.tokens.clone())
            else {
                continue;
            };

            for meta in &meta_list_parser.0 {
                if meta.path().is_ident("len_from") {
                    match meta {
                        Meta::NameValue(meta_name_value) => {
                            if let syn::Expr::Path(expr_path) = &meta_name_value.value {
                                let ident = expr_path.path.get_ident().unwrap().clone();
                                info.len_from = Some(ident.to_string());
                            }
                        }
                        Meta::List(meta_list) => {
                            let Ok(meta_list_parser) =
                                syn::parse2::<MetaListParser>(meta_list.tokens.clone())
                            else {
                                continue;
                            };

                            for meta in &meta_list_parser.0 {
                                if let Meta::NameValue(meta_name_value) = meta {
                                    if meta_name_value.path.is_ident("deserialize") {
                                        if let syn::Expr::Path(expr_path) = &meta_name_value.value {
                                            let ident = expr_path.path.get_ident().unwrap().clone();
                                            info.len_from = Some(ident.to_string());
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }

                if meta.path().is_ident("bit_width") {                    
                    if !is_enum_type(&ty_str) && !is_fixed_primitive_type_str(&ty_str) {
                        panic!("bit_width only support enum or primitive_type");
                    }

                    match meta {
                        Meta::NameValue(meta_name_value) => {
                            let bit_width = parse_int(&meta_name_value.value).unwrap();
                            info.bit_width = Some(bit_width);
                        }
                        Meta::List(meta_list) => {
                            let Ok(meta_list_parser) =
                                syn::parse2::<MetaListParser>(meta_list.tokens.clone())
                            else {
                                continue;
                            };

                            for meta in &meta_list_parser.0 {
                                if let Meta::NameValue(meta_name_value) = meta {
                                    if meta_name_value.path.is_ident("deserialize") {
                                        let bit_width = parse_int(&meta_name_value.value).unwrap();
                                        info.bit_width = Some(bit_width);
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }

                if meta.path().is_ident("default") {
                    match meta {
                        Meta::Path(_) => {
                            info.use_default = true;
                        }
                        Meta::NameValue(meta_name_value) => {
                            if let syn::Expr::Path(expr_path) = &meta_name_value.value {
                                let ident = expr_path.path.get_ident().unwrap().clone();
                                info.use_default = true;
                                info.default_value = Some(ident.to_string());
                            }
                        }
                        _ => (),
                    }
                }

                if meta.path().is_ident("delimiter") {
                    match meta {
                        Meta::NameValue(meta_name_value) => {
                            info.str_delimiter =
                                Some(Self::parse_delimiter_expr(&meta_name_value.value));
                        }
                        _ => (),
                    }
                }
            }
        }

        info
    }

    fn parse_delimiter_expr(expr: &syn::Expr) -> Vec<u8> {
        match expr {
            // delimiter = [0x00, 0xFF]
            syn::Expr::Array(arr) => Self::parse_u8_array(arr),

            // delimiter = b'\0'
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Byte(byte),
                ..
            }) => {
                vec![byte.value()]
            }

            // delimiter = 0x00 / 0 / 255 / 0b...
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(int),
                ..
            }) => {
                let v = int
                    .base10_parse::<u8>()
                    .expect("delimiter literal must fit in u8");
                vec![v]
            }

            // 不支持
            other => {
                let s = other.to_token_stream().to_string();
                panic!("Unsupported delimiter expression: {}", s);
            }
        }
    }

    fn parse_u8_array(arr: &syn::ExprArray) -> Vec<u8> {
        arr.elems
            .iter()
            .map(|elem| match elem {
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(int),
                    ..
                }) => int.base10_parse::<u8>().expect("delimiter must be u8"),
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Byte(byte),
                    ..
                }) => byte.value(),
                other => {
                    let s = other.to_token_stream().to_string();
                    panic!("Unsupported delimiter element: {}", s);
                }
            })
            .collect()
    }
}
