use crate::{
    database::OnChainIterableKeyValueView,
    service::{
        adapters::{
            BlockImporterAdapter,
            ConsensusParametersProvider,
            P2PAdapter,
            SharedMemoryPool,
            StaticGasPrice,
        },
        vm_pool::MemoryFromPool,
    },
};
use fuel_core_services::stream::BoxStream;
use fuel_core_storage::{
    tables::{
        Coins,
        ContractsRawCode,
        Messages,
    },
    Result as StorageResult,
    StorageAsRef,
};
use fuel_core_txpool::{
    ports::{
        BlockImporter,
        ConsensusParametersProvider as ConsensusParametersProviderTrait,
        GasPriceProvider,
        MemoryPool,
    },
    Result as TxPoolResult,
};
use fuel_core_types::{
    blockchain::header::ConsensusParametersVersion,
    entities::{
        coins::coin::CompressedCoin,
        relayer::message::Message,
    },
    fuel_tx::{
        BlobId,
        ConsensusParameters,
        Transaction,
        UtxoId,
    },
    fuel_types::{
        ContractId,
        Nonce,
    },
    fuel_vm::BlobData,
    services::{
        block_importer::SharedImportResult,
        p2p::{
            GossipsubMessageAcceptance,
            GossipsubMessageInfo,
            TransactionGossipData,
        },
    },
};
use std::sync::Arc;

impl BlockImporter for BlockImporterAdapter {
    fn block_events(&self) -> BoxStream<SharedImportResult> {
        self.events_shared_result()
    }
}

#[cfg(feature = "p2p")]
#[async_trait::async_trait]
impl fuel_core_txpool::ports::PeerToPeer for P2PAdapter {
    type GossipedTransaction = TransactionGossipData;

    fn broadcast_transaction(&self, transaction: Arc<Transaction>) -> anyhow::Result<()> {
        if let Some(service) = &self.service {
            service.broadcast_transaction(transaction)
        } else {
            Ok(())
        }
    }

    fn gossiped_transaction_events(&self) -> BoxStream<Self::GossipedTransaction> {
        use tokio_stream::{
            wrappers::BroadcastStream,
            StreamExt,
        };
        if let Some(service) = &self.service {
            Box::pin(
                BroadcastStream::new(service.subscribe_tx())
                    .filter_map(|result| result.ok()),
            )
        } else {
            fuel_core_services::stream::IntoBoxStream::into_boxed(tokio_stream::pending())
        }
    }

    fn new_tx_subscription(&self) -> BoxStream<Vec<u8>> {
        use tokio_stream::{
            wrappers::BroadcastStream,
            StreamExt,
        };
        if let Some(service) = &self.service {
            Box::pin(
                BroadcastStream::new(service.subscribe_new_tx_subscription())
                    .filter_map(|result| result.ok()),
            )
        } else {
            fuel_core_services::stream::IntoBoxStream::into_boxed(tokio_stream::pending())
        }
    }

    fn notify_gossip_transaction_validity(
        &self,
        message_info: GossipsubMessageInfo,
        validity: GossipsubMessageAcceptance,
    ) -> anyhow::Result<()> {
        if let Some(service) = &self.service {
            service.notify_gossip_transaction_validity(message_info, validity)
        } else {
            Ok(())
        }
    }

    async fn request_tx_ids(
        &self,
        peer_id: Vec<u8>,
    ) -> anyhow::Result<Vec<fuel_core_txpool::types::TxId>> {
        if let Some(service) = &self.service {
            match service.get_all_transactions_ids_from_peer(peer_id).await {
                Ok(txs) => Ok(txs.unwrap_or_default()),
                Err(e) => {
                    tracing::error!("Error getting tx ids from peer: {:?}", e);
                    Ok(vec![])
                }
            }
        } else {
            Ok(vec![])
        }
    }

    async fn request_txs(
        &self,
        peer_id: Vec<u8>,
        tx_ids: Vec<fuel_core_txpool::types::TxId>,
    ) -> anyhow::Result<Vec<Option<Transaction>>> {
        if let Some(service) = &self.service {
            match service
                .get_full_transactions_from_peer(peer_id, tx_ids)
                .await
            {
                Ok(txs) => Ok(txs.unwrap_or_default()),
                Err(e) => {
                    tracing::error!("Error getting tx ids from peer: {:?}", e);
                    Ok(vec![])
                }
            }
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(not(feature = "p2p"))]
#[async_trait::async_trait]
impl fuel_core_txpool::ports::PeerToPeer for P2PAdapter {
    type GossipedTransaction = TransactionGossipData;

    fn broadcast_transaction(
        &self,
        _transaction: Arc<Transaction>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn gossiped_transaction_events(&self) -> BoxStream<Self::GossipedTransaction> {
        Box::pin(fuel_core_services::stream::pending())
    }

    fn notify_gossip_transaction_validity(
        &self,
        _message_info: GossipsubMessageInfo,
        _validity: GossipsubMessageAcceptance,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn new_tx_subscription(&self) -> BoxStream<Vec<u8>> {
        Box::pin(fuel_core_services::stream::pending())
    }

    async fn request_tx_ids(
        &self,
        _peer_id: Vec<u8>,
    ) -> anyhow::Result<Vec<fuel_core_txpool::types::TxId>> {
        Ok(vec![])
    }

    async fn request_txs(
        &self,
        _peer_id: Vec<u8>,
        _tx_ids: Vec<fuel_core_txpool::types::TxId>,
    ) -> anyhow::Result<Vec<Option<Transaction>>> {
        Ok(vec![])
    }

}

impl fuel_core_txpool::ports::TxPoolDb for OnChainIterableKeyValueView {
    fn utxo(&self, utxo_id: &UtxoId) -> StorageResult<Option<CompressedCoin>> {
        self.storage::<Coins>()
            .get(utxo_id)
            .map(|t| t.map(|t| t.as_ref().clone()))
    }

    fn contract_exist(&self, contract_id: &ContractId) -> StorageResult<bool> {
        self.storage::<ContractsRawCode>().contains_key(contract_id)
    }

    fn blob_exist(&self, blob_id: &BlobId) -> StorageResult<bool> {
        self.storage::<BlobData>().contains_key(blob_id)
    }

    fn message(&self, id: &Nonce) -> StorageResult<Option<Message>> {
        self.storage::<Messages>()
            .get(id)
            .map(|t| t.map(|t| t.as_ref().clone()))
    }
}

#[async_trait::async_trait]
impl GasPriceProvider for StaticGasPrice {
    async fn next_gas_price(&self) -> TxPoolResult<u64> {
        Ok(self.gas_price)
    }
}

impl ConsensusParametersProviderTrait for ConsensusParametersProvider {
    fn latest_consensus_parameters(
        &self,
    ) -> (ConsensusParametersVersion, Arc<ConsensusParameters>) {
        self.shared_state.latest_consensus_parameters_with_version()
    }
}

#[async_trait::async_trait]
impl MemoryPool for SharedMemoryPool {
    type Memory = MemoryFromPool;

    async fn get_memory(&self) -> Self::Memory {
        self.memory_pool.take_raw().await
    }
}
