///
/// Author: Zatoichi Labs
///
// Adapted from https://github.com/substrate-developer-hub/utxo-workshop

use support::{
    decl_module, decl_storage, decl_event, ensure,
    dispatch::Result, StorageMap,
};
use system::ensure_signed;

use primitives::U256;

// Custom types
pub type TokenId = U256;

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as PlasmaCashModule {
        // State Database of Token: Transaction pairs
        Tokens build(
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

            let prev_owner: T::AccountId = <Tokens<T>>::get(token_id)
                .expect("No deposit recorded yet!");

            ensure!(
                prev_owner == who,
                "Current owner did not sign transaction!"
            );

            // Overwrite previous entry, but that's okay because we check it above
            <Tokens<T>>::insert(token_id, new_owner);

            // TODO Unsure why we can't this to work w/ types
            //Self::deposit_event(Event::Transfer(token_id, who, new_owner));
            Ok(())
        }

        //deposit(origin, transaction: Transaction)
        //  this is an inherent?
        //  only authorities can do this
        //  adds deposit from Rootchain into state/txn database

        //withdraw(origin, tokenId: TokenID)
        //  this is an inherent?
        //  removes tokenId from state database (after withdrawal finalizes)

        //on_finalize()
        //  publish block to rootchain
        //  reset txn database
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        Transfer(TokenId, AccountId, AccountId),
    }
);
