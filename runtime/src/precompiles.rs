use pallet_evm::{Precompile, PrecompileHandle, PrecompileResult, PrecompileSet};
use sp_core::H160;
use sp_std::marker::PhantomData;

use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};

pub struct CloverPrecompiles<R>(PhantomData<R>);

impl<R> CloverPrecompiles<R>
where
  R: pallet_evm::Config,
{
  pub fn new() -> Self {
    Self(Default::default())
  }
  pub fn used_addresses() -> sp_std::vec::Vec<H160> {
    sp_std::vec![1, 2, 3, 4]
      .into_iter()
      .map(|x| hash(x))
      .collect()
  }
}
impl<R> PrecompileSet for CloverPrecompiles<R>
where
  R: pallet_evm::Config,
{
  fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
    match handle.code_address() {
      // Ethereum precompiles :
      a if a == hash(1) => Some(ECRecover::execute(handle)),
      a if a == hash(2) => Some(Sha256::execute(handle)),
      a if a == hash(3) => Some(Ripemd160::execute(handle)),
      a if a == hash(4) => Some(Identity::execute(handle)),
      _ => None,
    }
  }

  fn is_precompile(&self, address: H160) -> bool {
    Self::used_addresses().contains(&address)
  }
}

fn hash(a: u64) -> H160 {
  H160::from_low_u64_be(a)
}
