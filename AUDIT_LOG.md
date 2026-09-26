# 🛡️ Automated Security & Mutation Audit Log
Generated on: Sat Sep 26 07:19:42 UTC 2026
---
## 📦 Dependency License & Advisory Checks (cargo-deny)
```text
[0m[1m[38;5;9merror[deprecated][0m[1m: this key has been removed, see https://github.com/EmbarkStudios/cargo-deny/pull/611 for migration information[0m
   [0m[36m┌─[0m /home/runner/work/PropChain-contract/PropChain-contract/deny.toml:20:1
   [0m[36m│[0m
[0m[36m20[0m [0m[36m│[0m [0m[31munlicensed[0m = "deny"
   [0m[36m│[0m [0m[31m━━━━━━━━━━[0m

2026-09-26 07:19:42 [[31mERROR[0m] failed to validate configuration file /home/runner/work/PropChain-contract/PropChain-contract/deny.toml[0m
```
---
## 🔍 Vulnerability Advisory Scans (cargo-audit)
```text
[0m[0m[1m[32m    Fetching[0m advisory database from `https://github.com/RustSec/advisory-db.git`
[0m[0m[1m[32m      Loaded[0m 1271 security advisories (from /home/runner/.cargo/advisory-db)
[0m[0m[1m[32m    Updating[0m crates.io index
[0m[0m[1m[32m    Scanning[0m Cargo.lock for vulnerabilities (856 crate dependencies)
[0m[0m[1m[31mCrate:    [0m h2
[0m[0m[1m[31mVersion:  [0m 0.3.27
[0m[0m[1m[31mTitle:    [0m h2 unbounded empty DATA frames
[0m[0m[1m[31mDate:     [0m 2026-08-17
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0258
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0258
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.4.16

[0m[0m[1m[31mCrate:    [0m rustls-webpki
[0m[0m[1m[31mVersion:  [0m 0.101.7
[0m[0m[1m[31mTitle:    [0m Name constraints for URI names were incorrectly accepted
[0m[0m[1m[31mDate:     [0m 2026-04-14
[0m[0m[1m[31mID:       [0m RUSTSEC-2026-0098
[0m[0m[1m[31mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0098
[0m[0m[1m[31mSolution: [0m Upgrade to >=0.103.12, <0.104.0-alpha.1 OR >=0.104.0-alpha.6

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

[0m[0m[1m[33mCrate:    [0m event-listener
[0m[0m[1m[33mVersion:  [0m 5.4.1
[0m[0m[1m[33mWarning:  [0m unsound
[0m[0m[1m[33mTitle:    [0m `event-listener` allows `!Send` tags to cross thread boundaries via `StackSlot`
[0m[0m[1m[33mDate:     [0m 2026-07-13
[0m[0m[1m[33mID:       [0m RUSTSEC-2026-0221
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0221

[0m[0m[1m[33mCrate:    [0m lru
[0m[0m[1m[33mVersion:  [0m 0.12.5
[0m[0m[1m[33mWarning:  [0m unsound
[0m[0m[1m[33mTitle:    [0m Potential use-after-free due to lack of panic safety in `LruCache::pop()`
[0m[0m[1m[33mDate:     [0m 2026-05-12
[0m[0m[1m[33mID:       [0m RUSTSEC-2026-0253
[0m[0m[1m[33mURL:      [0m https://rustsec.org/advisories/RUSTSEC-2026-0253

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

[0m[0m[1m[31merror:[0m 8 vulnerabilities found!
[0m[0m[1m[33mwarning:[0m 13 allowed warnings found
```
---
## 🧬 Mutation Gate: lending
```text
Found 0 mutants to test
[33m WARN[0m No mutants found under the active filters
```
---
## 🧬 Mutation Gate: bridge
```text
Found 0 mutants to test
[33m WARN[0m No mutants found under the active filters
```
---
## 🧬 Mutation Gate: oracle
```text
Found 0 mutants to test
[33m WARN[0m No mutants found under the active filters
```
---
## 🧪 Test Gate: bridge granular pause / sig payload / dex invariants
```text
[1m[33mwarning[0m: profiles for the non root package will be ignored, specify profiles at the workspace root:
package:   /home/runner/work/PropChain-contract/PropChain-contract/contracts/hello-world/Cargo.toml
workspace: /home/runner/work/PropChain-contract/PropChain-contract/Cargo.toml
[1m[33mwarning[0m[1m: unused workspace dependency `soroban-sdk`[0m
  [1m[94m--> [0mCargo.toml:51:1
   [1m[94m|[0m
[1m[94m51[0m [1m[94m|[0m soroban-sdk = "28.0.0"
   [1m[94m|[0m [1m[33m^^^^^^^^^^^[0m
   [1m[94m|[0m
   [1m[94m= [0m[1mnote[0m: `cargo::unused_workspace_dependencies` is set to `warn` by default
[1m[96mhelp[0m: consider removing the workspace dependency `soroban-sdk`
[1m[33mwarning[0m: workspace (manifest) generated 1 warning
[1m[33mwarning[0m: contracts/oracle/Cargo.toml: Found `feature = ...` in `target.'cfg(...)'.dependencies`. This key is not supported for selecting dependencies and will not work as expected. Use the [features] section instead: https://doc.rust-lang.org/cargo/reference/features.html
[1m[33mwarning[0m: `oracle` (manifest) generated 1 warning
[1m[33mwarning[0m: contracts/lending/Cargo.toml: Found `feature = ...` in `target.'cfg(...)'.dependencies`. This key is not supported for selecting dependencies and will not work as expected. Use the [features] section instead: https://doc.rust-lang.org/cargo/reference/features.html
[1m[33mwarning[0m: `propchain-lending` (manifest) generated 1 warning
[1m[92m Downloading[0m crates ...
[1m[92m  Downloaded[0m aead v0.5.2
[1m[92m  Downloaded[0m array-bytes v6.2.3
[1m[92m  Downloaded[0m Inflector v0.11.4
[1m[92m  Downloaded[0m array-init v2.1.0
[1m[92m  Downloaded[0m arrayref v0.3.9
[1m[92m  Downloaded[0m blake2 v0.10.6
[1m[92m  Downloaded[0m curve25519-dalek-derive v0.1.1
[1m[92m  Downloaded[0m darling_core v0.14.4
[1m[92m  Downloaded[0m byte-slice-cast v1.2.3
[1m[92m  Downloaded[0m environmental v1.1.4
[1m[92m  Downloaded[0m common-path v1.0.0
[1m[92m  Downloaded[0m const_env v0.1.5
[1m[92m  Downloaded[0m const_env_impl v0.1.5
[1m[92m  Downloaded[0m darling_macro v0.14.4
[1m[92m  Downloaded[0m docify v0.2.9
[1m[92m  Downloaded[0m dyn-clone v1.0.20
[1m[92m  Downloaded[0m getrandom_or_panic v0.0.3
[1m[92m  Downloaded[0m docify_macros v0.2.9
[1m[92m  Downloaded[0m ink_allocator v5.1.1
[1m[92m  Downloaded[0m bitflags v1.3.2
[1m[92m  Downloaded[0m derive-syn-parse v0.2.0
[1m[92m  Downloaded[0m impl-trait-for-tuples v0.2.3
[1m[92m  Downloaded[0m ink_prelude v5.1.1
[1m[92m  Downloaded[0m ink_primitives v5.1.1
[1m[92m  Downloaded[0m ink_storage_traits v5.1.1
[1m[92m  Downloaded[0m integer-sqrt v0.1.5
[1m[92m  Downloaded[0m keccak v0.1.6
[1m[92m  Downloaded[0m konst_macro_rules v0.2.19
[1m[92m  Downloaded[0m darling v0.14.4
[1m[92m  Downloaded[0m bounded-collections v0.2.4
[1m[92m  Downloaded[0m const_format_proc_macros v0.2.34
[1m[92m  Downloaded[0m ink_metadata v5.1.1
[1m[92m  Downloaded[0m derivative v2.2.0
[1m[92m  Downloaded[0m ink_storage v5.1.1
[1m[92m  Downloaded[0m ink v5.1.1
[1m[92m  Downloaded[0m ink_codegen v5.1.1
[1m[92m  Downloaded[0m ink_engine v5.1.1
[1m[92m  Downloaded[0m ink_macro v5.1.1
[1m[92m  Downloaded[0m derive_more v0.99.20
[1m[92m  Downloaded[0m derive_more v1.0.0
[1m[92m  Downloaded[0m impl-serde v0.4.0
[1m[92m  Downloaded[0m ink_env v5.1.1
[1m[92m  Downloaded[0m derive_more-impl v1.0.0
[1m[92m  Downloaded[0m const_format v0.2.36
[1m[92m  Downloaded[0m ink_ir v5.1.1
[1m[92m  Downloaded[0m konst v0.2.20
[1m[92m  Downloaded[0m cc v1.2.61
[1m[92m  Downloaded[0m linkme-impl v0.3.36
[1m[92m  Downloaded[0m itertools v0.12.1
[1m[92m  Downloaded[0m linkme v0.3.36
[1m[92m  Downloaded[0m hashbrown v0.17.0
[1m[92m  Downloaded[0m curve25519-dalek v4.1.3
[1m[92m  Downloaded[0m num-traits v0.2.19
[1m[92m  Downloaded[0m parity-scale-codec-derive v3.7.5
[1m[92m  Downloaded[0m merlin v3.0.0
[1m[92m  Downloaded[0m pallet-contracts-uapi v9.0.0
[1m[92m  Downloaded[0m paste v1.0.15
[1m[92m  Downloaded[0m proc-macro-crate v1.3.1
[1m[92m  Downloaded[0m proc-macro-crate v3.5.0
[1m[92m  Downloaded[0m parity-scale-codec v3.7.5
[1m[92m  Downloaded[0m rand v0.8.6
[1m[92m  Downloaded[0m rand_core v0.6.4
[1m[92m  Downloaded[0m rand_chacha v0.3.1
[1m[92m  Downloaded[0m rustc_version v0.4.1
[1m[92m  Downloaded[0m scale-decode-derive v0.11.1
[1m[92m  Downloaded[0m scale-info-derive v2.11.6
[1m[92m  Downloaded[0m schemars_derive v0.8.22
[1m[92m  Downloaded[0m scale-decode v0.11.1
[1m[92m  Downloaded[0m scale-encode-derive v0.6.0
[1m[92m  Downloaded[0m scale-type-resolver v0.1.1
[1m[92m  Downloaded[0m scale-bits v0.5.0
[1m[92m  Downloaded[0m schnorrkel v0.11.5
[1m[92m  Downloaded[0m scale-encode v0.6.0
[1m[92m  Downloaded[0m scale-info v2.11.6
[1m[92m  Downloaded[0m schemars v0.8.22
[1m[92m  Downloaded[0m secp256k1 v0.28.2
[1m[92m  Downloaded[0m secp256k1-sys v0.9.2
[1m[92m  Downloaded[0m serde_bytes v0.11.19
[1m[92m  Downloaded[0m serde_spanned v0.6.9
[1m[92m  Downloaded[0m sha3 v0.10.9
[1m[92m  Downloaded[0m sp-arithmetic v26.1.0
[1m[92m  Downloaded[0m sp-debug-derive v14.0.0
[1m[92m  Downloaded[0m sp-weights v31.1.0
[1m[92m  Downloaded[0m staging-xcm v11.0.0
[1m[92m  Downloaded[0m strsim v0.10.0
[1m[92m  Downloaded[0m syn v1.0.109
[1m[92m  Downloaded[0m toml_datetime v0.6.11
[1m[92m  Downloaded[0m toml v0.8.23
[1m[92m  Downloaded[0m toml_edit v0.25.11+spec-1.1.0
[1m[92m  Downloaded[0m toml_edit v0.22.27
[1m[92m  Downloaded[0m toml_edit v0.19.15
[1m[92m  Downloaded[0m toml_write v0.1.2
[1m[92m  Downloaded[0m winnow v0.5.40
[1m[92m  Downloaded[0m unicode-xid v0.2.6
[1m[92m  Downloaded[0m xcm-procedural v8.0.0
[1m[92m  Downloaded[0m xxhash-rust v0.8.15
[1m[92m  Downloaded[0m zeroize_derive v1.4.3
[1m[92m   Compiling[0m proc-macro2 v1.0.106
[1m[92m   Compiling[0m quote v1.0.45
[1m[92m   Compiling[0m unicode-ident v1.0.24
[1m[92m   Compiling[0m hashbrown v0.17.0
[1m[92m   Compiling[0m equivalent v1.0.2
[1m[92m   Compiling[0m serde_core v1.0.228
[1m[92m   Compiling[0m serde v1.0.228
[1m[92m   Compiling[0m rustversion v1.0.22
[1m[92m   Compiling[0m unicode-xid v0.2.6
[1m[92m   Compiling[0m indexmap v2.14.0
[1m[92m   Compiling[0m syn v2.0.117
[1m[92m   Compiling[0m winnow v1.0.2
[1m[92m   Compiling[0m toml_datetime v1.1.1+spec-1.1.0
[1m[92m   Compiling[0m const_format_proc_macros v0.2.34
[1m[92m   Compiling[0m toml_parser v1.1.2+spec-1.1.0
[1m[92m   Compiling[0m version_check v0.9.5
[1m[92m   Compiling[0m toml_edit v0.25.11+spec-1.1.0
[1m[92m   Compiling[0m generic-array v0.14.7
[1m[92m   Compiling[0m cfg-if v1.0.4
[1m[92m   Compiling[0m syn v1.0.109
[1m[92m   Compiling[0m proc-macro-crate v3.5.0
[1m[92m   Compiling[0m zmij v1.0.21
[1m[92m   Compiling[0m fnv v1.0.7
[1m[92m   Compiling[0m serde_json v1.0.149
[1m[92m   Compiling[0m once_cell v1.21.4
[1m[92m   Compiling[0m strsim v0.10.0
[1m[92m   Compiling[0m ident_case v1.0.1
[1m[92m   Compiling[0m konst_macro_rules v0.2.19
[1m[92m   Compiling[0m konst v0.2.20
[1m[92m   Compiling[0m parity-scale-codec v3.7.5
[1m[92m   Compiling[0m arrayvec v0.7.6
[1m[92m   Compiling[0m const_format v0.2.36
[1m[92m   Compiling[0m memchr v2.8.0
[1m[92m   Compiling[0m byte-slice-cast v1.2.3
[1m[92m   Compiling[0m winnow v0.5.40
[1m[92m   Compiling[0m serde_derive v1.0.228
[1m[92m   Compiling[0m parity-scale-codec-derive v3.7.5
[1m[92m   Compiling[0m impl-trait-for-tuples v0.2.3
[1m[92m   Compiling[0m darling_core v0.14.4
[1m[92m   Compiling[0m derive_more-impl v1.0.0
[1m[92m   Compiling[0m scale-info-derive v2.11.6
[1m[92m   Compiling[0m toml_datetime v0.6.11
[1m[92m   Compiling[0m darling_macro v0.14.4
[1m[92m   Compiling[0m toml_edit v0.19.15
[1m[92m   Compiling[0m derive_more v0.99.20
[1m[92m   Compiling[0m darling v0.14.4
[1m[92m   Compiling[0m serde_derive_internals v0.29.1
[1m[92m   Compiling[0m schemars v0.8.22
[1m[92m   Compiling[0m itoa v1.0.18
[1m[92m   Compiling[0m schemars_derive v0.8.22
[1m[92m   Compiling[0m smallvec v1.15.1
[1m[92m   Compiling[0m proc-macro-crate v1.3.1
[1m[92m   Compiling[0m typenum v1.20.0
[1m[92m   Compiling[0m dyn-clone v1.0.20
[1m[92m   Compiling[0m scale-decode-derive v0.11.1
[1m[92m   Compiling[0m scale-encode-derive v0.6.0
[1m[92m   Compiling[0m derive_more v1.0.0
[1m[92m   Compiling[0m aho-corasick v1.1.4
[1m[92m   Compiling[0m libc v0.2.186
[1m[92m   Compiling[0m regex-syntax v0.8.10
[1m[92m   Compiling[0m scale-info v2.11.6
[1m[92m   Compiling[0m regex-automata v0.4.14
[1m[92m   Compiling[0m crypto-common v0.1.7
[1m[92m   Compiling[0m block-buffer v0.10.4
[1m[92m   Compiling[0m scale-type-resolver v0.1.1
[1m[92m   Compiling[0m serde_spanned v0.6.9
[1m[92m   Compiling[0m winnow v0.7.15
[1m[92m   Compiling[0m zerocopy v0.8.48
[1m[92m   Compiling[0m autocfg v1.5.0
[1m[92m   Compiling[0m subtle v2.6.1
[1m[92m   Compiling[0m toml_write v0.1.2
[1m[92m   Compiling[0m digest v0.10.7
[1m[92m   Compiling[0m num-traits v0.2.19
[1m[92m   Compiling[0m getrandom v0.2.17
[1m[92m   Compiling[0m shlex v1.3.0
[1m[92m   Compiling[0m same-file v1.0.6
[1m[92m   Compiling[0m find-msvc-tools v0.1.9
[1m[92m   Compiling[0m regex v1.12.3
[1m[92m   Compiling[0m toml_edit v0.22.27
[1m[92m   Compiling[0m cc v1.2.61
[1m[92m   Compiling[0m walkdir v2.5.0
[1m[92m   Compiling[0m rand_core v0.6.4
[1m[92m   Compiling[0m scale-bits v0.5.0
[1m[92m   Compiling[0m derive-syn-parse v0.2.0
[1m[92m   Compiling[0m termcolor v1.4.1
[1m[92m   Compiling[0m toml v0.8.23
[1m[92m   Compiling[0m common-path v1.0.0
[1m[92m   Compiling[0m semver v1.0.28
[1m[92m   Compiling[0m linkme-impl v0.3.36
[1m[92m   Compiling[0m rustc_version v0.4.1
[1m[92m   Compiling[0m secp256k1-sys v0.9.2
[1m[92m   Compiling[0m scale-decode v0.11.1
[1m[92m   Compiling[0m docify_macros v0.2.9
[1m[92m   Compiling[0m scale-encode v0.6.0
[1m[92m   Compiling[0m zeroize_derive v1.4.3
[1m[92m   Compiling[0m ink_prelude v5.1.1
[1m[92m   Compiling[0m paste v1.0.15
[1m[92m   Compiling[0m xxhash-rust v0.8.15
[1m[92m   Compiling[0m cpufeatures v0.2.17
[1m[92m   Compiling[0m keccak v0.1.6
[1m[92m   Compiling[0m linkme v0.3.36
[1m[92m   Compiling[0m zeroize v1.8.2
[1m[92m   Compiling[0m docify v0.2.9
[1m[92m   Compiling[0m integer-sqrt v0.1.5
[1m[92m   Compiling[0m curve25519-dalek v4.1.3
[1m[92m   Compiling[0m ink_primitives v5.1.1
[1m[92m   Compiling[0m lazy_static v1.5.0
[1m[92m   Compiling[0m static_assertions v1.1.0
[1m[92m   Compiling[0m log v0.4.29
[1m[92m   Compiling[0m bounded-collections v0.2.4
[1m[92m   Compiling[0m ppv-lite86 v0.2.21
[1m[92m   Compiling[0m sp-arithmetic v26.1.0
[1m[92m   Compiling[0m Inflector v0.11.4
[1m[92m   Compiling[0m rand_chacha v0.3.1
[1m[92m   Compiling[0m rand v0.8.6
[1m[92m   Compiling[0m sha2 v0.10.9
[1m[92m   Compiling[0m impl-serde v0.4.0
[1m[92m   Compiling[0m sp-debug-derive v14.0.0
[1m[92m   Compiling[0m curve25519-dalek-derive v0.1.1
[1m[92m   Compiling[0m bitflags v1.3.2
[1m[92m   Compiling[0m either v1.15.0
[1m[92m   Compiling[0m byteorder v1.5.0
[1m[92m   Compiling[0m itertools v0.12.1
[1m[92m   Compiling[0m merlin v3.0.0
[1m[92m   Compiling[0m pallet-contracts-uapi v9.0.0
[1m[92m   Compiling[0m sp-weights v31.1.0
[1m[92m   Compiling[0m ink_metadata v5.1.1
[1m[92m   Compiling[0m getrandom_or_panic v0.0.3
[1m[92m   Compiling[0m xcm-procedural v8.0.0
[1m[92m   Compiling[0m secp256k1 v0.28.2
[1m[92m   Compiling[0m blake2 v0.10.6
[1m[92m   Compiling[0m sha3 v0.10.9
[1m[92m   Compiling[0m derivative v2.2.0
[1m[92m   Compiling[0m serde_bytes v0.11.19
[1m[92m   Compiling[0m const_env_impl v0.1.5
[1m[92m   Compiling[0m arrayref v0.3.9
[1m[92m   Compiling[0m array-bytes v6.2.3
[1m[92m   Compiling[0m environmental v1.1.4
[1m[92m   Compiling[0m const_env v0.1.5
[1m[92m   Compiling[0m schnorrkel v0.11.5
[1m[92m   Compiling[0m ink_engine v5.1.1
[1m[92m   Compiling[0m staging-xcm v11.0.0
[1m[92m   Compiling[0m ink_ir v5.1.1
[1m[92m   Compiling[0m ink_storage_traits v5.1.1
[1m[92m   Compiling[0m ink_allocator v5.1.1
[1m[92m   Compiling[0m heck v0.5.0
[1m[92m   Compiling[0m synstructure v0.13.2
[1m[92m   Compiling[0m ink_codegen v5.1.1
[1m[92m   Compiling[0m array-init v2.1.0
[1m[92m   Compiling[0m ink_macro v5.1.1
[1m[92m   Compiling[0m ink_env v5.1.1
[1m[92m   Compiling[0m ink_storage v5.1.1
[1m[92m   Compiling[0m ink v5.1.1
[1m[92m   Compiling[0m propchain-traits v1.0.0 (/home/runner/work/PropChain-contract/PropChain-contract/contracts/traits)
[1m[92m   Compiling[0m propchain-bridge v1.0.0 (/home/runner/work/PropChain-contract/PropChain-contract/contracts/bridge)
[1m[91merror[0m[1m: this file contains an unclosed delimiter[0m
    [1m[94m--> [0mcontracts/bridge/src/tests.rs:2654:3
     [1m[94m|[0m
[1m[94m   4[0m [1m[94m|[0m mod tests {
     [1m[94m|[0m           [1m[94m-[0m [1m[94munclosed delimiter[0m
[1m[94m...[0m
[1m[94m2061[0m [1m[94m|[0m     fn validator_registry_is_capped_by_bitmap_proof() {
     [1m[94m|[0m                                                       [1m[94m-[0m [1m[94mthis delimiter might not be properly closed...[0m
[1m[94m...[0m
[1m[94m2079[0m [1m[94m|[0m         }
     [1m[94m|[0m         [1m[94m-[0m [1m[94m...as it matches this but it has different indentation[0m
[1m[94m...[0m
[1m[94m2654[0m [1m[94m|[0m }
     [1m[94m|[0m  [1m[91m^[0m

[1m[91merror[0m: could not compile `propchain-bridge` (lib test) due to 1 previous error
