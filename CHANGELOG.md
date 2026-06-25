# Changelog

All notable changes to PropChain contracts are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
Commits follow [Conventional Commits](https://www.conventionalcommits.org/).

## [Unreleased]

### Features
- **analytics:** add portfolio rebalancing suggestions and health scoring ([#546](https://github.com/MettaChain/PropChain-contract/issues/546))
- **rental:** implement proportional rental income distribution system
- add interactive contract playground script ([#517](https://github.com/MettaChain/PropChain-contract/issues/517)) ([#551](https://github.com/MettaChain/PropChain-contract/issues/551))
- implement mutation testing via cargo-mutants and improve test suite coverage (issue [#483](https://github.com/MettaChain/PropChain-contract/issues/483)) ([#547](https://github.com/MettaChain/PropChain-contract/issues/547))
- **escrow:** cleanup mechanism ([#550](https://github.com/MettaChain/PropChain-contract/issues/550))

### Performance
- **bridge:** replace validator signature vecs with bitmap storage ([#544](https://github.com/MettaChain/PropChain-contract/issues/544))

### Bug Fixes
- **identity:** resolve KycTier merge markers; cargo fmt ([#553](https://github.com/MettaChain/PropChain-contract/issues/553))
- **identity:** make KycTier match exhaustive across variant naming styles
- **ci:** resolve lending build failure and clippy/format issues

### Testing
- expand escrow contract test coverage with edge cases ([#555](https://github.com/MettaChain/PropChain-contract/issues/555))

### Documentation
- add Mermaid sequence diagrams for all major contract workflows ([#523](https://github.com/MettaChain/PropChain-contract/issues/523))

---

> This file is auto-generated. Run `cargo make generate-changelog` to update it.
> Requires [git-cliff](https://git-cliff.org/) (`cargo install git-cliff --locked`).
