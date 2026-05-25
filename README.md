# flux-verify-api

**Natural Language Verification API** — prove or disprove claims with mathematical traces.

Post a claim in English. Get back `PROVEN` or `DISPROVEN` with a full physics trace, counterexample, and SHA-256 proof hash. Built in Rust with a custom bytecode VM, Ed25519 signing, and optional PLATO fleet integration.

## Quick Start

```bash
# Start the server
VERIFY_PORT=8080 cargo run

# Verify a sonar claim
curl -X POST http://localhost:8080/verify \
  -H "Content-Type: application/json" \
  -d '{
    "claim": "A 50kHz sonar at 200m depth can detect a 10dB target at 5km",
    "domain": "sonar",
    "rigor": "full"
  }'
```

Response:

```json
{
  "status": "DISPROVEN",
  "confidence": 0.97,
  "trace": [
    {"opcode": "LOAD", "value": 200.0, "desc": "depth (m)"},
    {"opcode": "LOAD", "value": 50000.0, "desc": "frequency (Hz)"},
    {"opcode": "SONAR_SVP", "result": 1482.3, "desc": "sound velocity (Mackenzie 1981)"},
    {"opcode": "SONAR_ABSORPTION", "result": 12.4, "desc": "absorption dB/km (FG 1982)"},
    {"opcode": "SONAR_TL", "result": 67.3, "desc": "transmission loss (dB)"},
    {"opcode": "ASSERT_GT", "expected": 0, "actual": -12.1, "desc": "signal excess (dB)"}
  ],
  "counterexample": {
    "depth_m": 200,
    "frequency_hz": 50000,
    "range_m": 5000,
    "sound_velocity_ms": 1482.3,
    "absorption_db_km": 12.4,
    "transmission_loss_db": 67.3,
    "signal_excess_db": -12.1
  },
  "proof_hash": "sha256:a4f2e8c..."
}
```

## What It Does

flux-verify-api is a **constraint verification server** that takes natural language claims, compiles them into a domain-specific bytecode, executes them in a virtual machine with real physics models, and returns a cryptographic proof of the result.

The verification pipeline:

```
Natural language claim
    ↓
Parser (NLP pattern extraction)
    ↓
ConstraintProblem (structured claim)
    ↓
Compiler (domain-specific bytecodes)
    ↓
FLUX VM (execute with physics)
    ↓
Trace + Provenance (SHA-256 Merkle + Ed25519 signature)
    ↓
Response (PROVEN / DISPROVEN / ERROR)
```

Every verification produces an auditable trace — each bytecode instruction, its inputs, and its outputs are recorded. The trace is hashed with SHA-256 and can be signed with Ed25519 for tamper-proof provenance.

## Verification Domains

### Sonar (`"domain": "sonar"`)

Full underwater acoustic detection analysis:

1. **Sound velocity profile** — Mackenzie (1981) nine-term equation for sound speed in seawater
2. **Absorption model** — Francois & Garrison (1982) three-component model (boric acid, MgSO4, pure water viscosity)
3. **Transmission loss** — Spherical spreading + frequency-dependent absorption
4. **Signal excess** — Target strength vs. accumulated losses

Example claims:
- `"A 50kHz sonar at 200m depth detects a 10dB target at 5km"`
- `"Active sonar at 12kHz can reach 50km range"`
- `"40kHz sidescan sonar absorption is less than 5dB/km"`

### Thermal (`"domain": "thermal"`)

Temperature bound checking for safe operating ranges:

- Validates temperatures against configurable safe ranges
- Reports margin (how far inside/outside bounds) and violation type
- Supports both Celsius and Kelvin contexts

Example claims:
- `"85°C is within safe operating range for electronics from -40°C to 125°C"`
- `"200°C exceeds the safe thermal boundary of 150°C"`

### Generic (`"domain": "generic"`)

General-purpose constraint verification:

- **Comparison operators**: `>`, `>=`, `<`, `<=`, `==`
- **Natural language comparisons**: "greater than", "at least", "is above", "is below"
- **Range checks**: "X is between Y and Z", "X is within [Y, Z]"
- **Bound checks**: "X is within Y of Z"

Example claims:
- `"37 is greater than 20"`
- `"100 is between 0 and 200"`
- `"5.5 is at least 3.0"`

## Architecture

### Bytecode VM

The FLUX VM executes domain-specific bytecodes:

| Opcode | Domain | Description |
|--------|--------|-------------|
| `LOAD` | All | Load a named value into a VM register |
| `SONAR_SVP` | Sonar | Compute sound velocity (Mackenzie 1981) |
| `SONAR_ABSORPTION` | Sonar | Compute absorption dB/km (FG 1982) |
| `SONAR_TL` | Sonar | Compute transmission loss (dB) |
| `THERMAL_BOUND` | Thermal | Check temperature against safe bounds |
| `GENERIC_COMPARE` | Generic | Compare two values with an operator |
| `GENERIC_BOUND` | Generic | Check value within [min, max] |
| `GENERIC_RANGE_CHECK` | Generic | Verify value in range |
| `ASSERT_GT` | All | Assert a condition holds (> 0) |

### Natural Language Parser

The parser extracts structured data from English claims using pattern matching:

- Number extraction: `"50kHz"` → `50000.0`, `"200m"` → `200.0`
- Range extraction: `"between X and Y"`, `"from X to Y"`
- Comparison extraction: `"greater than X"`, `"at least X"`, `"X > Y"`
- Unit-aware parsing: kHz, Hz, m, km, °C, dB

### Cryptographic Provenance

Every verification is cryptographically anchored:

1. **SHA-256 fingerprint** of the bytecode blob
2. **Ed25519 signature** over `(fingerprint || timestamp)` using `ed25519-dalek`
3. **Tamper detection** — any bytecode modification invalidates the signature

```rust
use flux_verify_api::signing::{sign_bytecode, verify_bytecode};

let sig = sign_bytecode(&bytecode, &private_key, None);
assert!(verify_bytecode(&bytecode, &sig, &public_key).is_ok());

// Tampered bytecode fails verification
let mut tampered = bytecode.clone();
tampered[5] ^= 0xFF;
assert!(verify_bytecode(&tampered, &sig, &public_key).is_err());
```

### PLATO Integration

When configured, verified results are submitted as tiles to the PLATO fleet coordination system:

- **Automatic tile submission** after each verification
- **Room routing** based on claim domain
- **Provenance chain** — each tile links back to its verification trace

## API Reference

### `POST /verify`

Verify a natural language claim.

**Request:**
```json
{
  "claim": "string — the claim to verify (English)",
  "domain": "sonar | thermal | generic",
  "rigor": "full | quick"
}
```

**Response:**
```json
{
  "status": "PROVEN | DISPROVEN | ERROR",
  "confidence": 0.97,
  "trace": [{"opcode": "...", "value": ..., "result": ..., "desc": "..."}],
  "counterexample": { ... },
  "proof_hash": "sha256:...",
  "signature": { "sig": "hex", "fingerprint": "hex", "timestamp": 1234567890 }
}
```

### `GET /status`

Verification statistics: total claims processed, proven/disproven/error counts.

### `GET /health`

Health check endpoint for load balancers and monitoring.

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `VERIFY_HOST` | `0.0.0.0` | Bind host |
| `VERIFY_PORT` | `8080` | Bind port |
| `VERIFY_PLATO_URL` | (none) | PLATO endpoint for automatic tile submission |
| `VERIFY_PLATO_TOKEN` | (none) | PLATO authentication token |

## Physics References

- **Mackenzie (1981)**: Nine-term equation for sound speed in seawater as a function of depth, temperature, and salinity
- **Francois & Garrison (1982)**: Three-component absorption model accounting for boric acid relaxation, magnesium sulfate relaxation, and pure water viscosity

## Development

```bash
cargo build
cargo test        # unit + integration + signing tests
cargo run         # start server on :8080
```

## Related SuperInstance Repos

| Repo | Description |
|------|-------------|
| [flux-vm-v3](https://github.com/SuperInstance/flux-vm-v3) | Full FLUX bytecode VM for constraint verification |
| [flux-check-js](https://github.com/SuperInstance/flux-check-js) | TypeScript constraint engine with fracture-coalesce |
| [flux-lib-py](https://github.com/SuperInstance/flux-lib-py) | Python constraint engine with thermodynamic analysis |
| [constraint-theory-core](https://github.com/SuperInstance/constraint-theory-core) | Core math primitives (Rust) |
| [plato-core](https://github.com/SuperInstance/plato-core) | PLATO room/tile types and mesh registry |
| [quality-gate-stream](https://github.com/SuperInstance/quality-gate-stream) | Quality scoring pipeline that consumes verification results |
| [fleet-stack](https://github.com/SuperInstance/fleet-stack) | Docker deployment including verification API |

## License

MIT
