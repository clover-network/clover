use codec::{Encode, Decode};


#[derive(Clone, PartialEq, Encode, Decode)]
pub enum EthereumNetworkType {
    Mainnet,
    Ropsten,
}
impl Default for EthereumNetworkType {
    fn default() -> EthereumNetworkType {
        EthereumNetworkType::Mainnet
    }
}
