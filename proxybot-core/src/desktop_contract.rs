//! Minimal Rust-first TypeScript declarations for desktop wire DTOs.
//!
//! This deliberately models only JSON shapes. It has no Tauri dependency and
//! keeps the Rust struct plus its exported TypeScript fields in one macro
//! invocation, so a field cannot be changed on one side without changing the
//! other.

/// A Rust value that has a stable JSON representation in TypeScript.
pub trait WireType {
    fn type_script_type() -> String;
}

/// A named wire DTO that can be emitted into the generated desktop contract.
pub trait DesktopContractType: WireType {
    const NAME: &'static str;

    fn type_script_declaration() -> String;
}

macro_rules! number_wire_types {
    ($($type:ty),+ $(,)?) => {
        $(
            impl WireType for $type {
                fn type_script_type() -> String {
                    "number".to_owned()
                }
            }
        )+
    };
}

number_wire_types!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize, f32, f64);

impl WireType for String {
    fn type_script_type() -> String {
        "string".to_owned()
    }
}

impl WireType for bool {
    fn type_script_type() -> String {
        "boolean".to_owned()
    }
}

impl WireType for serde_json::Value {
    fn type_script_type() -> String {
        "JsonValue".to_owned()
    }
}

impl<T: WireType> WireType for Option<T> {
    fn type_script_type() -> String {
        format!("{} | null", T::type_script_type())
    }
}

impl<T: WireType> WireType for Vec<T> {
    fn type_script_type() -> String {
        format!("Array<{}>", T::type_script_type())
    }
}

impl<A: WireType, B: WireType> WireType for (A, B) {
    fn type_script_type() -> String {
        format!("[{}, {}]", A::type_script_type(), B::type_script_type())
    }
}

/// Define a serializable Rust DTO and its generated TypeScript interface from
/// the same field list.
#[macro_export]
macro_rules! desktop_contract_type {
    (
        $(#[$struct_meta:meta])*
        $visibility:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                pub $field:ident: $field_type:ty
            ),* $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        $visibility struct $name {
            $(
                $(#[$field_meta])*
                pub $field: $field_type,
            )*
        }

        impl $crate::desktop_contract::WireType for $name {
            fn type_script_type() -> String {
                stringify!($name).to_owned()
            }
        }

        impl $crate::desktop_contract::DesktopContractType for $name {
            const NAME: &'static str = stringify!($name);

            fn type_script_declaration() -> String {
                use std::fmt::Write as _;

                let mut declaration = format!("export interface {} {{\n", Self::NAME);
                $(
                    writeln!(
                        declaration,
                        "  {}: {};",
                        stringify!($field),
                        <$field_type as $crate::desktop_contract::WireType>::type_script_type()
                    )
                    .expect("writing to a String cannot fail");
                )*
                declaration.push_str("}\n");
                declaration
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{DesktopContractType, WireType};

    crate::desktop_contract_type! {
        #[derive(Debug, serde::Serialize)]
        struct Example {
            pub id: String,
            pub values: Vec<Option<u64>>,
        }
    }

    #[test]
    fn renders_json_wire_types_deterministically() {
        assert_eq!(
            Example::type_script_declaration(),
            "export interface Example {\n  id: string;\n  values: Array<number | null>;\n}\n"
        );
        assert_eq!(Example::type_script_type(), "Example");
    }
}
