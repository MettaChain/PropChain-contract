import type { ApiPromise } from '@polkadot/api';

import type { ClientOptions } from '../types';

export interface CreateContractClientArgs {
  api: ApiPromise;
  address: string;
  options?: ClientOptions;
}

export interface ContractModule<TId extends string = string, TClient = unknown> {
  id: TId;
  createClient(args: CreateContractClientArgs): Promise<TClient>;
}

export type ContractModuleLoader = () => Promise<ContractModule>;
