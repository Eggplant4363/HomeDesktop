//! 布局持久化与备份合并。

use crate::migrate::migrate_layout;
use crate::schema::*;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

// ---------- 布局持久化 ----------

pub fn read_layout_from(path: &Path) -> Option<Layout> {
    let text = fs::read_to_string(path).ok()?;
    let mut value: serde_json::Value = serde_json::from_str(&text).ok()?;
    migrate_layout(&mut value);
    serde_json::from_value(value).ok()
}

pub fn write_layout_to(path: &Path, layout: &Layout) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(layout).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

/// 合并布局（M13 合并导入）：保留当前布局，把备份中"当前不存在（按 id 去重，含文件夹内）"的单元
/// 追加到对应页；备份页数超出当前页数时追加为新页。纯函数，便于单测。
pub fn merge_layout(current: &serde_json::Value, backup: &serde_json::Value) -> serde_json::Value {
    
    let cur_pages = current
        .get("pages")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();
    let bak_pages = backup
        .get("pages")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out: Vec<Value> = cur_pages;
    let mut existing: HashSet<String> = HashSet::new();
    for page in &out {
        collect_cell_ids(page, &mut existing);
    }

    for (i, bpage) in bak_pages.iter().enumerate() {
        let new_cells: Vec<Value> = bpage
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|c| {
                        let id = c.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        !id.is_empty() && existing.insert(id.to_string())
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if i < out.len() {
            if let Some(page) = out[i].as_array_mut() {
                page.extend(new_cells);
            }
        } else if !new_cells.is_empty() {
            out.push(Value::Array(new_cells));
        }
    }

    let mut result = current.clone();
    result["pages"] = Value::Array(out);
    result
}

/// 收集单元 id（页面图标 + 文件夹内图标）到集合
fn collect_cell_ids(page: &serde_json::Value, out: &mut HashSet<String>) {
    let Some(cells) = page.as_array() else { return };
    for cell in cells {
        if let Some(id) = cell.get("id").and_then(|v| v.as_str()) {
            out.insert(id.to_string());
        }
        if let Some(items) = cell.get("items").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    out.insert(id.to_string());
                }
            }
        }
    }
}

/// 合并配置（M13 合并导入）：以备份键覆盖，保留当前独有键
pub fn merge_config(current: &serde_json::Value, backup: &serde_json::Value) -> serde_json::Value {
    let mut out = current.as_object().cloned().unwrap_or_default();
    if let Some(b) = backup.as_object() {
        for (k, v) in b {
            out.insert(k.clone(), v.clone());
        }
    }
    serde_json::Value::Object(out)
}

