import { Abi } from '@polkadot/api-contract';

import type { PropertyTokenClient } from '../client/PropertyTokenClient';
import type { ContractModule, CreateContractClientArgs } from './types';

export const id = 'propertyToken' as const;

const module: ContractModule<typeof id, PropertyTokenClient> = {
  id,
  async createClient({ api, address, options }: CreateContractClientArgs) {
    const [{ PropertyTokenClient }, abiJsonModule] = await Promise.all([
      import('../client/PropertyTokenClient'),
      import('../abi/property_token.json'),
    ]);
    const abiJson = (abiJsonModule as unknown as { default?: unknown }).default ?? abiJsonModule;
    const abi = new Abi(abiJson as unknown as Record<string, unknown>);
    return new PropertyTokenClient(api, address, abi, options);
  },
};

export default module;

