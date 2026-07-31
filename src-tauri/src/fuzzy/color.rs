use std::path::Path;

use super::normalize_query;

pub const COLOR_SIGNATURE_SIZE: usize = 18;
pub const HUE_BIN_COUNT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColorKind {
    Red = 0,
    Orange = 1,
    Yellow = 2,
    Green = 3,
    Cyan = 4,
    Blue = 5,
    Purple = 6,
    Pink = 7,
    Black = 12,
    Gray = 13,
    White = 14,
}

impl ColorKind {
    fn neutral_bin(self) -> Option<usize> {
        match self {
            Self::Black => Some(0),
            Self::Gray => Some(1),
            Self::White => Some(2),
            _ => None,
        }
    }

    fn from_hue_bin(bin: usize) -> Self {
        match bin % HUE_BIN_COUNT {
            0 | 11 => Self::Red,
            1 => Self::Orange,
            2 => Self::Yellow,
            3 | 4 => Self::Green,
            5 => Self::Cyan,
            6 | 7 => Self::Blue,
            8 | 9 => Self::Purple,
            _ => Self::Pink,
        }
    }

    fn hue_bins(self) -> &'static [usize] {
        match self {
            Self::Red => &[0, 11],
            Self::Orange => &[1],
            Self::Yellow => &[2],
            Self::Green => &[3, 4],
            Self::Cyan => &[5],
            Self::Blue => &[6, 7],
            Self::Purple => &[8, 9],
            Self::Pink => &[10],
            Self::Black | Self::Gray | Self::White => &[],
        }
    }
}

pub fn parse_colors(query: &str) -> Vec<ColorKind> {
    let mut colors = Vec::new();
    for token in normalize_query(query) {
        let color = match token.as_str() {
            "red" | "rouge" => ColorKind::Red,
            "orange" => ColorKind::Orange,
            "yellow" | "jaune" => ColorKind::Yellow,
            "green" | "vert" | "verte" => ColorKind::Green,
            "cyan" | "turquoise" => ColorKind::Cyan,
            "blue" | "bleu" | "bleue" => ColorKind::Blue,
            "purple" | "violet" | "violette" => ColorKind::Purple,
            "pink" | "rose" => ColorKind::Pink,
            "black" | "noir" | "noire" => ColorKind::Black,
            "gray" | "grey" | "gris" | "grise" => ColorKind::Gray,
            "white" | "blanc" | "blanche" => ColorKind::White,
            _ => continue,
        };
        if !colors.contains(&color) {
            colors.push(color);
        }
    }
    colors
}

pub fn dominance_requested(query: &str) -> bool {
    normalize_query(query).iter().any(|token| {
        matches!(
            token.as_str(),
            "dominant"
                | "dominante"
                | "mostly"
                | "principal"
                | "principalement"
                | "predominant"
                | "predominantly"
        )
    })
}

pub fn signature_for_path(path: &Path) -> Result<Vec<u8>, image::ImageError> {
    let image = image::open(path)?.thumbnail(64, 64).to_rgb8();
    let mut hues = [0.0_f32; HUE_BIN_COUNT];
    let mut neutral = [0.0_f32; 3];
    let mut total = 0.0_f32;
    for pixel in image.pixels() {
        let [red, green, blue] = pixel.0.map(|value| value as f32 / 255.0);
        let max = red.max(green).max(blue);
        let min = red.min(green).min(blue);
        let delta = max - min;
        if max < 0.18 {
            neutral[0] += 1.0;
        } else if delta < 0.08 && max > 0.86 {
            neutral[2] += 1.0;
        } else if delta < 0.12 {
            neutral[1] += 1.0;
        } else {
            let mut hue = if (max - red).abs() < f32::EPSILON {
                (green - blue) / delta
            } else if (max - green).abs() < f32::EPSILON {
                2.0 + (blue - red) / delta
            } else {
                4.0 + (red - green) / delta
            } * 60.0;
            if hue < 0.0 {
                hue += 360.0;
            }
            let bin = ((hue / 30.0).round() as usize) % HUE_BIN_COUNT;
            hues[bin] += 1.0;
        }
        total += 1.0;
    }
    let mut signature = vec![0_u8; COLOR_SIGNATURE_SIZE];
    if total <= f32::EPSILON {
        return Ok(signature);
    }
    for (index, value) in hues.into_iter().enumerate() {
        signature[index] = ((value / total) * 255.0).round() as u8;
    }
    for (index, value) in neutral.into_iter().enumerate() {
        signature[HUE_BIN_COUNT + index] = ((value / total) * 255.0).round() as u8;
    }
    let (dominant_index, dominant_value) = (0..15)
        .map(|index| (index, signature[index]))
        .max_by_key(|(_, value)| *value)
        .unwrap_or((0, 0));
    signature[15] = if dominant_index < HUE_BIN_COUNT {
        ColorKind::from_hue_bin(dominant_index) as u8
    } else {
        match dominant_index - HUE_BIN_COUNT {
            0 => ColorKind::Black as u8,
            2 => ColorKind::White as u8,
            _ => ColorKind::Gray as u8,
        }
    };
    signature[16] = dominant_value;
    signature[17] = ((hues.iter().sum::<f32>() / total) * 255.0).round() as u8;
    Ok(signature)
}

pub fn score_signature(signature: &[u8], colors: &[ColorKind], dominant: bool) -> f32 {
    if signature.len() < COLOR_SIGNATURE_SIZE || colors.is_empty() {
        return 0.0;
    }
    let values = colors
        .iter()
        .map(|color| {
            color.neutral_bin().map_or_else(
                || {
                    color
                        .hue_bins()
                        .iter()
                        .map(|bin| signature[*bin] as f32 / 255.0)
                        .sum()
                },
                |bin| signature[HUE_BIN_COUNT + bin] as f32 / 255.0,
            )
        })
        .collect::<Vec<_>>();
    let average = values.iter().sum::<f32>() / values.len() as f32;
    let balance = if values.len() > 1 {
        values.iter().copied().fold(1.0_f32, f32::min)
            / values.iter().copied().fold(0.0_f32, f32::max).max(0.01)
    } else {
        1.0
    };
    let mut score = (average * 0.8 + balance * 0.2).clamp(0.0, 1.0);
    if dominant {
        let dominant_match = colors.iter().any(|color| signature[15] == *color as u8);
        score = score * 0.65 + if dominant_match { 0.35 } else { 0.0 };
    }
    score
}
