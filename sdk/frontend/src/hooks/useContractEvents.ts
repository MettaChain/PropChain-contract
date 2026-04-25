import { useEffect, useRef } from 'react';
import { PropChainClient } from '../client/PropChainClient';
import type { Subscription } from '../types';
import type { PropChainEventName, PropChainEventMap } from '../types/events';

export function useContractEvents<E extends PropChainEventName>(
  client: PropChainClient | null,
  eventName: E,
  callback: (event: PropChainEventMap[E]) => void,
): void {
  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    if (!client) return;

    let subscription: Subscription | null = null;
    let cancelled = false;

    client.propertyRegistry
      .on(eventName, (event) => {
        callbackRef.current(event);
      })
      .then((sub) => {
        if (cancelled) {
          sub.unsubscribe();
        } else {
          subscription = sub;
        }
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
      if (subscription) {
        subscription.unsubscribe();
      }
    };
  }, [client, eventName]);
}
