# flux-verify-api

Rust HTTP service that verifies natural-language constraint claims. Parses a claim, compiles it to FLUX bytecodes, executes on a VM, and returns PROVEN/DISPROVEN/UNKNOWN with a Merkle-hashed proof trace.

Built with Axum. Supports three domains: **sonar**, **thermal**, and **generic**.

## Build & Run

```bash
cargo build --release
cargo run --release

# With PLATO integration (optional)
VERIFY_PLATO_URL=http://localhost:8847 VERIFY_PLATO_TOKEN=secret cargo run --release
```

Defaults to `0.0.0.0:8080`. Override with `VERIFY_HOST` and `VERIFY_PORT`.

## API

### POST /verify

```json
{
  "claim": "at 10 kHz and 200m depth, a signal at 5 km range is detectable",
  "domain": "sonar",
  "rigor": "standard"
}
```

- `domain`: `sonar` | `thermal` | `generic` (default: `generic`)
- `rigor`: `standard` (default)

**Response (200):**

```json
{
  "status": "PROVEN",
  "confidence": 0.938,
  "trace": [
    { "opcode": "LOAD", "value": 200.0, "desc": "depth (m)" },
    { "opcode": "LOAD", "value": 10000.0, "desc": "frequency (Hz)" },
    { "opcode": "LOAD", "value": 5000.0, "desc": "range (m)" },
    { "opcode": "SONAR_SVP", "result": 1512.3, "desc": "sound velocity (Mackenzie 1981)" },
    { "opcode": "SONAR_ABSORPTION", "result": 1.06, "desc": "absorption dB/km (FG 1982)" },
    { "opcode": "SONAR_TL", "result": 78.3, "desc": "transmission loss (dB)" },
    { "opcode": "ASSERT_GT", "expected": 0.0, "actual": 68.4, "desc": "signal excess (dB)" }
  ],
  "counterexample": null,
  "proof_hash": "sha256:a1b2c3...",
  "plato_tile_id": null
}
```

### GET /status

```json
{
  "total_verifications": 42,
  "proven": 28,
  "disproven": 12,
  "unknown": 2,
  "avg_latency_ms": 3.4
}
```

### GET /health

```json
{ "status": "ok", "version": "0.1.0" }
```

## Domains

### Sonar

Verifies acoustic detection claims using the sonar equation:

```
SE = SL - 2·TL + TS - DT
```

Where SE > 0 means detection is possible.

**Physics models used:**
- **Sound velocity**: Mackenzie 1981 equation (valid 2–30°C, 25–40‰, 0–8000m)
- **Absorption**: Francois-Garrison 1982 (Boric acid + MgSO4 relaxation + viscous)
- **Transmission loss**: Spherical spreading + absorption (`TL = 20·log₁₀(r) + α·r/1000`)
- **Signal excess**: Source level 220 dB, detection threshold 5 dB, default target strength 10 dB

**Example claims the parser understands:**

```bash
# Detectability check
curl -X POST localhost:8080/verify -H 'Content-Type: application/json' -d '{
  "claim": "at 10 kHz and 200m depth, a signal at 5 km range is detectable",
  "domain": "sonar"
}'

# With environmental parameters
curl -X POST localhost:8080/verify -H 'Content-Type: application/json' -d '{
  "claim": "50 kHz at 100m depth, 2 km range, temperature 4, salinity 35, target 15 dB",
  "domain": "sonar"
}'
```

The parser extracts frequency (kHz/Hz), depth, range (km), temperature, salinity, and target strength from natural language.

### Thermal

Verifies temperature is within safe operating bounds.

```bash
curl -X POST localhost:8080/verify -H 'Content-Type: application/json' -d '{
  "claim": "temperature 72°C is within safe range of 60 to 85°C",
  "domain": "thermal"
}'
```

Parses: temperature value + safe range ("X to Y", "between X and Y", "from X to Y").

### Generic

Numeric comparisons and range checks without domain-specific physics.

```bash
# Comparison
curl -X POST localhost:8080/verify -H 'Content-Type: application/json' -d '{
  "claim": "150 is greater than 100",
  "domain": "generic"
}'

# Range check
curl -X POST localhost:8080/verify -H 'Content-Type: application/json' -d '{
  "claim": "value 42 is between 10 and 100",
  "domain": "generic"
}'

# Tolerance bound
curl -X POST localhost:8080/verify -H 'Content-Type: application/json' -d '{
  "claim": "99.5 is within 0.5 of 100",
  "domain": "generic"
}'
```

Supported comparison operators: `>`, `>=`, `<`, `<=`, `==`, and natural language equivalents ("greater than", "at least", "is above", etc.)

## FLUX VM

The verification pipeline:

```
Natural language claim
  → Parser (domain-specific)
  → ConstraintProblem
  → Compiler
  → FLUX bytecodes
  → VM execution
  → Trace + verdict
  → Merkle hash of trace
```

**Bytecode opcodes:**

| Opcode | Description |
|--------|-------------|
| `LOAD` | Load a named value into a register |
| `SONAR_SVP` | Compute sound velocity (Mackenzie 1981) |
| `SONAR_ABSORPTION` | Compute absorption (Francois-Garrison 1982) |
| `SONAR_TL` | Compute transmission loss |
| `THERMAL_BOUND` | Check temperature against safe range |
| `GENERIC_COMPARE` | Numeric comparison (gt/gte/lt/lte/eq) |
| `GENERIC_BOUND` | Value within tolerance of center |
| `GENERIC_RANGE_CHECK` | Value within [min, max] |
| `ASSERT_GT` | Assert signal excess > 0 (sonar) |
| `ASSERT_IN_RANGE` | Assert last result margin ≥ 0 |

Each opcode produces a `TraceEntry` with the operation result. The full trace is Merkle-hashed (SHA-256) for proof integrity.

## Proof Verification

```rust
use flux_verify_api::provenance::merkle;

// Recompute the hash from a trace
let computed = merkle::hash_trace(&trace);
assert_eq!(format!("sha256:{}", computed), response.proof_hash);

// Or use the helper
merkle::verify_proof_hash(&trace, &response.proof_hash);
```

## PLATO Integration

Set `VERIFY_PLATO_URL` to automatically submit verification results as tiles to a PLATO server. The proof hash, verdict, and original claim are submitted.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VERIFY_HOST` | `0.0.0.0` | Bind address |
| `VERIFY_PORT` | `8080` | Bind port |
| `VERIFY_PLATO_URL` | — | PLATO server URL (optional) |
| `VERIFY_PLATO_TOKEN` | — | PLATO auth token (optional) |

## License

MIT
