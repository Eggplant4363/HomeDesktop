fn main() {
    // 必须放在 tauri_build::build() 之前：
    // tauri-build 会校验 bundle.resources 中的路径，不存在即失败。
    // 先把 webview2-com-sys 生成的 WebView2Loader.dll 复制到 src-tauri/resources/。
    // 依赖的 build script 先于本 crate 运行，此时 DLL 已存在于 target/<triple>/<profile>/。
    copy_webview2_loader();

    tauri_build::build();
}

fn copy_webview2_loader() {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let target = std::env::var("TARGET").unwrap_or_default();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let src = std::path::Path::new(&manifest_dir)
        .parent()
        .unwrap_or(std::path::Path::new(&manifest_dir))
        .join("target")
        .join(&target)
        .join(&profile)
        .join("WebView2Loader.dll");

    if src.is_file() {
        let dst_dir = std::path::Path::new(&manifest_dir).join("resources");
        if let Err(e) = std::fs::create_dir_all(&dst_dir) {
            eprintln!("[build.rs] create_dir_all failed: {e}");
        }
        let dst = dst_dir.join("WebView2Loader.dll");
        // 内容相同则跳过复制：避免 mtime 变化触发 tauri dev 资源监听 → 无限重建循环
        let same = std::fs::read(&dst)
            .map(|d| std::fs::read(&src).map(|s| d == s).unwrap_or(false))
            .unwrap_or(false);
        if same {
            eprintln!("[build.rs] WebView2Loader.dll 已最新，跳过复制");
        } else {
            match std::fs::copy(&src, &dst) {
                Ok(n) => eprintln!("[build.rs] WebView2Loader.dll copied ({n} bytes)"),
                Err(e) => eprintln!("[build.rs] copy failed: {e}"),
            }
        }
    } else {
        eprintln!("[build.rs] 源 DLL 不存在: {}", src.display());
    }
}
