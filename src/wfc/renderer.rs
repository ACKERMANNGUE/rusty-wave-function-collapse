use image::{ Rgba, RgbaImage };

use crate::{ pattern::Pattern, wfc::wave::Wave };

pub fn render_wave(wave: &Wave, patterns: &[Pattern]) -> Option<RgbaImage> {
    let first_pattern = patterns.first()?;

    let pattern_size = first_pattern.get_size();

    if pattern_size == 0 {
        return None;
    }

    if wave.get_width() == 0 || wave.get_height() == 0 {
        return None;
    }

    let output_width = (wave.get_width() as u32) + pattern_size - 1;
    let output_height = (wave.get_height() as u32) + pattern_size - 1;

    let mut output = RgbaImage::new(output_width, output_height);

    // fiil the main Wave area, eahc collapsed cell contributes the top-left pixel of its pattern
    for wave_y in 0..wave.get_height() {
        for wave_x in 0..wave.get_width() {
            let cell = wave.get_cell(wave_x, wave_y)?;
            let pattern_id = cell.collapsed_pattern_id()?;
            let pattern = patterns.get(pattern_id)?;
            let pixel = pattern.get_pixel(0, 0)?;
            output.put_pixel(wave_x as u32, wave_y as u32, Rgba(*pixel));
        }
    }

    let last_wave_x = wave.get_width() - 1;
    let last_wave_y = wave.get_height() - 1;

    // then complete the right border using the patterns from the last Wave column
    for wave_y in 0..wave.get_height() {
        let cell = wave.get_cell(last_wave_x, wave_y)?;
        let pattern_id = cell.collapsed_pattern_id()?;
        let pattern = patterns.get(pattern_id)?;

        for pattern_x in 1..pattern_size {
            let pixel = pattern.get_pixel(pattern_x as usize, 0)?;
            let output_x = (last_wave_x as u32) + pattern_x;
            let output_y = wave_y as u32;
            output.put_pixel(output_x, output_y, Rgba(*pixel));
        }
    }

    // then complete the bottom border using the patterns from the last Wave row
    for wave_x in 0..wave.get_width() {
        let cell = wave.get_cell(wave_x, last_wave_y)?;
        let pattern_id = cell.collapsed_pattern_id()?;
        let pattern = patterns.get(pattern_id)?;

        for pattern_y in 1..pattern_size {
            let pixel = pattern.get_pixel(0, pattern_y as usize)?;
            let output_x = wave_x as u32;
            let output_y = (last_wave_y as u32) + pattern_y;
            output.put_pixel(output_x, output_y, Rgba(*pixel));
        }
    }

    // then complete the bottom-right corner using the pattern from the final Wave cell
    let cell = wave.get_cell(last_wave_x, last_wave_y)?;
    let pattern_id = cell.collapsed_pattern_id()?;
    let pattern = patterns.get(pattern_id)?;

    for pattern_y in 1..pattern_size {
        for pattern_x in 1..pattern_size {
            let pixel = pattern.get_pixel(pattern_x as usize, pattern_y as usize)?;
            let output_x = (last_wave_x as u32) + pattern_x;
            let output_y = (last_wave_y as u32) + pattern_y;
            output.put_pixel(output_x, output_y, Rgba(*pixel));
        }
    }

    Some(output)
}
