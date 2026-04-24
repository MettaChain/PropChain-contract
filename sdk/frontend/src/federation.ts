/**
 * Contract module federation entrypoint.
 *
 * Exposes the dynamic loader/registry and the federated client so consumers
 * can import only federation-related utilities when desired.
 */

export { FederatedPropChainClient } from './client/FederatedPropChainClient';

export type { ContractModule, CreateContractClientArgs } from './modules/types';
export type { BuiltInContractModuleId } from './modules/builtin';
export { loadContractModule, registerContractModule, listContractModules } from './modules/loader';

