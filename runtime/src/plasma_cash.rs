///
/// Author: Zatoichi Labs
///

use support::{
    decl_module, decl_storage, decl_event, ensure,
    dispatch::Result, StorageMap,
};
use system::ensure_signed;

// Serialization of Transactions
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use codec::{Decode, Encode};

use runtime_io::{sr25519_verify, blake2_256};
use primitives::sr25519::{Signature, Public};
use primitives::{H256, H512, U256};

// Custom types
pub type AccountId = Public;
pub type TokenId = U256;
pub type TxnHash = H256;

/// Transaction structure
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct Transaction {
    pub receiver: AccountId,
    pub token_id: TokenId,
    pub sender: AccountId,
    signature: H512,
}

impl Transaction {
    pub fn new(receiver: AccountId,
               token_id: TokenId,
               sender: AccountId,
               signature: Signature) -> Self
    {
        let signature = H512::from(signature);
        let txn = Self {
            receiver,
            token_id,
            sender,
            signature,
        };
        assert!(txn.valid());
        txn
    }

    pub fn hash(&self) -> TxnHash {
        TxnHash::from(blake2_256(self.encode().as_ref()))
    }

    pub fn valid(&self) -> bool {
        sr25519_verify(
            &Signature::from_h512(self.signature),
            self.hash().as_ref(),
            &self.sender,
        )
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    // TODO: Add other types and constants required configure this module.

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as PlasmaCashModule {
        // State Database of Token: Transaction pairs
        Tokens build(|config: &GenesisConfig| {
            config.initial_tokendb
                .iter()
                .cloned()
                // Note: Storage items must be unique, or they will be overwritten
                // TODO Fix this!
                .map(|txn| (txn.token_id, txn))
                .collect::<Vec<_>>()
        }): map TokenId => Option<Transaction>;
    }

    // Genesis may be empty (or not, if starting with some initial params)
    // Note: Might be desirable for privacy properties to start non-empty?
    add_extra_genesis {
        config(initial_tokendb): Vec<Transaction>;
    }
}

// The module's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn transfer(origin, txn: Transaction) -> Result {
            // TODO Coerce Origin into Transaction?
            let _who = ensure_signed(origin)?;

            ensure!(txn.valid(), "Transaction is not valid!");

            let prev_txn = Tokens::get(txn.token_id)
                .expect("No deposit recorded yet!");

            ensure!(
                 txn.sender == prev_txn.receiver,
                "Current owner did not sign transaction!"
            );

            Tokens::insert(txn.token_id, txn);

            // TODO Unsure why we can't this to work w/ types
            //Self::deposit_event(Event::Transfer(txn.token_id, txn.sender, txn.receiver));
            Ok(())
        }

        //deposit(origin, transaction: Transaction)
        //  only authorities can do this
        //  adds deposit from Rootchain into state/txn database
        //  Total::set(<Total<T>>::get() 1);

        //withdraw(origin, tokenId: TokenID)
        //  this is an inherent?
        //  removes tokenId from state database (after withdrawal finalizes)
        //  Total::set(<Total<T>>::get() - 1);
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        Transfer(TokenId, AccountId, AccountId),
    }
);
