use graphene::layers::layer_info::{Layer, LayerData, LayerDataTypeDiscriminant};
use graphene::layers::style::{RenderData, ViewMode};
use graphene::layers::text_layer::FontCache;
use graphene::LayerId;

use glam::{DAffine2, DVec2};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct RawBuffer(Vec<u8>);

impl From<Vec<u64>> for RawBuffer {
	fn from(iter: Vec<u64>) -> Self {
		// https://github.com/rust-lang/rust-clippy/issues/4484
		let v_from_raw: Vec<u8> = unsafe {
			// prepare for an auto-forget of the initial vec:
			let v_orig: &mut Vec<_> = &mut *std::mem::ManuallyDrop::new(iter);
			Vec::from_raw_parts(v_orig.as_mut_ptr() as *mut u8, v_orig.len() * 8, v_orig.capacity() * 8)
			// v_orig is never used again, so no aliasing issue
		};
		Self(v_from_raw)
	}
}

impl Serialize for RawBuffer {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		let mut buffer = serializer.serialize_struct("Buffer", 2)?;
		buffer.serialize_field("pointer", &(self.0.as_ptr() as usize))?;
		buffer.serialize_field("length", &(self.0.len()))?;
		buffer.end()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub struct LayerMetadata {
	pub selected: bool,
	pub expanded: bool,
}

impl LayerMetadata {
	pub fn new(expanded: bool) -> Self {
		Self { selected: false, expanded }
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerPanelEntry {
	pub name: String,
	pub visible: bool,
	pub layer_type: LayerDataTypeDiscriminant,
	pub layer_metadata: LayerMetadata,
	pub path: Vec<LayerId>,
	pub thumbnail: String,
}

impl LayerPanelEntry {
	pub fn new(layer_metadata: &LayerMetadata, transform: DAffine2, layer: &Layer, path: Vec<LayerId>, font_cache: &FontCache) -> Self {
		let name = layer.name.clone().unwrap_or_else(|| String::from(""));
		let arr = layer.data.bounding_box(transform, font_cache).unwrap_or([DVec2::ZERO, DVec2::ZERO]);
		let arr = arr.iter().map(|x| (*x).into()).collect::<Vec<(f64, f64)>>();

		let mut thumbnail = String::new();
		let mut svg_defs = String::new();
		let render_data = RenderData::new(ViewMode::Normal, font_cache, None, false);
		layer.data.clone().render(&mut thumbnail, &mut svg_defs, &mut vec![transform], render_data);
		let transform = transform.to_cols_array().iter().map(ToString::to_string).collect::<Vec<_>>().join(",");
		let thumbnail = if let [(x_min, y_min), (x_max, y_max)] = arr.as_slice() {
			format!(
				r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}"><defs>{}</defs><g transform="matrix({})">{}</g></svg>"#,
				x_min,
				y_min,
				x_max - x_min,
				y_max - y_min,
				svg_defs,
				transform,
				thumbnail,
			)
		} else {
			String::new()
		};

		LayerPanelEntry {
			name,
			visible: layer.visible,
			layer_type: (&layer.data).into(),
			layer_metadata: *layer_metadata,
			path,
			thumbnail,
		}
	}
}
