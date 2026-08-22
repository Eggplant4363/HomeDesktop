//! HomeAssistant 插件：通过 HA REST API 读取实体状态 + 控制开关/灯/风扇等
//! 配置（widget 设置）：url（HA 地址）、token（长期访问令牌）、entities（实体 ID 逗号分隔）
//! 复用 fetch_agent（跳过证书校验，兼容自签名 https 的 HA）

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HaState {
    pub entity_id: String,
    pub state: String,
    pub friendly_name: Option<String>,
    /// 单位（传感器）
    pub unit: Option<String>,
    /// 实体域（light/switch/sensor/...）
    pub domain: String,
}

#[derive(Debug, Deserialize)]
struct RawState {
    entity_id: String,
    state: String,
    attributes: serde_json::Value,
}

const HA_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(6);

/// 查询实体状态：一次拉取全部状态再按请求的实体过滤（比逐实体请求高效）
#[tauri::command]
pub async fn ha_states(
    url: String,
    token: String,
    entities: Vec<String>,
) -> Result<Vec<HaState>, String> {
    let base = url.trim_end_matches('/').to_string();
    let agent = crate::plugins::fetch_agent();
    tauri::async_runtime::spawn_blocking(move || {
        let res = agent
            .get(&format!("{base}/api/states"))
            .set("Authorization", &format!("Bearer {token}"))
            .timeout(HA_TIMEOUT)
            .call()
            .map_err(|e| format!("连接 HomeAssistant 失败: {e}"))?;
        let all: Vec<RawState> = res
            .into_json()
            .map_err(|e| format!("解析状态失败: {e}"))?;
        let mut out: Vec<HaState> = Vec::new();
        for raw in all {
            if entities.iter().any(|e| e == &raw.entity_id) {
                out.push(HaState {
                    domain: raw
                        .entity_id
                        .split_once('.')
                        .map(|(d, _)| d.to_string())
                        .unwrap_or_default(),
                    entity_id: raw.entity_id,
                    state: raw.state,
                    friendly_name: raw
                        .attributes
                        .get("friendly_name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    unit: raw
                        .attributes
                        .get("unit_of_measurement")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
        Ok(out)
    })
    .await
    .map_err(|e| format!("线程错误: {e}"))?
}

/// 获取指定域（或全部）的实体列表（设置菜单用：勾选实例，不需指定 entities）
#[tauri::command]
pub async fn ha_entities(
    url: String,
    token: String,
    domain: Option<String>,
) -> Result<Vec<HaState>, String> {
    let base = url.trim_end_matches('/').to_string();
    let agent = crate::plugins::fetch_agent();
    tauri::async_runtime::spawn_blocking(move || {
        let res = agent
            .get(&format!("{base}/api/states"))
            .set("Authorization", &format!("Bearer {token}"))
            .timeout(HA_TIMEOUT)
            .call()
            .map_err(|e| format!("连接 HomeAssistant 失败: {e}"))?;
        let status = res.status();
        if !(200..300).contains(&status) {
            return Err(format!("HomeAssistant 返回错误: HTTP {status}"));
        }
        let all: Vec<RawState> = res
            .into_json()
            .map_err(|e| format!("解析状态失败: {e}"))?;
        let prefix = domain.map(|d| format!("{d}."));
        let out: Vec<HaState> = all
            .into_iter()
            .filter(|raw| prefix.as_deref().map_or(true, |p| raw.entity_id.starts_with(p)))
            .map(|raw| HaState {
                domain: raw
                    .entity_id
                    .split_once('.')
                    .map(|(d, _)| d.to_string())
                    .unwrap_or_default(),
                entity_id: raw.entity_id,
                state: raw.state,
                friendly_name: raw
                    .attributes
                    .get("friendly_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                unit: raw
                    .attributes
                    .get("unit_of_measurement")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            })
            .collect();
        Ok(out)
    })
    .await
    .map_err(|e| format!("线程错误: {e}"))?
}
/// 调用 HA 服务（如 light/toggle、switch/turn_on）
#[tauri::command]
pub async fn ha_call(
    url: String,
    token: String,
    domain: String,
    service: String,
    entity_id: String,
) -> Result<(), String> {
    let base = url.trim_end_matches('/').to_string();
    let agent = crate::plugins::fetch_agent();
    tauri::async_runtime::spawn_blocking(move || {
        let body = serde_json::json!({ "entity_id": entity_id });
        let res = agent
            .post(&format!("{base}/api/services/{domain}/{service}"))
            .set("Authorization", &format!("Bearer {token}"))
            .timeout(HA_TIMEOUT)
            .send_json(body)
            .map_err(|e| format!("控制失败: {e}"))?;
        let status = res.status();
        if !(200..300).contains(&status) {
            return Err(format!("HomeAssistant 返回错误: HTTP {status}"));
        }
        Ok(())
    })
    .await
    .map_err(|e| format!("线程错误: {e}"))?
}
