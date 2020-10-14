use super::*;
use codec::{Decode, Encode};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use clover_primitives::CurrencyId;
use strum::IntoEnumIterator;
use std::string::ToString;
use int_enum::IntEnum;

pub struct Currency;

#[derive(Encode, Decode, Eq, PartialEq, Clone, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct CurrencyInfo {
    id: u32,
    name: String
}

#[rpc]
pub trait CurrencyRpc {
    #[rpc(name = "clover_getCurrencies")]
    fn get_currencies(&self) -> Result<Vec<CurrencyInfo>>;
}

impl CurrencyRpc for Currency {
    fn get_currencies(&self) -> Result<Vec<CurrencyInfo>> {
        let mut v: Vec<CurrencyInfo> = Vec::new();
        for item in CurrencyId::iter() {
            v.push(CurrencyInfo  {
                id: item.int_value(),
                name: item.to_string()
            });
        }
        Ok(v)
    }
}
