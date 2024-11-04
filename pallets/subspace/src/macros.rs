/// Defines subnet storage items and provides functionality to manage them.
/// Creates an enum `SubnetIncludes` with variants for double maps and regular maps,
/// generates default value types for maps where specified, and provides methods
/// to remove storage and get all variants.
#[macro_export]
macro_rules! define_subnet_includes {
    (
        double_maps: { $($d_variant:ident),* $(,)? },
        maps: {
            $( $m_variant:ident $(: $type:ty = $default:expr)? ),* $(,)?
        }
    ) => {
        $(
            $(
                paste::paste! {
                    pub struct [<$m_variant DefaultValue>];
                    impl Get<$type> for [<$m_variant DefaultValue>] {
                        fn get() -> $type {
                            $default
                        }
                    }
                }
            )?
        )*

        #[derive(strum::EnumIter)]
        pub enum SubnetIncludes {
            $($d_variant,)*
            $($m_variant,)*
        }

        impl SubnetIncludes {
            pub fn remove_storage<T: pallet::Config>(self, netuid: u16) {
                match self {
                    $(
                        Self::$d_variant => {
                            let _ = $d_variant::<T>::clear_prefix(netuid, u32::MAX, None);
                        }
                    )*
                    $(
                        Self::$m_variant => {
                            $m_variant::<T>::remove(netuid);
                        }
                    )*
                }
            }

            pub fn all() -> sp_std::vec::Vec<Self> {
                use strum::IntoEnumIterator;
                Self::iter().collect()
            }
        }
    };
}
