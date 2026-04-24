import { Abi } from '@polkadot/api-contract';

import type { OracleClient } from '../client/OracleClient';
import type { ContractModule, CreateContractClientArgs } from './types';

export const id = 'oracle' as const;

const module: ContractModule<typeof id, OracleClient> = {
  id,
  async createClient({ api, address, options }: CreateContractClientArgs) {
    const [{ OracleClient }, abiJsonModule] = await Promise.all([
      import('../client/OracleClient'),
      import('../abi/property_registry.json'),
    ]);
    const abiJson = (abiJsonModule as unknown as { default?: unknown }).default ?? abiJsonModule;
    const abi = new Abi(abiJson as unknown as Record<string, unknown>);
    return new OracleClient(api, address, abi, options);
  },
};

export default module;

