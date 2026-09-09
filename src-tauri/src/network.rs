//! 网络监控 v5：managed Networks 在两次调用间留存累计流量，
//! refresh 后取各网卡 received/transmitted，与上次快照求 bytes/s。

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use sysinfo::Networks;
use tauri::State;

#[derive(Default)]
pub struct NetState {
    nets: Mutex<Option<Networks>>,
    prev: Mutex<HashMap<String, (u64, u64, u128)>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetStat {
    iface: String,
    rx: f64,
    tx: f64,
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[tauri::command]
pub fn net_speed(state: State<'_, NetState>) -> Vec<NetStat> {
    let created = {
        let mut o = state.nets.lock().unwrap();
        if o.is_none() {
            let mut n = Networks::new();
            n.refresh_list();
            *o = Some(n);
            true
        } else {
            false
        }
    };

    let mut np = state.nets.lock().unwrap();
    let nets = np.as_mut().unwrap();
    nets.refresh_list();
    nets.refresh();

    let at = now_ms();
    let mut p = state.prev.lock().unwrap();
    if created {
        // 首次无基线，只记录
        for (name, data) in nets.iter() {
            p.insert(name.to_string(), (data.received(), data.transmitted(), at));
        }
        return Vec::new();
    }

    let mut out = Vec::new();
    for (name, data) in nets.iter() {
        let low = name.trim_end().to_lowercase();
        if name.is_empty()
            || low.contains("loopback")
            || low.starts_with("lo")
            || low.starts_with("bluetooth")
        {
            continue;
        }
        let rx = data.received();
        let tx = data.transmitted();
        if let Some(&(prx, ptx, pat)) = p.get(name) {
            let ms = at.saturating_sub(pat).max(100);
            out.push(NetStat {
                iface: name.to_string(),
                rx: rx.saturating_sub(prx) as f64 / (ms as f64 / 1000.0),
                tx: tx.saturating_sub(ptx) as f64 / (ms as f64 / 1000.0),
            });
        }
        p.insert(name.to_string(), (rx, tx, at));
    }
    out
}
