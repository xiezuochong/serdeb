use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("Field: {field:?}, Value: {value:?} owerflow, please check bitwidth limit")]
    BitWidthLimit { field: &'static str, value: String },
}

#[derive(Error, Debug)]
pub enum DecodeError {
    
}