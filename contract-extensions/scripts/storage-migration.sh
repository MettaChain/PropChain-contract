#!/usr/bin/env bash
# Migrate ink! contract storage using a JSON migration specification.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

CONTRACT=""
SPEC=""
NETWORK="local"
URL_OVERRIDE=""
SURI=""
BACKUP_DIR=""
DRY_RUN=0
EXECUTE=0

usage() {
  cat <<'EOF'
Usage: storage-migration.sh --contract ADDRESS --spec FILE [options]

  --contract ADDRESS       Deployed ink! contract address
  --spec FILE              JSON migration specification
  --network local|testnet  RPC network family (default: local)
  --url URL                RPC HTTP or WebSocket URL
  --suri SECRET            Signer URI (default: //Alice for local)
  --backup-dir DIR         Backup destination (default: contract-extensions/storage-backups/TIMESTAMP)
  --execute                Submit calls; without this flag the script is dry-run
  -h, --help               Show this help
EOF
}

log() { echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*" >&2; }
die() { log "ERROR: $*"; exit 1; }
need() { command -v "$1" >/dev/null 2>&1 || die "$1 is required"; }

while (($#)); do
  case "$1" in
    --contract) CONTRACT="${2:?missing contract address}"; shift 2 ;;
    --spec) SPEC="${2:?missing spec file}"; shift 2 ;;
    --network) NETWORK="${2:?missing network}"; shift 2 ;;
    --url) URL_OVERRIDE="${2:?missing URL}"; shift 2 ;;
    --suri) SURI="${2:?missing signer URI}"; shift 2 ;;
    --backup-dir) BACKUP_DIR="${2:?missing backup directory}"; shift 2 ;;
    --execute) EXECUTE=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1" ;;
  esac
done

[[ -n "$CONTRACT" ]] || die "--contract is required"
[[ -n "$SPEC" && -f "$SPEC" ]] || die "--spec must name an existing JSON file"
need python3
need jq
need curl
need cargo-contract

if [[ -z "$BACKUP_DIR" ]]; then
  BACKUP_DIR="$REPO_ROOT/contract-extensions/storage-backups/$(date -u +%Y%m%dT%H%M%SZ)"
fi
mkdir -p "$BACKUP_DIR"
LOG_FILE="$BACKUP_DIR/migration.log"
: >"$LOG_FILE"
exec 3> >(tee -a "$LOG_FILE" >&2)

case "$NETWORK" in
  local) URL="${URL_OVERRIDE:-http://127.0.0.1:9944}" ;;
  testnet) URL="${URL_OVERRIDE:-https://rococo-contracts-rpc.polkadot.io}" ;;
  *) die "unknown network: $NETWORK" ;;
esac
[[ -n "$SURI" ]] || SURI="//Alice"

python3 - "$SPEC" <<'PY' || die "invalid migration specification"
import json, sys
spec = json.load(open(sys.argv[1], "r", encoding="utf-8"))
required = ("old_layout", "new_layout", "transformation")
missing = [key for key in required if key not in spec]
if missing:
    raise SystemExit("missing spec fields: " + ", ".join(missing))
for key in ("old_layout", "new_layout"):
    if not isinstance(spec[key], dict):
        raise SystemExit(key + " must be a JSON object")
transform = spec["transformation"]
if not isinstance(transform, dict) or not isinstance(transform.get("function"), str):
    raise SystemExit("transformation.function must be a string")
PY

MIGRATE_MESSAGE="$(jq -r '.migration.message // "migrate_storage"' "$SPEC")"
VERIFY_MESSAGE="$(jq -r '.verification.message // "get_storage"' "$SPEC")"
ROLLBACK_MESSAGE="$(jq -r '.rollback.message // "rollback_migration"' "$SPEC")"
RPC_METHOD="$(jq -r '.storage.rpc_method // "state_getKeysPaged"' "$SPEC")"
KEY_PREFIX="$(jq -r '.storage.key_prefix // ""' "$SPEC")"
KEY_LIMIT="$(jq -r '.storage.page_size // 1000' "$SPEC")"

# Enumerate contract keys with state_getKeysPaged. A spec can also provide an
# explicit key list for nodes without key pagination or for known storage slots.
ENUM_JSON="$BACKUP_DIR/keys.json"
python3 - "$RPC_METHOD" "$KEY_PREFIX" "$KEY_LIMIT" "$URL" "$CONTRACT" "$ENUM_JSON" <<'PY'
import json, sys, urllib.request

method, prefix, limit, url, contract, out = sys.argv[1:]
payload = {"jsonrpc":"2.0","id":1,"method":method,"params":[prefix, int(limit), None]}
req = urllib.request.Request(url, data=json.dumps(payload).encode(),
                             headers={"Content-Type":"application/json"})
try:
    result = json.load(urllib.request.urlopen(req, timeout=30)).get("result", [])
except Exception as exc:
    print(json.dumps({"error": str(exc)}))
    result = []
open(out, "w", encoding="utf-8").write(json.dumps(result))
PY

EXPLICIT_KEYS="$(jq -c '.storage.keys // []' "$SPEC")"
if [[ "$(jq -r '.storage.mode // "iterate"' "$SPEC")" != "iterate" ]] ||
   [[ "$(jq '(.storage.keys // [] | length) > 0' "$SPEC")" == "true" ]]; then
  jq -r '.storage.keys[]' "$SPEC" >"$BACKUP_DIR/keys.txt"
else
  jq -r '.[]' "$ENUM_JSON" >"$BACKUP_DIR/keys.txt"
fi

# Back up each original storage value. This uses state_getStorage at each
# enumerated key and is independent of cargo-contract's message ABI.
python3 - "$URL" "$BACKUP_DIR/keys.txt" "$BACKUP_DIR/original.json" <<'PY'
import json, sys, urllib.request
url, keys_file, out = sys.argv[1:]
keys = [line.strip() for line in open(keys_file, encoding="utf-8") if line.strip()]
entries = {}
for i, key in enumerate(keys):
    payload = {"jsonrpc":"2.0","id":i+1,"method":"state_getStorage","params":[key]}
    req = urllib.request.Request(url, data=json.dumps(payload).encode(),
                                 headers={"Content-Type":"application/json"})
    try:
        entries[key] = json.load(urllib.request.urlopen(req, timeout=30)).get("result")
    except Exception as exc:
        entries[key] = {"error": str(exc)}
open(out, "w", encoding="utf-8").write(json.dumps(entries, indent=2))
PY
cp "$BACKUP_DIR/original.json" "$BACKUP_DIR/rollback.json"

# Transformation function is evaluated through a restricted Python namespace.
# The spec may define `module`, `function`, or an inline expression.
python3 - "$SPEC" "$BACKUP_DIR/original.json" "$BACKUP_DIR/transformed.json" "$BACKUP_DIR/migration.log" <<'PY'
import importlib.util, json, sys, traceback
spec_path, original_path, output_path, log_path = sys.argv[1:]
spec = json.load(open(spec_path, encoding="utf-8"))
old_entries = json.load(open(original_path, encoding="utf-8"))
definition = spec["transformation"]
module_path = definition.get("module")
function_name = definition.get("function")

namespace = {"entry": None, "old_layout": spec["old_layout"], "new_layout": spec["new_layout"]}
if module_path:
    resolved = module_path if module_path.startswith("/") else str(__import__("pathlib").Path(spec_path).parent / module_path)
    modspec = importlib.util.spec_from_file_location("contract_migration", resolved)
    module = importlib.util.module_from_spec(modspec)
    modspec.loader.exec_module(module)
    transform = getattr(module, function_name)
else:
    transform_source = definition.get("expression")
    if not transform_source:
        raise SystemExit("transformation.expression or transformation.module is required")
    def transform(entry):
        namespace["entry"] = entry
        return eval(transform_source, {"__builtins__": {}}, namespace)

new_entries = {}
with open(log_path, "a", encoding="utf-8") as log:
    for key, old in old_entries.items():
        try:
            new = transform(old)
        except Exception as exc:
            log.write(json.dumps({"key": key, "old": old, "error": repr(exc)}) + "\n")
            raise
        new_entries[key] = new
        log.write(json.dumps({"key": key, "old": old, "new": new}) + "\n")
json.dump(new_entries, open(output_path, "w", encoding="utf-8"), indent=2)
PY

if [[ "$EXECUTE" == "1" ]]; then
  log "executing migration $MIGRATE_MESSAGE"
  for key in $(jq -r 'keys[]' "$BACKUP_DIR/transformed.json"); do
    new_value="$(jq -c --arg key "$key" '.[$key]' "$BACKUP_DIR/transformed.json")"
    EXEC_OUTPUT="$(cargo contract call \
      --contract "$CONTRACT" --message "$MIGRATE_MESSAGE" \
      --args "$key" "$new_value" --suri "$SURI" --url "$URL" \
      --skip-confirm --execute --output-json 2>>"$LOG_FILE")" || {
      log "migration failed for $key; invoking rollback"
      cargo contract call --contract "$CONTRACT" --message "$ROLLBACK_MESSAGE" \
        --args "$BACKUP_DIR/rollback.json" --suri "$SURI" --url "$URL" \
        --skip-confirm --execute --output-json >>"$LOG_FILE" 2>&1 || true
      die "migration failed for key $key"
    }
  done
else
  log "dry run: migration writes were not submitted"
fi

# Verification reads each key and compares it with the expected transformed value.
if [[ "$EXECUTE" == "1" ]] && ! python3 - "$URL" "$BACKUP_DIR/transformed.json" "$BACKUP_DIR/verification.json" "$BACKUP_DIR/migration.log" <<'PY'
import json, sys, urllib.request
url, expected_path, output_path, log_path = sys.argv[1:]
expected = json.load(open(expected_path, encoding="utf-8"))
actual = {}
for i, (key, value) in enumerate(expected.items()):
    payload = {"jsonrpc":"2.0","id":i+1,"method":"state_getStorage","params":[key]}
    req = urllib.request.Request(url, data=json.dumps(payload).encode(),
                                 headers={"Content-Type":"application/json"})
    try:
        actual[key] = json.load(urllib.request.urlopen(req, timeout=30)).get("result")
    except Exception as exc:
        actual[key] = {"error": str(exc)}
open(output_path, "w", encoding="utf-8").write(json.dumps(actual, indent=2))
failed = [key for key, value in expected.items() if actual.get(key) != value]
with open(log_path, "a", encoding="utf-8") as log:
    log.write(json.dumps({"verification": "passed" if not failed else "failed", "failed_keys": failed}) + "\n")
if failed:
    raise SystemExit("verification failed for keys: " + ", ".join(failed))
PY
then
  if [[ "$EXECUTE" == "1" ]]; then
    log "verification failed; rolling back from $BACKUP_DIR/rollback.json"
    cargo contract call --contract "$CONTRACT" --message "$ROLLBACK_MESSAGE" \
      --args "$BACKUP_DIR/rollback.json" --suri "$SURI" --url "$URL" \
      --skip-confirm --execute --output-json >>"$LOG_FILE" 2>&1 || true
  fi
  die "migration verification failed"
fi
