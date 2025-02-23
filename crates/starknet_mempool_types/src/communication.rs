use std::sync::Arc;

use async_trait::async_trait;
#[cfg(any(feature = "testing", test))]
use mockall::automock;
use papyrus_network_types::network_types::BroadcastedMessageMetadata;
use papyrus_proc_macros::handle_response_variants;
use serde::{Deserialize, Serialize};
use starknet_api::executable_transaction::AccountTransaction;
use starknet_sequencer_infra::component_client::{
    ClientError,
    LocalComponentClient,
    RemoteComponentClient,
};
use starknet_sequencer_infra::component_definitions::{
    ComponentClient,
    ComponentRequestAndResponseSender,
};
use thiserror::Error;

use crate::errors::MempoolError;
use crate::mempool_types::{AddTransactionArgs, CommitBlockArgs};

pub type LocalMempoolClient = LocalComponentClient<MempoolRequest, MempoolResponse>;
pub type RemoteMempoolClient = RemoteComponentClient<MempoolRequest, MempoolResponse>;
pub type MempoolResult<T> = Result<T, MempoolError>;
pub type MempoolClientResult<T> = Result<T, MempoolClientError>;
pub type MempoolRequestAndResponseSender =
    ComponentRequestAndResponseSender<MempoolRequest, MempoolResponse>;
pub type SharedMempoolClient = Arc<dyn MempoolClient>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AddTransactionArgsWrapper {
    pub args: AddTransactionArgs,
    pub p2p_message_metadata: Option<BroadcastedMessageMetadata>,
}

/// Serves as the mempool's shared interface. Requires `Send + Sync` to allow transferring and
/// sharing resources (inputs, futures) across threads.
#[cfg_attr(any(feature = "testing", test), automock)]
#[async_trait]
pub trait MempoolClient: Send + Sync {
    // TODO: Add Option<BroadcastedMessageMetadata> as an argument for add_transaction
    // TODO: Rename tx to transaction
    async fn add_tx(&self, args: AddTransactionArgsWrapper) -> MempoolClientResult<()>;
    async fn commit_block(&self, args: CommitBlockArgs) -> MempoolClientResult<()>;
    async fn get_txs(&self, n_txs: usize) -> MempoolClientResult<Vec<AccountTransaction>>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MempoolRequest {
    AddTransaction(AddTransactionArgsWrapper),
    CommitBlock(CommitBlockArgs),
    GetTransactions(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum MempoolResponse {
    AddTransaction(MempoolResult<()>),
    CommitBlock(MempoolResult<()>),
    GetTransactions(MempoolResult<Vec<AccountTransaction>>),
}

#[derive(Clone, Debug, Error)]
pub enum MempoolClientError {
    #[error(transparent)]
    ClientError(#[from] ClientError),
    #[error(transparent)]
    MempoolError(#[from] MempoolError),
}

#[async_trait]
impl<ComponentClientType> MempoolClient for ComponentClientType
where
    ComponentClientType: Send + Sync + ComponentClient<MempoolRequest, MempoolResponse>,
{
    async fn add_tx(&self, args: AddTransactionArgsWrapper) -> MempoolClientResult<()> {
        let request = MempoolRequest::AddTransaction(args);
        let response = self.send(request).await;
        handle_response_variants!(MempoolResponse, AddTransaction, MempoolClientError, MempoolError)
    }

    async fn commit_block(&self, args: CommitBlockArgs) -> MempoolClientResult<()> {
        let request = MempoolRequest::CommitBlock(args);
        let response = self.send(request).await;
        handle_response_variants!(MempoolResponse, CommitBlock, MempoolClientError, MempoolError)
    }

    async fn get_txs(&self, n_txs: usize) -> MempoolClientResult<Vec<AccountTransaction>> {
        let request = MempoolRequest::GetTransactions(n_txs);
        let response = self.send(request).await;
        handle_response_variants!(
            MempoolResponse,
            GetTransactions,
            MempoolClientError,
            MempoolError
        )
    }
}
