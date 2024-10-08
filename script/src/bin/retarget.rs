use sp1_sdk::{utils, ProverClient, SP1Stdin};

const ELF: &[u8] = include_bytes!("../../../program/retarget/elf/riscv32im-succinct-zkvm-elf");

fn main() {
    utils::setup_logger();

    let parent_block_number: u64 = 203610; 
    let parent_hash = hex::decode("00000000000004b37ad1386a1f55d373725768ae46dbcc6994f9c657211f2ea1").unwrap();
    let period_start_hash = hex::decode("00000000000003a5e28bef30ad31f1f9be706e91ae9dda54179a95c9f9cd9ad0").unwrap();
    let target_bits = 0x1a057e08;

    let period_start_header_bytes = hex::decode(
        "010000009d6f4e09d579c93015a83e9081fee83a5c8b1ba3c86516b61f0400000000000025399317bb5c7c4daefe8fe2c4dfac0cea7e4e85913cd667030377240cadfe93a4906b50087e051a84297df7"
    ).unwrap();

    let period_end_header_bytes = hex::decode(
        "010000000871883fffc7d164c354c0116dbeaa78b781be6c43378c31c00400000000000029dd20eed8dfd8b33dfc477df8760e3e0a4a52deaff397edcf8f5a9494c48d026fea7d50087e051a01085698"
    ).unwrap();

    let header_bytes = hex::decode(
        "01000000a12e1f2157c6f99469ccdb46ae68577273d3551f6a38d17ab30400000000000017eef03b2be727ffa0e11f9253310b69295226ec411d2aa2afa5c65d77f6560b9fdd7d50087e051a8cc0fc07\
        0100000054ad817e5f17f21df17cbe9c7f24b03a7e63813e674d228d34020000000000003a3c8ca05aa5bfdb5412aaf8b333d75cf23f4e8da3fe7e63a1dfebe364b68c034ce57d50087e051aeed552ee\
        020000005413ee60042578dcead181d701414fd47326d761054e6dd58702000000000000051a053037aa3f3242024f1904e0841bb23c58fce99d935b3801aef429e8eb2d5fe37d50087e051af0248a86\
        020000009fb996556eecc92283be2dc16490deb66abc3b0d37d20cd303040000000000009fa7fbe3b7bdcf75ca641d7c24f600221aabc2600213e66f7208c138dd8212fa87e47d50087e051a537f9515\
        010000000871883fffc7d164c354c0116dbeaa78b781be6c43378c31c00400000000000029dd20eed8dfd8b33dfc477df8760e3e0a4a52deaff397edcf8f5a9494c48d026fea7d50087e051a01085698\
        010000000ebf6a3d19183ff32d572d66ebc4712d74ab2b7bc8a5f2c72f0200000000000017941b52576aedb449ea2ed600a7399957b30c7378ff4c2f57f43dcd4064decf08e67d50ef75051a45a58373\
        020000009efb44572449cb6332e6e911292074d8de279b843ae3300501030000000000006d360ae767686ddf7db3623fbb0d0da917e8bc6dbdf408e4925b08deff7e6bf935e77d50ef75051a04d9e5ee\
        0100000001e6a4d6616f58a21f5d17bb66ad2411dd10d91c763443cffa00000000000000475fbe3a9a80b363221745f453530394f0ea231f236fdb6d0dc27cc2e99f44d1efe77d50ef75051a16a78f01\
        010000009ff61057f1f5dfb72026f5c401436d003ebf3d6165706fb01203000000000000831df89e90952239521d9cee1f23b9108c9953f68de0d4dbd08232bcf721a181caea7d50ef75051a6de1d466\
        010000002a5927cf6e7671e76dcada6d41c26faee7ef847e0a8262e20e0000000000000020d619151e4e3f44cc1a35cb4422d1738bfeecb48d8e5408d0662de899730f47d2e97d50ef75051a054e8939").unwrap();
    println!("header_bytes input len: {:?}", header_bytes.len());

    let mut stdin = SP1Stdin::new();
    stdin.write::<u64>(&parent_block_number);
    stdin.write_vec(parent_hash.into_iter().rev().collect());
    stdin.write_vec(period_start_hash.into_iter().rev().collect());
    stdin.write::<u32>(&target_bits);
    stdin.write_vec(period_start_header_bytes);
    stdin.write_vec(period_end_header_bytes);
    stdin.write_vec(header_bytes);

    let client = ProverClient::new();
    let public_values = client.execute(ELF, stdin).unwrap();

    println!("proof generated");

    let output = public_values.as_slice();

    println!("output: {}", hex::encode(output));
}