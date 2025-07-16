use crate::{simulate, SimRequest};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::{wrap_pyfunction, Bound}; // ← 追加
use serde_json;

// ─── Rust ロジック ──────────────────────────────────
fn simulate_impl(json_text: &str) -> PyResult<String> {
    let req: SimRequest =
        serde_json::from_str(json_text).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let resp = simulate(req);
    serde_json::to_string(&resp).map_err(|e| PyValueError::new_err(e.to_string()))
}

// ─── Python から直接呼ぶ関数 ─────────────────────────
#[pyfunction]
fn simulate_py(json_text: &str) -> PyResult<String> {
    simulate_impl(json_text)
}

// ─── モジュール初期化関数 ────────────────────────────
//            ↓↓↓ ここを &Bound<'_, PyModule> に変更
#[pymodule]
fn redstonesim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(simulate_py, m)?);
    Ok(())
}
