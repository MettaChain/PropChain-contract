import { Abi } from '@polkadot/api-contract';

import type { PropertyRegistryClient } from '../client/PropertyRegistryClient';
import type { ContractModule, CreateContractClientArgs } from './types';

export const id = 'propertyRegistry' as const;

const module: ContractModule<typeof id, PropertyRegistryClient> = {
  id,
  async createClient({ api, address, options }: CreateContractClientArgs) {
    const [{ PropertyRegistryClient }, abiJsonModule] = await Promise.all([
      import('../client/PropertyRegistryClient'),
      import('../abi/property_registry.json'),
    ]);
    const abiJson = (abiJsonModule as unknown as { default?: unknown }).default ?? abiJsonModule;
    const abi = new Abi(abiJson as unknown as Record<string, unknown>);
    return new PropertyRegistryClient(api, address, abi, options);
  },
};

export default module;

