//SPDX-License-Identifier: MIT
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::{sol, SolType};
use bitcoin::{Block, TxIn};
use rustreexo::accumulator::node_hash::NodeHash;
use rustreexo::accumulator::pollard::OwnedPollard;
use rustreexo::accumulator::pollard::Pollard;
use std::collections::HashMap;

mod btc_structs;
mod process_block;

use crate::btc_structs::CompactLeafData;
use crate::process_block::process_block;

sol! {
    struct PublicValuesStruct {
        uint32 height;
    }
}

pub fn main() {
    let block = sp1_zkvm::io::read::<Block>();
    let height = sp1_zkvm::io::read::<u32>();
    // a hack to read Pollard
    let acc_owned = sp1_zkvm::io::read::<OwnedPollard>();
    let mut acc = Pollard::from_owned_pollard(acc_owned);
    let input_leaf_hashes = sp1_zkvm::io::read::<HashMap<TxIn, (NodeHash, CompactLeafData)>>();
    let (_proof, _compact_leaves) = process_block(&block, height, &mut acc, input_leaf_hashes);
    // TODO: have _proof and _compact_leaves as outputs instead of this
    let bytes = PublicValuesStruct::abi_encode(&PublicValuesStruct { height });
    sp1_zkvm::io::commit_slice(&bytes);
}
