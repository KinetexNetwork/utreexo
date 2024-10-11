use bitcoin::TxIn;
use bitcoin::{block, Block};
use reqwest;
use rustreexo::accumulator::node_hash::NodeHash;
use rustreexo::accumulator::pollard::{OwnedPollard, Pollard};
use serde::{Deserialize, Serialize};
use sp1_sdk::{utils, ProverClient, SP1Stdin};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub struct CompactLeafData {
    /// Header code tells the height of creating for this UTXO and whether it's a coinbase
    pub header_code: u32,
    /// The amount locked in this UTXO
    pub amount: u64,
    /// The type of the locking script for this UTXO
    pub spk_ty: ScriptPubkeyType,
}

#[derive(PartialEq, Eq, Clone, Debug, Serialize, Deserialize)]
pub enum ScriptPubkeyType {
    /// An non-specified type, in this case the script is just copied over
    Other(Box<[u8]>),
    /// p2pkh
    PubKeyHash,
    /// p2wsh
    WitnessV0PubKeyHash,
    /// p2sh
    ScriptHash,
    /// p2wsh
    WitnessV0ScriptHash,
}

const ELF: &[u8] = include_bytes!("../../../program/utreexo/elf/riscv32im-succinct-zkvm-elf");

async fn get_block(height: u32) -> Result<Block, Box<dyn Error>> {
    // Step 1: Get the block hash for the given height
    let block_hash_url = format!("https://blockstream.info/api/block-height/{}", height);
    let block_hash_response = reqwest::get(&block_hash_url).await?;
    let block_hash = block_hash_response.text().await?;

    let raw_block_url = format!(
        "https://blockstream.info/api/block/{}/raw",
        block_hash.trim()
    );
    let raw_block_response = reqwest::get(&raw_block_url).await?;
    let raw_block_bytes = raw_block_response.bytes().await?;

    // Step 3: Deserialize the raw block data into a Block struct
    let block: Block = bitcoin::consensus::deserialize(&raw_block_bytes).unwrap();
    Ok(block)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    utils::setup_logger();

    let height = 1;
    let acc_file = File::open("../acc-data/acc-before-1.txt")?;
    // let acc = Pollard::deserialize(reader).unwrap();
    let owned_acc = OwnedPollard::default();
    // let owned_acc = OwnedPollard::from_pollard(acc);
    let block: Block = get_block(height).await?;
    let input_leaf_hashes: HashMap<TxIn, (NodeHash, CompactLeafData)> = Default::default();

    let mut stdin = SP1Stdin::new();

    stdin.write::<Block>(&block);
    stdin.write::<u32>(&height);
    stdin.write::<OwnedPollard>(&owned_acc);
    stdin.write::<HashMap<TxIn, (NodeHash, CompactLeafData)>>(&input_leaf_hashes);

    let client = ProverClient::new();
    let public_values = client.execute(ELF, stdin).run().unwrap();

    println!("proof generated");
    // println!("output: {:#?}", public_values);
    Ok(())
}
