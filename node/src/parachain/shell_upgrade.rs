//! Special [`ParachainConsensus`] implementation that waits for the upgrade from shell to a
//! parachain runtime that implements Aura.
use peaq_primitives_xcm::*;
use sc_consensus::{import_queue::Verifier as VerifierT, BlockImportParams};
use sp_api::ApiExt;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_runtime::traits::Header as HeaderT;
use std::sync::Arc;

pub struct Verifier<Client> {
	pub client: Arc<Client>,
	pub aura_verifier: Box<dyn VerifierT<Block>>,
	pub relay_chain_verifier: Box<dyn VerifierT<Block>>,
}

#[async_trait::async_trait]
impl<Client> VerifierT<Block> for Verifier<Client>
where
	Client: sp_api::ProvideRuntimeApi<Block> + Send + Sync,
	Client::Api: AuraApi<Block, AuraId>,
{
	async fn verify(
		&self,
		block_import: BlockImportParams<Block>,
	) -> Result<BlockImportParams<Block>, String> {
		let block_hash = *block_import.header.parent_hash();

		if self
			.client
			.runtime_api()
			.has_api::<dyn AuraApi<Block, AuraId>>(block_hash)
			.unwrap_or(false)
		{
			self.aura_verifier.verify(block_import).await
		} else {
			self.relay_chain_verifier.verify(block_import).await
		}
	}
}
