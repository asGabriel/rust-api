// Macro to generate getters for a struct
#[macro_export]
macro_rules! getters {
    ($struct_name:ident { $($field:ident: $type:ty),* $(,)? }) => {
        impl $struct_name {
            $(
                pub fn $field(&self) -> &$type {
                    &self.$field
                }
            )*
        }
    };
}

// Macro to generate from_row constructor for database structs
#[macro_export]
macro_rules! from_row_constructor {
    ($struct_name:ident { $($field:ident: $type:ty),* $(,)? }) => {
        impl $struct_name {
            /// Constructor para criar instância com todos os campos (útil para dados vindos do banco)
            #[allow(clippy::too_many_arguments)]
            pub fn from_row(
                $($field: $type),*
            ) -> Self {
                Self {
                    $($field),*
                }
            }
        }
    };
}
