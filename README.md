# UTREEXO SP1

## Introduction
This project aims to build a SP1 based circuit, which will prove mutations on [utreexo](https://bitcoinops.org/en/topics/utreexo/) 
data structure -- Merkle forest based accumulator of utxos -- bitcoin unspent transaction outputs.

## General Structure
Utreexo structure was already implemented in rust -- [rustreexo](https://crates.io/crates/rustreexo) crate. Thus, we want to use it, and 
wrap to SP1 circuit to generate the proofs, which will be validated onchain. For this circuit we
want to have block and previous utreexo as inputs, parse block and update utreexo in the circuit,
and updated utreexo will be the output of the circuit.

## Known Issues
First of all, some system info, which relevant to all issues:
- MacOs on M2 chip
- rustc 1.80.0-nightly (9cdfe285c 2024-05-22)
- cargo 1.80.0-nightly (84dc5dc11 2024-05-20)
- Homebrew clang version 19.1.0

### Secp256k1-sys compilation problems
Sometimes `cargo prove build` fails with a weird error looking somehow like this:
```shell
[sp1]  warning: secp256k1-sys@0.10.1: #include <string.h>
[sp1]  warning: secp256k1-sys@0.10.1:          ^~~~~~~~~~
[sp1]  warning: secp256k1-sys@0.10.1: 1 error generated.
[sp1]  warning: secp256k1-sys@0.10.1: error: unable to create target: 'No available targets are compatible with triple "riscv32-succinct-zkvm-elf"'
[sp1]  warning: secp256k1-sys@0.10.1: 1 error generated.
[sp1] 
[sp1]  error: failed to run custom build command for `secp256k1-sys v0.10.1`
```

After some manipulations with clang, it works, but not very stable. For example, if I downgrade
bitcoin crate to 0.29 it fails again.


### How to have inputs with smart pointers?
When I started work on circuit, I found that `sp1_zkvm::io::read` seems to be the only function to
read circuit inputs. This function requires `DeserializeOwned` trait, which is not implemented for
utreexo structures from rustreexo, and they are not friendly to this trait because have `Rc` and 
`Weak` inside. Thus, I have a question: what is a good way to have inputs with smart pointers? Just
avoid such structures as inputs?

### Linker problem
I made some draft implementation, put some quick temporary solution for smart-pointers problem, and
got a linker error:
```shell
[sp1]    = note: rust-lld: error: undefined symbol: sys_rand
[sp1]            >>> referenced by std.2f355c700227e7ed-cgu.06
[sp1]            >>>               std-0d9675e6a289e02b.std.2f355c700227e7ed-cgu.06.rcgu.o:(std::sys::pal::zkvm::hashmap_random_keys::hd2a58019ba89423c) in archive /Users/viktarmakouski/.sp1/toolchains/bxEaOxKq99/lib/rustlib/riscv32im-succinct-zkvm-elf/lib/libstd-0d9675e6a289e02b.rlib
[sp1]
```

Not sure why it happens, but the most uncommon thing I did in the code is using patched crates.

## Aknowledgements
I took some code from https://github.com/Davidson-Souza/bridge, so thanks to Davidson-Souza for his
work.
