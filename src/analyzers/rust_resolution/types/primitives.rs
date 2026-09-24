//! Explicit primitive identities for receiver constraints.

/// Primitive identities cannot alias nominal project declarations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PrimitiveType {
    Bool,
    Char,
    Str,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    F32,
    F64,
}

impl PrimitiveType {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "bool" => Some(Self::Bool),
            "char" => Some(Self::Char),
            "str" => Some(Self::Str),
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "usize" => Some(Self::Usize),
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "isize" => Some(Self::Isize),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            _ => None,
        }
    }
}
