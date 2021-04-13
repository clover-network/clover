// Copyright (C) 2021 Clover Network
// This file is part of Clover.

use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};
use pallet_balances::Error as BalancesError;

#[test]
fn mint_and_claim_should_works() {
  new_test_ext().execute_with(|| {
    // set bridge account to root
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 1));

    // Set claim limit = 100
    assert_ok!(CloverClaims::set_claim_limit(Origin::root(), 100));
    assert_eq!(CloverClaims::claim_limit(), 100);

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim(
      Origin::signed(1),
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // claims and limit should updated
    assert_eq!(
      CloverClaims::claims(&tx_hash),
      Some((eth_addr.clone(), 100, false))
    ); // new tx
    assert_eq!(CloverClaims::claim_limit(), 0);

    // Claim it with correct msg sig
    // Pay CLVs to the TEST account:0100000000000000
    let sig = get_legal_eth_sig();
    assert_eq!(Balances::free_balance(1), 0);
    assert_ok!(CloverClaims::claim(
      Origin::none(),
      1,
      tx_hash.clone(),
      sig.clone()
    ));

    // 5. Claim success
    assert_eq!(Balances::free_balance(1), 100);
    assert_eq!(
      CloverClaims::claims(&tx_hash),
      Some((eth_addr.clone(), 100, true))
    ); // tx has already be claimed
  });
}

#[test]
fn mint_fee_should_work() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_mint_fee(Origin::root(), 1000));
    // set bridge account to root
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 1));
    assert_ok!(CloverClaims::set_claim_limit(Origin::root(), 2000));

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_noop!(
      CloverClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100),
      Error::<Test>::InvalidAmount
    );

    assert_eq!(CloverClaims::claim_limit(), 2000);

    assert_ok!(CloverClaims::mint_claim(
      Origin::signed(1),
      tx_hash.clone(),
      eth_addr.clone(),
      1200
    ));

    // claim limit only update with the real amount
    assert_eq!(CloverClaims::claim_limit(), 1800);
    // Claim it with correct msg sig
    // Pay CLVs to the TEST account:0100000000000000
    let sig = get_legal_eth_sig();
    assert_eq!(Balances::free_balance(1), 0);
    assert_ok!(CloverClaims::claim(
      Origin::none(),
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
      CloverClaims::set_bridge_account(Origin::signed(1), 1),
      DispatchError::BadOrigin
    );

    // set bridger
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 1)); // 1 is bridger

    // 1. Mint a claim with 2, no way
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_noop!(
      CloverClaims::mint_claim(Origin::signed(2), tx_hash.clone(), eth_addr.clone(), 100),
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
      CloverClaims::claim(Origin::none(), 1, tx_hash, sig),
      Error::<Test>::TxNotMinted
    );
  });
}

#[test]
fn illegal_sig_claim_should_fail() {
  new_test_ext().execute_with(|| {
    // only root account can change bridge account
    assert_noop!(
      CloverClaims::set_bridge_account(Origin::signed(2), 1),
      DispatchError::BadOrigin
    );
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 1));

    assert_ok!(CloverClaims::set_claim_limit(Origin::root(), 100));

    //  Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim(
      Origin::signed(1),
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // Claim it with illegal sig
    let sig1 = get_another_account_eth_sig();
    assert_noop!(
      CloverClaims::claim(Origin::none(), 1, tx_hash.clone(), sig1.clone()),
      Error::<Test>::SignatureNotMatch
    );

    // Sig with wrong message should failed
    let sig2 = get_wrong_msg_eth_sig();
    assert_noop!(
      CloverClaims::claim(Origin::none(), 1, tx_hash.clone(), sig2.clone()),
      Error::<Test>::SignatureNotMatch
    );
  });
}

#[test]
fn mint_twice_should_fail() {
  new_test_ext().execute_with(|| {
    // Set miner and superior
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 1));

    // Set limit
    assert_ok!(CloverClaims::set_claim_limit(Origin::root(), 100));

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim(
      Origin::signed(1),
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // 3. Mint the same eth again
    assert_noop!(
      CloverClaims::mint_claim(Origin::signed(1), tx_hash.clone(), eth_addr.clone(), 100),
      Error::<Test>::AlreadyMinted
    );
  });
}

#[test]
fn claim_twice_should_fail() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 1));

    // Set limit
    assert_ok!(CloverClaims::set_claim_limit(Origin::root(), 100));

    // Mint a claim
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_ok!(CloverClaims::mint_claim(
      Origin::signed(1),
      tx_hash.clone(),
      eth_addr.clone(),
      100
    ));

    // Claim it
    // Pay CLVs to the TEST account:0100000000000000
    let sig = get_legal_eth_sig();
    assert_eq!(Balances::free_balance(1), 0);
    assert_ok!(CloverClaims::claim(
      Origin::none(),
      1,
      tx_hash.clone(),
      sig.clone()
    ));
    assert_eq!(Balances::free_balance(1), 100);

    // failed on the second try
    assert_noop!(
      CloverClaims::claim(Origin::none(), 1, tx_hash.clone(), sig.clone()),
      Error::<Test>::AlreadyClaimed
    );
    assert_eq!(Balances::free_balance(1), 100);
  });
}

#[test]
fn claim_limit_should_work() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 2));

    // Mint a claim should failed without limitation
    let tx_hash = get_legal_tx_hash();
    let eth_addr = get_legal_eth_addr();
    assert_noop!(
      CloverClaims::mint_claim(Origin::signed(2), tx_hash.clone(), eth_addr.clone(), 10),
      Error::<Test>::ClaimLimitExceeded
    );

    // Set limit
    assert_ok!(CloverClaims::set_claim_limit(Origin::root(), 10));
    assert_eq!(CloverClaims::claim_limit(), 10);

    // Claim amount with limitation should be ok
    assert_ok!(CloverClaims::mint_claim(
      Origin::signed(2),
      tx_hash.clone(),
      eth_addr.clone(),
      10
    ));
    assert_eq!(CloverClaims::claim_limit(), 0);
  });
}

#[test]
fn burn_should_work() {
  new_test_ext().execute_with(|| {
    let eth_addr = get_legal_eth_addr();

    // should burn balance from account 4
    assert_ok!(CloverClaims::burn(Origin::signed(4), eth_addr, 40));
    assert_eq!(Balances::free_balance(4), 60);

    assert_ok!(CloverClaims::burn(Origin::signed(4), eth_addr, 40));
    assert_eq!(Balances::free_balance(4), 20);

    // should failed if burn all the balances from an account
    assert_noop!(
      CloverClaims::burn(Origin::signed(5), eth_addr, 100),
      BalancesError::<Test, _>::KeepAlive
    );
    assert_eq!(Balances::free_balance(5), 100);
  });
}

#[test]
fn burn_fee_should_work() {
  new_test_ext().execute_with(|| {
    assert_ok!(CloverClaims::set_burn_fee(Origin::root(), 30));
    // set bridge account to root
    assert_ok!(CloverClaims::set_bridge_account(Origin::root(), 1));
    assert_ok!(CloverClaims::set_claim_limit(Origin::root(), 2000));

    let eth_addr = get_legal_eth_addr();

    // burn should failed if there is not enough balance to pay fee
    assert_noop!(
      CloverClaims::burn(Origin::signed(4), eth_addr, 20),
      Error::<Test>::InvalidAmount
    );
    assert_eq!(Balances::free_balance(4), 100);

    assert_ok!(CloverClaims::burn(Origin::signed(5), eth_addr, 40));
    assert_eq!(Balances::free_balance(5), 60);

    // module has the fee deposited
    assert_eq!(Balances::free_balance(CloverClaims::account_id()), 30);
  });
}
