///
/// Author: Zatoichi Labs
///

use support::{
    decl_module, decl_storage, decl_event, ensure,
    dispatch::Result, StorageMap,
};
use system::{
    //ensure_inherent,
    ensure_signed,
};

use primitives::U256;

// Custom types
pub type TokenId = U256;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as PlasmaCashModule {
        // State Database of Token: Transaction pairs
        Tokens get(tokens) build(
            |config: &GenesisConfig<T>| config.initial_db.clone()
        ): map TokenId => Option<T::AccountId>;
    }

    // Genesis may be empty (or not, if starting with some initial params)
    // Note: Might be desirable for privacy properties to start non-empty?
    add_extra_genesis {
        config(initial_db): Vec<(TokenId, T::AccountId)>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn transfer(origin, token_id: TokenId, new_owner: T::AccountId) -> Result {
            let who = ensure_signed(origin)?;

            // Token should exist, check and get current owner
            ensure!(
                <Tokens<T>>::exists(token_id),
                "Token must exist!"
            );
            let prev_owner: T::AccountId = <Tokens<T>>::get(token_id)
                .expect("should pass if above works; qed");

            // Check provenence
            ensure!(
                prev_owner == who,
                "Current owner did not sign transaction!"
            );

            // Overwrite previous entry, but that's okay because we check it above
            <Tokens<T>>::insert(token_id, &new_owner);

            // Emit Event
            Self::deposit_event(RawEvent::Transfer(token_id, who, new_owner));
            Ok(())
        }

        pub fn deposit(origin, token_id: TokenId) -> Result {
            let who = ensure_signed(origin)?;

            // TODO find a way to sync with rootchain deposit events

            // Token should not exist yet
            ensure!(
                !<Tokens<T>>::exists(token_id),
                "Token already exists!"
            );

            //  adds deposit from Rootchain into state/txn database
            <Tokens<T>>::insert(token_id, &who);

            // Emit Event
            Self::deposit_event(RawEvent::Deposit(token_id, who));
            Ok(())
        }

        pub fn withdraw(origin, token_id: TokenId) -> Result {
            let who = ensure_signed(origin)?;

            // TODO find a way to sync with rootchain finalize withdrawal events

            // Token should exist, check and get current owner
            ensure!(
                <Tokens<T>>::exists(token_id),
                "Token must exist!"
            );
            let prev_owner: T::AccountId = <Tokens<T>>::get(token_id)
                .expect("should pass if above works; qed");

            // Check provenence
            ensure!(
                prev_owner == who,
                "Current owner did not sign transaction!"
            );

            //  removes tokenId from state database (after withdrawal finalizes)
            <Tokens<T>>::remove(token_id);

            // Emit Event
            Self::deposit_event(RawEvent::Withdraw(token_id, who));
            Ok(())
        }

        fn on_finalize() {
            //  publish block to rootchain (not sure if this is possible)
            //  reset txn database (since last synced with rootchain)
        }
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        Deposit(TokenId, AccountId),
        Transfer(TokenId, AccountId, AccountId),
        Withdraw(TokenId, AccountId),
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    use runtime_io::with_externalities;
    use primitives::{H256, Blake2Hasher};
    use support::{impl_outer_origin, assert_ok, parameter_types, assert_noop, impl_outer_event};
    use sr_primitives::{traits::{BlakeTwo256, IdentityLookup}, testing::Header};
    use sr_primitives::weights::Weight;
    use sr_primitives::Perbill;

    impl_outer_origin! {
        pub enum Origin for Test {}
    }

    use crate::plasma_cash as module;
    impl_outer_event! {
        pub enum TestEvent for Test {
            module<T>,
        }
    }

    #[derive(Clone, Eq, PartialEq)]
    pub struct Test;
    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaximumBlockWeight: Weight = 1024;
        pub const MaximumBlockLength: u32 = 2 * 1024;
        pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    }
	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type WeightMultiplierUpdate = ();
		type Event = TestEvent;
		type BlockHashCount = BlockHashCount;
		type MaximumBlockWeight = MaximumBlockWeight;
		type MaximumBlockLength = MaximumBlockLength;
		type AvailableBlockRatio = AvailableBlockRatio;
		type Version = ();
	}
	impl Trait for Test {
		type Event = TestEvent;
	}

	type PlasmaCash = Module<Test>;
	//type SystemModule = system::Module<Test>; // Used for events

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
    }

    #[test]
    fn can_deposit() {
        with_externalities(&mut new_test_ext(), || {
            let token_id = U256::from(123);
            let account_id = 1;
            assert_eq!(PlasmaCash::tokens(token_id), None);
            assert_ok!(PlasmaCash::deposit(Origin::signed(account_id), token_id));
            assert_eq!(PlasmaCash::tokens(token_id), Some(account_id));
        });
    }

    #[test]
    fn can_withdraw() {
        with_externalities(&mut new_test_ext(), || {
            let token_id = U256::from(123);
            let account_id = 1;
            assert_ok!(PlasmaCash::deposit(Origin::signed(account_id), token_id));
            assert_eq!(PlasmaCash::tokens(token_id), Some(account_id));
            assert_ok!(PlasmaCash::withdraw(Origin::signed(account_id), token_id));
            assert_eq!(PlasmaCash::tokens(token_id), None);
        });
    }

    #[test]
    fn cant_withdraw_dne() {
        with_externalities(&mut new_test_ext(), || {
            let token_id = U256::from(123);
            let account_id = 1;
            assert_noop!(
                PlasmaCash::withdraw(Origin::signed(account_id), token_id),
                "Token must exist!"
            );
        });
    }

    #[test]
    fn only_owner_can_withdraw() {
        with_externalities(&mut new_test_ext(), || {
            let token_id = U256::from(123);
            let account1_id = 1;
            let account2_id = 2;
            assert_ok!(PlasmaCash::deposit(Origin::signed(account1_id), token_id));
            assert_eq!(PlasmaCash::tokens(token_id), Some(account1_id));
            assert_noop!(
                PlasmaCash::withdraw(Origin::signed(account2_id), token_id),
                "Current owner did not sign transaction!"
            );
            assert_eq!(PlasmaCash::tokens(token_id), Some(account1_id));
        });
    }

    #[test]
    fn can_transfer() {
        with_externalities(&mut new_test_ext(), || {
            let token_id = U256::from(123);
            let account1_id = 1;
            let account2_id = 2;
            assert_ok!(PlasmaCash::deposit(Origin::signed(account1_id), token_id));
            assert_eq!(PlasmaCash::tokens(token_id), Some(account1_id));
            assert_ok!(PlasmaCash::transfer(Origin::signed(account1_id), token_id, account2_id));
            assert_eq!(PlasmaCash::tokens(token_id), Some(account2_id));
        });
    }

    #[test]
    fn cant_transfer_dne() {
        with_externalities(&mut new_test_ext(), || {
            let token_id = U256::from(123);
            let account1_id = 1;
            let account2_id = 2;
            assert_noop!(
                PlasmaCash::transfer(Origin::signed(account1_id), token_id, account2_id),
                "Token must exist!"
            );
        });
    }

    #[test]
    fn only_owner_can_transfer() {
        with_externalities(&mut new_test_ext(), || {
            let token_id = U256::from(123);
            let account1_id = 1;
            let account2_id = 2;
            assert_ok!(PlasmaCash::deposit(Origin::signed(account1_id), token_id));
            assert_noop!(
                PlasmaCash::transfer(Origin::signed(account2_id), token_id, account1_id),
                "Current owner did not sign transaction!"
            );
        });
    }
}
