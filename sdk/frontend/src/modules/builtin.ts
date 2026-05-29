import type { OracleClient } from '../client/OracleClient';
import type { PropertyRegistryClient } from '../client/PropertyRegistryClient';
import type { PropertyTokenClient } from '../client/PropertyTokenClient';
import type { ContractModuleLoader } from './types';

export type BuiltInContractModuleId = 'propertyRegistry' | 'propertyToken' | 'oracle';

export interface BuiltInContractClients {
  propertyRegistry: PropertyRegistryClient;
  propertyToken: PropertyTokenClient;
  oracle: OracleClient;
}

export const builtInContractModuleLoaders: Record<BuiltInContractModuleId, ContractModuleLoader> = {
  propertyRegistry: () => import('./propertyRegistry').then((m) => m.default),
  propertyToken: () => import('./propertyToken').then((m) => m.default),
  oracle: () => import('./oracle').then((m) => m.default),
};

