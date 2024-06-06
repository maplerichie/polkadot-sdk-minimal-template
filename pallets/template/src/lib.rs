//! A shell pallet built with [`frame`].

#![cfg_attr(not(feature = "std"), no_std)]

use frame::prelude::*;

// Re-export all pallet parts, this is needed to properly import the pallet into the runtime.
pub use pallet::*;

#[frame::pallet(dev_mode)]
// #[frame::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>>
            + IsType<<Self as frame_system::Config>::RuntimeEvent>
            + TryInto<Event<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    pub type Balance = u128;

    #[pallet::storage]
    pub type TotalIssuance<T: Config> = StorageValue<_, Balance>;

    #[pallet::storage]
    // pub type Balances<T: Config> = StorageMap<_, _, T::AccountId, Balance>;
    pub type Balances<T: Config> = StorageMap<Key = T::AccountId, Value = Balance>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A transfer succeeded.
        Transferred {
            from: T::AccountId,
            to: T::AccountId,
            amount: Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Account does not exist.
        NonExistentAccount,
        /// Account does not have enough balance.
        InsufficientBalance,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// An unsafe mint that can be called by anyone. Not a great idea.
        pub fn mint_unsafe(
            origin: T::RuntimeOrigin,
            dest: T::AccountId,
            amount: Balance,
        ) -> DispatchResult {
            // ensure that this is a signed account, but we don't really check `_anyone`.
            let _anyone = ensure_signed(origin)?;

            // update the balances map. Notice how all `<T: Config>` remains as `<T>`.
            Balances::<T>::mutate(dest, |b| *b = Some(b.unwrap_or(0) + amount));
            // update total issuance.
            TotalIssuance::<T>::mutate(|t| *t = Some(t.unwrap_or(0) + amount));

            Ok(())
        }

        /// Transfer `amount` from `origin` to `dest`.
        pub fn transfer(
            origin: T::RuntimeOrigin,
            dest: T::AccountId,
            amount: Balance,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // ensure sender has enough balance, and if so, calculate what is left after `amount`.
            let sender_balance =
                Balances::<T>::get(&sender).ok_or(Error::<T>::NonExistentAccount)?;
            // ensure!(sender_balance >= amount, "InsufficientBalance");
            let reminder = sender_balance
                .checked_sub(amount)
                .ok_or(Error::<T>::InsufficientBalance)?;

            // update sender and dest balances.
            Balances::<T>::mutate(&dest, |b| *b = Some(b.unwrap_or(0) + amount));
            Balances::<T>::insert(&sender, reminder);

            Self::deposit_event(Event::<T>::Transferred {
                from: sender,
                to: dest,
                amount,
            });

            Ok(())
        }
    }
}
