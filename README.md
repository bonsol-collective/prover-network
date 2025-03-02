# prover-network

Spin up a `solana test validator` and `N bonsol prover nodes` to be used in rust test

## Requirements

* `x86-64-amd` due to current `stark to snark` c++ constraint - virtualization will also not work here
* ports `8899` and `8900` must be free to be used by the `solana-test-validator`
* both `bonsol.so` and your onchain programs must be located in  ______ .

## Immediate Changes

* remove `figment` toml deserializer and leverage something a library that uses `Serialization` trait for programmatic bonsol-node configs
* pull bonsol-node docker image from a repository programmatically rather then having to build from the bonsol repository

## Setup

1. build dockerfile image within `https://github.com/bonsol-collective/bonsol/blob/main/Dockerfile` 
   as the image `bonsol-node/latest` so you have working image that the test harness expects. the 
   image must be named `bonsol-node/latest`
2. `cargo add ctor` and other required libraries

## Use

initiate the rust prover network harness with a `ctor` constructor before implementing tests, then tear down the harness in your final test.

```
    const BONSOL_PROVER_NODE_COUNT: usize = 1;
    const SOLANA_RPC_URL: &'static str = "http://localhost:8899";
    const GAME_PROGRAM: &'static str = "BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew";

    #[ctor::ctor]
    fn init() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tear_down_systems().await;
            let upgrade_authority = solana_sdk::pubkey::new_rand().to_string();
            let callback_program_address = solana_sdk::pubkey::new_rand().to_string();
            bonsol::solana::start(
                upgrade_authority.as_str(),
                callback_program_address.as_str(),
            )
            .await
            .unwrap();
            bonsol::prover_network::start(BONSOL_PROVER_NODE_COUNT)
                .await
                .unwrap();
        });
  }

    async fn test_bonsol_test_1() -> anyhow::Result<()> {
      /*
        Implement test code here
      */
    }

    async fn test_bonsol_final() -> anyhow::Result<()> {
      /*
          Implement test code here ... 
      */

      bonsol::solana::stop().await?;
      bonsol::prover_network::stop(BONSOL_PROVER_NODE_COUNT).await?;
    }

```
