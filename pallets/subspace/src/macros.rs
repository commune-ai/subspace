#[macro_export]
macro_rules! define_subnet_includes {
      (
          double_maps: { $($d_variant:ident),* $(,)? },
          maps: { $($m_variant:ident),* $(,)? }
      ) => {
          #[derive(EnumIter)]
          pub enum SubnetIncludes {
              $($d_variant,)*
              $($m_variant,)*
          }

          impl SubnetIncludes {
              pub fn remove_storage<T: pallet::Config>(self, netuid: u16) {
                  match self {
                      $(
                          SubnetIncludes::$d_variant => {
                              let _ = $d_variant::<T>::clear_prefix(netuid, u32::MAX, None);
                          }
                      )*
                      $(
                          SubnetIncludes::$m_variant => {
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
