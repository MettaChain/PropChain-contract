# PropChain Interactive Architecture Explorer

A zero-dependency interactive viewer that renders all Mermaid diagrams from PropChain's architecture docs as clickable, explorable SVG visualizations.

## Quick Start

The explorer must be served via HTTP (not `file://`). From the `docs/` directory:

```bash
npx -y serve .
# Then open http://localhost:3000/interactive-diagrams/
```

## Features

| Feature | Description |
|---------|-------------|
| **Auto-discovery** | Parses Mermaid blocks from Markdown files at runtime |
| **Category sidebar** | Diagrams grouped by architecture domain |
| **Click & Hover** | Click nodes to see connections, cross-references |
| **Zoom & Pan** | Mouse wheel zoom, drag-to-pan |
| **Step-through** | Animate sequence diagrams message by message |
| **Cross-diagram nav** | Click a participant → see all diagrams it appears in |
| **Search** | Filter by name, category, or content |
| **Deep linking** | `index.html?diagram=property-registration-sequence` |
| **Export** | Download as SVG or PNG |
| **Fullscreen** | Distraction-free exploration |
| **Minimap** | Corner overview for large diagrams |

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+K` | Focus search |
| `F` | Toggle fullscreen |
| `+` / `-` | Zoom in / out |
| `0` | Reset zoom |
| `←` `→` | Navigate diagrams |
| `Space` | Step forward (in step mode) |
| `Esc` | Close panels |

## Deep Linking

Link to a specific diagram from any document:

```markdown
[View interactive diagram](./interactive-diagrams/index.html?diagram=property-registration-sequence)
```

### Available Diagram IDs

- `property-registration-sequence`
- `property-update-flow`
- `escrow-creation-funding`
- `escrow-release-property-transfer`
- `dispute-resolution-flow`
- `user-kyc-aml-verification`
- `jurisdiction-specific-compliance`
- `bridge-token-transfer-source-chain`
- `cross-chain-message-passing`
- `insurance-policy-creation`
- `insurance-claim-processing`
- `multi-source-price-aggregation`
- `oracle-manipulation-detection`
- `protocol-upgrade-proposal`
- `emergency-pause-mechanism`
- `failed-transaction-rollback`
- `insufficient-gas-handling`
- `oracle-data-staleness`
- `property-lifecycle-state-machine`
- `escrow-state-machine`
- `compliance-status-state-machine`
- `contract-deployment-pipeline`

## Adding New Diagrams

Simply add a `\`\`\`mermaid` code block under a `### Title` heading in any of the source Markdown files. The explorer auto-discovers them on next load.
