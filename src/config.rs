// todo: this file can be removed in its entirety when the Serialization trait
//       is implemented for the bonsol-node config

use {
    figment::{
        providers::{Format, Toml},
        Figment,
    },
    serde::{Deserialize, Serialize},
    std::path::Path,
};

pub fn load_config(config_path: &str) -> ProverNodeConfig {
    let figment = Figment::new().merge(Toml::file(config_path));
    figment.extract().unwrap()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum IngesterConfig {
    RpcBlockSubscription {
        wss_rpc_url: String,
    },
    GrpcSubscription {
        grpc_url: String,
        connection_timeout_secs: u32,
        timeout_secs: u32,
        token: String,
    },
    WebsocketSub, //not implemented
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum TransactionSenderConfig {
    Rpc { rpc_url: String },
    //--- below not implemented yet
    Tpu,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum SignerConfig {
    KeypairFile { path: String }, //--- below not implemented yet maybe hsm, signer server or some weird sig agg shiz
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub enum MissingImageStrategy {
    #[default]
    DownloadAndClaim,
    DownloadAndMiss,
    Fail,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProverNodeConfig {
    pub env: Option<String>,
    #[serde(default = "default_bonsol_program")]
    pub bonsol_program: String,
    #[serde(default = "default_risc0_image_folder")]
    pub risc0_image_folder: String,
    #[serde(default = "default_max_image_size_mb")]
    pub max_image_size_mb: u32,
    #[serde(default = "default_image_compression_ttl_hours")]
    pub image_compression_ttl_hours: u32,
    #[serde(default = "default_max_input_size_mb")]
    pub max_input_size_mb: u32,
    #[serde(default = "default_image_download_timeout_secs")]
    pub image_download_timeout_secs: u32,
    #[serde(default = "default_input_download_timeout_secs")]
    pub input_download_timeout_secs: u32,
    #[serde(default = "default_maximum_concurrent_proofs")]
    pub maximum_concurrent_proofs: u32,
    #[serde(default = "default_ingester_config")]
    pub ingester_config: IngesterConfig,
    #[serde(default = "default_transaction_sender_config")]
    pub transaction_sender_config: TransactionSenderConfig,
    #[serde(default = "default_signer_config")]
    pub signer_config: SignerConfig,
    #[serde(default = "default_stark_compression_tools_path")]
    pub stark_compression_tools_path: String,
    #[serde(default = "default_metrics_config")]
    pub metrics_config: MetricsConfig,
    #[serde(default)]
    pub missing_image_strategy: MissingImageStrategy,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum MetricsConfig {
    Prometheus {},
    None,
}

const fn default_metrics_config() -> MetricsConfig {
    MetricsConfig::None
}

fn default_stark_compression_tools_path() -> String {
    std::env::current_dir()
        .unwrap_or(Path::new("./").into())
        .join("stark")
        .to_string_lossy()
        .to_string()
}

impl ProverNodeConfig {
    pub fn to_custom_toml(&self) -> String {
        let base_config = format!(
            r#"risc0_image_folder = "{}"
max_input_size_mb = {}
image_download_timeout_secs = {}
input_download_timeout_secs = {}
maximum_concurrent_proofs = {}
max_image_size_mb = {}
image_compression_ttl_hours = {}
stark_compression_tools_path = "{}"
env = "{}""#,
            self.risc0_image_folder,
            self.max_input_size_mb,
            self.image_download_timeout_secs,
            self.input_download_timeout_secs,
            self.maximum_concurrent_proofs,
            self.max_image_size_mb,
            self.image_compression_ttl_hours,
            self.stark_compression_tools_path,
            self.env.as_deref().unwrap_or("dev")
        );

        let ingester = match &self.ingester_config {
            IngesterConfig::RpcBlockSubscription { wss_rpc_url } => {
                format!(
                    r#"
[ingester_config]
RpcBlockSubscription = {{ wss_rpc_url = "{}" }}"#,
                    wss_rpc_url
                )
            }
            IngesterConfig::GrpcSubscription {
                grpc_url,
                connection_timeout_secs,
                timeout_secs,
                token,
            } => {
                format!(
                    r#"
[ingester_config]
GrpcSubscription = {{ grpc_url = "{}", connection_timeout_secs = {}, timeout_secs = {}, token = "{}" }}"#,
                    grpc_url, connection_timeout_secs, timeout_secs, token
                )
            }
            IngesterConfig::WebsocketSub => r#"
[ingester_config]
WebsocketSub"#
                .to_string(),
        };

        let tx_sender = match &self.transaction_sender_config {
            TransactionSenderConfig::Rpc { rpc_url } => {
                format!(
                    r#"
[transaction_sender_config]
Rpc = {{ rpc_url = "{}" }}"#,
                    rpc_url
                )
            }
            TransactionSenderConfig::Tpu => r#"
[transaction_sender_config]
Tpu"#
                .to_string(),
        };

        let signer = match &self.signer_config {
            SignerConfig::KeypairFile { path } => {
                format!(
                    r#"
[signer_config]
KeypairFile = {{ path = "{}" }}"#,
                    path
                )
            }
        };

        format!("{}{}{}{}", base_config, ingester, tx_sender, signer)
    }

    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        std::fs::write(path, self.to_custom_toml())
    }
}

fn default_bonsol_program() -> String {
    "BoNsHRcyLLNdtnoDf8hiCNZpyehMC4FDMxs6NTxFi3ew".to_string()
}

fn default_risc0_image_folder() -> String {
    "./elf".to_string()
}

const fn default_max_image_size_mb() -> u32 {
    10
}

const fn default_image_compression_ttl_hours() -> u32 {
    5
}

const fn default_max_input_size_mb() -> u32 {
    1
}

const fn default_image_download_timeout_secs() -> u32 {
    120
}

const fn default_input_download_timeout_secs() -> u32 {
    30
}

const fn default_maximum_concurrent_proofs() -> u32 {
    100
}

fn default_ingester_config() -> IngesterConfig {
    IngesterConfig::GrpcSubscription {
        grpc_url: "http://localhost:8899".to_string(),
        connection_timeout_secs: 10,
        timeout_secs: 30,
        token: "default-token".to_string(),
    }
}

fn default_transaction_sender_config() -> TransactionSenderConfig {
    TransactionSenderConfig::Rpc {
        rpc_url: "http://localhost:8899".to_string(),
    }
}

fn default_signer_config() -> SignerConfig {
    SignerConfig::KeypairFile {
        path: "./node-keypair.json".to_string(),
    }
}

impl Default for ProverNodeConfig {
    fn default() -> Self {
        ProverNodeConfig {
            env: Some("dev".to_string()),
            bonsol_program: default_bonsol_program(),
            risc0_image_folder: default_risc0_image_folder(),
            max_image_size_mb: default_max_image_size_mb(),
            image_compression_ttl_hours: default_image_compression_ttl_hours(),
            max_input_size_mb: default_max_input_size_mb(),
            image_download_timeout_secs: default_image_download_timeout_secs(),
            input_download_timeout_secs: default_input_download_timeout_secs(),
            maximum_concurrent_proofs: default_maximum_concurrent_proofs(),
            ingester_config: default_ingester_config(),
            transaction_sender_config: default_transaction_sender_config(),
            signer_config: default_signer_config(),
            stark_compression_tools_path: default_stark_compression_tools_path(),
            metrics_config: default_metrics_config(),
            missing_image_strategy: MissingImageStrategy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correct_serialization_grpc_subscription() -> anyhow::Result<()> {
        let config_content = r#"
risc0_image_folder = "/elf"
max_input_size_mb = 1
image_download_timeout_secs = 7
input_download_timeout_secs = 60
maximum_concurrent_proofs = 10
max_image_size_mb = 10
image_compression_ttl_hours = 24
stark_compression_tools_path = "/stark"
env = "dev"

[ingester_config]
GrpcSubscription = { grpc_url = "http://localhost:8899", connection_timeout_secs = 15, timeout_secs = 45, token = "default-token" }

[transaction_sender_config]
Rpc = { rpc_url = "http://localhost:8899" }

[signer_config]
KeypairFile = { path = "node_keypair.json" }"#;

        let content = Toml::string(config_content);
        let figment = Figment::new().merge(content);
        let config: ProverNodeConfig = figment.extract().unwrap();
        config.save_to_file("./src/config-test")?;

        let content = Toml::file("./src/config-test");
        let config: ProverNodeConfig = Figment::new().merge(content).extract().unwrap();

        assert!(config.max_image_size_mb == 10);

        if let TransactionSenderConfig::Rpc { rpc_url } = &config.transaction_sender_config {
            assert!(rpc_url == "http://localhost:8899");
        } else {
            panic!("Expected Rpc transaction sender config");
        }

        if let IngesterConfig::GrpcSubscription {
            grpc_url,
            connection_timeout_secs,
            timeout_secs,
            token,
        } = &config.ingester_config
        {
            assert_eq!(grpc_url, "http://localhost:8899");
            assert_eq!(*connection_timeout_secs, 15);
            assert_eq!(*timeout_secs, 45);
            assert_eq!(token, "default-token");
        } else {
            panic!("Expected GrpcSubscription ingester config");
        }

        Ok(())
    }

    #[test]
    fn test_rpc_block_subscription_serialization() -> anyhow::Result<()> {
        let config_content = r#"
risc0_image_folder = "/elf"
max_input_size_mb = 1
image_download_timeout_secs = 7
input_download_timeout_secs = 60
maximum_concurrent_proofs = 10
max_image_size_mb = 10
image_compression_ttl_hours = 24
stark_compression_tools_path = "/stark"
env = "dev"

[ingester_config]
RpcBlockSubscription = { wss_rpc_url = "ws://example.com:8900" }

[transaction_sender_config]
Rpc = { rpc_url = "http://localhost:8899" }

[signer_config]
KeypairFile = { path = "node_keypair.json" }"#;

        // Parse the TOML string into a config
        let content = Toml::string(config_content);
        let figment = Figment::new().merge(content);
        let config: ProverNodeConfig = figment.extract().unwrap();

        // Save the config to a file using to_custom_toml
        config.save_to_file("./src/rpc-config-test")?;

        // Read it back
        let content = Toml::file("./src/rpc-config-test");
        let loaded_config: ProverNodeConfig = Figment::new().merge(content).extract().unwrap();

        // Verify basic properties
        assert_eq!(loaded_config.max_image_size_mb, 10);

        // Check the transaction sender config
        if let TransactionSenderConfig::Rpc { rpc_url } = &loaded_config.transaction_sender_config {
            assert_eq!(rpc_url, "http://localhost:8899");
        } else {
            panic!("Expected Rpc transaction sender config");
        }

        // Verify the RpcBlockSubscription configuration
        if let IngesterConfig::RpcBlockSubscription { wss_rpc_url } = &loaded_config.ingester_config
        {
            assert_eq!(wss_rpc_url, "ws://example.com:8900");
        } else {
            panic!("Expected RpcBlockSubscription ingester config");
        }

        // Also test creating a configuration manually
        let manual_config = ProverNodeConfig {
            env: Some("dev".to_string()),
            bonsol_program: default_bonsol_program(),
            risc0_image_folder: "/elf".to_string(),
            max_image_size_mb: 10,
            image_compression_ttl_hours: 24,
            max_input_size_mb: 1,
            image_download_timeout_secs: 7,
            input_download_timeout_secs: 60,
            maximum_concurrent_proofs: 10,
            ingester_config: IngesterConfig::RpcBlockSubscription {
                wss_rpc_url: "ws://solana.com:8900".to_string(),
            },
            transaction_sender_config: TransactionSenderConfig::Rpc {
                rpc_url: "http://solana:8899".to_string(),
            },
            signer_config: SignerConfig::KeypairFile {
                path: "node_keypair.json".to_string(),
            },
            stark_compression_tools_path: "/stark".to_string(),
            metrics_config: default_metrics_config(),
            missing_image_strategy: MissingImageStrategy::default(),
        };

        let custom_toml = manual_config.to_custom_toml();
        assert!(custom_toml.contains("[ingester_config]"));
        assert!(custom_toml
            .contains("RpcBlockSubscription = { wss_rpc_url = \"ws://solana.com:8900\" }"));

        manual_config.save_to_file("./src/manual-rpc-config-test")?;
        let content = Toml::file("./src/manual-rpc-config-test");
        let loaded_manual: ProverNodeConfig = Figment::new().merge(content).extract().unwrap();

        if let IngesterConfig::RpcBlockSubscription { wss_rpc_url } = &loaded_manual.ingester_config
        {
            assert_eq!(wss_rpc_url, "ws://solana.com:8900");
        } else {
            panic!("Expected RpcBlockSubscription ingester config in manually created config");
        }

        std::fs::remove_file("./src/rpc-config-test")?;
        std::fs::remove_file("./src/manual-rpc-config-test")?;
        Ok(())
    }
}
