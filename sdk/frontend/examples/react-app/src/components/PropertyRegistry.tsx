import React, { useState } from 'react';

interface PropertyRegistryProps {
  account: string;
}

interface PropertyForm {
  location: string;
  size: string;
  legalDescription: string;
  valuation: string;
  documentsUrl: string;
}

/**
 * Property Registry component demonstrating:
 * - Property registration with metadata
 * - Property querying
 * - Property transfer
 * - Metadata updates
 *
 * @example SDK usage:
 * ```typescript
 * import { PropChainClient } from '@propchain/sdk';
 *
 * const client = await PropChainClient.create('ws://localhost:9944', {
 *   propertyRegistry: contractAddress,
 * });
 *
 * // Register a property
 * const { propertyId } = await client.propertyRegistry.registerProperty(signer, {
 *   location: '123 Main St',
 *   size: 2000,
 *   legalDescription: 'Lot 1 Block 2',
 *   valuation: BigInt(500000_00000000),
 *   documentsUrl: 'ipfs://Qm...',
 * });
 * ```
 */
export function PropertyRegistry({ account }: PropertyRegistryProps) {
  const [form, setForm] = useState<PropertyForm>({
    location: '',
    size: '',
    legalDescription: '',
    valuation: '',
    documentsUrl: '',
  });
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState<{ type: 'success' | 'error'; text: string } | null>(null);
  const [queryId, setQueryId] = useState('');

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    const { name, value } = e.target;
    setForm((prev) => ({ ...prev, [name]: value }));
  };

  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setMessage(null);

    try {
      // In production:
      // const { propertyId } = await client.propertyRegistry.registerProperty(signer, {
      //   location: form.location,
      //   size: parseInt(form.size),
      //   legalDescription: form.legalDescription,
      //   valuation: parseBalance(form.valuation, 8),
      //   documentsUrl: form.documentsUrl,
      // });

      // Simulate success
      await new Promise((resolve) => setTimeout(resolve, 1500));
      setMessage({
        type: 'success',
        text: `Property registered successfully! Property ID: ${Math.floor(Math.random() * 1000)}`,
      });
      setForm({ location: '', size: '', legalDescription: '', valuation: '', documentsUrl: '' });
    } catch (err) {
      setMessage({ type: 'error', text: `Registration failed: ${err}` });
    } finally {
      setLoading(false);
    }
  };

  const handleQuery = async () => {
    if (!queryId) return;
    setLoading(true);
    setMessage(null);

    try {
      // In production:
      // const property = await client.propertyRegistry.getProperty(parseInt(queryId));
      await new Promise((resolve) => setTimeout(resolve, 800));
      setMessage({
        type: 'success',
        text: `Property #${queryId}: 123 Example St, 2500 sqm, Valuation: $500,000`,
      });
    } catch (err) {
      setMessage({ type: 'error', text: `Query failed: ${err}` });
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="panel">
      <div className="panel-header">
        <h2>🏠 Property Registry</h2>
        <p className="subtitle">Register and manage on-chain properties</p>
      </div>

      <div className="two-column">
        {/* Registration Form */}
        <div className="card">
          <h3>Register New Property</h3>
          <form onSubmit={handleRegister} className="form">
            <div className="form-group">
              <label htmlFor="reg-location">Location</label>
              <input
                id="reg-location"
                name="location"
                value={form.location}
                onChange={handleInputChange}
                placeholder="123 Main St, City, State"
                required
              />
            </div>
            <div className="form-row">
              <div className="form-group">
                <label htmlFor="reg-size">Size (sqm)</label>
                <input
                  id="reg-size"
                  name="size"
                  type="number"
                  value={form.size}
                  onChange={handleInputChange}
                  placeholder="2500"
                  required
                />
              </div>
              <div className="form-group">
                <label htmlFor="reg-valuation">Valuation (USD)</label>
                <input
                  id="reg-valuation"
                  name="valuation"
                  type="number"
                  value={form.valuation}
                  onChange={handleInputChange}
                  placeholder="500000"
                  required
                />
              </div>
            </div>
            <div className="form-group">
              <label htmlFor="reg-legal">Legal Description</label>
              <textarea
                id="reg-legal"
                name="legalDescription"
                value={form.legalDescription}
                onChange={handleInputChange}
                placeholder="Lot 1, Block 2, City Subdivision"
                rows={2}
                required
              />
            </div>
            <div className="form-group">
              <label htmlFor="reg-docs">Documents URL</label>
              <input
                id="reg-docs"
                name="documentsUrl"
                value={form.documentsUrl}
                onChange={handleInputChange}
                placeholder="ipfs://Qm..."
              />
            </div>
            <button type="submit" className="btn btn-primary btn-full" disabled={loading}>
              {loading ? '⏳ Registering...' : '📝 Register Property'}
            </button>
          </form>
        </div>

        {/* Query Panel */}
        <div className="card">
          <h3>Query Property</h3>
          <div className="form">
            <div className="form-group">
              <label htmlFor="query-id">Property ID</label>
              <div className="input-row">
                <input
                  id="query-id"
                  value={queryId}
                  onChange={(e) => setQueryId(e.target.value)}
                  placeholder="Enter property ID"
                  type="number"
                />
                <button
                  className="btn btn-secondary"
                  onClick={handleQuery}
                  disabled={loading || !queryId}
                >
                  🔍 Query
                </button>
              </div>
            </div>
          </div>

          <div className="divider" />

          <h3>SDK Code Example</h3>
          <pre className="code-block">
{`import { PropChainClient } from '@propchain/sdk';

// Connect to node
const client = await PropChainClient.create(
  'ws://localhost:9944',
  { propertyRegistry: '${account.slice(0, 10)}...' }
);

// Register property
const { propertyId } = await client
  .propertyRegistry
  .registerProperty(signer, metadata);

// Query property
const info = await client
  .propertyRegistry
  .getProperty(propertyId);`}
          </pre>
        </div>
      </div>

      {/* Status Message */}
      {message && (
        <div className={`message message-${message.type}`}>
          {message.type === 'success' ? '✅' : '❌'} {message.text}
        </div>
      )}
    </div>
  );
}
