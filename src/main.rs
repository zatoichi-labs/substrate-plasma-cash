//! Substrate Node Template CLI library.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;

pub use substrate_cli::{VersionInfo, IntoExit, error};

fn main() {
    let version = VersionInfo {
        name: "Plasma Cash",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "plasma-cash",
        author: "Zatoichi Labs",
        description: "Plasma Cash Client, written in Substrate",
        support_url: "https://github.com/zatoichi-labs/substrate-plasma-cash",
    };

    if let Err(e) = cli::run(::std::env::args(), cli::Exit, version) {
        eprintln!("Fatal error: {}\n\n{:?}", e, e);
        std::process::exit(1)
    }
}
