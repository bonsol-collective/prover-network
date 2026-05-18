use anyhow::anyhow;
use std::env::{self};

enum ProverNetworkState {
    Start,
    Stop,
    FullCycle,
}
impl ProverNetworkState {
    fn from_env() -> Result<Self, anyhow::Error> {
        let state = env::var("PROVER_NETWORK_STATE");
        match state?.as_str() {
            "start" => Ok(ProverNetworkState::Start),
            "stop" => Ok(ProverNetworkState::Stop),
            "fullcycle" => Ok(ProverNetworkState::FullCycle),
            _ => {
                return Err(anyhow!(
                    "wrong state given need 'start', 'stop', or 'fullcycle'"
                ));
            }
        }
    }
}
