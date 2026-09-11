//! HomeDesktop 纯逻辑核心：插件 manifest 解析、插件扫描、布局序列化、动作分发。
//!
//! 本 crate 不依赖 tauri / WebView2，保证可以在任意环境直接 `cargo test`。
//!
//! 阶段4 重构：原先集中在本文件的实现已按职责拆分为子模块，
//! 本文件只做模块声明与聚合 re-export（对外 API 路径保持不变）。

pub mod action;
pub mod discovery;
pub mod png;
pub mod migrate;
pub mod persist;
pub mod schema;

pub use action::{execute_action, launch_plugin_action};
pub use discovery::{collect_plugins, collect_plugins_with_builtin, scan_plugin_dir};
pub use png::encode_rgba_png;
pub use migrate::migrate_layout;
pub use persist::{merge_config, merge_layout, read_layout_from, write_layout_to};
pub use schema::*;

// ---------- 单元测试 ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_DIR_SEQ: AtomicU32 = AtomicU32::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "homedesktop-test-{}-{}-{}",
            name,
            std::process::id(),
            seq
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn sample_manifest() -> &'static str {
        r#"{
            "id": "dev.homedesktop.demo",
            "name": "Demo",
            "version": "0.1.0",
            "type": "icon",
            "emoji": "🧪",
            "actions": [{ "kind": "app", "path": "C:\\Windows\\notepad.exe" }]
        }"#
    }

    #[test]
    fn manifest_parses_valid_json() {
        let m: Manifest = serde_json::from_str(sample_manifest()).expect("valid manifest");
        assert_eq!(m.id, "dev.homedesktop.demo");
        assert_eq!(m.plugin_type, "icon");
        assert_eq!(m.emoji.as_deref(), Some("🧪"));
        assert_eq!(m.actions.len(), 1);
        assert_eq!(m.actions[0].kind, ActionKind::App);
        assert_eq!(
            m.actions[0].path.as_deref(),
            Some(r"C:\Windows\notepad.exe")
        );
    }

    #[test]
    fn manifest_rejects_invalid_json() {
        assert!(serde_json::from_str::<Manifest>("{ not json").is_err());
        // 缺少必填字段
        assert!(serde_json::from_str::<Manifest>(r#"{"id":"x"}"#).is_err());
    }

    #[test]
    fn scan_plugin_dir_skips_invalid_and_non_manifest() {
        let dir = temp_dir("scan");
        fs::create_dir_all(dir.join("good")).unwrap();
        fs::create_dir_all(dir.join("bad")).unwrap();
        fs::create_dir_all(dir.join("no-manifest")).unwrap();
        fs::write(dir.join("good/manifest.json"), sample_manifest()).unwrap();
        fs::write(dir.join("bad/manifest.json"), "{ broken").unwrap();
        // no-manifest 目录没有 manifest.json，应被跳过

        let mut seen = HashSet::new();
        let mut out = Vec::new();
        scan_plugin_dir(&dir, &mut seen, &mut out);

        assert_eq!(out.len(), 1, "只有合法 manifest 被收集");
        assert_eq!(out[0].id, "dev.homedesktop.demo");
    }

    #[test]
    fn collect_plugins_dedupes_by_id_with_later_priority() {
        let low = temp_dir("low");
        let high = temp_dir("high");
        fs::create_dir_all(low.join("p1")).unwrap();
        fs::create_dir_all(high.join("p1")).unwrap();
        // 同一 id，高优先级目录（列表靠前）版本号更高
        fs::write(
            high.join("p1/manifest.json"),
            sample_manifest().replace("0.1.0", "0.2.0-high"),
        )
        .unwrap();
        fs::write(
            low.join("p1/manifest.json"),
            sample_manifest().replace("0.1.0", "0.1.0-low"),
        )
        .unwrap();

        // 列表顺序 = 优先级：靠前的目录先扫描、先到先得
        let plugins = collect_plugins(&[high.clone(), low.clone()]);
        assert_eq!(plugins.len(), 1, "重复 id 只保留一个");
        assert_eq!(plugins[0].version, "0.2.0-high", "高优先级目录（靠前）胜出");
    }

    #[test]
    fn collect_plugins_marks_builtin_by_dir_index() {
        use std::collections::HashMap;
        let user = temp_dir("u");
        let res = temp_dir("r");
        fs::create_dir_all(user.join("p1")).unwrap();
        fs::create_dir_all(res.join("p1")).unwrap();
        fs::create_dir_all(res.join("p2")).unwrap();
        fs::write(user.join("p1/manifest.json"), sample_manifest()).unwrap();
        fs::write(res.join("p1/manifest.json"), sample_manifest()).unwrap();
        fs::write(
            res.join("p2/manifest.json"),
            sample_manifest().replace("dev.homedesktop.demo", "dev.homedesktop.demo2"),
        )
        .unwrap();

        // 索引 0 = 用户数据目录（非内置）；索引 1+ = 资源/内置目录（内置）
        let list = collect_plugins_with_builtin(&[user, res]);
        let m: HashMap<String, bool> = list.into_iter().map(|(p, b)| (p.id, b)).collect();
        assert_eq!(m.get("dev.homedesktop.demo"), Some(&false), "用户目录插件非内置");
        assert_eq!(m.get("dev.homedesktop.demo2"), Some(&true), "资源目录插件为内置");
    }

    #[test]
    fn layout_roundtrip_via_file() {
        let dir = temp_dir("layout");
        let path = dir.join("layout.json");
        let layout = Layout {
            version: 3,
            pages: vec![vec![
                Cell::Icon {
                    id: "item-1".into(),
                    plugin_id: "dev.homedesktop.demo".into(),
                    title: "Demo".into(),
                    size: Size { w: 1, h: 1 },
                    action: None,
                    emoji: None,
                    color: None,
                    icon_path: None,
                    icon_image: None,
            show_label: None,
                    x: Some(0),
                    y: Some(0),
                },
                Cell::Folder {
                    id: "folder-1".into(),
                    name: "工具".into(),
                    emoji: "📁".into(),
                    items: vec![IconItem {
                        id: "item-2".into(),
                        plugin_id: "dev.homedesktop.demo".into(),
                        title: "内部图标".into(),
                        size: Size { w: 1, h: 1 },
                        action: None,
                        emoji: None,
                        color: None,
                        icon_path: None,
                        icon_image: None,
            show_label: None,
                        x: Some(0),
                        y: Some(0),
                    }],
                    size: Size { w: 2, h: 2 },
                    x: Some(1),
                    y: Some(0),
                },
            ]],
        };

        write_layout_to(&path, &layout).expect("write ok");
        let loaded = read_layout_from(&path).expect("read ok");
        assert_eq!(loaded, layout, "序列化往返一致（含文件夹）");
    }

    #[test]
    fn layout_v1_auto_migrates_to_v3() {
        // v1 布局：图标项没有 kind 字段、无 version
        let v1 = r#"{
            "pages": [[
                { "id": "a", "pluginId": "p1", "title": "A", "size": { "w": 1, "h": 1 } }
            ]]
        }"#;
        let mut value: serde_json::Value = serde_json::from_str(v1).unwrap();
        migrate_layout(&mut value);
        assert_eq!(value["version"], 3);
        assert_eq!(value["pages"][0][0]["kind"], "icon");
        // v3：自动分配自由摆放坐标
        assert_eq!(value["pages"][0][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["y"], 0);

        // 迁移后能正常反序列化为 Layout v3
        let layout: Layout = serde_json::from_value(value).unwrap();
        assert_eq!(layout.version, 3);
        match &layout.pages[0][0] {
            Cell::Icon { id, .. } => assert_eq!(id, "a"),
            _ => panic!("应为图标单元"),
        }
    }

    #[test]
    fn migrate_layout_is_idempotent_and_assigns_positions() {
        // v2：已有 kind、无坐标；两个 1×1 图标 + 一个 2×1 小组件
        let v2 = r#"{
            "version": 2,
            "pages": [[
                { "kind": "icon", "id": "a", "pluginId": "p1", "title": "A", "size": { "w": 1, "h": 1 } },
                { "kind": "icon", "id": "b", "pluginId": "p1", "title": "B", "size": { "w": 2, "h": 1 } }
            ]]
        }"#;
        let mut value: serde_json::Value = serde_json::from_str(v2).unwrap();
        migrate_layout(&mut value);
        assert_eq!(value["version"], 3);
        assert_eq!(value["pages"][0][0]["kind"], "icon");
        // a 占 (0,0) 1×1；b 为 2×1 → 起点 (1,0)（a 后一个槽）
        assert_eq!(value["pages"][0][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["y"], 0);
        assert_eq!(value["pages"][0][1]["x"], 1);
        assert_eq!(value["pages"][0][1]["y"], 0);
        // 幂等：再跑一次结果不变
        let snapshot = value.clone();
        migrate_layout(&mut value);
        assert_eq!(value, snapshot, "幂等");
    }

    #[test]
    fn migrate_assigns_folder_item_positions() {
        let v2 = r#"{
            "version": 2,
            "pages": [[
                { "kind": "folder", "id": "f", "name": "F", "emoji": "📁", "items": [
                    { "id": "i1", "pluginId": "p", "title": "1", "size": { "w": 1, "h": 1 } },
                    { "id": "i2", "pluginId": "p", "title": "2", "size": { "w": 1, "h": 1 } }
                ] }
            ]]
        }"#;
        let mut value: serde_json::Value = serde_json::from_str(v2).unwrap();
        migrate_layout(&mut value);
        assert_eq!(value["pages"][0][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["y"], 0);
        // 文件夹内图标按 FOLDER_COLS(6) 列分配
        assert_eq!(value["pages"][0][0]["items"][0]["x"], 0);
        assert_eq!(value["pages"][0][0]["items"][0]["y"], 0);
        assert_eq!(value["pages"][0][0]["items"][1]["x"], 1);
        assert_eq!(value["pages"][0][0]["items"][1]["y"], 0);
    }

    #[test]
    fn cell_serializes_with_kind_tag() {
        let cell = Cell::Icon {
            id: "x".into(),
            plugin_id: "p".into(),
            title: "X".into(),
            size: Size { w: 1, h: 1 },
            action: None,
            emoji: None,
            color: None,
            icon_path: None,
                    icon_image: None,
            show_label: None,
                    x: None,
                    y: None,
        };
        let json = serde_json::to_value(&cell).unwrap();
        assert_eq!(json["kind"], "icon");
        assert_eq!(json["pluginId"], "p");

        // 图标自带动作时序列化包含 action；无动作时不输出该字段
        let with_action = Cell::Icon {
            id: "y".into(),
            plugin_id: "builtin.app".into(),
            title: "Y".into(),
            size: Size { w: 1, h: 1 },
            action: Some(ActionSpec {
                kind: ActionKind::App,
                path: Some("C:\\app.exe".into()),
                cmd: None,
            }),
            emoji: None,
            color: None,
            icon_path: None,
                    icon_image: None,
            show_label: None,
                    x: None,
                    y: None,
        };
        let json = serde_json::to_value(&with_action).unwrap();
        assert_eq!(json["action"]["kind"], "app");
        assert!(!json["action"].get("path").is_none());

        // 自定义字段（M9）往返不丢：emoji / color / iconPath
        let custom = Cell::Icon {
            id: "z".into(),
            plugin_id: "p".into(),
            title: "Z".into(),
            size: Size { w: 1, h: 1 },
            action: None,
            emoji: Some("🎮".into()),
            color: Some("#e53935".into()),
            icon_path: Some("C:\\game.exe".into()),
            icon_image: Some("data:image/png;base64,AAAA".into()),
            show_label: Some(false),
            x: None,
            y: None,
        };
        let json = serde_json::to_value(&custom).unwrap();
        assert_eq!(json["emoji"], "🎮");
        assert_eq!(json["color"], "#e53935");
        assert_eq!(json["iconPath"], "C:\\game.exe");
        assert_eq!(json["showLabel"], false);
        assert_eq!(json["iconImage"], "data:image/png;base64,AAAA");
        let back: Cell = serde_json::from_value(json).unwrap();
        assert_eq!(back, custom, "自定义字段序列化往返一致");

        let folder = Cell::Folder {
            id: "f".into(),
            name: "F".into(),
            emoji: "📁".into(),
            items: vec![],
            size: Size { w: 2, h: 2 },
            x: None,
            y: None,
        };
        let json = serde_json::to_value(&folder).unwrap();
        assert_eq!(json["kind"], "folder");
        assert_eq!(json["emoji"], "📁");
    }

    #[test]
    fn execute_action_dispatches_and_errors() {
        // app 类型缺 path → 报错（不实际启动）
        let bad_app = ActionSpec {
            kind: ActionKind::App,
            path: None,
            cmd: None,
        };
        assert!(execute_action(&bad_app).is_err());

        // command 类型缺 cmd → 报错
        let bad_cmd = ActionSpec {
            kind: ActionKind::Command,
            path: None,
            cmd: None,
        };
        assert!(execute_action(&bad_cmd).is_err());
    }

    #[test]
    fn system_apps_action_is_frontend_handled() {
        // 序列化：kind → "system_apps"
        let spec = ActionSpec {
            kind: ActionKind::SystemApps,
            path: None,
            cmd: None,
        };
        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["kind"], "system_apps");

        // 反序列化：manifest 里的 {"kind":"system_apps"} 可解析
        let parsed: ActionSpec = serde_json::from_str(r#"{"kind":"system_apps"}"#).unwrap();
        assert_eq!(parsed.kind, ActionKind::SystemApps);

        // 后端不执行（由前端处理）
        assert!(execute_action(&spec).is_err());
    }

    #[test]
    fn encode_rgba_png_roundtrip() {
        let w = 4u32;
        let h = 3u32;
        let mut rgba = Vec::new();
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[
                    (x * 60) as u8,
                    (y * 80) as u8,
                    200,
                    if (x + y) % 2 == 0 { 255 } else { 128 },
                ]);
            }
        }
        let png = encode_rgba_png(w, h, &rgba).expect("encode ok");
        // PNG 魔数
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);

        // 解码往返验证像素一致
        let decoded = image::load_from_memory(&png).expect("decode ok").to_rgba8();
        assert_eq!(decoded.dimensions(), (w, h));
        assert_eq!(decoded.as_raw(), &rgba, "PNG 往返像素一致");
    }

    #[test]
    fn encode_rgba_png_rejects_wrong_size() {
        assert!(encode_rgba_png(2, 2, &[0u8; 8]).is_err(), "长度不足应报错");
        assert!(encode_rgba_png(0, 0, &[]).is_err(), "0 尺寸应报错");
        assert!(encode_rgba_png(10, 10, &[0u8; 400]).is_ok());
    }

    #[test]
    fn layout_load_missing_file_returns_none() {
        let dir = temp_dir("layout-missing");
        assert!(read_layout_from(&dir.join("nope.json")).is_none());
    }

    #[test]
    fn launch_plugin_action_errors() {
        // 无 actions
        let no_action = PluginInfo {
            id: "a".into(),
            name: "a".into(),
            version: "0".into(),
            plugin_type: "icon".into(),
            emoji: None,
            actions: vec![],
            widget_component: None,
            sizes: vec![],
            settings: vec![],
            builtin: false,
            provider_id: None,
            provider_name: None,
            domain: None,
            widget_file: None,
            widget_element: None,
            dir: None,
        };
        assert!(launch_plugin_action(&no_action).is_err());

        // app 类型缺 path
        let no_path = PluginInfo {
            id: "b".into(),
            name: "b".into(),
            version: "0".into(),
            plugin_type: "icon".into(),
            emoji: None,
            actions: vec![ActionSpec {
                kind: ActionKind::App,
                path: None,
                cmd: None,
            }],
            widget_component: None,
            sizes: vec![],
            settings: vec![],
            builtin: false,
            provider_id: None,
            provider_name: None,
            domain: None,
            widget_file: None,
            widget_element: None,
            dir: None,
        };
        assert!(launch_plugin_action(&no_path).is_err());
    }

    #[test]
    fn plugin_info_serializes_camel_case() {
        let info = PluginInfo {
            id: "x".into(),
            name: "X".into(),
            version: "1".into(),
            plugin_type: "widget".into(),
            emoji: Some("🕐".into()),
            actions: vec![ActionSpec {
                kind: ActionKind::Command,
                path: None,
                cmd: Some("echo hi".into()),
            }],
            widget_component: Some("clock".into()),
            sizes: vec![Size { w: 2, h: 1 }, Size { w: 2, h: 2 }],
            settings: vec![SettingSpec {
                key: "showSeconds".into(),
                label: "显示秒".into(),
                setting_type: SettingType::Toggle,
                options: None,
                default: Some(serde_json::json!(true)),
            }],
            builtin: false,
            provider_id: None,
            provider_name: None,
            domain: None,
            widget_file: Some("widget.js".into()),
            widget_element: Some("hd-x-widget".into()),
            dir: Some("C:\\plugins\\x".into()),
        };
        let json = serde_json::to_value(&info).unwrap();
        assert_eq!(json["pluginType"], "widget");
        assert_eq!(json["actions"][0]["kind"], "command");
        assert_eq!(json["widgetComponent"], "clock");
        assert_eq!(json["sizes"][1]["w"], 2);
        assert_eq!(json["settings"][0]["type"], "toggle");
        assert_eq!(json["settings"][0]["default"], true);
        assert_eq!(json["widgetFile"], "widget.js");
        assert_eq!(json["widgetElement"], "hd-x-widget");
        assert_eq!(json["dir"], "C:\\plugins\\x");

        // builtin 标记：true 序列化、false 省略
        let builtin = PluginInfo {
            builtin: true,
            provider_id: None,
            provider_name: None,
            domain: None,
            ..info
        };
        let json = serde_json::to_value(&builtin).unwrap();
        assert_eq!(json["builtin"], true);
        assert!(!json.get("builtin").is_none());
    }

    #[test]
    fn widget_manifest_parses() {
        let manifest = r#"{
            "id": "dev.homedesktop.clock",
            "name": "时钟",
            "version": "0.1.0",
            "type": "widget",
            "emoji": "🕐",
            "actions": [],
            "widgetComponent": "clock"
        }"#;
        let m: Manifest = serde_json::from_str(manifest).expect("widget manifest valid");
        assert_eq!(m.plugin_type, "widget");
        assert_eq!(m.widget_component.as_deref(), Some("clock"));
        // 旧 manifest（无 widgetComponent）仍可解析
        let old = serde_json::from_str::<Manifest>(
            r#"{"id":"a","name":"A","version":"1","type":"icon","emoji":"📦","actions":[]}"#,
        )
        .expect("old manifest still parses");
        assert_eq!(old.widget_component, None);
    }

    #[test]
    fn merge_layout_keeps_new_and_appends_backup_missing() {
        // 当前布局：页1 有 A(新增)、B
        let current = serde_json::json!({
            "version": 2,
            "pages": [[
                { "kind": "icon", "id": "A", "pluginId": "p", "title": "A", "size": { "w": 1, "h": 1 } },
                { "kind": "icon", "id": "B", "pluginId": "p", "title": "B", "size": { "w": 1, "h": 1 } }
            ]]
        });
        // 备份：页1 有 A(重复)、C；页2 有 D
        let backup = serde_json::json!({
            "version": 2,
            "pages": [
                [{ "kind": "icon", "id": "A", "pluginId": "p", "title": "A", "size": { "w": 1, "h": 1 } },
                 { "kind": "icon", "id": "C", "pluginId": "p", "title": "C", "size": { "w": 1, "h": 1 } }],
                [{ "kind": "icon", "id": "D", "pluginId": "p", "title": "D", "size": { "w": 1, "h": 1 } }]
            ]
        });
        let merged = merge_layout(&current, &backup);
        let pages = merged["pages"].as_array().unwrap();
        assert_eq!(pages.len(), 2, "页1 合并 + 页2 新增");
        let p1: Vec<&str> = pages[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        assert_eq!(p1, vec!["A", "B", "C"], "保留新增 A/B，追加备份独有的 C");
        assert_eq!(pages[1][0]["id"], "D");
    }

    #[test]
    fn merge_layout_dedupes_by_id_inside_folders() {
        let current = serde_json::json!({
            "version": 2,
            "pages": [[
                { "kind": "folder", "id": "f", "name": "F", "emoji": "📁",
                  "items": [{ "kind": "icon", "id": "inner", "pluginId": "p", "title": "x", "size": { "w": 1, "h": 1 } }] }
            ]]
        });
        let backup = serde_json::json!({
            "version": 2,
            "pages": [[
                { "kind": "icon", "id": "inner", "pluginId": "p", "title": "x", "size": { "w": 1, "h": 1 } },
                { "kind": "icon", "id": "new", "pluginId": "p", "title": "n", "size": { "w": 1, "h": 1 } }
            ]]
        });
        let merged = merge_layout(&current, &backup);
        let p1 = merged["pages"][0].as_array().unwrap();
        let ids: Vec<&str> = p1.iter().map(|c| c["id"].as_str().unwrap()).collect();
        assert_eq!(ids, vec!["f", "new"], "文件夹内的 inner 已存在则跳过");
    }

    #[test]
    fn merge_config_backup_wins_and_keeps_others() {
        let current = serde_json::json!({ "plugin.a.k": "old", "plugin.keep.k": 1 });
        let backup = serde_json::json!({ "plugin.a.k": "new", "plugin.b.k": true });
        let merged = merge_config(&current, &backup);
        assert_eq!(merged["plugin.a.k"], "new");
        assert_eq!(merged["plugin.keep.k"], 1);
        assert_eq!(merged["plugin.b.k"], true);
    }

    #[test]
    fn plugin_widget_file_manifest_parses() {
        let manifest = r#"{
            "id": "com.example.timer",
            "name": "倒计时",
            "version": "1.0.0",
            "type": "widget",
            "emoji": "⏱️",
            "widgetComponent": "__plugin__",
            "widgetFile": "widget.js",
            "widgetElement": "hd-timer-widget",
            "actions": [],
            "sizes": [{ "w": 2, "h": 1 }]
        }"#;
        let m: Manifest = serde_json::from_str(manifest).expect("parse ok");
        assert_eq!(m.widget_component.as_deref(), Some("__plugin__"));
        assert_eq!(m.widget_file.as_deref(), Some("widget.js"));
        assert_eq!(m.widget_element.as_deref(), Some("hd-timer-widget"));
        // 旧 manifest（无这些字段）仍可解析
        let old = serde_json::from_str::<Manifest>(
            r#"{"id":"a","name":"A","version":"1","type":"icon","emoji":"📦","actions":[]}"#,
        )
        .expect("old manifest still parses");
        assert_eq!(old.widget_file, None);
        assert_eq!(old.widget_element, None);
    }
}
