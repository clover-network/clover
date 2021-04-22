// Copyright (C) 2021 Clover Network
// This file is part of Clover.

use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError, traits::OnRuntimeUpgrade};
use hex_literal::hex;
use pallet_balances::Error as BalancesError;
pub use type_utils::option_utils::OptionExt;

#[test]
fn mint_and_claim_should_works() {
  new_test_ext().execute_with(|| {
    // set bridge account to root
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1
    ));

    // Set claim limit = 100
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      100
    ));

    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::Ethereum,
      200
    ));
    assert_eq!(CloverClaims::elastic_claim_limits(BridgeNetworks::BSC), 100);
    assert_eq!(
      CloverClaims::elastic_claim_limits(BridgeNetworks::Ethereum),
      200
    );

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim_elastic(
      Origin::signed(1),
      BridgeNetworks::BSC,
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // claims and limit should updated
    assert_eq!(
      CloverClaims::elastic_claims(BridgeNetworks::BSC, &tx_hash),
      Some((eth_addr.clone(), 100, false))
    ); // new tx
    assert_eq!(CloverClaims::elastic_claim_limits(BridgeNetworks::BSC), 0);

    assert_eq!(
      CloverClaims::elastic_claim_limits(BridgeNetworks::Ethereum),
      200
    );

    // Claim it with correct msg sig
    // Pay CLVs to the TEST account:0100000000000000
    let sig = get_legal_eth_sig();
    assert_eq!(Balances::free_balance(1), 0);
    assert_ok!(CloverClaims::claim_elastic(
      Origin::none(),
      BridgeNetworks::BSC,
      1,
      tx_hash.clone(),
      sig.clone()
    ));

    // 5. Claim success
    assert_eq!(Balances::free_balance(1), 100);
    assert_eq!(
      CloverClaims::elastic_claims(BridgeNetworks::BSC, &tx_hash),
      Some((eth_addr.clone(), 100, true))
    ); // tx has already be claimed
  });
}

#[test]
fn ethereum_mint_and_claim_should_works() {
  new_test_ext().execute_with(|| {
    // set bridge account to root
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::Ethereum,
      2
    ));

    // Set claim limit = 100
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::Ethereum,
      100
    ));

    assert_eq!(
      CloverClaims::elastic_claim_limits(BridgeNetworks::Ethereum),
      100
    );

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim_elastic(
      Origin::signed(2),
      BridgeNetworks::Ethereum,
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // claims and limit should updated
    assert_eq!(
      CloverClaims::elastic_claims(BridgeNetworks::Ethereum, &tx_hash),
      Some((eth_addr.clone(), 100, false))
    ); // new tx
    assert_eq!(
      CloverClaims::elastic_claim_limits(BridgeNetworks::Ethereum),
      0
    );

    // Claim it with correct msg sig
    // Pay CLVs to the TEST account:0100000000000000
    let sig = get_legal_eth_sig();
    assert_eq!(Balances::free_balance(1), 0);
    assert_noop!(
      CloverClaims::claim_elastic(
        Origin::none(),
        BridgeNetworks::BSC,
        1,
        tx_hash.clone(),
        sig.clone()
      ),
      Error::<Test>::TxNotMinted
    );

    assert_ok!(CloverClaims::claim_elastic(
      Origin::none(),
      BridgeNetworks::Ethereum,
      1,
      tx_hash.clone(),
      sig.clone()
    ));

    // 5. Claim success
    assert_eq!(Balances::free_balance(1), 100);
    assert_eq!(
      CloverClaims::elastic_claims(BridgeNetworks::Ethereum, &tx_hash),
      Some((eth_addr.clone(), 100, true))
    ); // tx has already be claimed
  });
}

#[test]
fn mint_fee_should_work() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_bridge_fee_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1000,
      0
    ));
    // set bridge account to root
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1
    ));
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      2000
    ));

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_noop!(
      CloverClaims::mint_claim_elastic(
        Origin::signed(1),
        BridgeNetworks::BSC,
        tx_hash.clone(),
        eth_addr.clone(),
        100
      ),
      Error::<Test>::InvalidAmount
    );

    assert_eq!(
      CloverClaims::elastic_claim_limits(BridgeNetworks::BSC),
      2000
    );

    assert_ok!(CloverClaims::mint_claim_elastic(
      Origin::signed(1),
      BridgeNetworks::BSC,
      tx_hash.clone(),
      eth_addr.clone(),
      1200
    ));

    // claim limit only update with the real amount
    assert_eq!(
      CloverClaims::elastic_claim_limits(BridgeNetworks::BSC),
      1800
    );
    // Claim it with correct msg sig
    // Pay CLVs to the TEST account:0100000000000000
    let sig = get_legal_eth_sig();
    assert_eq!(Balances::free_balance(1), 0);
    assert_ok!(CloverClaims::claim_elastic(
      Origin::none(),
      BridgeNetworks::BSC,
      1,
      tx_hash.clone(),
      sig.clone()
    ));

    // 5. Claim success
    assert_eq!(Balances::free_balance(1), 200);

    // module has the fee deposited
    assert_eq!(Balances::free_balance(CloverClaims::account_id()), 1000);
  })
}

#[test]
fn change_bridger_should_work() {
  new_test_ext().execute_with(|| {
    assert_noop!(
      CloverClaims::set_bridge_account_elastic(Origin::signed(1), BridgeNetworks::BSC, 1),
      DispatchError::BadOrigin
    );

    // set bridger
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1
    ));

    // 1. Mint a claim with 2, no way
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_noop!(
      CloverClaims::mint_claim_elastic(
        Origin::signed(2),
        BridgeNetworks::BSC,
        tx_hash.clone(),
        eth_addr.clone(),
        100
      ),
      Error::<Test>::NoPermission
    );
  });
}

#[test]
fn non_exist_tx_should_fail() {
  new_test_ext().execute_with(|| {
    let tx_hash = get_legal_tx_hash();
    let sig = get_legal_eth_sig();
    assert_noop!(
      CloverClaims::claim_elastic(Origin::none(), BridgeNetworks::BSC, 1, tx_hash, sig),
      Error::<Test>::TxNotMinted
    );
  });
}

#[test]
fn illegal_sig_claim_should_fail() {
  new_test_ext().execute_with(|| {
    // only root account can change bridge account
    assert_noop!(
      CloverClaims::set_bridge_account_elastic(Origin::signed(2), BridgeNetworks::BSC, 1),
      DispatchError::BadOrigin
    );
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1
    ));

    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      100
    ));

    //  Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim_elastic(
      Origin::signed(1),
      BridgeNetworks::BSC,
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // Claim it with illegal sig
    let sig1 = get_another_account_eth_sig();
    assert_noop!(
      CloverClaims::claim_elastic(
        Origin::none(),
        BridgeNetworks::BSC,
        1,
        tx_hash.clone(),
        sig1.clone()
      ),
      Error::<Test>::SignatureNotMatch
    );

    // Sig with wrong message should failed
    let sig2 = get_wrong_msg_eth_sig();
    assert_noop!(
      CloverClaims::claim_elastic(
        Origin::none(),
        BridgeNetworks::BSC,
        1,
        tx_hash.clone(),
        sig2.clone()
      ),
      Error::<Test>::SignatureNotMatch
    );
  });
}

#[test]
fn mint_twice_should_fail() {
  new_test_ext().execute_with(|| {
    // Set miner and superior
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1
    ));

    // Set limit
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      100
    ));

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim_elastic(
      Origin::signed(1),
      BridgeNetworks::BSC,
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // 3. Mint the same eth again
    assert_noop!(
      CloverClaims::mint_claim_elastic(
        Origin::signed(1),
        BridgeNetworks::BSC,
        tx_hash.clone(),
        eth_addr.clone(),
        100
      ),
      Error::<Test>::AlreadyMinted
    );
  });
}

#[test]
fn claim_twice_should_fail() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1
    ));

    // Set limit
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      100
    ));

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim_elastic(
      Origin::signed(1),
      BridgeNetworks::BSC,
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // Claim it
    // Pay CLVs to the TEST account:0100000000000000
    let sig = get_legal_eth_sig();
    assert_eq!(Balances::free_balance(1), 0);
    assert_ok!(CloverClaims::claim_elastic(
      Origin::none(),
      BridgeNetworks::BSC,
      1,
      tx_hash.clone(),
      sig.clone()
    ));
    assert_eq!(Balances::free_balance(1), 100);

    // failed on the second try
    assert_noop!(
      CloverClaims::claim_elastic(
        Origin::none(),
        BridgeNetworks::BSC,
        1,
        tx_hash.clone(),
        sig.clone()
      ),
      Error::<Test>::AlreadyClaimed
    );
    assert_eq!(Balances::free_balance(1), 100);
  });
}

#[test]
fn claim_limit_should_work() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      2
    ));

    // Mint a claim should failed without limitation
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_noop!(
      CloverClaims::mint_claim_elastic(
        Origin::signed(2),
        BridgeNetworks::BSC,
        tx_hash.clone(),
        eth_addr.clone(),
        10
      ),
      Error::<Test>::ClaimLimitExceeded
    );

    // Set limit
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      10
    ));
    assert_eq!(CloverClaims::elastic_claim_limits(BridgeNetworks::BSC), 10);

    // Claim amount with limitation should be ok
    assert_ok!(CloverClaims::mint_claim_elastic(
      Origin::signed(2),
      BridgeNetworks::BSC,
      tx_hash.clone(),
      eth_addr.clone(),
      10
    ));
    assert_eq!(CloverClaims::elastic_claim_limits(BridgeNetworks::BSC), 0);
  });
}

#[test]
fn burn_should_work() {
  new_test_ext().execute_with(|| {
    let eth_addr = get_legal_eth_addr();

    // should burn balance from account 4
    assert_ok!(CloverClaims::burn_elastic(
      Origin::signed(4),
      BridgeNetworks::BSC,
      eth_addr,
      40
    ));
    assert_eq!(Balances::free_balance(4), 60);

    assert_ok!(CloverClaims::burn_elastic(
      Origin::signed(4),
      BridgeNetworks::BSC,
      eth_addr,
      40
    ));
    assert_eq!(Balances::free_balance(4), 20);

    // should failed if burn all the balances from an account
    assert_noop!(
      CloverClaims::burn_elastic(Origin::signed(5), BridgeNetworks::BSC, eth_addr, 100),
      BalancesError::<Test, _>::KeepAlive
    );
    assert_eq!(Balances::free_balance(5), 100);
  });
}

#[test]
fn burn_fee_should_work() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_bridge_fee_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      0,
      30
    ));
    // set bridge account to root
    assert_ok!(CloverClaims::set_bridge_account_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      1
    ));
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      2000
    ));

    let eth_addr = get_legal_eth_addr();

    // burn should failed if there is not enough balance to pay fee
    assert_noop!(
      CloverClaims::burn_elastic(Origin::signed(4), BridgeNetworks::BSC, eth_addr, 20),
      Error::<Test>::InvalidAmount
    );
    assert_eq!(Balances::free_balance(4), 100);

    assert_ok!(CloverClaims::burn_elastic(
      Origin::signed(5),
      BridgeNetworks::BSC,
      eth_addr,
      40
    ));
    assert_eq!(Balances::free_balance(5), 60);

    // module has the fee deposited
    assert_eq!(Balances::free_balance(CloverClaims::account_id()), 30);
  });
}

#[test]
fn elastic_burn_fee_should_work() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_claim_limit_elastic(
      Origin::root(),
      BridgeNetworks::BSC,
      10
    ));
  });
}

#[cfg(test)]
pub fn init_mock_data(
  account: &<Test as frame_system::Config>::AccountId,
  fee_mint: BalanceOf<Test>,
  fee_burn: BalanceOf<Test>,
  limit: BalanceOf<Test>,
  claims: Vec<(EthereumTxHash, EthereumAddress, BalanceOf<Test>, bool)>,
) {
  BridgeAccount::<Test>::put(account.clone().some());
  MintFee::<Test>::put(fee_mint.some());
  BurnFee::<Test>::put(fee_burn.some());
  ClaimLimit::<Test>::put(limit);
  for (tx, addr, amount, claimed) in claims {
    Claims::<Test>::insert(tx, (addr, amount, claimed).some());
  }
}

#[test]
fn runtime_upgrade_should_copy_data() {
  new_test_ext().execute_with(|| {
    // storage version is 1 by default
    assert_eq!(CloverClaims::pallet_storage_version(), 1);

    let tx2 = EthereumTxHash(hex![
      "4c5adaad6ca9cd2ae9f372b59ff6765fb66082c08caf6e61e6fbc39c35e82beb"
    ]);
    let claims = vec![
      (get_legal_tx_hash(), get_legal_eth_addr(), 100, false),
      (tx2, get_legal_eth_addr(), 50, true),
    ];

    init_mock_data(&1u64, 1000u32.into(), 500u32.into(), 2000u32.into(), claims);
    assert_eq!(CloverClaims::mint_fee(), 1000u64.some());
    assert_eq!(CloverClaims::burn_fee(), 500u64.some());
    assert_eq!(CloverClaims::claim_limit(), 2000);
    assert_eq!(CloverClaims::claims(get_legal_tx_hash()).is_some(), true);
    assert_eq!(
      CloverClaims::elastic_claims(BridgeNetworks::BSC, get_legal_tx_hash()).is_none(),
      true
    );

    // elastic config should be empty/default values
    assert_eq!(
      CloverClaims::elastic_bridge_accounts(BridgeNetworks::BSC),
      None
    );
    assert_eq!(CloverClaims::elastic_claim_limits(BridgeNetworks::BSC), 0);
    assert_eq!(CloverClaims::bridge_fees(BridgeNetworks::BSC), (0, 0));

    // upgrade the data schema
    <CloverClaims as OnRuntimeUpgrade>::on_runtime_upgrade();

    assert_eq!(CloverClaims::pallet_storage_version(), 2);

    // check that data are migrated
    assert_eq!(
      CloverClaims::elastic_bridge_accounts(BridgeNetworks::BSC),
      Some(1)
    );
    assert_eq!(
      CloverClaims::elastic_claim_limits(BridgeNetworks::BSC),
      2000
    );
    assert_eq!(CloverClaims::bridge_fees(BridgeNetworks::BSC), (1000, 500));

    assert_eq!(
      CloverClaims::elastic_claims(BridgeNetworks::BSC, get_legal_tx_hash()),
      (get_legal_eth_addr(), 100, false).some(),
    );

    // keys should not be copied to the wrong destination
    assert_eq!(
      ElasticClaims::<Test>::contains_key(BridgeNetworks::Ethereum, get_legal_tx_hash()),
      false,
    );
    assert_eq!(
      ElasticClaims::<Test>::contains_key(BridgeNetworks::Ethereum, tx2),
      false,
    );
  });
}
