// TODO: Consider AnySignature instead of H512
use primitives::{Pair, Public, U256, sr25519};
use plasma_cash_runtime::{
    AccountId, Transaction, TokenId,
    BabeConfig, GenesisConfig, GrandpaConfig, SystemConfig, PlasmaCashConfig,
    WASM_BINARY,
};
use babe_primitives::{AuthorityId as BabeId};
use grandpa_primitives::{AuthorityId as GrandpaId};
use substrate_service;

// Note this is the URL for the telemetry server
//const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = substrate_service::ChainSpec<GenesisConfig>;

/// The chain specification option. This is expected to come in from the CLI and
/// is little more than one of a number of alternatives which can easily be converted
/// from a string (`--chain=...`) into a `ChainSpec`.
#[derive(Clone, Debug)]
pub enum Alternative {
    /// Whatever the current runtime is, with just Alice as an auth.
    Development,
    /// Whatever the current runtime is, with simple Alice/Bob auths.
    LocalTestnet,
}

pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, GrandpaId, BabeId) {
    (
        get_from_seed::<AccountId>(&format!("{}//stash", seed)),
        get_from_seed::<AccountId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
    )
}

fn txn_for_genesis_acct(seed: &str, token_id: TokenId) -> Transaction<AccountId> {
    let owner = sr25519::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed");
    // Construct unsigned transaction
    let unsigned_txn = Transaction::new(
        owner.public().clone(),
        token_id,
        U256::from(0),
    );
    let signature = owner.sign(unsigned_txn.hash().as_ref());
    unsigned_txn.add_signature(owner.public(), signature).unwrap()
}

impl Alternative {
    /// Get an actual chain config from one of the alternatives.
    pub(crate) fn load(self) -> Result<ChainSpec, String> {
        Ok(match self {
            Alternative::Development => ChainSpec::from_genesis(
                "Development", // Network Name
                "dev", // Network ID
                || testnet_genesis(
                    vec![ // Authorities
                        get_authority_keys_from_seed("Alice"),
                    ],
                    vec![ // Token Distribution
                        txn_for_genesis_acct("Alice", TokenId::from(1)),
                    ],
                    true, // Enable println!
                ), // Genesis constructor
                vec![], // Boot Nodes
                None, // Telemetry Endpoints
                None, // Protocol ID
                None, // Consensus Engine
                None, // Properties
            ),
            Alternative::LocalTestnet => ChainSpec::from_genesis(
                "Local Testnet", // Network Name
                "local_testnet", // Network ID
                || testnet_genesis(
                    vec![ // Authorities
                        get_authority_keys_from_seed("Alice"),
                        get_authority_keys_from_seed("Bob"),
                    ],
                    vec![ // Token Distribution
                        txn_for_genesis_acct("Charlie", TokenId::from(1)),
                        txn_for_genesis_acct("Dave",    TokenId::from(2)),
                        txn_for_genesis_acct("Eve",     TokenId::from(3)),
                        txn_for_genesis_acct("Ferdie",  TokenId::from(4)),
                    ], // Token Distribution
                    true, // Enable println!
                ), // Genesis constructor
                vec![], // Boot Nodes
                None, // Telemetry Endpoints
                None, // Protocol ID
                None, // Consensus Engine
                None, // Properties
            ),
        })
    }

    pub(crate) fn from(s: &str) -> Option<Self> {
        match s {
            // Dev config is used for live testing
            "dev" => Some(Alternative::Development),
            // Default chain is local config, used for demos
            "" | "local" => Some(Alternative::LocalTestnet),
            _ => None,
        }
    }
}

fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AccountId, GrandpaId, BabeId)>,
    initial_tokendb: Vec<Transaction<AccountId>>,
    _enable_println: bool
) -> GenesisConfig {
    GenesisConfig {
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        indices: None,
        babe: Some(BabeConfig {
            authorities: initial_authorities.iter().map(|x| (x.3.clone(), 1)).collect(),
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
        }),
        plasma_cash: Some(PlasmaCashConfig {
            initial_tokendb, // Initialize SMT
        }),
    }
}
