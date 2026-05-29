/**
 * @propchain/sdk — Event Utilities
 *
 * Helpers for decoding, subscribing to, and filtering contract events
 * from PropChain smart contracts.
 *
 * @module utils/events
 */

import type { ApiPromise } from '@polkadot/api';
import type { Abi } from '@polkadot/api-contract';

import type { ContractEvent, Subscription } from '../types';
import type { PropChainEventName, PropChainEventMap } from '../types/events';

// ============================================================================
// Event Decoding
// ============================================================================

/**
 * Decodes raw contract event data using the contract ABI.
 *
 * @param abi - The contract ABI
 * @param eventData - Raw event data bytes
 * @returns A decoded `ContractEvent` or `null` if decoding fails
 *
 * @example
 * ```typescript
 * const decoded = decodeEvent(abi, rawEventData);
 * if (decoded) {
 *   console.log('Event:', decoded.name, decoded.args);
 * }
 * ```
 */
export function decodeEvent(
  abi: Abi,
  eventData: Uint8Array,
): ContractEvent | null {
  try {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const decodedEvent = abi.decodeEvent(eventData as any);
    const args: Record<string, unknown> = {};

    decodedEvent.event.args.forEach((arg, index) => {
      const argName = decodedEvent.event.args[index].name;
      args[argName] = decodedEvent.args[index].toJSON();
    });

    return {
      name: decodedEvent.event.identifier,
      args,
    };
  } catch {
    return null;
  }
}

/**
 * Decodes contract events from a transaction result.
 *
 * @param abi - The contract ABI
 * @param events - Array of system events from a transaction
 * @param contractAddress - The contract address to filter events for
 * @returns Array of decoded `ContractEvent` objects
 *
 * @example
 * ```typescript
 * const events = decodeTransactionEvents(abi, systemEvents, contractAddress);
 * events.forEach(e => console.log(`${e.name}:`, e.args));
 * ```
 */
export function decodeTransactionEvents(
  abi: Abi,
  events: Array<{ event: { data: Uint8Array; section: string; method: string } }>,
  contractAddress: string,
): ContractEvent[] {
  const decoded: ContractEvent[] = [];

  for (const record of events) {
    const { event } = record;

    // Only process contract events
    if (event.section === 'contracts' && event.method === 'ContractEmitted') {
      const [address, data] = event.data as unknown as [
        { toString: () => string },
        Uint8Array,
      ];

      if (address.toString() === contractAddress) {
        const contractEvent = decodeEvent(abi, data);
        if (contractEvent) {
          decoded.push(contractEvent);
        }
      }
    }
  }

  return decoded;
}

// ============================================================================
// Event Subscription
// ============================================================================

/**
 * Subscribes to contract events from a specific contract address.
 *
 * @param api - Connected `ApiPromise` instance
 * @param contractAddress - The contract address to listen to events from
 * @param abi - The contract ABI for decoding events
 * @param callback - Function called for each decoded event
 * @returns A `Subscription` that can be unsubscribed
 *
 * @example
 * ```typescript
 * const sub = await subscribeToEvents(api, contractAddr, abi, (event) => {
 *   console.log('New event:', event.name, event.args);
 * });
 *
 * // Later...
 * sub.unsubscribe();
 * ```
 */
export async function subscribeToEvents(
  api: ApiPromise,
  contractAddress: string,
  abi: Abi,
  callback: (event: ContractEvent) => void,
): Promise<Subscription> {
  const unsub = await (api.query.system.events as Function)((events: unknown[]) => {
    const typedEvents = events as Array<{
      event: { data: Uint8Array; section: string; method: string };
    }>;

    const decoded = decodeTransactionEvents(abi, typedEvents, contractAddress);
    decoded.forEach(callback);
  });

  return {
    unsubscribe: () => {
      if (typeof unsub === 'function') {
        unsub();
      }
    },
  };
}

/**
 * Subscribes to a specific named event from a contract.
 *
 * @typeParam E - The event name from `PropChainEventName`
 * @param api - Connected `ApiPromise` instance
 * @param contractAddress - The contract address
 * @param abi - The contract ABI
 * @param eventName - The specific event name to listen for
 * @param callback - Called with the typed event payload
 * @returns A `Subscription`
 *
 * @example
 * ```typescript
 * const sub = await subscribeToNamedEvent(
 *   api, contractAddr, abi,
 *   'PropertyRegistered',
 *   (event) => {
 *     console.log('Property registered:', event.propertyId);
 *   }
 * );
 * ```
 */
export async function subscribeToNamedEvent<E extends PropChainEventName>(
  api: ApiPromise,
  contractAddress: string,
  abi: Abi,
  eventName: E,
  callback: (event: PropChainEventMap[E]) => void,
): Promise<Subscription> {
  return subscribeToEvents(api, contractAddress, abi, (event) => {
    if (event.name === eventName) {
      callback(event.args as unknown as PropChainEventMap[E]);
    }
  });
}

// ============================================================================
// Event Filtering
// ============================================================================

/**
 * Filters an array of contract events by name.
 *
 * @param events - Array of contract events
 * @param eventName - The event name to filter by
 * @returns Filtered array of matching events
 *
 * @example
 * ```typescript
 * const transfers = filterEvents(allEvents, 'PropertyTransferred');
 * console.log(`${transfers.length} transfers found`);
 * ```
 */
export function filterEvents(
  events: ContractEvent[],
  eventName: PropChainEventName,
): ContractEvent[] {
  return events.filter((e) => e.name === eventName);
}

/**
 * Extracts events of a specific type from transaction events,
 * returning typed payloads.
 *
 * @typeParam E - The event name from `PropChainEventName`
 * @param events - Array of contract events
 * @param eventName - The event name to extract
 * @returns Typed array of event payloads
 *
 * @example
 * ```typescript
 * const registered = extractTypedEvents(txEvents, 'PropertyRegistered');
 * registered.forEach(e => console.log(e.propertyId, e.owner));
 * ```
 */
export function extractTypedEvents<E extends PropChainEventName>(
  events: ContractEvent[],
  eventName: E,
): PropChainEventMap[E][] {
  return filterEvents(events, eventName).map(
    (e) => e.args as unknown as PropChainEventMap[E],
  );
}
