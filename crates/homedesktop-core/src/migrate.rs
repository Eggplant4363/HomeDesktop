//! 旧版布局迁移（v1/v2 → v3）：补齐 kind/坐标。

use crate::schema::*;

fn rects_overlap(a: (u16, u16, u16, u16), b: (u16, u16, u16, u16)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
}

/// 在 occupied 矩形集中找 w×h 的首个空位（行优先扫描；x+w 超出列宽则换行）
fn find_free_slot(
    occupied: &[(u16, u16, u16, u16)],
    cols: u16,
    w: u16,
    h: u16,
) -> Option<(u16, u16)> {
    for y in 0..MAX_ROWS {
        for x in 0..cols {
            if x + w > cols {
                continue;
            }
            let rect = (x, y, w, h);
            if !occupied.iter().any(|o| rects_overlap(rect, *o)) {
                return Some((x, y));
            }
        }
    }
    None
}

fn cell_size_from_json(obj: &serde_json::Map<String, serde_json::Value>) -> (u16, u16) {
    let size = obj.get("size");
    let w = size
        .and_then(|s| s.get("w"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1) as u16;
    let h = size
        .and_then(|s| s.get("h"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1) as u16;
    (w, h)
}

/// 给缺 x/y 的单元分配坐标（cols 列虚拟网格，行优先找空位）；幂等
fn assign_positions(cells: &mut [serde_json::Value], cols: u16) {
    let mut occupied: Vec<(u16, u16, u16, u16)> = Vec::new();
    for cell in cells {
        let Some(obj) = cell.as_object_mut() else {
            continue;
        };
        let (w, h) = cell_size_from_json(obj);
        match (
            obj.get("x").and_then(|v| v.as_u64()),
            obj.get("y").and_then(|v| v.as_u64()),
        ) {
            (Some(x), Some(y)) => {
                occupied.push((x as u16, y as u16, w, h));
            }
            _ => {
                if let Some((x, y)) = find_free_slot(&occupied, cols, w, h) {
                    obj.insert("x".into(), serde_json::json!(x));
                    obj.insert("y".into(), serde_json::json!(y));
                    occupied.push((x, y, w, h));
                }
            }
        }
        // 文件夹内部图标同样分配坐标（FOLDER_COLS 列）
        if let Some(items) = obj.get_mut("items").and_then(|i| i.as_array_mut()) {
            let mut focc: Vec<(u16, u16, u16, u16)> = Vec::new();
            for item in items {
                let Some(iobj) = item.as_object_mut() else {
                    continue;
                };
                let (iw, ih) = cell_size_from_json(iobj);
                match (
                    iobj.get("x").and_then(|v| v.as_u64()),
                    iobj.get("y").and_then(|v| v.as_u64()),
                ) {
                    (Some(x), Some(y)) => {
                        focc.push((x as u16, y as u16, iw, ih));
                    }
                    _ => {
                        if let Some((x, y)) = find_free_slot(&focc, FOLDER_COLS, iw, ih) {
                            iobj.insert("x".into(), serde_json::json!(x));
                            iobj.insert("y".into(), serde_json::json!(y));
                            focc.push((x, y, iw, ih));
                        }
                    }
                }
            }
        }
    }
}

/// v1 → v2 → v3 迁移：v1 图标项无 `kind`（补 `kind:"icon"`）；v3 起为自由摆放，
/// 给缺 `x`/`y` 的单元分配虚拟网格坐标。幂等：对任意输入执行都是安全的。
pub fn migrate_layout(value: &mut serde_json::Value) {
    if let Some(pages) = value.get_mut("pages").and_then(|p| p.as_array_mut()) {
        for page in pages {
            if let Some(cells) = page.as_array_mut() {
                // v1 → v2：kind 补全
                for cell in cells.iter_mut() {
                    if let Some(obj) = cell.as_object_mut() {
                        if !obj.contains_key("kind") {
                            obj.insert("kind".into(), serde_json::json!("icon"));
                        }
                    }
                }
                // v2 → v3：自由摆放坐标
                assign_positions(cells, PAGE_COLS);
            }
        }
    }
    let version = value.get("version").and_then(|x| x.as_u64()).unwrap_or(0);
    if version < 3 {
        value["version"] = serde_json::json!(3);
    }
}

