# upgradeable-proxy

Delegating proxy used by `upgradeable-factory` (part of #1445): state lives in the proxy, unknown messages are
`delegate_call`-ed to the current implementation hash, and only the deploying factory can change it.
The forwarding and initializer paths need a real node; unit tests cover storage and authorisation.
