use sp1_sdk::{utils, ProverClient, SP1Stdin};

const ELF: &[u8] = include_bytes!("../../../program/verify/elf/riscv32im-succinct-zkvm-elf");

fn main() {
    utils::setup_logger();

    let parent_hash = hex::decode("0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let bits: u32 = 0x1d00ffff;
    let header_bytes = hex::decode(
        "0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c\
        010000006fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000982051fd1e4ba744bbbe680e1fee14677ba1a3c3540bf7b1cdb606e857233e0e61bc6649ffff001d01e36299"
    ).unwrap();
    println!("header_bytes input len: {:?}", header_bytes.len());

    let mut stdin = SP1Stdin::new();
    stdin.write_vec(parent_hash);
    stdin.write::<u32>(&bits);
    stdin.write_vec(header_bytes);

    let client = ProverClient::new();
    let public_values = client.execute(ELF, stdin).unwrap();

    println!("proof generated");

    let output = public_values.as_slice();

    println!("output: {}", hex::encode(output));
}