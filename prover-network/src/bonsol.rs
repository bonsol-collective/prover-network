use {once_cell::sync::Lazy, std::path::PathBuf, thiserror::Error};

static WORKING_DIR: Lazy<PathBuf> = Lazy::new(|| env::current_dir().unwrap());

const BONSOL_PROGRAM_ADDRESS: &str = "BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew";
const BONSOL_PROGRAM_NAME: &str = "bonsol.so";
// const BONSOL_PROGRAM_PATH: &str = "";

const DEFAULT_SOLANA_RPC_PORT: u64 = 8899;
const DEFAULT_SOLANA_WS_PORT: u64 = 8900;

pub trait SolanaProverNetworkError {
    fn is_running(container_name: &str) -> anyhow::Error;
}

pub mod solana {
    use {
        super::{Error, BONSOL_PROGRAM_ADDRESS},
        bollard::{
            container::{CreateContainerOptions, StartContainerOptions},
            Docker,
        },
    };

    // const BONSOL_PROGRAM_ADDRESS: &str = "BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew";
    // const BONSOL_PROGRAM_PATH: &str = "";

    const SOLANA_IMAGE: &str = "anzaxyz/agave:v2.0.13";
    const SOLANA_NODE_NAME: &str = "bonsol-prover-network-validator";

    #[derive(Error, Debug)]
    pub enum SolanaError {
        #[error("Solana start error occured {0}")]
        Start(String),
        #[error("Solana stop error occured {0}")]
        Stop(String),
        #[error("Solana reset error occured {0}")]
        Reset(String),
        #[error("Solana upgrade error occured {0}")]
        Upgrade(String),
    }

    impl SolanaError {
        async fn is_running(container_name: &str) -> Result<(), SolanaError> {
            let docker = Docker::connect_with_local_defaults()
                .map_err(|e| return SolanaError::Start(e.to_string()))?;
            let results = docker
                .inspect_container(container_name, None)
                .await
                .map_err(|e| return SolanaError::Start(e.to_string()))?;
            match results.state {
                Some(result) => {
                    match result.running {
                        Some(is) => {
                            if !is {
                                return Err(SolanaError::Start(
                                    "container is not running".to_string(),
                                ));
                            } else {
                                return Ok(());
                            }
                        }
                        None => {
                            return Err(SolanaError::Start("container is not running".to_string()));
                        }
                    };
                }
                None => {
                    return Err(SolanaError::Start("container is not running".to_string()));
                }
            };
        }
    }

    use {bollard::models::PortBinding, hashmap_macro::hashmap};

    // NOTE: this will not pull the docker image manually it must be pulled by running
    //       anzaxyx/agave:v2.0.13
    //
    //  https://solana.com/developers/guides/getstarted/solana-test-validator
    pub async fn start(
        upgrade_authority: &str,
        callback_program_address: &str,
    ) -> Result<(), SolanaError> {
        println!("starting solana node");
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| return SolanaError::Start(e.to_string()))?;
        let options = CreateContainerOptions {
            name: SOLANA_NODE_NAME,
            platform: Some("linux/amd64"),
        };

        let rpc_port = "8899/tcp";

        let exposed_port_rpc = std::collections::HashMap::<(), ()>::new();

        let ws_port = "8900/tcp";

        let exposed_port_ws = std::collections::HashMap::<(), ()>::new();

        let mut exposed_ports = std::collections::HashMap::new();

        exposed_ports.insert(rpc_port, exposed_port_rpc);
        exposed_ports.insert(ws_port, exposed_port_ws);

        let working_dir = super::WORKING_DIR.parent().unwrap().display();
        let onchain_src = format!("{}{}", working_dir, "/src/artifacts/onchain");
        std::fs::create_dir_all(onchain_src.clone())
            .map_err(|e| return SolanaError::Start(e.to_string()))?;
        let onchain_dst = "/etc/onchain";
        let validator_mount = format!("{}:{}", onchain_src, onchain_dst);
        let host_config = bollard::models::HostConfig {
            binds: Some(vec![validator_mount]),
            port_bindings: Some(hashmap! {
            super::DEFAULT_SOLANA_RPC_PORT.to_string() + &"/tcp".to_string() => Some(vec![PortBinding {
                host_ip: Some("".to_string()),
                host_port: Some(super::DEFAULT_SOLANA_RPC_PORT.to_string()),
            }]),
            super::DEFAULT_SOLANA_WS_PORT.to_string() + &"/tcp".to_string() => Some(vec![PortBinding {
                host_ip: Some("".to_string()),
                host_port: Some(super::DEFAULT_SOLANA_WS_PORT.to_string()),
            }]),
            }),
            ..Default::default()
        };
        println!(
            "connecting to bonsol at solana rpc port {}",
            super::DEFAULT_SOLANA_RPC_PORT.to_string()
        );
        println!(
            "connecting to bonsol at solana ws port {}",
            super::DEFAULT_SOLANA_WS_PORT.to_string()
        );
        docker
            .create_container(
                Some(options),
                bollard::container::Config {
                    hostname: Some("solana"),
                    image: Some(SOLANA_IMAGE),
                    entrypoint: Some(vec![
                        "solana-test-validator",
                        "--limit-ledger-size",
                        "100000",
                        "--rpc-port",
                        super::DEFAULT_SOLANA_RPC_PORT.to_string().as_str(),
                        "--rpc-pubsub-enable-block-subscription",
                        "--upgradeable-program",
                        BONSOL_PROGRAM_ADDRESS,
                        "/etc/onchain/bonsol.so",
                        upgrade_authority,
                        "--upgradeable-program",
                        callback_program_address,
                        "/etc/onchain/onchain.so",
                        upgrade_authority,
                    ]),
                    networking_config: Some(bollard::container::NetworkingConfig {
                        endpoints_config: {
                            let mut map = std::collections::HashMap::new();
                            map.insert("bonsol", bollard::models::EndpointSettings::default());
                            map
                        },
                    }),
                    host_config: Some(host_config),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| return SolanaError::Start(e.to_string()))?;
        let opt: Option<StartContainerOptions<String>> = None;
        docker
            .start_container(SOLANA_NODE_NAME, opt)
            .await
            .map_err(|e| return SolanaError::Start(e.to_string()))?;
        SolanaError::is_running(SOLANA_NODE_NAME).await?;
        Ok(())
    }

    pub async fn stop() -> Result<(), SolanaError> {
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| return SolanaError::Stop(e.to_string()))?;
        docker
            .stop_container("bonsol-prover-network-validator", None)
            .await
            .map_err(|e| return SolanaError::Stop(e.to_string()))?;
        docker
            .remove_container("bonsol-prover-network-validator", None)
            .await
            .map_err(|e| return SolanaError::Stop(e.to_string()))?;
        Ok(())
    }

    /*
        pub async fn upgrade_callback_program(callback_program_fp: &str) -> Result<(), SolanaError> {
            upgrade(
                SOLANA_RPC,
                payer,
                upgrade_authority,
                CALLBACK_PROGRAM_ADDRESS,
                BONSOL_PROGRAM_WORK_DIRECTORY,
            )
            .await?;
            Ok(())
        }

        // todo: how should we upgrade the program?
        pub async fn upgrade_bonsol_program(
            upgrade_authority: &Pubkey,
            payer: &Pubkey,
        ) -> Result<(), SolanaError> {
            upgrade(
                SOLANA_RPC,
                payer,
                upgrade_authority,
                BONSOL_PROGRAM_ADDRESS,
                BONSOL_PROGRAM_WORK_DIRECTORY,
            )
            .await?;
            Ok(())
        }

        async fn upgrade(
            program_id: &Pubkey,
            payer: &Keypair,
            upgrade_authority: &Keypair,
            program_filepath: &str,
        ) -> Result<(), SolanaError> {
            let rpc_client = RpcClient::new_with_commitment(SOLANA_RPC, CommitmentConfig::confirmed());
            let mut program_data = Vec::new();
            let mut file =
                File::open(program_filepath).map_err(|e| return SolanaError::Upgrade(e.to_string()))?;
            file.read_to_end(&mut program_data)
                .map_err(|e| return SolanaError::Upgrade(e.to_string()))?;
            let program_account = rpc_client
                .get_account(program_id)
                .map_err(|e| return SolanaError::Upgrade(e.to_string()))?;
            let program_data_address = match program_account.data.as_slice() {
                [1, rest @ ..] => {
                    let state: UpgradeableLoaderState = bincode::deserialize(rest)
                        .map_err(|e| return SolanaError::Reset(e.to_string()))?;
                    match state {
                        UpgradeableLoaderState::Program {
                            programdata_address,
                        } => programdata_address,
                        _ => {
                            return Err(SolanaError::Upgrade(
                                "failed to update solana program".to_string(),
                            ))
                        }
                    }
                }
                _ => {
                    return Err(SolanaError::Upgrade(
                        "failed to update solana program".to_string(),
                    ))
                }
            };
            let buffer = Keypair::new();
            let program_len = program_data.len();

            let buffer_rent = rpc_client
                .get_minimum_balance_for_rent_exemption(program_len)
                .map_err(|e| return SolanaError::Upgrade(e.to_string()))?;

            // todo: air drop into this account
            let create_buffer_ix = system_instruction::create_account(
                &payer.pubkey(),
                &buffer.pubkey(),
                buffer_rent,
                program_len as u64,
                &bpf_loader_upgradeable::id(),
            );
            let write_ix = bpf_loader_upgradeable::write(
                &buffer.pubkey(),
                &upgrade_authority.pubkey(),
                0,
                program_data,
            );

            let bh = rpc_client
                .get_latest_blockhash()
                .map_err(|e| return SolanaError::Reset(e.to_string()))?;

            let deploy_buffer_tx = Transaction::new_signed_with_payer(
                &[create_buffer_ix, write_ix],
                Some(&payer.pubkey()),
                &[payer, &buffer, upgrade_authority],
                bh,
            );

            rpc_client
                .send_and_confirm_transaction(&deploy_buffer_tx)
                .map_err(|e| return SolanaError::Reset(e.to_string()))?;

            let upgrade_ix = bpf_loader_upgradeable::upgrade(
                program_id,
                &buffer.pubkey(),
                &upgrade_authority.pubkey(),
                &program_data_address,
            );

            let bh = rpc_client
                .get_latest_blockhash()
                .map_err(|e| return SolanaError::Reset(e.to_string()))?;

            let upgrade_tx = Transaction::new_signed_with_payer(
                &[upgrade_ix],
                Some(&payer.pubkey()),
                &[payer, upgrade_authority],
                bh,
            );

            rpc_client
                .send_and_confirm_transaction(&upgrade_tx)
                .map_err(|e| return SolanaError::Reset(e.to_string()))?;

            Ok(())
        }
    */
}

pub mod prover_network {
    use {
        super::{bonsol, Error},
        bollard::{network::CreateNetworkOptions, Docker},
    };

    const MAX_BONSOL_NODES: usize = 10;

    #[derive(Error, Debug)]
    pub enum ProverNetworkError {
        #[error("max bonsol nodes {0} exceeded")]
        MaxBonsolNodesExceeded(usize),
        #[error("minimum of 1 bonsol node required")]
        MinimumBonsolNodesRequired,
        #[error("failed to start prover network due to bonsol node error {0}")]
        BonsolBootErrorOccured(bonsol::BonsolNodeError),
        #[error("failed to clean up prover network due to bonsol node error {0}")]
        BonsolCleanupErrorOccured(bonsol::BonsolNodeError),
    }

    pub async fn start(nodes: usize) -> Result<(), ProverNetworkError> {
        if nodes > MAX_BONSOL_NODES {
            return Err(ProverNetworkError::MaxBonsolNodesExceeded(nodes));
        }
        if nodes < 1 {
            return Err(ProverNetworkError::MinimumBonsolNodesRequired);
        }
        // todo: can we build these containers concurrently?
        for i in 1..=nodes {
            bonsol::create(i as u64)
                .await
                .map_err(|e| return ProverNetworkError::BonsolBootErrorOccured(e))?;
            bonsol::start(i as u64)
                .await
                .map_err(|e| return ProverNetworkError::BonsolBootErrorOccured(e))?;
        }
        Ok(())
    }

    pub async fn stop(nodes: usize) -> anyhow::Result<()> {
        for i in 1..=nodes {
            bonsol::clean_up(i as u64).await?;
        }
        Ok(())
    }

    pub async fn network(network: &str) -> anyhow::Result<()> {
        let docker = Docker::connect_with_local_defaults()?;
        let mut labels = std::collections::HashMap::new();
        labels.insert("environment", "development");
        let options = CreateNetworkOptions {
            name: network,
            driver: "standard",
            internal: false,
            attachable: true,
            ingress: false,
            enable_ipv6: false,
            labels,
            ..Default::default()
        };
        docker.create_network(options).await?;
        Ok(())
    }

    pub fn create_artifacts() -> anyhow::Result<()> {
        std::fs::create_dir("./src/artifacts")?;
        Ok(())
    }
}

pub mod bonsol {
    use {
        super::{Error, Lazy, PathBuf, WORKING_DIR},
        crate::config::{
            IngesterConfig, MetricsConfig, MissingImageStrategy, ProverNodeConfig, SignerConfig,
            TransactionSenderConfig,
        },
        bollard::{
            container::{
                CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
                StopContainerOptions,
            },
            Docker,
        },
        solana_sdk::signer::keypair::Keypair,
        std::{fs::File, io::Write},
    };

    static BONSOL_NODE_CONFIG_PATH_MOUNTED: Lazy<PathBuf> =
        Lazy::new(|| WORKING_DIR.join("/src/artifacts/config.toml"));

    static SOLANA_PROVER_SIGNER_MOUNTED: Lazy<PathBuf> =
        Lazy::new(|| WORKING_DIR.join("/src/artifacts/signer.json"));

    const BONSOL_NODE_IMAGE: &str = "bonsol-node:latest";
    const RISC0_IMAGE_FOLDER: &str = "/usr/opt/bonsol/risc0_images";
    const STARK_COMPRESSION_TOOLS_PATH: &str = "/usr/opt/bonsol/stark";

    #[derive(Error, Debug)]
    pub enum BonsolNodeError {
        #[error("failed to create bonsol node folder {0}")]
        InitiationFailedToCreateFolder(String),
        #[error("failed to create bonsol config due to {0}")]
        InitiationFailedToCreateBonsolConfig(String),
        #[error("failed to create solana signer due to {0}")]
        InitiationFailedToCreateSolanaSigner(String),
        #[error("failed to remove bonsol config due to {0}")]
        ResetFailedToRemoveBonsolConfig(String),
        #[error("failed to remove solana signer due to {0}")]
        ResetFailedToRemoveSolanaSigner(String),
        #[error("failed to run container due to {0}")]
        RunFailure(String),
        #[error("failed to clean up container due to{0}")]
        CleanupFailure(String),
    }

    impl BonsolNodeError {
        async fn is_running(container_name: &str) -> Result<(), BonsolNodeError> {
            let docker = Docker::connect_with_local_defaults()
                .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
            let results = docker
                .inspect_container(container_name, None)
                .await
                .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
            match results.state {
                Some(result) => {
                    match result.running {
                        Some(is) => {
                            if !is {
                                return Err(BonsolNodeError::RunFailure(
                                    "container is not running".to_string(),
                                ));
                            } else {
                                return Ok(());
                            }
                        }
                        None => {
                            return Err(BonsolNodeError::RunFailure(
                                "container is not running".to_string(),
                            ));
                        }
                    };
                }
                None => {
                    return Err(BonsolNodeError::RunFailure(
                        "container is not running".to_string(),
                    ));
                }
            };
        }
    }

    pub fn initiate_artifacts(prover: u64) -> Result<(), BonsolNodeError> {
        let path = format!("{}{}{}", WORKING_DIR.display(), "/artifacts/node-", prover);
        println!("creating directory {:?}", path);
        std::fs::create_dir_all(path.clone())
            .map_err(|e| return BonsolNodeError::InitiationFailedToCreateFolder(e.to_string()))?;
        create_solana_pubkey(prover)?;
        create_bonsol_config(prover)?;
        Ok(())
    }

    pub async fn clean_up(prover: u64) -> Result<(), BonsolNodeError> {
        stop(prover).await?;
        remove_bonsol_config(prover)?;
        remove_solana_pubkey(prover)?;
        Ok(())
    }

    pub fn remove_bonsol_config(prover: u64) -> Result<(), BonsolNodeError> {
        let file = BONSOL_NODE_CONFIG_PATH_MOUNTED
            .to_str()
            .and_then(|_| {
                let path = format!(
                    "{}{}{}{}",
                    WORKING_DIR.display(),
                    "/artifacts/node-",
                    prover,
                    "/config.toml"
                );
                Some(path)
            })
            .unwrap();
        std::fs::remove_file(file)
            .map_err(|e| return BonsolNodeError::CleanupFailure(e.to_string()))?;
        Ok(())
    }

    pub fn remove_solana_pubkey(prover: u64) -> Result<(), BonsolNodeError> {
        let file = SOLANA_PROVER_SIGNER_MOUNTED
            .to_str()
            .and_then(|_| {
                let working_dir = super::WORKING_DIR.parent();
                let path = format!(
                    "{}{}{}{}",
                    working_dir?.display(),
                    "/artifacts/node-",
                    prover,
                    "/signer.json"
                );
                println!("removing file {}", path);
                Some(path)
            })
            .unwrap();
        std::fs::remove_file(file)
            .map_err(|e| return BonsolNodeError::CleanupFailure(e.to_string()))?;
        Ok(())
    }

    // todo: make ingester mode configurable
    pub fn create_bonsol_config(prover: u64) -> Result<(), BonsolNodeError> {
        let ingester_rpc_url_websocket =
            "ws://solana:".to_owned() + &super::DEFAULT_SOLANA_WS_PORT.to_string();

        let transaction_sender_rpc_url =
            "http://solana:".to_owned() + &super::DEFAULT_SOLANA_RPC_PORT.to_string();

        println!("ingester block grpc url is {}", ingester_rpc_url_websocket);
        println!("transaction_sender_url is {}", transaction_sender_rpc_url);

        let config = ProverNodeConfig {
            env: Some("dev".to_string()),
            bonsol_program: "".to_string(),
            risc0_image_folder: String::from(RISC0_IMAGE_FOLDER),
            max_image_size_mb: 10,
            image_compression_ttl_hours: 3000,
            max_input_size_mb: 30,
            image_download_timeout_secs: 300,
            input_download_timeout_secs: 300,
            maximum_concurrent_proofs: 300,
            ingester_config: IngesterConfig::RpcBlockSubscription {
                wss_rpc_url: ingester_rpc_url_websocket,
            },
            transaction_sender_config: TransactionSenderConfig::Rpc {
                rpc_url: transaction_sender_rpc_url,
            },
            signer_config: SignerConfig::KeypairFile {
                path: "/opt/node/signer.json".to_string(),
            },
            stark_compression_tools_path: String::from(STARK_COMPRESSION_TOOLS_PATH),
            metrics_config: MetricsConfig::Prometheus {},
            missing_image_strategy: MissingImageStrategy::default(),
        };
        let str = config.to_custom_toml();
        let file = BONSOL_NODE_CONFIG_PATH_MOUNTED
            .to_str()
            .and_then(|_| {
                let path = format!(
                    "{}{}{}{}",
                    WORKING_DIR.display(),
                    "/artifacts/node-",
                    prover,
                    "/config.toml"
                );
                let file = File::create(path).map_err(|e| {
                    BonsolNodeError::InitiationFailedToCreateBonsolConfig(e.to_string())
                });
                Some(file)
            })
            .unwrap();
        file.unwrap().write_all(str.as_bytes()).map_err(|e| {
            return BonsolNodeError::InitiationFailedToCreateBonsolConfig(e.to_string());
        })?;
        Ok(())
    }

    struct PubKeyCreationResult {
        file: File,
        path: String,
    }

    pub fn create_solana_pubkey(prover: u64) -> Result<String, BonsolNodeError> {
        let mut result = SOLANA_PROVER_SIGNER_MOUNTED
            .to_str()
            .and_then(|_| {
                let path = format!(
                    "{}{}{}{}",
                    WORKING_DIR.display(),
                    "/artifacts/node-",
                    prover,
                    "/signer.json"
                );
                println!("creating pub key config for prover in {:?}", path);

                let file = File::create(&path).map_err(|e| {
                    BonsolNodeError::InitiationFailedToCreateSolanaSigner(e.to_string())
                });
                return Some(PubKeyCreationResult {
                    file: file.ok()?,
                    path,
                });
            })
            .unwrap();
        let keypair_bytes = Keypair::new().to_bytes();
        let json = serde_json::to_string(&keypair_bytes.to_vec()).map_err(|e| {
            return BonsolNodeError::InitiationFailedToCreateSolanaSigner(e.to_string());
        })?;
        result.file.write_all(json.as_bytes()).map_err(|e| {
            return BonsolNodeError::InitiationFailedToCreateSolanaSigner(e.to_string());
        })?;
        Ok(result.path)
    }

    #[allow(dead_code)]
    //
    // User should build from the Dockerfile at https://github.com/anagrambuild/bonsol/blob/main/Dockerfile
    // for now --
    //
    // todo: when everything necessary is merged we can just pull the valid bonsol node
    /*
    pub async fn build() -> anyhow::Result<()> {
        let docker = Docker::connect_with_local_defaults()?;
        let ba: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        let opts = BuildImageOptions {
            dockerfile: "Dockerfile",
            t: super::BONSOL_IMAGE,
            rm: true,
            forcerm: true,
            pull: true,
            buildargs: ba,
            ..Default::default()
        };
        docker.build_image(opts, None, None).try_next().await?;
        Ok(())
    }
    */

    pub async fn node(bonsol_node_id: u64) -> Result<(), BonsolNodeError> {
        create(bonsol_node_id).await?;
        start(bonsol_node_id).await?;
        Ok(())
    }

    pub async fn create(bonsol_node_id: u64) -> Result<(), BonsolNodeError> {
        println!("creating store for prover {}", bonsol_node_id);
        initiate_artifacts(bonsol_node_id)?;
        let work_dir =
            env::current_dir().map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        let config_src = format!(
            "{}{}-{}",
            work_dir.display(),
            "/artifacts/node",
            bonsol_node_id
        );
        let config_dst = "/opt/node";
        let node_mount = format!("{}:{}", config_src, config_dst);
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        let options: CreateContainerOptions<String> = CreateContainerOptions {
            name: ("bonsol-node-".to_owned() + &bonsol_node_id.to_string()),
            platform: None,
        };
        let host_config = Some(bollard::models::HostConfig {
            binds: Some(vec![node_mount]),
            ..Default::default()
        });

        let rpc_port = "8899/tcp";
        let exposed_port_rpc: std::collections::HashMap<(), ()> = std::collections::HashMap::new();
        let ws_port = "8900/tcp";

        let exposed_port_ws: std::collections::HashMap<(), ()> = std::collections::HashMap::new();
        let mut exposed_ports = std::collections::HashMap::new();

        exposed_ports.insert(rpc_port, exposed_port_rpc);
        exposed_ports.insert(ws_port, exposed_port_ws);

        docker
            .create_container(
                Some(options),
                bollard::container::Config {
                    hostname: Some("bonsol-node"),
                    image: Some(BONSOL_NODE_IMAGE),
                    entrypoint: Some(vec![
                        "/usr/opt/bonsol/bonsol-node",
                        "-f",
                        "/opt/node/config.toml",
                    ]),
                    exposed_ports: Some(exposed_ports),
                    host_config,
                    networking_config: Some(bollard::container::NetworkingConfig {
                        endpoints_config: {
                            let mut map = std::collections::HashMap::new();
                            map.insert("bonsol", bollard::models::EndpointSettings::default());
                            map
                        },
                    }),
                    ..Default::default()
                },
            )
            .await
            .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        Ok(())
    }

    pub async fn start(bonsol_node_id: u64) -> Result<(), BonsolNodeError> {
        println!("starting bonsol prover {}", bonsol_node_id);
        let opt: Option<StartContainerOptions<String>> = None;
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        docker
            .start_container(
                ("bonsol-node-".to_owned() + &bonsol_node_id.to_string()).as_str(),
                opt,
            )
            .await
            .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        BonsolNodeError::is_running(
            ("bonsol-node-".to_owned() + &bonsol_node_id.to_string()).as_str(),
        )
        .await?;
        Ok(())
    }

    pub async fn stop(bonsol_node_id: u64) -> Result<(), BonsolNodeError> {
        println!("stopping bonsol prover {}", bonsol_node_id);
        let node = "bonsol-node-".to_owned() + &bonsol_node_id.to_string();
        let docker = Docker::connect_with_local_defaults()
            .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        let opts = Some(StopContainerOptions { t: 30 });
        docker
            .stop_container(node.clone().as_str(), opts)
            .await
            .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let opts = Some(RemoveContainerOptions {
            force: true,
            v: true,
            link: false,
        });
        docker
            .remove_container(node.clone().as_str(), opts)
            .await
            .map_err(|e| return BonsolNodeError::RunFailure(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::{bonsol, prover_network, solana},
        crate::config::*,
        anyhow, tokio,
    };

    //  NOTE: we should really press for changes on the config format for the bonsol node /
    //        the figment library. this was work around necesasry to deal with that particular debt
    #[test]
    fn test_config_serialization_cycle() -> anyhow::Result<()> {
        let original_toml = r#"risc0_image_folder = "elf"
max_input_size_mb = 10
image_download_timeout_secs = 60
input_download_timeout_secs = 60
maximum_concurrent_proofs = 10
max_image_size_mb = 4
image_compression_ttl_hours = 24
stark_compression_tools_path = "./stark/"
env = "dev"

[ingester_config]
GrpcSubscription = { grpc_url = "http://solana:50051", connection_timeout_secs = 15, timeout_secs = 45, token = "test-token" }

[transaction_sender_config]
Rpc = { rpc_url = "http://localhost:8899" }

[signer_config]
KeypairFile = { path = "node_keypair.json" }"#;

        let config: ProverNodeConfig = toml::from_str(original_toml).unwrap();
        let serialized_toml = toml::to_string(&config).unwrap();
        let final_config: ProverNodeConfig = toml::from_str(&serialized_toml).unwrap();

        // Check basic config fields
        assert_eq!(config.risc0_image_folder, final_config.risc0_image_folder);
        assert_eq!(config.max_input_size_mb, final_config.max_input_size_mb);
        assert_eq!(
            config.image_download_timeout_secs,
            final_config.image_download_timeout_secs
        );
        assert_eq!(
            config.input_download_timeout_secs,
            final_config.input_download_timeout_secs
        );
        assert_eq!(
            config.maximum_concurrent_proofs,
            final_config.maximum_concurrent_proofs
        );
        assert_eq!(config.max_image_size_mb, final_config.max_image_size_mb);
        assert_eq!(
            config.image_compression_ttl_hours,
            final_config.image_compression_ttl_hours
        );
        assert_eq!(
            config.stark_compression_tools_path,
            final_config.stark_compression_tools_path
        );
        assert_eq!(config.env, final_config.env);

        match (&config.ingester_config, &final_config.ingester_config) {
            (
                IngesterConfig::GrpcSubscription {
                    grpc_url: url1,
                    connection_timeout_secs: timeout1,
                    timeout_secs: ts1,
                    token: token1,
                },
                IngesterConfig::GrpcSubscription {
                    grpc_url: url2,
                    connection_timeout_secs: timeout2,
                    timeout_secs: ts2,
                    token: token2,
                },
            ) => {
                assert_eq!(url1, url2);
                assert_eq!(timeout1, timeout2);
                assert_eq!(ts1, ts2);
                assert_eq!(token1, token2);
            }
            _ => panic!("Ingester configs don't match or aren't GrpcSubscription"),
        }

        match (
            &config.transaction_sender_config,
            &final_config.transaction_sender_config,
        ) {
            (
                TransactionSenderConfig::Rpc { rpc_url: url1 },
                TransactionSenderConfig::Rpc { rpc_url: url2 },
            ) => {
                assert_eq!(url1, url2);
            }
            _ => panic!("Transaction sender configs don't match"),
        }

        match (&config.signer_config, &final_config.signer_config) {
            (
                SignerConfig::KeypairFile { path: path1 },
                SignerConfig::KeypairFile { path: path2 },
            ) => {
                assert_eq!(path1, path2);
            }
            _ => panic!("Signer configs don't match"),
        }

        let toml_string = toml::to_string(&config).expect("Failed to serialize config");
        std::fs::write("./config", toml_string)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_create_bonsol_config() -> anyhow::Result<()> {
        match std::fs::remove_dir("./src/artifacts") {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                prover_network::create_artifacts()?;
                bonsol::create_bonsol_config(1)?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                bonsol::create_bonsol_config(1)?;
                Ok(())
            }
            _ => {
                bonsol::create_bonsol_config(1)?;
                Ok(())
            }
        }
    }

    #[tokio::test]
    async fn test_create_solana_public_key() -> anyhow::Result<()> {
        match std::fs::remove_dir("./src/artifacts") {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                prover_network::create_artifacts()?;
                bonsol::create_solana_pubkey(1)?;
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                bonsol::create_solana_pubkey(1)?;
                Ok(())
            }
            _ => {
                bonsol::create_solana_pubkey(1)?;
                Ok(())
            }
        }
    }

    #[test]
    fn test_remove_config() -> anyhow::Result<()> {
        bonsol::create_bonsol_config(1)?;
        bonsol::remove_bonsol_config(1)?;
        Ok(())
    }

    #[test]
    fn test_remove_solana_key() -> anyhow::Result<()> {
        bonsol::create_solana_pubkey(1)?;
        bonsol::remove_solana_pubkey(1)?;
        Ok(())
    }

    #[tokio::test]
    async fn test_single_node_mount() -> anyhow::Result<()> {
        bonsol::stop(1).await?;
        bonsol::create(1).await?;
        bonsol::stop(1).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_start_single_provers() -> anyhow::Result<()> {
        bonsol::create(1).await?;
        bonsol::start(1).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_prover_network() -> anyhow::Result<()> {
        prover_network::stop(10).await?;
        Ok(())
    }

    #[tokio::test]
    async fn reset() -> anyhow::Result<()> {
        prover_network::stop(1).await?;
        solana::stop().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_solana_start_upgradable_programs() -> anyhow::Result<()> {
        let upgrade_authority = solana_sdk::pubkey::new_rand().to_string();
        let callback_program_address = solana_sdk::pubkey::new_rand().to_string();
        solana::start(
            upgrade_authority.as_str(),
            callback_program_address.as_str(),
        )
        .await?;
        solana::stop().await?;
        Ok(())
    }

    use bollard::{container::InspectContainerOptions, secret::ContainerStateStatusEnum, Docker};
    async fn check_exit_status(provers: usize, container_id: &str) -> anyhow::Result<()> {
        let docker = Docker::connect_with_local_defaults()?;
        let inspect_options = InspectContainerOptions::default();
        let container_info = docker
            .inspect_container(container_id, Some(inspect_options))
            .await?;
        if let Some(state) = container_info.state {
            if let Some(status) = state.status {
                if status == ContainerStateStatusEnum::EXITED {
                    prover_network::stop(provers).await?;
                    solana::stop().await?;
                    return Err(anyhow::anyhow!("failed to run container"));
                }
            }
        }
        Ok(())
    }

    // todo: exit this loop
    #[tokio::test]
    async fn test_prover_network_connectivity() -> anyhow::Result<()> {
        let prover_nodes = 1;
        let bonsol_node_name = "bonsol-node-1";
        let upgrade_authority = solana_sdk::pubkey::new_rand().to_string();
        let callback_program_address = solana_sdk::pubkey::new_rand().to_string();
        solana::start(
            upgrade_authority.as_str(),
            callback_program_address.as_str(),
        )
        .await?;
        prover_network::start(prover_nodes).await?;

        loop {
            check_exit_status(prover_nodes, bonsol_node_name).await?;
        }
    }

    #[tokio::test]
    async fn test_prover_networks() -> anyhow::Result<()> {
        let bonsol_prover_node = 10;
        let upgrade_authority = solana_sdk::pubkey::new_rand().to_string();
        let callback_program_address = solana_sdk::pubkey::new_rand().to_string();
        solana::start(
            upgrade_authority.as_str(),
            callback_program_address.as_str(),
        )
        .await?;
        prover_network::start(bonsol_prover_node).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        prover_network::stop(bonsol_prover_node).await?;
        solana::stop().await?;
        Ok(())
    }
}
