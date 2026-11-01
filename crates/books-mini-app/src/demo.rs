//! The synthetic demo book (`fixtures/demo/*.json`): a list of Books tool
//! calls made as the app, replayed into a store. It seeds the test the Bot for
//! the screenshots and the golden report test (M3). Synthetic data only.

use serde_json::Value;

use crate::service::{Books, Origin};
use crate::tools;

/// Replay every step; stop at the first refusal. Returns the step count.
pub fn seed(books: &mut Books, demo: &Value) -> Result<usize, String> {
    let steps = demo["steps"].as_array().ok_or("a demo has steps[]")?;
    for (i, step) in steps.iter().enumerate() {
        let tool = step["tool"].as_str().ok_or("a step names its tool")?;
        let args = step["args"].as_object().ok_or("a step has args")?;
        let out = tools::call(tool, args, Origin::App, books)
            .ok_or_else(|| format!("step {}: no tool {tool}", i + 1))?;
        if out["isError"] == true {
            return Err(format!(
                "step {} ({tool}): {}",
                i + 1,
                out["structuredContent"]["message"]
                    .as_str()
                    .unwrap_or("refused")
            ));
        }
    }
    Ok(steps.len())
}
