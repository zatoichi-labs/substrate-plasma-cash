///
/// Author: Zatoichi Labs
///
// Adapted from https://github.com/substrate-developer-hub/utxo-workshop

use support::{
    decl_module, decl_storage, decl_event, ensure,
    dispatch::Result, StorageMap,
};
use system::ensure_signed;

// Serialization of Transactions
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use codec::{Decode, Encode};

// Cryptography primitives
use runtime_io::{sr25519_verify, blake2_256};
use primitives::sr25519::{Public, Signature};
use primitives::{H256, H512, U256};

// Use Custom logic module
use plasma_cash_tokens::{
    PlasmaCashTxn, TxnCmp,
    BigEndian, BitVec,
};

// Custom types
pub type AccountId = Public;
pub type TokenId = U256;
pub type BlkNum = U256;
pub type TxnHash = H256;

/// Transaction structure
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct Transaction {
    pub receiver: AccountId,
    pub token_id: TokenId,
    pub prev_blk_num: BlkNum,
    sender: AccountId,
    signature: H512,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct UnsignedTransaction {
    pub receiver: AccountId,
    pub token_id: TokenId,
    pub prev_blk_num: BlkNum,
}

impl UnsignedTransaction {
    pub fn new(receiver: AccountId,
               token_id: TokenId,
               prev_blk_num: BlkNum) -> Self
    {
        Self {
            receiver,
            token_id,
            prev_blk_num,
        }
    }

    pub fn hash(&self) -> TxnHash {
        H256::from(blake2_256(&self.encode()))
    }

    #[cfg(feature = "std")]
    pub fn add_signature(&self,
                         sender: AccountId,
                         signature: Signature,
    ) -> core::result::Result<Transaction, &'static str> {
        if sr25519_verify(
            &signature,
            &self.hash().as_ref(),
            &sender,
        ) {
            Ok(Transaction {
                receiver: self.receiver.clone(),
                token_id: self.token_id,
                prev_blk_num: self.prev_blk_num,
                sender,
                signature: H512::from_slice(signature.as_ref()),
            })
        } else {
            Err("Transaction is not signed by sender!")
        }
    }
}

impl Transaction {
    pub fn new(receiver: AccountId,
               token_id: TokenId,
               prev_blk_num: BlkNum) -> UnsignedTransaction
    {
        UnsignedTransaction {
            receiver,
            token_id,
            prev_blk_num,
        }
    }
}

impl PlasmaCashTxn<TxnHash> for Transaction {
    fn token_id(&self) -> BitVec {
        // Convert U256 to BitVec
        let mut uid_bytes: [u8; 32] = [0; 32];
        self.token_id.to_big_endian(&mut uid_bytes);
        BitVec::<BigEndian, u8>::from_slice(&uid_bytes)
    }

    fn hash_fn() -> (fn(&[u8]) -> TxnHash) {
        |u| TxnHash::from(blake2_256(&u))
    }

    fn empty_leaf_hash() -> TxnHash {
        // Encode empty leaf
        UnsignedTransaction::new(
            AccountId::from_raw([0; 32]),
            TokenId::zero(),
            BlkNum::zero(),
        ).hash()
    }

    fn leaf_hash(&self) -> TxnHash {
        // Encode leaf
        UnsignedTransaction::new(
            self.receiver.clone(),
            self.token_id,
            self.prev_blk_num,
        ).hash()
    }

    fn valid(&self) -> bool {
        sr25519_verify(
            &Signature::from_h512(self.signature),
            self.leaf_hash().as_ref(),
            &self.sender,
        )
    }

    fn compare(&self, other: &Self) -> TxnCmp {
        // &self.valid() is already true due to constructor
        // other.valid() is already true due to constructor
        // Transactions must be with the same tokenId to be related
        if self.token_id == other.token_id {

            // The other one is the direct parent of this one
            if self.receiver == other.sender {
                return TxnCmp::Parent; // FIXME Because this comes first, a cycle is possible

            // This one is the direct parent of the other one
            } else if self.sender == other.receiver {
                return TxnCmp::Child;

            // Both of us have the same parent
            // Note: due to how Plasma Cash is designed, one of these is
            //       most likely not in the txn trie, unless the operator
            //       made malicious modifications.
            } else if self.sender == other.sender {

                // But mine comes before, so I'm earlier
                if self.prev_blk_num < other.prev_blk_num {
                    return TxnCmp::EarlierSibling;

                // The other comes before, so I'm later
                } else if self.prev_blk_num > other.prev_blk_num {
                    return TxnCmp::LaterSibling;

                // We're both at the same height, but different destinations!
                } else if self.receiver != other.receiver {
                    return TxnCmp::DoubleSpend;
                }

                // We're both the same transaction (same tokenId, reciever, and sender)
                return TxnCmp::Same;
            }
        }

        // All else fails, we're unrelated
        TxnCmp::Unrelated
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait {
    type Event: From<Event> + Into<<Self as system::Trait>::Event>;
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

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn transfer(origin, txn: Transaction) -> Result {
            // TODO Coerce Origin into Transaction?
            let _who = ensure_signed(origin)?;

            // Validate transaction
            ensure!(txn.valid(), "Transaction is not valid!");

            ensure!(Tokens::exists(txn.token_id), "No deposit recorded yet!");
            let prev_txn = Tokens::get(txn.token_id)
                .expect("should pass if above works; qed");

            ensure!(
                txn.compare(&prev_txn) == TxnCmp::Parent,
                "Current owner did not sign transaction!"
            );

            //  TODO reject if currently in withdrawal

            Tokens::insert(txn.token_id, &txn);

            Self::deposit_event(Event::Transfer(txn.token_id, txn.sender, txn.receiver));
            Ok(())
        }

        //deposit(origin, transaction: Transaction)
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
    pub enum Event {
        Transfer(TokenId, AccountId, AccountId),
    }
);
