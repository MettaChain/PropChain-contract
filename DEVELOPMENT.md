# PropChain Development Environment Setup

This guide will help you set up a complete development environment for PropChain smart contracts.

## Quick Start

```bash
# Clone and setup
git clone https://github.com/MettaChain/PropChain-contract.git
cd PropChain-contract
./scripts/setup.sh

# Start local development environment
docker-compose up -d

# Run tests
./scripts/test.sh

# Build contracts
./scripts/build.sh --release
```

## Prerequisites

- **Rust** 1.70+ with stable toolchain
- **Docker** and Docker Compose
- **Node.js** 16+ (for frontend development)
- **Git**

## Manual Setup

### 1. Install Rust and Tools

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Install cargo-contract
cargo install cargo-contract --locked

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### 2. Setup Pre-commit Hooks

```bash
./scripts/setup-pre-commit.sh
```

### 3. Start Local Development

```bash
# Start blockchain node
./scripts/local-node.sh start

# Or use Docker Compose for full stack
docker-compose up -d
```

## Development Workflow

### Building Contracts

```bash
# Debug build
./scripts/build.sh

# Release build
./scripts/build.sh --release

# Clean build
./scripts/build.sh --clean
```

### Running Tests

```bash
# All tests
./scripts/test.sh

# Unit tests only
./scripts/test.sh --no-integration

# With coverage
./scripts/test.sh --coverage

# E2E tests
./scripts/e2e-test.sh
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linting
cargo clippy

# Pre-commit checks
pre-commit run --all-files
```

### Deployment

```bash
# Local deployment
./scripts/deploy.sh --network local

# Testnet deployment
./scripts/deploy.sh --network westend

# Mainnet deployment
./scripts/deploy.sh --network polkadot
```

## Project Structure

```
PropChain-contract/
├── contracts/              # Smart contract source code
│   ├── lib/               # Main contract implementations
│   ├── traits/            # Shared trait definitions
│   └── tests/             # Contract-specific tests
├── scripts/               # Development and deployment scripts
├── tests/                 # Integration and E2E tests
├── docs/                  # Documentation
│   ├── tutorials/         # Step-by-step guides
│   ├── contracts.md       # API documentation
│   ├── integration.md     # Integration guide
│   ├── deployment.md      # Deployment guide
│   └── architecture.md    # Technical architecture
├── .github/workflows/     # CI/CD pipelines
├── docker-compose.yml     # Local development stack
└── rust-toolchain.toml    # Rust version configuration
```

## Environment Configuration

### Local Development (.env.local)

```env
NETWORK=local
NODE_URL=ws://localhost:9944
SURI=//Alice
```

### Testnet (.env.westend)

```env
NETWORK=westend
NODE_URL=wss://westend-rpc.polkadot.io
SURI=your-testnet-mnemonic
```

### Mainnet (.env.polkadot)

```env
NETWORK=polkadot
NODE_URL=wss://rpc.polkadot.io
SURI=your-mainnet-mnemonic
```

## Common Issues and Solutions

### Rust Installation Issues

```bash
# If Rust is not found
source ~/.cargo/env

# Update Rust toolchain
rustup update stable
```

### Contract Build Failures

```bash
# Clean build artifacts
cargo clean
rm -rf target/

# Rebuild
./scripts/build.sh --clean
```

### Node Connection Issues

```bash
# Check if node is running
curl http://localhost:9933/health

# Restart local node
./scripts/local-node.sh restart
```

### Pre-commit Hook Issues

```bash
# Reinstall hooks
./scripts/setup-pre-commit.sh --test-only

# Run hooks manually
pre-commit run --all-files
```

## IDE Configuration

### VS Code

Install these extensions:
- Rust Analyzer
- TOML Language Support
- Docker
- GitLens

### Workspace Settings (.vscode/settings.json)

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.loadOutDirsFromCheck": true,
    "editor.formatOnSave": true,
    "files.trimTrailingWhitespace": true
}
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests and linting
5. Submit a pull request

## Getting Help

- **Documentation**: Check the `docs/` directory
- **Issues**: [GitHub Issues](https://github.com/MettaChain/PropChain-contract/issues)
- **Discord**: [PropChain Community](https://discord.gg/propchain)
- **Email**: dev@propchain.io

## Next Steps

1. Read the [Architecture Guide](docs/architecture.md)
2. Follow the [Basic Property Registration Tutorial](docs/tutorials/basic-property-registration.md)
3. Explore the [Contract API](docs/contracts.md)
4. Set up your [Frontend Integration](docs/integration.md)
