//! 用纯代码生成应用图标（RGBA 像素）。
//!
//! 无需任何图片资源：蓝色圆形底 + 白色双向箭头，寓意“互译”。
//! 同一份像素可用于窗口图标与系统托盘图标。

/// 生成的图标边长（像素）。
pub const SIZE: u32 = 64;

type P = (f64, f64);
type Poly = Vec<P>;

fn rect_poly(x0: f64, y0: f64, x1: f64, y1: f64) -> Poly {
    vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
}

fn point_in_poly(x: f64, y: f64, poly: &Poly) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// 生成 RGBA 像素（straight alpha）。
pub fn rgba_icon() -> Vec<u8> {
    const N: f64 = SIZE as f64;
    const CX: f64 = N / 2.0;
    const CY: f64 = N / 2.0;
    const RADIUS: f64 = N / 2.0 - 2.0;
    const BLUE: (f64, f64, f64) = (
        0x2f as f64 / 255.0,
        0x80 as f64 / 255.0,
        0xed as f64 / 255.0,
    );
    const WHITE: (f64, f64, f64) = (1.0, 1.0, 1.0);

    // 上半箭头（向右）
    let top_shaft = rect_poly(15.0, 21.5, 33.0, 25.5);
    let top_head = vec![(33.0, 18.0), (45.5, 23.5), (33.0, 29.0)];
    // 下半箭头（向左）
    let bot_shaft = rect_poly(31.0, 38.5, 49.0, 42.5);
    let bot_head = vec![(31.0, 35.0), (18.5, 40.5), (31.0, 46.0)];

    let glyphs = [&top_shaft, &top_head, &bot_shaft, &bot_head];
    let inside_circle = |x: f64, y: f64| (x - CX).hypot(y - CY) <= RADIUS;

    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for py in 0..SIZE {
        for px in 0..SIZE {
            const SS: usize = 3; // 超采样
            let mut hits = 0usize;
            let (mut r, mut g, mut b) = (0.0f64, 0.0f64, 0.0f64);
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = px as f64 + (sx as f64 + 0.5) / SS as f64;
                    let y = py as f64 + (sy as f64 + 0.5) / SS as f64;
                    if !inside_circle(x, y) {
                        continue;
                    }
                    hits += 1;
                    let in_glyph = glyphs.iter().any(|g| point_in_poly(x, y, g));
                    let (cr, cg, cb) = if in_glyph { WHITE } else { BLUE };
                    r += cr;
                    g += cg;
                    b += cb;
                }
            }
            if hits == 0 {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                let n = hits as f64;
                let alpha = (255.0 * n / (SS * SS) as f64).round() as u8;
                rgba.extend_from_slice(&[
                    (255.0 * r / n).round() as u8,
                    (255.0 * g / n).round() as u8,
                    (255.0 * b / n).round() as u8,
                    alpha,
                ]);
            }
        }
    }
    rgba
}

/// 生成窗口图标（eframe 需要 `Arc<IconData>`）。
pub fn window_icon() -> std::sync::Arc<egui::viewport::IconData> {
    let rgba = rgba_icon();
    std::sync::Arc::new(egui::viewport::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_dimensions() {
        let rgba = rgba_icon();
        assert_eq!(rgba.len(), (SIZE * SIZE * 4) as usize);
        // 中心像素应为不透明的白色箭头（两条箭头在中心附近有留白，
        // 因此取中心偏上/偏下的像素更稳妥）——这里仅检查尺寸与非空。
        assert!(!rgba.iter().all(|&x| x == 0));
    }
}
