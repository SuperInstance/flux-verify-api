use flux_verify_api::compiler;
use flux_verify_api::engine::vm::FluxVm;
use flux_verify_api::provenance::merkle;

use axum::Router;
use std::sync::Arc;
use tokio::sync::Mutex;

// Helper to build a test app
fn test_app() -> Router {
    use flux_verify_api::api::routes::{self, AppState};
    use flux_verify_api::config::Config;
    let state = Arc::new(Mutex::new(AppState::new(Config::from_env())));
    routes::router().with_state(state)
}

// ── HTTP Endpoint Tests ──

#[tokio::test]
async fn test_health_endpoint() {
    let app = test_app();
    let resp = axum_test::TestServer::new(app)
        .unwrap()
        .get("/health")
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], "0.1.0");
}

#[tokio::test]
async fn test_status_endpoint_initial() {
    let app = test_app();
    let resp = axum_test::TestServer::new(app)
        .unwrap()
        .get("/status")
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["total_verifications"], 0);
    assert_eq!(body["proven"], 0);
    assert_eq!(body["disproven"], 0);
    assert_eq!(body["unknown"], 0);
    assert_eq!(body["avg_latency_ms"], 0.0);
}

#[tokio::test]
async fn test_verify_endpoint_generic_proven() {
    let app = test_app();
    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/verify")
        .json(&serde_json::json!({
            "claim": "10 is greater than 5",
            "domain": "generic",
            "rigor": "standard"
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "PROVEN");
    assert!(body["confidence"].as_f64().unwrap() > 0.0);
    assert!(body["proof_hash"].as_str().unwrap().starts_with("sha256:"));
}

#[tokio::test]
async fn test_verify_endpoint_generic_disproven() {
    let app = test_app();
    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/verify")
        .json(&serde_json::json!({
            "claim": "5 is greater than 10",
            "domain": "generic"
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "DISPROVEN");
}

#[tokio::test]
async fn test_verify_endpoint_thermal_proven() {
    let app = test_app();
    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/verify")
        .json(&serde_json::json!({
            "claim": "temperature 45°C is within safe range of 20°C to 80°C",
            "domain": "thermal"
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "PROVEN");
}

#[tokio::test]
async fn test_verify_endpoint_thermal_disproven() {
    let app = test_app();
    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/verify")
        .json(&serde_json::json!({
            "claim": "temperature 95°C is within safe range of 20°C to 80°C",
            "domain": "thermal"
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "DISPROVEN");
    assert!(body["counterexample"].is_object());
}

#[tokio::test]
async fn test_verify_endpoint_invalid_domain() {
    let app = test_app();
    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/verify")
        .json(&serde_json::json!({
            "claim": "test claim",
            "domain": "quantum"
        }))
        .await;
    resp.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_verify_endpoint_sonar() {
    let app = test_app();
    let server = axum_test::TestServer::new(app).unwrap();
    let resp = server
        .post("/verify")
        .json(&serde_json::json!({
            "claim": "A 1kHz sonar at 100m depth can detect a 10dB target at 2km",
            "domain": "sonar"
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["status"], "PROVEN");
}

#[tokio::test]
async fn test_status_after_verifications() {
    let app = test_app();
    let server = axum_test::TestServer::new(app).unwrap();

    // Do a verification first
    server
        .post("/verify")
        .json(&serde_json::json!({
            "claim": "10 > 5",
            "domain": "generic"
        }))
        .await;

    let resp = server.get("/status").await;
    let body: serde_json::Value = resp.json();
    assert_eq!(body["total_verifications"], 1);
    assert_eq!(body["proven"], 1);
    assert!(body["avg_latency_ms"].as_f64().unwrap() > 0.0);
}

// ── Serde Tests for Request/Response ──

#[test]
fn test_verify_request_serde_roundtrip() {
    use flux_verify_api::api::request::VerifyRequest;
    let req = VerifyRequest {
        claim: "test claim".into(),
        domain: "generic".into(),
        rigor: "standard".into(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: VerifyRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.claim, "test claim");
    assert_eq!(parsed.domain, "generic");
}

#[test]
fn test_verify_request_defaults() {
    use flux_verify_api::api::request::VerifyRequest;
    let json = r#"{"claim":"test"}"#;
    let parsed: VerifyRequest = serde_json::from_str(json).unwrap();
    assert_eq!(parsed.domain, "generic");
    assert_eq!(parsed.rigor, "standard");
}

#[test]
fn test_verify_response_serde_roundtrip() {
    use flux_verify_api::api::response::{TraceEntry, VerifyResponse};
    let resp = VerifyResponse {
        status: "PROVEN".into(),
        confidence: 0.95,
        trace: vec![TraceEntry {
            opcode: "LOAD".into(),
            value: Some(42.0),
            result: None,
            expected: None,
            actual: None,
            desc: "test".into(),
        }],
        counterexample: None,
        proof_hash: "sha256:abc123".into(),
        plato_tile_id: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: VerifyResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.status, "PROVEN");
    assert_eq!(parsed.trace.len(), 1);
    assert!(parsed.counterexample.is_none());
}

#[test]
fn test_health_response_serde() {
    use flux_verify_api::api::response::HealthResponse;
    let resp = HealthResponse {
        status: "ok".into(),
        version: "0.1.0".into(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: HealthResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.status, "ok");
}

#[test]
fn test_status_response_serde() {
    use flux_verify_api::api::response::StatusResponse;
    let resp = StatusResponse {
        total_verifications: 10,
        proven: 7,
        disproven: 2,
        unknown: 1,
        avg_latency_ms: 42.5,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: StatusResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.total_verifications, 10);
    assert_eq!(parsed.avg_latency_ms, 42.5);
}

// ── Sonar Domain Tests ──

#[test]
fn test_sonar_50khz_disproven() {
    // "A 50kHz sonar at 200m depth can detect a 10dB target at 5km"
    // This should be DISPROVEN — 50kHz has too much absorption at 5km
    let problem = compiler::parse_claim(
        "A 50kHz sonar at 200m depth can detect a 10dB target at 5km",
        "sonar",
    )
    .expect("should parse");

    assert_eq!(problem.domain, "sonar");
    assert_eq!(problem.variables.len(), 4);

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (verdict, confidence, counterexample) = vm.evaluate(&trace, &problem);

    assert_eq!(verdict, "DISPROVEN");
    assert!(confidence > 0.9);
    assert!(counterexample.is_some());
    let ce = counterexample.unwrap();
    assert!(ce["signal_excess_db"].as_f64().unwrap() < 0.0);
}

#[test]
fn test_sonar_1khz_proven() {
    // Low frequency sonar should be able to detect at moderate range
    let problem = compiler::parse_claim(
        "A 1kHz sonar at 100m depth can detect a 10dB target at 2km",
        "sonar",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (verdict, _confidence, counterexample) = vm.evaluate(&trace, &problem);

    // 1kHz at 2km should be PROVEN (low absorption)
    assert_eq!(verdict, "PROVEN");
    assert!(counterexample.is_none());
}

#[test]
fn test_sonar_trace_has_physics() {
    let problem = compiler::parse_claim(
        "A 10kHz sonar at 50m depth can detect a 15dB target at 1km",
        "sonar",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);

    let opcodes: Vec<&str> = trace.iter().map(|e| e.opcode.as_str()).collect();
    assert!(opcodes.contains(&"LOAD"), "trace should have LOAD ops");
    assert!(
        opcodes.contains(&"SONAR_SVP"),
        "trace should have SONAR_SVP"
    );
    assert!(
        opcodes.contains(&"SONAR_ABSORPTION"),
        "trace should have SONAR_ABSORPTION"
    );
    assert!(opcodes.contains(&"SONAR_TL"), "trace should have SONAR_TL");
    assert!(
        opcodes.contains(&"ASSERT_GT"),
        "trace should have ASSERT_GT"
    );
}

#[test]
fn test_sonar_mackenzie_velocity() {
    let problem = compiler::parse_claim(
        "A 5kHz sonar at 200m depth can detect a 10dB target at 3km",
        "sonar",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);

    // Find the SVP entry and check the result is reasonable
    let svp = trace
        .iter()
        .find(|e| e.opcode == "SONAR_SVP")
        .expect("should have SVP");
    let sv = svp.result.unwrap();
    assert!(
        sv > 1450.0 && sv < 1550.0,
        "Sound velocity {} should be ~1480-1520",
        sv
    );
}

// ── Thermal Domain Tests ──

#[test]
fn test_thermal_in_range() {
    let problem = compiler::parse_claim(
        "temperature 45°C is within safe range of 20°C to 80°C",
        "thermal",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (verdict, _confidence, counterexample) = vm.evaluate(&trace, &problem);

    assert_eq!(verdict, "PROVEN");
    assert!(counterexample.is_none());
}

#[test]
fn test_thermal_out_of_range() {
    let problem = compiler::parse_claim(
        "temperature 95°C is within safe range of 20°C to 80°C",
        "thermal",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (verdict, _confidence, counterexample) = vm.evaluate(&trace, &problem);

    assert_eq!(verdict, "DISPROVEN");
    assert!(counterexample.is_some());
}

// ── Generic Domain Tests ──

#[test]
fn test_generic_gt_proven() {
    let problem = compiler::parse_claim("10 is greater than 5", "generic").expect("should parse");
    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (verdict, _, _) = vm.evaluate(&trace, &problem);
    assert_eq!(verdict, "PROVEN");
}

#[test]
fn test_generic_lt_disproven() {
    let problem = compiler::parse_claim("10 is less than 5", "generic").expect("should parse");
    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (verdict, _, _) = vm.evaluate(&trace, &problem);
    assert_eq!(verdict, "DISPROVEN");
}

#[test]
fn test_generic_operator_direct() {
    let problem = compiler::parse_claim("100 > 50", "generic").expect("should parse");
    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (verdict, _, _) = vm.evaluate(&trace, &problem);
    assert_eq!(verdict, "PROVEN");
}

// ── Merkle Provenance Tests ──

#[test]
fn test_merkle_deterministic() {
    let problem = compiler::parse_claim(
        "A 10kHz sonar at 100m depth can detect a 10dB target at 2km",
        "sonar",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);

    let mut vm1 = FluxVm::new();
    let trace1 = vm1.execute(&bytecodes);

    let mut vm2 = FluxVm::new();
    let trace2 = vm2.execute(&bytecodes);

    let hash1 = merkle::hash_trace(&trace1);
    let hash2 = merkle::hash_trace(&trace2);
    assert_eq!(hash1, hash2, "Same inputs should produce same hash");
}

#[test]
fn test_merkle_different_claims() {
    let problem1 = compiler::parse_claim(
        "A 10kHz sonar at 100m depth can detect a 10dB target at 2km",
        "sonar",
    )
    .expect("should parse");
    let problem2 = compiler::parse_claim(
        "A 50kHz sonar at 200m depth can detect a 10dB target at 5km",
        "sonar",
    )
    .expect("should parse");

    let bc1 = compiler::compile(&problem1);
    let bc2 = compiler::compile(&problem2);

    let mut vm1 = FluxVm::new();
    let mut vm2 = FluxVm::new();
    let t1 = vm1.execute(&bc1);
    let t2 = vm2.execute(&bc2);

    assert_ne!(merkle::hash_trace(&t1), merkle::hash_trace(&t2));
}

#[test]
fn test_proof_hash_format() {
    let problem = compiler::parse_claim(
        "A 5kHz sonar at 50m depth can detect a 10dB target at 1km",
        "sonar",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let hash = merkle::hash_trace(&trace);

    // SHA-256 hex should be 64 characters
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── Parser Tests ──

#[test]
fn test_unknown_domain() {
    let result = compiler::parse_claim("test", "quantum");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unknown domain"));
}

#[test]
fn test_sonar_counterexample_fields() {
    let problem = compiler::parse_claim(
        "A 50kHz sonar at 200m depth can detect a 10dB target at 5km",
        "sonar",
    )
    .expect("should parse");

    let bytecodes = compiler::compile(&problem);
    let mut vm = FluxVm::new();
    let trace = vm.execute(&bytecodes);
    let (_, _, ce) = vm.evaluate(&trace, &problem);

    let ce = ce.unwrap();
    assert!(ce.get("depth_m").is_some());
    assert!(ce.get("frequency_hz").is_some());
    assert!(ce.get("range_m").is_some());
    assert!(ce.get("sound_velocity_ms").is_some());
    assert!(ce.get("transmission_loss_db").is_some());
    assert!(ce.get("signal_excess_db").is_some());
}
