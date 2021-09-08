// Copyright (C) 2021 Clover Network
// This file is part of Clover.

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use std::str::FromStr;
pub use type_utils::option_utils::OptionExt;

#[test]
fn transfer_to_evm_should_works() {
    new_test_ext().execute_with(|| {
        // transfer to evm account 0x1
        assert_ok!(CloverEvmInterOp::transfer_to_evm(
            Origin::signed(4),
            EvmAddress::from_str("2200000000000000000000000000000000000001").unwrap(),
            1000
        ));
        assert_eq!(Balances::free_balance(9), 1000);
    });
}
