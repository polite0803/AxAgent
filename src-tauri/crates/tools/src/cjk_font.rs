// SPDX-License-Identifier: AGPL-3.0-only

//! CJK 字体加载与 PDF CID 编码
//!
//! 提供跨平台 CJK 字体自动发现（Windows / macOS / Linux 系统字体 + 项目 `app_dir/fonts/`），
//! 并把 TrueType 字体嵌入 PDF（Type0 → CIDFontType2 复合字体 + Identity-H 编码）。
//!
//! 没有 CJK 字体时返回 `None`，由调用方降级到内置 Type1 Helvetica（仅渲染西文）。
//!
//! 参考：PDF 1.7 § 9.7（复合字体） / § 9.10（CIDFontType2 / TrueType）。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ttf_parser::Face;

/// 全局缓存的 CJK 字体单例。`bytes` 必须 `'static`，所以用 `Box::leak`。
static CJK_FONT: OnceLock<Option<CjkFont>> = OnceLock::new();

/// CJK TrueType 字体（已嵌入 `'static` 字节）。
pub struct CjkFont {
    /// TTF 字节（leaked，进程生命周期有效）。
    bytes: &'static [u8],
    /// ttf-parser 视图。
    face: Face<'static>,
    /// 设计单位每 em。
    units_per_em: u16,
    /// 上升（设计单位）。
    ascent: i16,
    /// 下降（设计单位，负值）。
    descent: i16,
}

/// 字符串是否包含 CJK 字符（决定走 CID 字体还是 Type1）。
pub fn needs_cjk_font(text: &str) -> bool {
    text.chars().any(is_cjk_codepoint)
}

pub fn is_cjk_codepoint_public(c: char) -> bool {
    is_cjk_codepoint(c)
}

fn is_cjk_codepoint(c: char) -> bool {
    let cp = c as u32;
    matches!(cp,
        0x3000..=0x303F |  // CJK 标点
        0x3400..=0x4DBF |  // CJK 扩展 A
        0x4E00..=0x9FFF |  // CJK 基本
        0xF900..=0xFAFF |  // CJK 兼容
        0xFF00..=0xFFEF    // 半角全角
    ) || (0x20000..=0x2FFFF).contains(&cp)  // CJK 扩展 B-G
    || (0xAC00..=0xD7AF).contains(&cp) // 韩文音节
}

/// 字符是否需要复合字体（CID/CJK 字体）而非内置 Type1。
///
/// 包含 CJK 字符 + Type1 字体（Courier/Helvetica）无法渲染的 Unicode 符号，
/// 例如框线字符（U+2500–257F）、几何形状（U+25A0–25FF）、箭头（U+2190–21FF 等）。
/// 这些符号在 CJK TTF 字体（如微软雅黑）中均有字形。
pub fn needs_cid_font(c: char) -> bool {
    let cp = c as u32;
    is_cjk_codepoint(c)
        || (0x2190..=0x21FF).contains(&cp)    // 箭头
        || (0x2500..=0x257F).contains(&cp)    // 框线字符
        || (0x25A0..=0x25FF).contains(&cp)    // 几何形状
        || cp == 0x2713                         // ✓ 勾号
        || cp == 0x2717 // ✗ 叉号
}

/// 获取全局 CJK 字体（首次调用时执行文件系统查找）。
pub fn cjk_font() -> Option<&'static CjkFont> {
    CJK_FONT.get_or_init(load_cjk_font).as_ref()
}

fn load_cjk_font() -> Option<CjkFont> {
    let candidates = collect_candidate_paths();
    for path in candidates {
        if let Some(font) = try_load_path(&path) {
            tracing::info!(font_path = %path.display(), "CJK 字体加载成功");
            return Some(font);
        }
    }
    None
}

fn collect_candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(dir) = std::env::var("AXAGENT_FONT_DIR") {
        collect_font_files(&PathBuf::from(dir), &mut paths);
    }

    if let Ok(cwd) = std::env::current_dir() {
        collect_font_files(&cwd.join("fonts"), &mut paths);
    }

    #[cfg(target_os = "windows")]
    {
        let win_fonts = PathBuf::from(r"C:\Windows\Fonts");
        for name in &["msyh.ttc", "msyhbd.ttc", "simsun.ttc", "simhei.ttf", "simfang.ttf"] {
            let p = win_fonts.join(name);
            if p.exists() {
                paths.push(p);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        for p in &[
            "/System/Library/Fonts/PingFang.ttc",
            "/System/Library/Fonts/STHeiti Medium.ttc",
            "/Library/Fonts/Songti.ttc",
        ] {
            let pb = PathBuf::from(p);
            if pb.exists() {
                paths.push(pb);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        for p in &[
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
            "/usr/share/fonts/truetype/arphic/uming.ttc",
        ] {
            let pb = PathBuf::from(p);
            if pb.exists() {
                paths.push(pb);
            }
        }
    }

    paths
}

fn collect_font_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|s| s.to_str()).is_some_and(|s| {
                    let s = s.to_ascii_lowercase();
                    s == "ttf" || s == "otf" || s == "ttc"
                })
        })
        .collect();
    found.sort();
    out.extend(found);
}

fn try_load_path(path: &Path) -> Option<CjkFont> {
    let bytes = fs::read(path).ok()?;
    load_from_bytes(bytes)
}

fn load_from_bytes(bytes: Vec<u8>) -> Option<CjkFont> {
    // TTC 解出第一个 face 的 TTF 字节
    let ttf_vec = if is_ttc(&bytes) {
        extract_first_ttf_from_ttc(&bytes)?
    } else {
        bytes
    };

    // leak 到 'static 借用，使 Face<'static> 可用
    let ttf_static: &'static [u8] = Box::leak(ttf_vec.into_boxed_slice());

    let face = Face::parse(ttf_static, 0).ok()?;

    if !face_has_cjk_cmap(&face) {
        return None;
    }

    let units_per_em = face.units_per_em();
    let ascent = face.ascender();
    let descent = face.descender();

    Some(CjkFont { bytes: ttf_static, face, units_per_em, ascent, descent })
}

impl CjkFont {
    /// 原始 TTF 字节（用于嵌入 PDF stream）。
    pub fn bytes(&self) -> &'static [u8] {
        self.bytes
    }

    /// 查询字符的字形索引（GID）。缺字形时返回 None。
    /// 用于配合 Identity-H + CIDToGIDMap=Identity 把内容流 CID 设为 GID。
    pub fn glyph_index(&self, c: char) -> Option<u16> {
        self.face.glyph_index(c).map(|g| g.0)
    }

    /// 设计单位每 em。
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// 上升（pt）。
    pub fn ascent_pt(&self, font_size_pt: f64) -> f64 {
        self.ascent as f64 * font_size_pt / self.units_per_em as f64
    }

    /// 下降（绝对值，pt）。
    pub fn descent_pt(&self, font_size_pt: f64) -> f64 {
        -self.descent as f64 * font_size_pt / self.units_per_em as f64
    }

    /// 上升（设计单位）。
    pub fn ascent(&self) -> i16 {
        self.ascent
    }

    /// 下降（设计单位，负值）。
    pub fn descent(&self) -> i16 {
        self.descent
    }

    /// 字符串宽度（pt）。未找到字形时按 fallback（CJK=1em，ASCII=0.5em）。
    pub fn measure(&self, text: &str, font_size_pt: f64) -> f64 {
        let mut total = 0.0;
        let scale = font_size_pt / self.units_per_em as f64;
        for c in text.chars() {
            let advance = self
                .face
                .glyph_index(c)
                .and_then(|gid| self.face.glyph_hor_advance(gid))
                .map(|adv| adv as f64 * scale)
                .unwrap_or_else(|| fallback_char_width(c, font_size_pt));
            total += advance;
        }
        total
    }

    /// Identity-H 复合字体字节流（十六进制字符串，不含 `<>` 包裹）。
    ///
    /// 配合 [`crate::tools::document::register_cjk_font`] 的 `CIDToGIDMap = Identity`，
    /// 内容流里的 CID 必须等于 TrueType 的**字形索引 (GID)**，渲染器据此取正确字形。
    /// 因此这里编码的是 GID（2 字节大端），而非 Unicode 码点。
    /// 字符在字体中缺字形时回退为 GID 0（不渲染），并由 ToUnicode CMap 兜底映射。
    pub fn encode_cid_hex(&self, text: &str) -> String {
        let mut hex = String::with_capacity(text.len() * 4);
        for c in text.chars() {
            let gid = self.glyph_index(c).unwrap_or(0);
            hex.push_str(&format!("{:04X}", gid));
        }
        hex
    }
}

fn fallback_char_width(c: char, font_size_pt: f64) -> f64 {
    if is_cjk_codepoint(c) {
        font_size_pt
    } else {
        font_size_pt * 0.5
    }
}

fn face_has_cjk_cmap(face: &Face) -> bool {
    // 任一 CJK Unicode 字符能被 face 映射到字形即视为 CJK 字体
    for cp in [0x4E00u32, 0x5000, 0x9E00, 0x3000, 0xFF00, 0xAC00] {
        if let Some(c) = char::from_u32(cp)
            && face.glyph_index(c).is_some()
        {
            return true;
        }
    }
    false
}

// ── TTC 最小解析（仅取第一个 face 的 TTF 字节）──────────────────────────────

fn is_ttc(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && &bytes[0..4] == b"ttcf"
}

/// TTC header: 4B "ttcf" + 4B version + 4B numFonts + numFonts × 4B offset
fn extract_first_ttf_from_ttc(ttc: &[u8]) -> Option<Vec<u8>> {
    if ttc.len() < 12 {
        return None;
    }
    let num_fonts = u32::from_be_bytes(ttc[8..12].try_into().ok()?) as usize;
    if num_fonts == 0 || ttc.len() < 16 {
        return None;
    }
    let first_offset = u32::from_be_bytes(ttc[12..16].try_into().ok()?) as usize;
    if first_offset >= ttc.len() {
        return None;
    }
    Some(ttc[first_offset..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_detection() {
        assert!(needs_cjk_font("你好世界"));
        assert!(needs_cjk_font("Hello 中文"));
        assert!(!needs_cjk_font("Hello World"));
        assert!(!needs_cjk_font("123 !@#"));
    }

    #[test]
    fn cid_font_detection() {
        // CJK 字符
        assert!(needs_cid_font('中'));
        assert!(needs_cid_font('文'));
        // 框线字符
        assert!(needs_cid_font('┌'));
        assert!(needs_cid_font('─'));
        assert!(needs_cid_font('│'));
        assert!(needs_cid_font('└'));
        assert!(needs_cid_font('▼'));
        // 几何形状
        assert!(needs_cid_font('◆'));
        assert!(needs_cid_font('◇'));
        // 箭头
        assert!(needs_cid_font('→'));
        // ASCII 不需要
        assert!(!needs_cid_font('A'));
        assert!(!needs_cid_font('z'));
        assert!(!needs_cid_font('0'));
        assert!(!needs_cid_font(' '));
    }

    #[test]
    fn ttc_magic() {
        assert!(is_ttc(b"ttcf\0\0\0\0\x00\x00\x00\x00"));
        assert!(!is_ttc(b"\x00\x01\x00\x00"));
    }

    #[test]
    fn ttc_extract_first_offset() {
        // 构造最小 TTC: 12B header + 4B offset = 16B
        let mut ttc = Vec::new();
        ttc.extend_from_slice(b"ttcf"); // magic
        ttc.extend_from_slice(&0x00010000u32.to_be_bytes()); // version 1.0
        ttc.extend_from_slice(&1u32.to_be_bytes()); // numFonts
        ttc.extend_from_slice(&16u32.to_be_bytes()); // first offset
        ttc.extend_from_slice(b"OTTO\x00\x01"); // fake TTF data
        let out = extract_first_ttf_from_ttc(&ttc).expect("测试应成功");
        assert_eq!(out, b"OTTO\x00\x01");
    }

    #[test]
    fn cid_hex_encode_basic() {
        // 不需要真字体，仅测字符编码函数（用 leak 的 dummy）
        // 跳过 — encode_cid_hex 需要 &CjkFont，依赖 face
    }
}
