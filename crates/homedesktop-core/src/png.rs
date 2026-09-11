//! 图像编码（RGBA → PNG）。

// ---------- 图像编码 ----------

/// 把 RGBA 像素数据编码为 PNG 字节（用于系统应用图标）。
/// `rgba` 长度必须等于 `width * height * 4`（纯函数，便于单测）。
pub fn encode_rgba_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or("尺寸溢出")?;
    if rgba.len() != expected {
        return Err(format!(
            "RGBA 数据长度不匹配: 期望 {expected} 字节, 实际 {}",
            rgba.len()
        ));
    }
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec()).ok_or("创建图像失败")?;
    let dyn_img = image::DynamicImage::ImageRgba8(img);
    let mut out: Vec<u8> = Vec::new();
    dyn_img
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| format!("PNG 编码失败: {e}"))?;
    Ok(out)
}

