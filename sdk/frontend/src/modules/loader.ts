import { builtInContractModuleLoaders } from './builtin';
import type { ContractModule, ContractModuleLoader } from './types';

type Registerable = ContractModule | ContractModuleLoader;

const registry = new Map<string, ContractModuleLoader>();

for (const [id, loader] of Object.entries(builtInContractModuleLoaders)) {
  registry.set(id, loader);
}

export function registerContractModule(id: string, moduleOrLoader: Registerable): void {
  if (!id || id.trim().length === 0) {
    throw new Error('Contract module id must be a non-empty string.');
  }
  const normalizedId = id.trim();
  const loader: ContractModuleLoader = typeof moduleOrLoader === 'function'
    ? moduleOrLoader
    : async () => moduleOrLoader;
  registry.set(normalizedId, loader);
}

export function listContractModules(): string[] {
  return [...registry.keys()].sort();
}

export async function loadContractModule<TClient = unknown>(id: string): Promise<ContractModule<string, TClient>> {
  const loader = registry.get(id);
  if (!loader) {
    const available = listContractModules();
    throw new Error(
      `Unknown contract module "${id}". Available modules: ${available.length ? available.join(', ') : '(none)'}`,
    );
  }

  const module = await loader();
  if (!module || typeof module !== 'object') {
    throw new Error(`Contract module loader for "${id}" did not return a module.`);
  }
  if (module.id !== id) {
    throw new Error(`Contract module id mismatch. Expected "${id}", got "${module.id}".`);
  }
  return module as ContractModule<string, TClient>;
}

