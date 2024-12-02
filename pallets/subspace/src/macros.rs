/// Defines subnet storage items and provides functionality to manage them.
///
/// This macro creates storage management structures for subnets with two main categories:
/// - double_maps: For storage items requiring two keys
/// - maps: For storage items requiring one key, with optional default values
///
/// # Arguments
///
/// * `double_maps` - List of identifiers for double map storage items
/// * `maps` - List of map storage items, optionally with type and default value
///
/// # Example
///
/// ```rust,ignore
/// use pallet_subspace::define_subnet_includes;
/// use frame_support::traits::Get;
///
/// define_subnet_includes!(
///     double_maps: {
///         Bonds,              // Simple double map
///         Uids,              // Another double map
///     },
///     maps: {
///         BondsMovingAverage: u64 = 900_000,  // Map with type and default
///         ValidatorPermits,                    // Simple map without default
///         MaxAllowedUids: u16 = 420,          // Another map with default
///     }
/// );
/// ```
///
/// # Generated Items
///
/// * An enum `SubnetIncludes` containing all storage variants
/// * Default value implementations for maps where specified
/// * Methods:
///   - `remove_storage`: Removes storage for a given netuid
///   - `all`: Returns a vector of all variants
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
/// Defines module vectors and storage items with swap functionality.
///
/// This macro creates storage management structures for modules with three main categories:
/// - vectors: For vector storage items
/// - swap_storages: Storage items that can be swapped (optional and required)
/// - key_storages: For managing UID-key mappings
///
/// # Arguments
///
/// * `vectors` - List of vector storage items
/// * `swap_storages` - Two categories of swappable storage:
///   - `optional`: Items that may or may not exist
///   - `required`: Items that must exist
/// * `key_storages` - UID and key mapping storage identifiers
///
/// # Example
///
/// ```rust,ignore
/// use pallet_subspace::define_module_includes;
/// use frame_support::{traits::Get, dispatch::DispatchResult};
/// use sp_std::vec::Vec;
///
/// define_module_includes!(
///     vectors: {
///         Active: bool = true,
///         Trust: u64 = 0,
///         Rank: u64 = 0
///     },
///     swap_storages: {
///         optional: {
///         },
///         required: {
///             RegistrationBlock: u64 = 0,
///             Address: Vec<u8> = Vec::new()
///         }
///     },
///     key_storages: {
///         uid_key: Uids,
///         key_uid: Keys
///     }
/// );
/// ```
///
/// # Generated Items
///
/// * Enum `ModuleVectors` for vector storage items
/// * Enum `ModuleSwapStorages` for swappable storage items
/// * Struct `KeyStorageHandler` for managing key-related operations
/// * Methods for each type:
///   - `swap_and_remove`: Handles swapping and removing items
///   - `all`: Returns a vector of all variants
#[macro_export]
macro_rules! define_module_includes {
    (
        vectors: {
            $( $vec:ident $(: $vec_type:ty = $vec_default:expr)? ),* $(,)?
        },
        swap_storages: {
            optional: {
                $( $opt_swap:ident $(: $opt_type:ty = $opt_default:expr)? ),* $(,)?
            },
            required: {
                $( $req_swap:ident $(: $req_type:ty = $req_default:expr)? ),* $(,)?
            }
        },
        key_storages: {
            $( uid_key: $uid_storage:ident, )?
            $( key_uid: $key_storage:ident )?
        },
        key_only_storages: {
            $( $key_only:ident $(: $key_only_type:ty)? ),* $(,)?
        }
    ) => {
        #[allow(dead_code)]
        #[derive(strum::EnumIter)]
        pub enum ModuleVectors {
            $($vec,)*
        }

        #[allow(dead_code)]
        impl ModuleVectors {
            pub fn swap_and_remove<T: Config>(self, netuid: u16, uid: u16, replace_uid: u16) -> DispatchResult {
                match self {
                    $(
                        Self::$vec => {
                            let mut vec = $vec::<T>::get(netuid);
                            if let (Some(replace_value), Some(entry)) = (
                                vec.get(replace_uid as usize).copied(),
                                vec.get_mut(uid as usize)
                            ) {
                                *entry = replace_value;
                                vec.pop();
                                $vec::<T>::insert(netuid, vec);
                            };
                            Ok(())
                        },
                    )*
                }
            }

            pub fn append<T: Config>(self, netuid: u16) -> DispatchResult {
                match self {
                    $(
                        Self::$vec => {
                            let mut vec = $vec::<T>::get(netuid);
                            $(
                                vec.push($vec_default);
                            )?
                            $vec::<T>::insert(netuid, vec);
                            Ok(())
                        },
                    )*
                }
            }

            pub fn all() -> sp_std::vec::Vec<Self> {
                use strum::IntoEnumIterator;
                Self::iter().collect()
            }
        }

        #[allow(dead_code)]
        #[derive(strum::EnumIter)]
        pub enum ModuleSwapStorages {
            $($opt_swap,)*
            $($req_swap,)*
        }

        #[allow(dead_code)]
        impl ModuleSwapStorages {
            pub fn swap_and_remove<T: Config>(self, netuid: u16, uid: u16, replace_uid: u16) -> DispatchResult {
                match self {
                    $(
                        Self::$opt_swap => {
                            if let Some(value) = $opt_swap::<T>::get(netuid, replace_uid) {
                                $opt_swap::<T>::insert(netuid, uid, value);
                                $opt_swap::<T>::remove(netuid, replace_uid);
                            }
                            Ok(())
                        },
                    )*
                    $(
                        Self::$req_swap => {
                            if let Ok(value) = $req_swap::<T>::try_get(netuid, replace_uid) {
                                $req_swap::<T>::insert(netuid, uid, value);
                                $req_swap::<T>::remove(netuid, replace_uid);
                            }
                            Ok(())
                        },
                    )*
                }
            }

            pub fn initialize<T: Config>(self, netuid: u16, uid: u16) -> DispatchResult {
                match self {
                    $(
                        Self::$opt_swap => {
                            $($opt_swap::<T>::insert(netuid, uid, $opt_default);)?
                            Ok(())
                        },
                    )*
                    $(
                        Self::$req_swap => {
                            $($req_swap::<T>::insert(netuid, uid, $req_default);)?
                            Ok(())
                        },
                    )*
                }
            }

            pub fn all() -> sp_std::vec::Vec<Self> {
                use strum::IntoEnumIterator;
                Self::iter().collect()
            }
        }

        #[allow(dead_code)]
        #[derive(strum::EnumIter)]
        pub enum ModuleKeyOnlyStorages {
            $($key_only,)*
        }

        #[allow(dead_code)]
        impl ModuleKeyOnlyStorages {
            pub fn remove<T: Config>(self, netuid: u16, key: &T::AccountId) -> DispatchResult {
                match self {
                    $(
                        Self::$key_only => {
                            $key_only::<T>::remove(netuid, key);
                            Ok(())
                        },
                    )*
                }
            }

            pub fn all() -> sp_std::vec::Vec<Self> {
                use strum::IntoEnumIterator;
                Self::iter().collect()
            }
        }

        #[allow(dead_code)]
        pub struct StorageHandler;

        #[allow(dead_code)]
        impl StorageHandler {
            pub fn remove_all<T: Config>(
                netuid: u16,
                uid: u16,
                replace_uid: u16,
                module_key: &T::AccountId,
                replace_key: &T::AccountId,
            ) -> DispatchResult {
                $(
                    $uid_storage::<T>::insert(netuid, replace_key, uid);
                    $uid_storage::<T>::remove(netuid, module_key);
                )?
                $(
                    $key_storage::<T>::insert(netuid, uid, replace_key.clone());
                    $key_storage::<T>::remove(netuid, replace_uid);
                )?

                for storage in ModuleKeyOnlyStorages::all() {
                    storage.remove::<T>(netuid, module_key)?;
                }

                for vector in ModuleVectors::all() {
                    vector.swap_and_remove::<T>(netuid, uid, replace_uid)?;
                }

                for storage in ModuleSwapStorages::all() {
                    storage.swap_and_remove::<T>(netuid, uid, replace_uid)?;
                }

                Ok(())
            }

            pub fn initialize_all<T: Config>(
                netuid: u16,
                uid: u16,
                key: &T::AccountId,
            ) -> DispatchResult {
                $(
                    $uid_storage::<T>::insert(netuid, key, uid);
                )?
                $(
                    $key_storage::<T>::insert(netuid, uid, key.clone());
                )?

                for storage in ModuleSwapStorages::all() {
                    storage.initialize::<T>(netuid, uid)?;
                }

                for vector in ModuleVectors::all() {
                    vector.append::<T>(netuid)?;
                }

                Ok(())
            }
        }
    };
}
