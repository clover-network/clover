use sc_cli::{KeySubcommand, SignCmd, VanityCmd, VerifyCmd};
use structopt::StructOpt;

/// Possible subcommands of the main binary.
#[derive(Debug, StructOpt)]
pub enum Subcommand {
  /// Key management cli utilities
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
}

#[allow(missing_docs)]
#[derive(Debug, StructOpt)]
pub struct RunCmd {
	#[allow(missing_docs)]
	#[structopt(flatten)]
	pub base: sc_cli::RunCmd,

	/// Maximum number of logs in a query.
	#[structopt(long, default_value = "10000")]
	pub max_past_logs: u32,

    #[structopt(long = "manual-seal")]
    pub manual_seal: bool,

}

#[derive(Debug, StructOpt)]
pub struct Cli {
  #[structopt(subcommand)]
  pub subcommand: Option<Subcommand>,

  #[structopt(flatten)]
  pub run: RunCmd,
}
