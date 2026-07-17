# 🛡️ Automated Security & Mutation Audit Log
Generated on: Fri Jul 17 05:17:56 UTC 2026
---
## 📦 Dependency License & Advisory Checks (cargo-deny)
```text
cargo-deny failed or flagged warnings
```
---
## 🔍 Vulnerability Advisory Scans (cargo-audit)
```text
[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.101.7
[0m[0m[1m[31mTitle:    [0m Reachable panic in certificate revocation list parsing
[0m[0m[1m[31mDate:     [0m 2026-04-22
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0104
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0104
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.13, <0.104.0-alpha.1 OR >=0.104.0-alpha.7

[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.101.7
[0m[0m[1m[31mTitle:    [0m Name constraints were accepted for certificates asserting a wildcard name
[0m[0m[1m[31mDate:     [0m 2026-04-14
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0099
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0099
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.12, <0.104.0-alpha.1 OR >=0.104.0-alpha.6

[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.101.7
[0m[0m[1m[31mTitle:    [0m Name constraints for URI names were incorrectly accepted
[0m[0m[1m[31mDate:     [0m 2026-04-14
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0098
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0098
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.12, <0.104.0-alpha.1 OR >=0.104.0-alpha.6

[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.102.8
[0m[0m[1m[31mTitle:    [0m Reachable panic in certificate revocation list parsing
[0m[0m[1m[31mDate:     [0m 2026-04-22
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0104
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0104
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.13, <0.104.0-alpha.1 OR >=0.104.0-alpha.7

[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.102.8
[0m[0m[1m[31mTitle:    [0m Name constraints were accepted for certificates asserting a wildcard name
[0m[0m[1m[31mDate:     [0m 2026-04-14
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0099
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0099
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.12, <0.104.0-alpha.1 OR >=0.104.0-alpha.6

[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.102.8
[0m[0m[1m[31mTitle:    [0m Name constraints for URI names were incorrectly accepted
[0m[0m[1m[31mDate:     [0m 2026-04-14
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0098
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0098
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.12, <0.104.0-alpha.1 OR >=0.104.0-alpha.6

[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.102.8
[0m[0m[1m[31mTitle:    [0m CRLs not considered authoritative by Distribution Point due to faulty matching logic
[0m[0m[1m[31mDate:     [0m 2026-03-20
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0049
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0049
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.10

[0m[0m[1m[33mCrate:    [0m instant
[0m[0m[1m[33mVersion:  [0m 0.1.13
[0m[0m[1m[33mWarning:  [0m unmaintained
[0m[0m[1m[33mTitle:    [0m `instant` is unmaintained
[0m[0m[1m[33mDate:     [0m 2024-09-01
[0m[0m[1m[33mID:       [0m RUSTSEC-2024-0384
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2024-0384

[0m[0m[1m[33mCrate:    [0m libsecp256k1
[0m[0m[1m[33mVersion:  [0m 0.7.2
[0m[0m[1m[33mWarning:  [0m unmaintained
[0m[0m[1m[33mTitle:    [0m libsecp256k1 is unmaintained
[0m[0m[1m[33mDate:     [0m 2025-01-14
[0m[0m[1m[33mID:       [0m RUSTSEC-2025-0161
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2025-0161

[0m[0m[1m[33mCrate:    [0m parity-wasm
[0m[0m[1m[33mVersion:  [0m 0.45.0
[0m[0m[1m[33mWarning:  [0m unmaintained
[0m[0m[1m[33mTitle:    [0m Crate `parity-wasm` deprecated by the author
[0m[0m[1m[33mDate:     [0m 2022-10-01
[0m[0m[1m[33mID:       [0m RUSTSEC-2022-0061
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2022-0061

[0m[0m[1m[33mCrate:    [0m paste
[0m[0m[1m[33mVersion:  [0m 1.0.15
[0m[0m[1m[33mWarning:  [0m unmaintained
[0m[0m[1m[33mTitle:    [0m paste - no longer maintained
[0m[0m[1m[33mDate:     [0m 2024-10-07
[0m[0m[1m[33mID:       [0m RUSTSEC-2024-0436
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2024-0436

[0m[0m[1m[33mCrate:    [0m rustls-pemfile
[0m[0m[1m[33mVersion:  [0m 1.0.4
[0m[0m[1m[33mWarning:  [0m unmaintained
[0m[0m[1m[33mTitle:    [0m rustls-pemfile is unmaintained
[0m[0m[1m[33mDate:     [0m 2025-11-28
[0m[0m[1m[33mID:       [0m RUSTSEC-2025-0134
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2025-0134

[0m[0m[1m[33mCrate:    [0m rustls-pemfile
[0m[0m[1m[33mVersion:  [0m 2.2.0
[0m[0m[1m[33mWarning:  [0m unmaintained
[0m[0m[1m[33mTitle:    [0m rustls-pemfile is unmaintained
[0m[0m[1m[33mDate:     [0m 2025-11-28
[0m[0m[1m[33mID:       [0m RUSTSEC-2025-0134
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2025-0134

[0m[0m[1m[33mCrate:    [0m term_size
[0m[0m[1m[33mVersion:  [0m 0.3.2
[0m[0m[1m[33mWarning:  [0m unmaintained
[0m[0m[1m[33mTitle:    [0m `term_size` is unmaintained; use `terminal_size` instead
[0m[0m[1m[33mDate:     [0m 2020-11-03
[0m[0m[1m[33mID:       [0m RUSTSEC-2020-0163
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2020-0163

[0m[0m[1m[33mCrate:    [0m anyhow
[0m[0m[1m[33mVersion:  [0m 1.0.102
[0m[0m[1m[33mWarning:  [0m unsound
[0m[0m[1m[33mTitle:    [0m Unsoundness in `Error::downcast_mut()`
[0m[0m[1m[33mDate:     [0m 2026-06-25
[0m[0m[1m[33mID:       [0m RUSTSEC-2026-0190
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0190

[0m[0m[1m[33mCrate:    [0m cxx
[0m[0m[1m[33mVersion:  [0m 1.0.194
[0m[0m[1m[33mWarning:  [0m unsound
[0m[0m[1m[33mTitle:    [0m `let_cxx_string!` uses uninitialized value due to exception safety violations
[0m[0m[1m[33mDate:     [0m 2026-07-05
[0m[0m[1m[33mID:       [0m RUSTSEC-2026-0202
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0202

[0m[0m[1m[33mCrate:    [0m lru
[0m[0m[1m[33mVersion:  [0m 0.12.5
[0m[0m[1m[33mWarning:  [0m unsound
[0m[0m[1m[33mTitle:    [0m `IterMut` violates Stacked Borrows by invalidating internal pointer
[0m[0m[1m[33mDate:     [0m 2026-01-07
[0m[0m[1m[33mID:       [0m RUSTSEC-2026-0002
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0002

[0m[0m[1m[33mCrate:    [0m spin
[0m[0m[1m[33mVersion:  [0m 0.9.8
[0m[0m[1m[33mWarning:  [0m yanked

cargo-audit detected critical vulnerability markers
```
---
## 🧬 Mutation Testing Resilience Analytics (cargo-mutants)
```text
Found 0 mutants to test
```
