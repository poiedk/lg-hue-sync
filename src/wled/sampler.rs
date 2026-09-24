use crate::{
    color::{ActiveRect, RgbColor},
    config::{NanoleafAlignment, NanoleafStartCorner},
    nanoleaf::NanoleafPerimeterSampler,
};

/// Reuses the proven perimeter sampling geometry while translating the
/// canonical screen-edge order into WLED's contiguous LED index order.
///
/// WLED alignment semantics are intentionally physical:
/// - start_corner is the screen position of LED index 0
/// - reverse_direction controls which way indices increase from LED 0
/// - perimeter_offset fine-tunes LED 0 along that selected direction
pub struct WledPerimeterSampler {
    inner: NanoleafPerimeterSampler,
    led_count: usize,
    alignment: NanoleafAlignment,
}

impl WledPerimeterSampler {
    pub fn new(
        led_count: u16,
        hdr_tone_mapping: bool,
        saturation_boost: f32,
        noise_gate_threshold: f32,
        brightness_multiplier: f32,
        alignment: NanoleafAlignment,
    ) -> Self {
        let led_count = led_count.max(4);
        let output_ids: Vec<u16> = (0..led_count).collect();

        // Keep the sampling geometry in its canonical screen order. WLED has a
        // contiguous index space, so physical alignment is applied after
        // sampling rather than by borrowing Nanoleaf panel-ID semantics.
        let inner = NanoleafPerimeterSampler::new_aligned(
            led_count,
            &output_ids,
            hdr_tone_mapping,
            saturation_boost,
            noise_gate_threshold,
            brightness_multiplier,
            NanoleafAlignment::default(),
        );

        Self {
            inner,
            led_count: led_count as usize,
            alignment,
        }
    }

    pub fn led_count(&self) -> usize {
        self.led_count
    }

    pub fn set_alignment(&mut self, alignment: NanoleafAlignment) {
        self.alignment = alignment;
    }

    pub fn set_smoothing_factor(&mut self, factor: f32) {
        self.inner.set_smoothing_factor(factor);
    }

    pub fn set_temporal_response(&mut self, rise: f32, fall: f32) {
        self.inner.set_temporal_response(rise, fall);
    }

    pub fn set_strict_blackout(&mut self, enabled: bool) {
        self.inner.set_strict_blackout(enabled);
    }

    pub fn set_brightness_multiplier(&mut self, multiplier: f32) {
        self.inner.set_brightness_multiplier(multiplier);
    }

    pub fn set_saturation_boost(&mut self, boost: f32) {
        self.inner.set_saturation_boost(boost);
    }

    pub fn set_hdr_tone_mapping(&mut self, enabled: bool) {
        self.inner.set_hdr_tone_mapping(enabled);
    }

    pub fn set_peak_weight(&mut self, weight: f32) {
        self.inner.set_peak_weight(weight);
    }

    pub fn set_gamma(&mut self, gamma: f32) {
        self.inner.set_gamma(gamma);
    }

    pub fn set_max_color_step(&mut self, step: u8) {
        self.inner.set_max_color_step(step);
    }

    pub fn set_noise_gate_threshold(&mut self, threshold: f32) {
        self.inner.set_noise_gate_threshold(threshold);
    }

    pub fn set_active_rect(&mut self, rect: ActiveRect) {
        self.inner.set_active_rect(rect);
    }

    pub fn sample_frame(
        &mut self,
        frame_data: &[u8],
        width: u32,
        height: u32,
        is_bgra: bool,
        is_scene_cut: bool,
    ) -> Vec<RgbColor> {
        let canonical = self
            .inner
            .sample_frame(frame_data, width, height, is_bgra, is_scene_cut);
        remap_colors(&canonical, self.alignment)
    }
}

fn generic_corner_index(len: usize, corner: NanoleafStartCorner) -> usize {
    if len == 0 {
        return 0;
    }
    if len == 40 {
        return match corner {
            NanoleafStartCorner::BottomCenter => 0,
            NanoleafStartCorner::BottomLeft => 7,
            NanoleafStartCorner::TopLeft => 15,
            NanoleafStartCorner::TopRight => 28,
            NanoleafStartCorner::BottomRight => 35,
        };
    }

    // Mirror NanoleafPerimeterSampler::build_generic_perimeter_zones so the
    // coarse corner choices land on the same actual geometry.
    let left_count = ((9.0 / 50.0) * len as f32).round().max(1.0) as usize;
    let top_count = ((16.0 / 50.0) * len as f32).round().max(1.0) as usize;
    let right_count = ((9.0 / 50.0) * len as f32).round().max(1.0) as usize;
    let bottom_count = len
        .saturating_sub(left_count + top_count + right_count)
        .max(1);
    let bottom_left_half = bottom_count / 2;

    match corner {
        NanoleafStartCorner::BottomCenter => 0,
        NanoleafStartCorner::BottomLeft => bottom_left_half % len,
        NanoleafStartCorner::TopLeft => (bottom_left_half + left_count) % len,
        NanoleafStartCorner::TopRight => (bottom_left_half + left_count + top_count) % len,
        NanoleafStartCorner::BottomRight => {
            (bottom_left_half + left_count + top_count + right_count) % len
        }
    }
}

fn remap_colors(colors: &[RgbColor], alignment: NanoleafAlignment) -> Vec<RgbColor> {
    let len = colors.len();
    if len == 0 {
        return Vec::new();
    }

    let direction = if alignment.reverse_direction {
        -1isize
    } else {
        1isize
    };
    let coarse_start = generic_corner_index(len, alignment.start_corner) as isize;
    let effective_start =
        (coarse_start + direction * alignment.perimeter_offset as isize).rem_euclid(len as isize);

    let mut output = vec![RgbColor::new(0, 0, 0); len];
    for (canonical_index, color) in colors.iter().enumerate() {
        let delta = canonical_index as isize - effective_start;
        let output_index = (direction * delta).rem_euclid(len as isize) as usize;
        output[output_index] = *color;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colors(count: usize) -> Vec<RgbColor> {
        (0..count)
            .map(|index| RgbColor::new(index as u8, 0, 0))
            .collect()
    }

    #[test]
    fn default_alignment_preserves_canonical_order() {
        let input = colors(12);
        assert_eq!(remap_colors(&input, NanoleafAlignment::default()), input);
    }

    #[test]
    fn start_corner_is_physical_led_zero() {
        let input = colors(40);
        let output = remap_colors(
            &input,
            NanoleafAlignment {
                start_corner: NanoleafStartCorner::BottomLeft,
                reverse_direction: false,
                perimeter_offset: 0,
            },
        );

        assert_eq!(output[0], input[7]);
        assert_eq!(output[1], input[8]);
        assert_eq!(output[39], input[6]);
    }

    #[test]
    fn reverse_direction_increases_indices_the_other_way() {
        let input = colors(40);
        let output = remap_colors(
            &input,
            NanoleafAlignment {
                start_corner: NanoleafStartCorner::TopLeft,
                reverse_direction: true,
                perimeter_offset: 0,
            },
        );

        assert_eq!(output[0], input[15]);
        assert_eq!(output[1], input[14]);
        assert_eq!(output[39], input[16]);
    }

    #[test]
    fn perimeter_offset_fine_tunes_led_zero_along_selected_direction() {
        let input = colors(40);
        let output = remap_colors(
            &input,
            NanoleafAlignment {
                start_corner: NanoleafStartCorner::BottomLeft,
                reverse_direction: false,
                perimeter_offset: 2,
            },
        );

        assert_eq!(output[0], input[9]);
        assert_eq!(output[1], input[10]);
    }

    #[test]
    fn generic_corner_indices_match_generic_geometry() {
        assert_eq!(
            generic_corner_index(120, NanoleafStartCorner::BottomCenter),
            0
        );
        assert_eq!(
            generic_corner_index(120, NanoleafStartCorner::BottomLeft),
            19
        );
        assert_eq!(generic_corner_index(120, NanoleafStartCorner::TopLeft), 41);
        assert_eq!(generic_corner_index(120, NanoleafStartCorner::TopRight), 79);
        assert_eq!(
            generic_corner_index(120, NanoleafStartCorner::BottomRight),
            101
        );
    }

    #[test]
    fn enforces_minimum_four_perimeter_leds() {
        let sampler =
            WledPerimeterSampler::new(1, false, 1.0, 0.0, 1.0, NanoleafAlignment::default());
        assert_eq!(sampler.led_count(), 4);
    }
}
