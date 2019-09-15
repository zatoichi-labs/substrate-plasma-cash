///
/// Author: Zatoichi Labs
///




use support::{decl_module, decl_storage, decl_event, StorageValue, dispatch::Result};
use system::ensure_signed;

/// The module's configuration trait.
pub trait Trait: system::Trait {
    // TODO: Add other types and constants required configure this module.

    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

// This module's storage items.
decl_storage! {
    trait Store for Module<T: Trait> as PlasmaCash {
        Something get(something): Option<u32>;
    }
}

// The module's dispatchable functions.
decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        pub fn do_something(origin, something: u32) -> Result {
            let who = ensure_signed(origin)?;

            Something::put(something);

            Self::deposit_event(RawEvent::SomethingStored(something, who));
            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T> where AccountId = <T as system::Trait>::AccountId {
        SomethingStored(u32, AccountId),
    }
);
