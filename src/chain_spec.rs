use primitives::{Pair, Public};
use plasma_cash_runtime::{
    AccountId, BabeConfig, BalancesConfig, GenesisConfig, GrandpaConfig,
    IndicesConfig, SystemConfig, WASM_BINARY,
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

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(seed: &str) -> (AccountId, AccountId, GrandpaId, BabeId) {
    (
        get_from_seed::<AccountId>(&format!("{}//stash", seed)),
        get_from_seed::<AccountId>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
    )
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
                        get_from_seed::<AccountId>("Alice"),
                        get_from_seed::<AccountId>("Bob"),
                        get_from_seed::<AccountId>("Alice//stash"),
                        get_from_seed::<AccountId>("Bob//stash"),
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
                         get_from_seed::<AccountId>("Alice"),
                         get_from_seed::<AccountId>("Bob"),
                         get_from_seed::<AccountId>("Charlie"),
                         get_from_seed::<AccountId>("Dave"),
                         get_from_seed::<AccountId>("Eve"),
                         get_from_seed::<AccountId>("Ferdie"),
                         get_from_seed::<AccountId>("Alice//stash"),
                         get_from_seed::<AccountId>("Bob//stash"),
                         get_from_seed::<AccountId>("Charlie//stash"),
                         get_from_seed::<AccountId>("Dave//stash"),
                         get_from_seed::<AccountId>("Eve//stash"),
                         get_from_seed::<AccountId>("Ferdie//stash"),
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
    endowed_accounts: Vec<AccountId>,
    _enable_println: bool
) -> GenesisConfig {
    GenesisConfig {
        system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        indices: Some(IndicesConfig {
            ids: endowed_accounts.clone(),
        }),
        balances: Some(BalancesConfig {
            balances: endowed_accounts.iter().cloned().map(|k|(k, 1 << 60)).collect(),
            vesting: vec![],
        }),
        babe: Some(BabeConfig {
            authorities: initial_authorities.iter().map(|x| (x.3.clone(), 1)).collect(),
        }),
        grandpa: Some(GrandpaConfig {
            authorities: initial_authorities.iter().map(|x| (x.2.clone(), 1)).collect(),
        }),
    }
}
