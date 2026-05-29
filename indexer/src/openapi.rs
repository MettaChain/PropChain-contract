use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "PropChain Indexer API",
        version = "0.1.0",
        description = "Query API for indexed PropChain smart contract events on Substrate/Polkadot."
    ),
    paths(
        crate::api::health,
        crate::api::list_events,
        crate::api::list_contracts,
    ),
    components(
        schemas(crate::db::IndexedEvent, crate::api::EventsParams)
    ),
    tags(
        (name = "System", description = "Service health"),
        (name = "Events", description = "Contract event queries")
    )
)]
pub struct ApiDoc;
