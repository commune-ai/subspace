use frame_system::ensure_root;

use crate::*;

impl<T: Config> Pallet<T> {
    pub fn is_senate_member(
      key: T::AccountId,
    ) -> bool {
      SenateMembers::<T>::contains_key(key)
    }

    pub fn do_add_senate_member(
        origin: OriginFor<T>,
        senate_member_key: T::AccountId,
    ) -> DispatchResult {
        ensure_root(origin)?;

        // Check if the senate member already exists
        ensure!(
            !SenateMembers::<T>::contains_key(&senate_member_key),
            Error::<T>::SenateMemberExists
        );

        // Add the senate member
        SenateMembers::<T>::insert(&senate_member_key, ());

        Ok(())
    }

    pub fn do_remove_senate_member(
      origin: OriginFor<T>,
      senate_member_key: T::AccountId,
    ) -> DispatchResult {
      ensure_root(origin)?;

      // Check if the senate member exists
      ensure!(
          SenateMembers::<T>::contains_key(&senate_member_key),
          Error::<T>::SenateMemberNotFound
      );

      // Add the senate member
      SenateMembers::<T>::remove(&senate_member_key);

      Ok(())
    }
}
