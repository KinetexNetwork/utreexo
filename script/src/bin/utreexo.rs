use alloy_sol_types::{sol, SolType};
use bitcoin::TxIn;
use bitcoin::{block, Block};
use clap::Parser;
use reqwest;
use rustreexo::accumulator::node_hash::NodeHash;
use rustreexo::accumulator::pollard::{OwnedPollard, Pollard};
use serde::{Deserialize, Serialize};
use sp1_sdk::network::client;
use sp1_sdk::{utils, ProverClient, SP1Stdin};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::ops::Deref;

// TODO: one of this variables determines all other, so to work properly we need to be careful
// setting them. It will be nice to make all of them automatically calculated based on just one.
static HEIGHT: u32 = 586;
static ACC_BEFORE_PATH: &'static str = "block-3txs/acc-beffore.txt";
static ACC_AFTER_PATH: &'static str = "block-3txs/acc-after.txt";
static INPUT_LEAF_HASHES_PATH: &'static str = "block-3txs/input-leaf-hashes.txt";

type PublicValuesTuple = sol! {
    (
        bytes, // acc roots
    )
};

/// The arguments for the command.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    execute: bool,

    #[clap(long)]
    prove: bool,

    #[clap(long, default_value = "20")]
    n: u32,
}

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

fn get_expected_bytes() -> Vec<u8> {
    get_output_bytes(ACC_AFTER_PATH)
}

fn get_unexpected_bytes() -> Vec<u8> {
    get_output_bytes(ACC_BEFORE_PATH)
}

fn get_output_bytes(path: &str) -> Vec<u8> {
    let acc_file = File::open(path).unwrap();
    let acc_after = Pollard::deserialize(acc_file).unwrap();
    let acc_roots: Vec<NodeHash> = acc_after
        .get_roots()
        .to_vec()
        .iter()
        .map(|rc| rc.get_data())
        .collect();
    let acc_roots_bytes: Vec<[u8; 32]> = acc_roots.iter().map(|hash| *hash.deref()).collect();
    let acc_roots_bytes_flat: Vec<u8> = acc_roots_bytes.concat();
    let expected_bytes = PublicValuesTuple::abi_encode(&(acc_roots_bytes_flat,));
    expected_bytes
}

fn get_input_leaf_hashes() -> HashMap<TxIn, (NodeHash, CompactLeafData)> {
    let file_path = INPUT_LEAF_HASHES_PATH;
    let file = File::open(file_path).unwrap();
    let reader = BufReader::new(file);

    let deserialized_struct: Vec<(TxIn, (NodeHash, CompactLeafData))> =
        serde_json::from_reader(reader).unwrap();
    let mut res: HashMap<TxIn, (NodeHash, CompactLeafData)> = Default::default();
    for (k, v) in deserialized_struct {
        res.insert(k, v);
    }
    res
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    utils::setup_logger();

    // Parse the command line arguments.
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    let serialized_acc_before = fs::read(ACC_BEFORE_PATH).unwrap();
    let block: Block = get_block(HEIGHT).await?;
    let input_leaf_hashes: HashMap<TxIn, (NodeHash, CompactLeafData)> = get_input_leaf_hashes();

    let mut stdin = SP1Stdin::new();

    stdin.write::<Block>(&block);
    stdin.write::<u32>(&HEIGHT);
    stdin.write::<Vec<u8>>(&serialized_acc_before);
    stdin.write::<HashMap<TxIn, (NodeHash, CompactLeafData)>>(&input_leaf_hashes);

    if args.execute {
        let client = ProverClient::new();
        let public_values = client.execute(ELF, stdin).run().unwrap();
        let actual_bytes = public_values.0.as_slice();

        let expected_bytes = get_expected_bytes();
        let unexpected_bytes = get_unexpected_bytes();
        assert_ne!(actual_bytes, unexpected_bytes);
        assert_eq!(actual_bytes, expected_bytes);
        println!("Succesfully executed");
    } else {
        let client = ProverClient::new();
        let (pk, vk) = client.setup(ELF);
        let proof = client
            .prove(&pk, stdin)
            .run()
            .expect("failed to generate proof");
        println!("Successfully generated proof!");
        client.verify(&proof, &vk).expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
    Ok(())
}
