//! On-chain settlement via ethers

use ethers::{
    prelude::*,
    types::{Address as EthAddress, Bytes, H256, U256},
};
use std::sync::Arc;
use zk402_core::*;

abigen!(
    ZkSettleVerifier,
    r#"[
        struct PublicInputs {
            uint256 amount;
            address asset;
            address payTo;
            address from;
            string network;
            bytes32 nonce;
            uint64 validAfter;
            uint64 validBefore;
        }
        function settle(bytes calldata proof, PublicInputs calldata inputs) external
    ]"#
);

/// Configuration for on-chain settlement
#[derive(Clone)]
pub struct SettlementConfig {
    pub rpc_url: String,
    pub verifier_address: [u8; 20],
    pub private_key: Option<String>,
}

impl SettlementConfig {
    pub fn new(rpc_url: String, verifier_address: [u8; 20]) -> Self {
        Self {
            rpc_url,
            verifier_address,
            private_key: None,
        }
    }

    pub fn with_private_key(mut self, key: String) -> Self {
        self.private_key = Some(key);
        self
    }
}

/// Submit a settlement transaction to the on-chain verifier contract
pub async fn submit_settlement(
    config: &SettlementConfig,
    payload: &PaymentPayload,
) -> Result<String, SchemeError> {
    let provider = Provider::<Http>::try_from(&config.rpc_url)
        .map_err(|e| SchemeError::ChainError(format!("invalid RPC URL: {}", e)))?;

    let client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>> = if let Some(pk) = &config.private_key {
        let wallet: LocalWallet = pk
            .parse()
            .map_err(|e| SchemeError::ChainError(format!("invalid private key: {}", e)))?;
        let chain_id = provider
            .get_chainid()
            .await
            .map_err(|e| SchemeError::ChainError(format!("failed to get chain ID: {}", e)))?;
        Arc::new(SignerMiddleware::new(provider, wallet.with_chain_id(chain_id.as_u64())))
    } else {
        return Err(SchemeError::Internal("no signer configured".to_string()));
    };

    let verifier_address = EthAddress::from_slice(&config.verifier_address);
    let contract = ZkSettleVerifier::new(verifier_address, client);

    // Convert PublicInputs to Solidity format
    let inputs = convert_public_inputs(&payload.public_inputs)?;
    let proof = Bytes::from(payload.proof.clone());

    // Submit transaction
    let tx = contract
        .settle(proof, inputs)
        .send()
        .await
        .map_err(|e| SchemeError::ChainError(format!("transaction failed: {}", e)))?;

    let receipt = tx
        .await
        .map_err(|e| SchemeError::ChainError(format!("failed to get receipt: {}", e)))?
        .ok_or_else(|| SchemeError::ChainError("transaction receipt not found".to_string()))?;

    Ok(format!("0x{}", hex::encode(receipt.transaction_hash.as_bytes())))
}

fn convert_public_inputs(
    inputs: &PublicInputs,
) -> Result<self::PublicInputs, SchemeError> {
    let amount = inputs
        .amount
        .parse::<U256>()
        .map_err(|e| SchemeError::Internal(format!("invalid amount: {}", e)))?;

    Ok(self::PublicInputs {
        amount,
        asset: EthAddress::from_slice(&inputs.asset),
        pay_to: EthAddress::from_slice(&inputs.pay_to),
        from: EthAddress::from_slice(&inputs.from),
        network: inputs.network.clone(),
        nonce: H256::from_slice(&inputs.nonce),
        valid_after: inputs.valid_after,
        valid_before: inputs.valid_before,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_public_inputs_correctly() {
        let inputs = PublicInputs {
            amount: "100".to_string(),
            asset: [1u8; 20],
            pay_to: [2u8; 20],
            from: [3u8; 20],
            network: "eip155:31337".to_string(),
            nonce: [4u8; 32],
            valid_after: 1000,
            valid_before: 2000,
            public_key: vec![5u8; 32],
        };

        let result = convert_public_inputs(&inputs).unwrap();
        assert_eq!(result.amount, U256::from(100));
        assert_eq!(result.network, "eip155:31337");
        assert_eq!(result.valid_after, 1000);
        assert_eq!(result.valid_before, 2000);
    }
}
