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
    pub type Balances<T: Config> = StorageMap<_, _, T::AccountId, Balance>;
    // pub type Balances<T: Config> = StorageMap<Key = T::AccountId, Value = Balance>;

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

#[cfg(test)]
mod test {
    use super::{pallet as pallet_currency, *};
    use frame::testing_prelude::*;

    construct_runtime!(
        pub enum Runtime {
            System: frame_system,
            Pallet: pallet_currency
        }
    );

    #[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
    impl frame_system::Config for Runtime {
        type Block = MockBlock<Runtime>;
        type AccountId = u64;
    }

    // our simple pallet has nothing to be configured.
    impl pallet_currency::Config for Runtime {
        type RuntimeEvent = RuntimeEvent;
    }

    const ALICE: <Runtime as frame_system::Config>::AccountId = 0;
    const BOB: <Runtime as frame_system::Config>::AccountId = 1;
    const CHARLIE: <Runtime as frame_system::Config>::AccountId = 2;

    pub(crate) struct StateBuilder {
        balances: Vec<(<Runtime as frame_system::Config>::AccountId, Balance)>,
    }

    impl Default for StateBuilder {
        fn default() -> Self {
            Self {
                balances: vec![(ALICE, 100), (BOB, 100)],
            }
        }
    }

    impl StateBuilder {
        pub(crate) fn build_and_execute(self, test: impl FnOnce() -> ()) {
            let mut ext = TestState::new_empty();
            ext.execute_with(|| {
                for (who, amount) in &self.balances {
                    Balances::<Runtime>::insert(who, amount);
                    TotalIssuance::<Runtime>::mutate(|b| *b = Some(b.unwrap_or(0) + amount));
                }
            });

            ext.execute_with(test);

            // assertions that must always hold
            ext.execute_with(|| {
                assert_eq!(
                    Balances::<Runtime>::iter().map(|(_, x)| x).sum::<u128>(),
                    TotalIssuance::<Runtime>::get().unwrap_or_default()
                );
            })
        }
    }

    impl StateBuilder {
        fn add_balance(
            mut self,
            who: <Runtime as frame_system::Config>::AccountId,
            amount: Balance,
        ) -> Self {
            self.balances.push((who, amount));
            self
        }
    }

    #[test]
    fn state_builder_works() {
        StateBuilder::default().build_and_execute(|| {
            assert_eq!(Balances::<Runtime>::get(&ALICE), Some(100));
            assert_eq!(Balances::<Runtime>::get(&BOB), Some(100));
            assert_eq!(Balances::<Runtime>::get(&CHARLIE), None);
            assert_eq!(TotalIssuance::<Runtime>::get(), Some(200));
        });
    }

    #[test]
    fn state_builder_add_balance() {
        StateBuilder::default()
            .add_balance(CHARLIE, 42)
            .build_and_execute(|| {
                assert_eq!(Balances::<Runtime>::get(&CHARLIE), Some(42));
                assert_eq!(TotalIssuance::<Runtime>::get(), Some(242));
            })
    }

    #[test]
    fn mint_works() {
        StateBuilder::default().build_and_execute(|| {
            // given the initial state, when:
            assert_ok!(Pallet::mint_unsafe(RuntimeOrigin::signed(ALICE), BOB, 100));

            // then:
            assert_eq!(Balances::<Runtime>::get(&BOB), Some(200));
            assert_eq!(TotalIssuance::<Runtime>::get(), Some(300));

            // given:
            assert_ok!(Pallet::mint_unsafe(
                RuntimeOrigin::signed(ALICE),
                CHARLIE,
                100
            ));

            // then:
            assert_eq!(Balances::<Runtime>::get(&CHARLIE), Some(100));
            assert_eq!(TotalIssuance::<Runtime>::get(), Some(400));
        });
    }

    #[test]
    fn transfer_works() {
        StateBuilder::default().build_and_execute(|| {
            // given the the initial state, when:
            assert_ok!(Pallet::transfer(RuntimeOrigin::signed(ALICE), BOB, 50));

            // then:
            assert_eq!(Balances::<Runtime>::get(&ALICE), Some(50));
            assert_eq!(Balances::<Runtime>::get(&BOB), Some(150));
            assert_eq!(TotalIssuance::<Runtime>::get(), Some(200));

            System::set_block_number(1);
            // when:
            assert_ok!(Pallet::transfer(RuntimeOrigin::signed(BOB), ALICE, 50));

            // test event
            System::assert_has_event(
                Event::Transferred {
                    from: BOB,
                    to: ALICE,
                    amount: 50,
                }
                .into(),
            );

            // then:
            assert_eq!(Balances::<Runtime>::get(&ALICE), Some(100));
            assert_eq!(Balances::<Runtime>::get(&BOB), Some(100));
            assert_eq!(TotalIssuance::<Runtime>::get(), Some(200));
        });
    }

    #[test]
    fn transfer_from_non_existent_fails() {
        StateBuilder::default().build_and_execute(|| {
            // given the the initial state, when:
            assert_err!(
                Pallet::transfer(RuntimeOrigin::signed(CHARLIE), ALICE, 10),
                Error::<Runtime>::NonExistentAccount
            );

            // then nothing has changed.
            assert_eq!(Balances::<Runtime>::get(&ALICE), Some(100));
            assert_eq!(Balances::<Runtime>::get(&BOB), Some(100));
            assert_eq!(Balances::<Runtime>::get(&CHARLIE), None);
            assert_eq!(TotalIssuance::<Runtime>::get(), Some(200));
        });
    }

    #[test]
    fn transfer_exceed_balance_fails() {
        StateBuilder::default().build_and_execute(|| {
            // then nothing has changed.
            assert_eq!(Balances::<Runtime>::get(&ALICE), Some(100));

            assert_err!(
                Pallet::transfer(RuntimeOrigin::signed(ALICE), BOB, 101),
                Error::<Runtime>::InsufficientBalance
            );
        });
    }
}
