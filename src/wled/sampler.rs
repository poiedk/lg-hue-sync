use crate::{
    color::{ActiveRect, RgbColor},
    config::NanoleafAlignment,
    nanoleaf::NanoleafPerimeterSampler,
};

/// Reuses the proven perimeter sampling geometry while translating the
/// canonical screen-edge order into WLED's contiguous LED index order.
pub struct WledPerimeterSampler {
    inner: NanoleafPerimeterSampler,
    led_count: usize,
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
        let inner = NanoleafPerimeterSampler::new_aligned(
            led_count,
            &output_ids,
            hdr_tone_mapping,
            saturation_boost,
            noise_gate_threshold,
            brightness_multiplier,
            alignment,
        );

        Self {
            inner,
            led_count: led_count as usize,
        }
    }

    pub fn led_count(&self) -> usize {
        self.led_count
    }

    pub fn set_alignment(&mut self, alignment: NanoleafAlignment) {
        self.inner.set_alignment(alignment);
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
        let output_ids = self.inner.panel_ids();
        remap_colors(&canonical, &output_ids)
    }
}

fn remap_colors(colors: &[RgbColor], output_ids: &[u16]) -> Vec<RgbColor> {
    let mut output = vec![RgbColor::new(0, 0, 0); colors.len()];
    for (color, output_id) in colors.iter().zip(output_ids) {
        let output_index = *output_id as usize;
        if output_index < output.len() {
            output[output_index] = *color;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaps_canonical_colors_to_wled_indices() {
        let colors = [
            RgbColor::new(1, 0, 0),
            RgbColor::new(2, 0, 0),
            RgbColor::new(3, 0, 0),
            RgbColor::new(4, 0, 0),
        ];
        let output = remap_colors(&colors, &[2, 1, 3, 0]);

        assert_eq!(output[0], colors[3]);
        assert_eq!(output[1], colors[1]);
        assert_eq!(output[2], colors[0]);
        assert_eq!(output[3], colors[2]);
    }

    #[test]
    fn enforces_minimum_four_perimeter_leds() {
        let sampler = WledPerimeterSampler::new(
            1,
            false,
            1.0,
            0.0,
            1.0,
            NanoleafAlignment::default(),
        );
        assert_eq!(sampler.led_count(), 4);
    }
}
