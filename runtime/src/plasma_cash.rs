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
            let prev_owner: T::AccountId = <Tokens<T>>::get(token_id)
                .expect("Deposit does not exist!");

            // Check provenence
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

        pub fn deposit(origin, token_id: TokenId) -> Result {
            let who = ensure_signed(origin)?;

            // TODO find a way to sync with rootchain deposit events

            // Token should not exist yet
            ensure!(
                !<Tokens<T>>::exists(token_id),
                "Token already exists!"
            );

            //  adds deposit from Rootchain into state/txn database
            <Tokens<T>>::insert(token_id, who);

            // TODO Unsure why we can't this to work w/ types
            //Self::deposit_event(Event::Deposit(token_id, who));
            Ok(())
        }

        pub fn withdraw(origin, token_id: TokenId) -> Result {
            let who = ensure_signed(origin)?;

            // TODO find a way to sync with rootchain finalize withdrawal events

            // Token should exist, check and get current owner
            let prev_owner: T::AccountId = <Tokens<T>>::get(token_id)
                .expect("Deposit does not exist!");

            // Check provenence
            ensure!(
                prev_owner == who,
                "Current owner did not sign transaction!"
            );

            //  removes tokenId from state database (after withdrawal finalizes)
            <Tokens<T>>::remove(token_id);

            // TODO Unsure why we can't this to work w/ types
            //Self::deposit_event(Event::Withdraw(token_id, who));
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
