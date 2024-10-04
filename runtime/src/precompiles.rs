use pallet_evm_precompile_simple::{ECRecover, Identity, Ripemd160, Sha256};
use sp_core::H160;
use sp_std::marker::PhantomData;

use pallet_evm::{
	IsPrecompileResult, Precompile, PrecompileHandle, PrecompileResult, PrecompileSet,
};


/// Precompiles for the Clover network.
pub struct CloverPrecompiles<T>(PhantomData<T>);

impl<R> CloverPrecompiles<R>
where
	R: pallet_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	pub fn used_addresses() -> [H160; 4] {
		[
			hash(1),
			hash(2),
			hash(3),
            hash(4),
		]
	}
}

impl<R> PrecompileSet for CloverPrecompiles<R>
where
	R: pallet_evm::Config,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
        let (code_addr, context_addr) = (handle.code_address(), handle.context().address);
        // For chains that possess their own stateful precompiles, it is advisable to activate this verification measure.
        // This measure prohibits the use of DELEGATECALL or CALLCODE for any precompiles other than the official Ethereum precompiles, which are inherently stateless.
        if Self::used_addresses().contains(&code_addr)
            && code_addr > hash(9)
            && code_addr != context_addr
        {
            return Some(Err(fp_evm::PrecompileFailure::Error{
                exit_status: fp_evm::ExitError::Other(sp_std::borrow::Cow::Borrowed("cannot be called with DELEGATECALL or CALLCODE")),
            }));
        };

        match handle.code_address() {
            // Ethereum precompiles :
            a if a == hash(1) => Some(ECRecover::execute(handle)),
            a if a == hash(2) => Some(Sha256::execute(handle)),
            a if a == hash(3) => Some(Ripemd160::execute(handle)),
            a if a == hash(4) => Some(Identity::execute(handle)),
            _ => None,
        }
    }

    fn is_precompile(&self, address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: Self::used_addresses().contains(&address),
            extra_cost: 0,
        }
    }
}

fn hash(a: u64) -> H160 {
    H160::from_low_u64_be(a)
}
