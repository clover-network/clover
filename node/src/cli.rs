use polkadot_sdk::sc_cli;

use fc_cli::FrontierDbCmd;
use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};

/// Possible subcommands of the main binary.
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management cli utilities
    #[command(subcommand)]
    Key(KeySubcommand),

    /// Verify a signature for a message, provided on STDIN, with a given
    /// (public or secret) key.
    Verify(VerifyCmd),

    /// Generate a seed that provides a vanity address.
    Vanity(VanityCmd),

    /// Sign a message, with a given (secret) key.
    Sign(SignCmd),

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// Db meta columns information.
    FrontierDb(FrontierDbCmd),
}

#[allow(missing_docs)]
#[derive(Debug, clap::Parser)]
pub struct CloverRunCmd {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub base: sc_cli::RunCmd,

    #[arg(long)]
    pub manual_seal: bool,

    #[command(flatten)]
    pub eth: crate::eth::EthConfiguration,
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[clap(flatten)]
    pub run: CloverRunCmd,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub mixnet_params: sc_cli::MixnetParams,

    /// Disable automatic hardware benchmarks.
    ///
    /// By default these benchmarks are automatically ran at startup and measure
    /// the CPU speed, the memory bandwidth and the disk speed.
    ///
    /// The results are then printed out in the logs, and also sent as part of
    /// telemetry, if telemetry is enabled.
    #[arg(long)]
    pub no_hardware_benchmarks: bool,
}
