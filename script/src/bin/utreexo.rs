use alloy_sol_types::{sol, SolType};
use bitcoin::consensus::Encodable;
use bitcoin::TxIn;
use bitcoin::{block, Block};
use clap::Parser;
use regex::Regex;
use reqwest;
use rustreexo::accumulator::node_hash::NodeHash;
use rustreexo::accumulator::pollard::{OwnedPollard, Pollard};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sp1_sdk::network::client;
use sp1_sdk::{utils, ProverClient, SP1Stdin};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::io::Cursor;
use std::ops::Deref;
use std::time;
use std::time::{Duration, Instant};
use tokio::time::error::Elapsed;
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

fn get_input_leaf_hashes(file_path: &str) -> HashMap<TxIn, (NodeHash, CompactLeafData)> {
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

#[derive(Debug, Default, Deserialize, Serialize)]
struct Metrics {
    pub prove_duration: Duration,
    pub acc_size: u64,
    pub block_size: u64,
    pub block_height: u64,
    pub tx_count: u64,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct MetricsCycles {
    pub total_instructions: u64,
    pub acc_size: u64,
    pub block_size: u64,
    pub block_height: u64,
    pub tx_count: u64,
}

async fn calculate_current_height(num_tx: u64) -> Result<u32, Box<dyn Error>> {
    let filename = format!("../acc-data/block-{num_tx}txs/block.txt");
    let contents = fs::read_to_string(filename)?;

    // Regular expression to find the prev_blockhash
    let re = Regex::new(r"prev_blockhash:\s*([0-9a-fA-F]{64}),")?;
    let mut captures_iter = re.captures_iter(&contents);

    // Ensure prev_blockhash occurs exactly once
    let capture = captures_iter.next();
    if capture.is_none() || captures_iter.next().is_some() {
        panic!("prev_blockhash occurs not exactly once in the file");
    }

    // Extract the block hash
    let prev_blockhash = &capture.unwrap()[1];
    println!("Previous Block Hash: {}", prev_blockhash);

    // Use the BlockCypher API to get the block height
    let url = format!(
        "https://api.blockcypher.com/v1/btc/main/blocks/{}",
        prev_blockhash
    );
    let response = reqwest::get(&url).await?;
    let json: Value = response.json().await?;

    // Extract and print the block height
    if let Some(height) = json["height"].as_u64() {
        Ok(height as u32 + 1)
    } else {
        panic!("Block height not found for hash: {}", prev_blockhash);
    }
}

fn get_block_heights(data_path: &str) -> Result<Vec<u64>, Box<dyn Error>> {
    let mut heights = Vec::new();
    let re = Regex::new(r"^block-(\d+)txs$").unwrap();

    for entry in fs::read_dir(data_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(caps) = re.captures(folder_name) {
                    if let Some(num_match) = caps.get(1) {
                        if let Ok(height) = num_match.as_str().parse::<u64>() {
                            heights.push(height);
                        } else {
                            eprintln!("Warning: Couldn't parse height from '{}'", folder_name);
                        }
                    }
                }
            }
        }
    }

    Ok(heights)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    utils::setup_logger();
    let args = Args::parse();
    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }
    let mut available_tx_counts = get_block_heights("../acc/data/").unwrap();
    available_tx_counts.sort();
    for tx_count in available_tx_counts {
        let height: u32 = calculate_current_height(tx_count).await?;
        println!("Calculated height: {height}");
        let acc_before_path: String = format!("../acc-data/block-{tx_count}txs/acc-beffore.txt");
        let acc_after_path: String = format!("../acc-data/block-{tx_count}txs/acc-after.txt");
        let input_leaf_hashes_path: String =
            format!("../acc-data/block-{tx_count}txs/input-leaf-hashes.txt");

        let serialized_acc_before = fs::read(&acc_before_path).unwrap();
        let block: Block = get_block(height).await?;
        let input_leaf_hashes: HashMap<TxIn, (NodeHash, CompactLeafData)> =
            get_input_leaf_hashes(&input_leaf_hashes_path);

        let mut stdin = SP1Stdin::new();

        stdin.write::<Block>(&block);
        stdin.write::<u32>(&height);
        stdin.write::<Vec<u8>>(&serialized_acc_before);
        stdin.write::<HashMap<TxIn, (NodeHash, CompactLeafData)>>(&input_leaf_hashes);

        if args.execute {
            let client = ProverClient::new();
            let public_values = client.execute(ELF, stdin).run().unwrap();
            let actual_bytes = public_values.0.as_slice();
            let expected_bytes = get_output_bytes(&acc_after_path);
            let unexpected_bytes = get_output_bytes(&acc_before_path);
            assert_ne!(actual_bytes, unexpected_bytes);
            assert_eq!(actual_bytes, expected_bytes);
            println!("Succesfully executed. Generating report.");

            let cycles = public_values.1.total_instruction_count();
            let acc_size = fs::File::open(acc_before_path)
                .unwrap()
                .metadata()
                .unwrap()
                .len();
            let mut block_str: Vec<u8> = Default::default();
            let _ = get_block(height)
                .await?
                .consensus_encode(&mut block_str)
                .unwrap();
            let block_size = block_str.len();

            let metrics = MetricsCycles {
                total_instructions: cycles,
                acc_size: acc_size,
                block_size: block_size as u64,
                block_height: height as u64,
                tx_count: tx_count as u64,
            };

            let file = File::create(format!("../metrics-cycles/{}.json", tx_count))?;
            serde_json::to_writer_pretty(file, &metrics)?;
            println!("Report saved to ../metrics-cycles/{}.json", tx_count);
        } else {
            let client = ProverClient::new();
            let (pk, vk) = client.setup(ELF);

            let start = Instant::now();
            let proof = client
                .prove(&pk, stdin)
                .run()
                .expect("failed to generate proof");
            let duration = start.elapsed();
            let acc_size = fs::File::open(&acc_before_path)
                .unwrap()
                .metadata()
                .unwrap()
                .len();
            let mut block_str: Vec<u8> = Default::default();
            let _ = get_block(height)
                .await?
                .consensus_encode(&mut block_str)
                .unwrap();
            let block_size = block_str.len();

            let metrics = Metrics {
                prove_duration: duration,
                acc_size: acc_size,
                block_size: block_size as u64,
                block_height: height as u64,
                tx_count: tx_count as u64,
            };

            let file = File::create(format!("../metrics/{}.json", tx_count))?;
            serde_json::to_writer_pretty(file, &metrics)?;

            println!("Successfully generated proof!");
            client.verify(&proof, &vk).expect("failed to verify proof");
            println!("Successfully verified proof!");
        }
    }
    Ok(())
}
