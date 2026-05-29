import React, { useState } from 'react';

interface EscrowManagerProps {
  account: string;
}

/**
 * Escrow Manager component demonstrating:
 * - Creating escrows for property transfers
 * - Releasing/refunding escrows
 * - Querying escrow status
 *
 * @example SDK usage:
 * ```typescript
 * // Create escrow
 * const { escrowId } = await client.escrow.create(
 *   signer, propertyId, buyerAddr, sellerAddr, amount,
 * );
 *
 * // Release escrow
 * await client.escrow.release(sellerSigner, escrowId);
 * ```
 */
export function EscrowManager({ account }: EscrowManagerProps) {
  const [propertyId, setPropertyId] = useState('');
  const [buyerAddress, setBuyerAddress] = useState('');
  const [amount, setAmount] = useState('');
  const [escrowId, setEscrowId] = useState('');
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setMessage(null);

    try {
      // In production:
      // const { escrowId } = await client.escrow.create(
      //   signer, parseInt(propertyId), buyerAddress, account, parseBalance(amount, 12),
      // );
      await new Promise((resolve) => setTimeout(resolve, 1500));
      const newId = Math.floor(Math.random() * 100);
      setMessage({
        type: 'success',
        text: `Escrow created! ID: ${newId}`,
      });
    } catch (err) {
      setMessage({ type: 'error', text: `Failed: ${err}` });
    } finally {
      setLoading(false);
    }
  };

  const handleRelease = async () => {
    if (!escrowId) return;
    setLoading(true);
    try {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      setMessage({ type: 'success', text: `Escrow #${escrowId} released successfully!` });
    } catch (err) {
      setMessage({ type: 'error', text: `Release failed: ${err}` });
    } finally {
      setLoading(false);
    }
  };

  const handleRefund = async () => {
    if (!escrowId) return;
    setLoading(true);
    try {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      setMessage({ type: 'success', text: `Escrow #${escrowId} refunded successfully!` });
    } catch (err) {
      setMessage({ type: 'error', text: `Refund failed: ${err}` });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="panel">
      <div className="panel-header">
        <h2>🔐 Escrow Manager</h2>
        <p className="subtitle">Secure property transfers with on-chain escrow</p>
      </div>

      <div className="two-column">
        <div className="card">
          <h3>Create Escrow</h3>
          <form onSubmit={handleCreate} className="form">
            <div className="form-group">
              <label htmlFor="esc-property">Property ID</label>
              <input
                id="esc-property"
                type="number"
                value={propertyId}
                onChange={(e) => setPropertyId(e.target.value)}
                placeholder="Property ID"
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="esc-buyer">Buyer Address</label>
              <input
                id="esc-buyer"
                value={buyerAddress}
                onChange={(e) => setBuyerAddress(e.target.value)}
                placeholder="5FHneW46..."
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="esc-amount">Amount</label>
              <input
                id="esc-amount"
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="Amount in tokens"
                required
              />
            </div>
            <button type="submit" className="btn btn-primary btn-full" disabled={loading}>
              {loading ? '⏳ Creating...' : '🔒 Create Escrow'}
            </button>
          </form>
        </div>

        <div className="card">
          <h3>Manage Escrow</h3>
          <div className="form">
            <div className="form-group">
              <label htmlFor="esc-manage-id">Escrow ID</label>
              <input
                id="esc-manage-id"
                type="number"
                value={escrowId}
                onChange={(e) => setEscrowId(e.target.value)}
                placeholder="Enter escrow ID"
              />
            </div>
            <div className="btn-row">
              <button
                className="btn btn-success"
                onClick={handleRelease}
                disabled={loading || !escrowId}
              >
                ✅ Release
              </button>
              <button
                className="btn btn-danger"
                onClick={handleRefund}
                disabled={loading || !escrowId}
              >
                ↩️ Refund
              </button>
            </div>
          </div>

          <div className="divider" />

          <h3>SDK Code Example</h3>
          <pre className="code-block">
{`// Create escrow
const { escrowId } = await client
  .escrow.create(
    signer,
    propertyId,
    buyerAddress,
    sellerAddress,
    BigInt('500000000000000')
  );

// Release after conditions met
await client.escrow.release(
  sellerSigner, escrowId
);

// Or refund if deal falls through
await client.escrow.refund(
  buyerSigner, escrowId
);`}
          </pre>
        </div>
      </div>

      {message && (
        <div className={`message message-${message.type}`}>
          {message.type === 'success' ? '✅' : '❌'} {message.text}
        </div>
      )}
    </div>
  );
}
