//! egui 绘制辅助：字体等全局渲染资源。
//! 轮盘（阶段 3）、标记（阶段 4）、设置窗口（阶段 6）、debug HUD 的绘制随后续阶段加入。

use std::path::Path;

use egui::Context;

/// 安装 CJK 字体回退（egui 内置字体不含 CJK）。
/// 依次尝试常见中文字体，全部失败时仅警告（UI 退化为方块但不崩溃）。
pub fn install_cjk_fallback(ctx: &Context) {
    const CANDIDATES: &[(&str, &str)] = &[
        // (字体目录下的文件名, 注册名)
        ("msyh.ttc", "Microsoft YaHei"),
        ("msyh.ttf", "Microsoft YaHei"),
        ("simhei.ttf", "SimHei"),
        ("simsun.ttc", "SimSun"),
    ];

    let mut loaded = false;
    for (file, name) in CANDIDATES {
        let path = Path::new(r"C:\Windows\Fonts").join(file);
        let Ok(data) = std::fs::read(&path) else {
            continue;
        };
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            name.to_string(),
            std::sync::Arc::new(egui::FontData::from_owned(data)),
        );
        // 追加到两个家族的回退链尾：拉丁用内置字体，CJK 落到中文字体
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push(name.to_string());
        }
        ctx.set_fonts(fonts);
        tracing::info!(?path, "CJK 字体回退已安装");
        loaded = true;
        break;
    }
    if !loaded {
        tracing::warn!("未找到系统中文字体，UI 中文可能显示为方块");
    }
}
