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
use runtime_io::blake2_256;
use primitives::{H256, U256, sr25519::Public};
use sr_primitives::{AnySignature, traits::Verify};

// Use Custom logic module
use plasma_cash_tokens::{
    PlasmaCashTxn, TxnCmp,
    BigEndian, BitVec,
};

// Custom types
pub type TokenId = U256;
pub type BlkNum = U256;

/// Transaction structure
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct Transaction<AccountId>
    where AccountId: Encode + Clone + Default + PartialEq
{
    pub receiver: AccountId,
    pub token_id: TokenId,
    pub prev_blk_num: BlkNum,
    pub sender: AccountId,
    signature: AnySignature,
}

#[cfg_attr(feature = "std", derive(Debug))]
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
pub struct UnsignedTransaction<AccountId>
    where AccountId: Encode + Clone + Default + PartialEq
{
    pub receiver: AccountId,
    pub token_id: TokenId,
    pub prev_blk_num: BlkNum,
}

impl<AccountId> UnsignedTransaction<AccountId>
    where AccountId: Encode + Clone + Default + PartialEq
{
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

    pub fn hash(&self) -> H256 {
        H256::from(blake2_256(&self.encode()))
    }

    #[cfg(feature = "std")]
    pub fn add_signature<Signature>(&self,
                                    sender: AccountId,
                                    signature: Signature,
    ) -> core::result::Result<Transaction<AccountId>, &'static str>
        where Signature: Encode + Verify<Signer = AccountId> + AsRef<[u8]>
    {
        if signature.verify(self.hash().as_ref(), &sender) {
            let encoded_signature = signature.encode();
            let encoded_signature = encoded_signature.clone();
            let mut encoded_signature = encoded_signature.as_ref();
            if let Ok(signature) = AnySignature::decode(&mut encoded_signature) {
                Ok(Transaction {
                    receiver: self.receiver.clone(),
                    token_id: self.token_id,
                    prev_blk_num: self.prev_blk_num,
                    sender,
                    signature,
                })
            } else {
                Err("Transaction encoding error!")
            }
        } else {
            Err("Transaction is not signed by sender!")
        }
    }
}

impl<AccountId> Transaction<AccountId>
    where AccountId: Encode + Clone + Default + PartialEq
{
    pub fn new(receiver: AccountId,
               token_id: TokenId,
               prev_blk_num: BlkNum) -> UnsignedTransaction<AccountId>
    {
        UnsignedTransaction {
            receiver,
            token_id,
            prev_blk_num,
        }
    }
}

impl<AccountId> PlasmaCashTxn for Transaction<AccountId>
    where AccountId: Encode + Clone + Default + PartialEq
{
    type HashType = H256;

    fn token_id(&self) -> BitVec {
        // Convert U256 to BitVec
        let mut uid_bytes: [u8; 32] = [0; 32];
        self.token_id.to_big_endian(&mut uid_bytes);
        BitVec::<BigEndian, u8>::from_slice(&uid_bytes)
    }

    fn hash_fn() -> (fn(&[u8]) -> H256) {
        |u| H256::from(blake2_256(&u))
    }

    fn empty_leaf_hash() -> H256 {
        // Encode empty leaf
        UnsignedTransaction::new(
            AccountId::default(),
            TokenId::zero(),
            BlkNum::zero(),
        ).hash()
    }

    fn leaf_hash(&self) -> H256 {
        // Encode leaf
        UnsignedTransaction::new(
            self.receiver.clone(),
            self.token_id,
            self.prev_blk_num,
        ).hash()
    }

    fn valid(&self) -> bool {
        // This trick is safe because we validate the signature in `add_signature()`,
        // and any decoding failures will return false
        let encoded_sender = self.sender.encode();
        let encoded_sender = encoded_sender.clone();
        let mut encoded_sender = encoded_sender.as_ref();
        if let Ok(sender) = Public::decode(&mut encoded_sender) {
            self.signature.verify(self.leaf_hash().as_ref(), &sender)
        } else {
            false // decoding error
        }
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
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as PlasmaCashModule {
        // State Database of Token: Transaction pairs
        Tokens get(tokens) build(|config: &GenesisConfig<T>| {
            config.initial_tokendb
                .iter()
                .cloned()
                // Note: Storage items must be unique, or they will be overwritten
                // TODO Fix this!
                .map(|txn| (txn.token_id, txn))
                .collect::<Vec<_>>()
        }): map TokenId => Option<Transaction<T::AccountId>>;
    }

    // Genesis may be empty (or not, if starting with some initial params)
    // Note: Might be desirable for privacy properties to start non-empty?
    add_extra_genesis {
        config(initial_tokendb): Vec<Transaction<T::AccountId>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn transfer(origin, txn: Transaction<T::AccountId>) -> Result {
            // TODO Coerce Origin into Transaction?
            let who = ensure_signed(origin)?;
            // NOTE This is temporary until the extrinsic itself is the transaction
            ensure!(who == txn.sender, "Only Transaction signer can submit!");

            // Validate transaction
            ensure!(txn.valid(), "Transaction is not valid!");

            ensure!(<Tokens<T>>::exists(txn.token_id), "No deposit recorded yet!");
            let prev_txn = <Tokens<T>>::get(txn.token_id)
                .expect("should pass if above works; qed");

            ensure!(
                txn.compare(&prev_txn) == TxnCmp::Child,
                "Current owner did not sign transaction!"
            );

            //  TODO reject if currently in withdrawal

            <Tokens<T>>::insert(txn.token_id, &txn);

            Self::deposit_event(RawEvent::Transfer(txn.token_id, txn.sender, txn.receiver));
            Ok(())
        }

        pub fn deposit(origin, txn: Transaction<T::AccountId>) -> Result {
            // TODO only authorities can do this.
            // TODO Should this be an inherent?
            let who = ensure_signed(origin)?;
            // NOTE This is temporary until the extrinsic itself is the transaction
            ensure!(who == txn.sender, "Only Transaction signer can submit!");

            // Validate transaction
            ensure!(txn.valid(), "Transaction is not valid!");

            ensure!(!<Tokens<T>>::exists(txn.token_id), "Token already exists!");

            <Tokens<T>>::insert(txn.token_id, &txn);

            Self::deposit_event(RawEvent::Deposit(txn.token_id, txn.receiver));
            Ok(())
        }

        pub fn withdraw(origin, token_id: TokenId) -> Result {
            // TODO Should this be an inherent?
            let who = ensure_signed(origin)?;

            ensure!(<Tokens<T>>::exists(token_id), "No deposit recorded yet!");

            let txn = <Tokens<T>>::get(token_id)
                .expect("should pass if above works; qed");

            ensure!(who == txn.sender, "Only current owner can withdraw!");

            <Tokens<T>>::remove(token_id);

            Self::deposit_event(RawEvent::Withdraw(txn.token_id, txn.sender));
            Ok(())
        }

        //on_finalize()
        //  publish block to rootchain
        //  reset txn database
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
    use primitives::{Pair, H256, Blake2Hasher, sr25519};
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

    type AccountId = sr25519::Public;

	impl system::Trait for Test {
		type Origin = Origin;
		type Call = ();
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type AccountId = AccountId;
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

    fn create_acct(id: u64) -> sr25519::Pair {
        sr25519::Pair::from_string(&format!("//{}", id), None)
            .expect("static values are valid; qed")
    }

    fn create_txn(from: &sr25519::Pair,
                  to: AccountId,
                  token_id: TokenId,
                  blk_num: BlkNum) -> Transaction<AccountId>
    {
            let unsigned_txn = Transaction::new(
                to,
                token_id,
                blk_num,
            );
            let signature = from.sign(unsigned_txn.hash().as_ref());
            unsigned_txn.add_signature(from.public(), signature).unwrap()
    }

    // This function basically just builds a genesis storage key/value store according to
    // our desired mockup.
    fn empty_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
    }

    // TODO Move initial deposit to here
    fn with_deposit_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
        let token_id = U256::from(123);
        let account = create_acct(1);
        let deposit_txn = create_txn(&account, account.public(), token_id, U256::from(0));
        let mut ext = system::GenesisConfig::default().build_storage::<Test>().unwrap().into();
        GenesisConfig::<Test> {
            initial_tokendb: vec![deposit_txn]
        }.assimilate_storage(&mut ext).unwrap();
        ext.into()
    }

    #[test]
    fn test_can_deposit() {
        with_externalities(&mut empty_test_ext(), || {
            let token_id = U256::from(123);
            assert_eq!(PlasmaCash::tokens(token_id), None);
            let account = create_acct(1);
            let txn = create_txn(&account, account.public(), token_id, U256::from(0));
            assert_ok!(PlasmaCash::deposit(Origin::signed(account.public()), txn.clone()));
            assert_eq!(PlasmaCash::tokens(token_id), Some(txn));
        });
    }

    #[test]
    fn test_can_withdraw() {
        with_externalities(&mut with_deposit_test_ext(), || {
            let token_id = U256::from(123);
            let account = create_acct(1);
            assert_ok!(PlasmaCash::withdraw(Origin::signed(account.public()), token_id));
            assert_eq!(PlasmaCash::tokens(token_id), None);
        });
    }

    #[test]
    fn test_cant_withdraw_dne() {
        with_externalities(&mut empty_test_ext(), || {
            let token_id = U256::from(123);
            let account = create_acct(1);
            assert_noop!(
                PlasmaCash::withdraw(Origin::signed(account.public()), token_id),
                "No deposit recorded yet!"
            );
        });
    }

    #[test]
    fn test_only_owner_can_withdraw() {
        with_externalities(&mut with_deposit_test_ext(), || {
            let token_id = U256::from(123);
            let account2 = create_acct(2);
            let txn = PlasmaCash::tokens(token_id).unwrap();
            assert_noop!(
                PlasmaCash::withdraw(Origin::signed(account2.public()), token_id),
                "Only current owner can withdraw!"
            );
            assert_eq!(PlasmaCash::tokens(token_id), Some(txn));
        });
    }

    #[test]
    fn test_can_transfer() {
        with_externalities(&mut with_deposit_test_ext(), || {
            let token_id = U256::from(123);
            let account1 = create_acct(1);
            let account2 = create_acct(2);
            let txn = create_txn(&account1, account2.public(), token_id, U256::from(0));
            assert_ok!(PlasmaCash::transfer(Origin::signed(account1.public()), txn.clone()));
            assert_eq!(PlasmaCash::tokens(token_id), Some(txn.clone()));
        });
    }

    #[test]
    fn test_cant_transfer_dne() {
        with_externalities(&mut empty_test_ext(), || {
            let token_id = U256::from(123);
            let account1 = create_acct(1);
            let account2 = create_acct(2);
            let txn = create_txn(&account1, account2.public(), token_id, U256::from(0));
            assert_noop!(
                PlasmaCash::transfer(Origin::signed(account1.public()), txn.clone()),
                "No deposit recorded yet!"
            );
        });
    }

    #[test]
    fn test_only_owner_can_transfer() {
        with_externalities(&mut with_deposit_test_ext(), || {
            let token_id = U256::from(123);
            let account2 = create_acct(2);
            let txn = create_txn(&account2, account2.public(), token_id, U256::from(0));
            assert_noop!(
                PlasmaCash::transfer(Origin::signed(account2.public()), txn.clone()),
                "Current owner did not sign transaction!"
            );
        });
    }
}
