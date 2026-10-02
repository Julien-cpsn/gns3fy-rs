use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::error::Result;

/// Overlay the keys of `update` on top of `target` (the Python `_update`): known keys are
/// replaced, unknown keys are ignored by serde, `null` clears an optional field.
pub(crate) fn merge_update<T>(target: &T, update: &Value) -> Result<T>
where
    T: Serialize + DeserializeOwned,
{
    let mut current = serde_json::to_value(target)?;
    if let (Some(cur), Some(upd)) = (current.as_object_mut(), update.as_object()) {
        for (k, v) in upd {
            cur.insert(k.clone(), v.clone());
        }
    }
    Ok(serde_json::from_value(current)?)
}

/// Serialize `value` as a JSON object without `null` entries and without the `exclude` keys,
/// i.e. the request body the Python library built from `asdict(self)`.
pub(crate) fn payload<T: Serialize>(value: &T, exclude: &[&str]) -> Result<Value> {
    let mut v = serde_json::to_value(value)?;
    if let Some(obj) = v.as_object_mut() {
        obj.retain(|k, val| !val.is_null() && !exclude.contains(&k.as_str()));
    }
    Ok(v)
}

pub(crate) fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}
