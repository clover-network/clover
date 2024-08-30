use polkadot_sdk::sc_cli as sc_cli;

use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
use fc_cli::FrontierDbCmd;

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
pub struct RunCmd {
	#[allow(missing_docs)]
	#[clap(flatten)]
	pub base: sc_cli::RunCmd,
   
	/// Maximum number of logs in a query.
	#[arg(long, default_value = "10000")]
	pub max_past_logs: u32,

  #[arg(long)]
  pub manual_seal: bool,

  #[command(flatten)]
	pub eth: crate::eth::EthConfiguration,
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
  #[structopt(subcommand)]
  pub subcommand: Option<Subcommand>,

  #[structopt(flatten)]
  pub run: RunCmd,
}
