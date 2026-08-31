use image::{ Rgba, RgbaImage };

use crate::{ pattern::Pattern, wfc::wave::Wave };

pub fn render_wave(wave: &Wave, patterns: &[Pattern]) -> Option<RgbaImage> {
    let first_pattern = patterns.first()?;
    let pattern_size = first_pattern.get_size();

    let output_width = (wave.get_width() as u32) + pattern_size - 1;
    let output_height = (wave.get_height() as u32) + pattern_size - 1;

    let mut output = RgbaImage::new(output_width, output_height);

    for wave_y in 0..wave.get_height() {
        for wave_x in 0..wave.get_width() {
            let cell = wave.get_cell(wave_x, wave_y)?;
            let pattern_id = cell.collapsed_pattern_id()?;
            let pattern = patterns.get(pattern_id)?;

            for pattern_y in 0..pattern_size {
                for pattern_x in 0..pattern_size {
                    let pixel = pattern.get_pixel(pattern_x as usize, pattern_y as usize)?;
                    let output_x = (wave_x as u32) + pattern_x;
                    let output_y = (wave_y as u32) + pattern_y;
                    output.put_pixel(output_x, output_y, Rgba(*pixel));
                }
            }
        }
    }

    Some(output)
}
