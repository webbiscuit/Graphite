use crate::consts::{DRAG_THRESHOLD, LINE_ROTATE_SNAP_ANGLE};
use crate::messages::frontend::utility_types::MouseCursorIcon;
use crate::messages::input_mapper::utility_types::input_keyboard::{Key, KeysGroup, MouseMotion};
use crate::messages::input_mapper::utility_types::input_mouse::ViewportPosition;
use crate::messages::layout::utility_types::layout_widget::{Layout, LayoutGroup, PropertyHolder, Widget, WidgetCallback, WidgetHolder, WidgetLayout};
use crate::messages::layout::utility_types::widgets::input_widgets::NumberInput;
use crate::messages::prelude::*;
use crate::messages::tool::common_functionality::snapping::SnapManager;
use crate::messages::tool::utility_types::{EventToMessageMap, Fsm, ToolActionHandlerData, ToolMetadata, ToolTransition, ToolType};
use crate::messages::tool::utility_types::{HintData, HintGroup, HintInfo};

use graphene::layers::style;
use graphene::LayerId;
use graphene::Operation;

use glam::{DAffine2, DVec2};
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct LineTool {
	fsm_state: LineToolFsmState,
	tool_data: LineToolData,
	options: LineOptions,
}

pub struct LineOptions {
	line_weight: f64,
}

impl Default for LineOptions {
	fn default() -> Self {
		Self { line_weight: 5. }
	}
}

#[remain::sorted]
#[impl_message(Message, ToolMessage, Line)]
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum LineToolMessage {
	// Standard messages
	#[remain::unsorted]
	Abort,

	// Tool-specific messages
	DragStart,
	DragStop,
	Redraw {
		center: Key,
		lock_angle: Key,
		snap_angle: Key,
	},
	UpdateOptions(LineOptionsUpdate),
}

#[remain::sorted]
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum LineOptionsUpdate {
	LineWeight(f64),
}

impl ToolMetadata for LineTool {
	fn icon_name(&self) -> String {
		"VectorLineTool".into()
	}
	fn tooltip(&self) -> String {
		"Line Tool".into()
	}
	fn tool_type(&self) -> crate::messages::tool::utility_types::ToolType {
		ToolType::Line
	}
}

impl PropertyHolder for LineTool {
	fn properties(&self) -> Layout {
		Layout::WidgetLayout(WidgetLayout::new(vec![LayoutGroup::Row {
			widgets: vec![WidgetHolder::new(Widget::NumberInput(NumberInput {
				unit: " px".into(),
				label: "Weight".into(),
				value: Some(self.options.line_weight as f64),
				is_integer: false,
				min: Some(0.),
				on_update: WidgetCallback::new(|number_input: &NumberInput| LineToolMessage::UpdateOptions(LineOptionsUpdate::LineWeight(number_input.value.unwrap())).into()),
				..NumberInput::default()
			}))],
		}]))
	}
}

impl<'a> MessageHandler<ToolMessage, ToolActionHandlerData<'a>> for LineTool {
	fn process_message(&mut self, message: ToolMessage, tool_data: ToolActionHandlerData<'a>, responses: &mut VecDeque<Message>) {
		if message == ToolMessage::UpdateHints {
			self.fsm_state.update_hints(responses);
			return;
		}

		if message == ToolMessage::UpdateCursor {
			self.fsm_state.update_cursor(responses);
			return;
		}

		if let ToolMessage::Line(LineToolMessage::UpdateOptions(action)) = message {
			match action {
				LineOptionsUpdate::LineWeight(line_weight) => self.options.line_weight = line_weight,
			}
			return;
		}

		let new_state = self.fsm_state.transition(message, &mut self.tool_data, tool_data, &self.options, responses);

		if self.fsm_state != new_state {
			self.fsm_state = new_state;
			self.fsm_state.update_hints(responses);
			self.fsm_state.update_cursor(responses);
		}
	}

	fn actions(&self) -> ActionList {
		use LineToolFsmState::*;

		match self.fsm_state {
			Ready => actions!(LineToolMessageDiscriminant;
				DragStart,
			),
			Drawing => actions!(LineToolMessageDiscriminant;
				DragStop,
				Redraw,
				Abort,
			),
		}
	}
}

impl ToolTransition for LineTool {
	fn event_to_message_map(&self) -> EventToMessageMap {
		EventToMessageMap {
			document_dirty: None,
			tool_abort: Some(LineToolMessage::Abort.into()),
			selection_changed: None,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LineToolFsmState {
	Ready,
	Drawing,
}

impl Default for LineToolFsmState {
	fn default() -> Self {
		LineToolFsmState::Ready
	}
}

#[derive(Clone, Debug, Default)]
struct LineToolData {
	drag_start: ViewportPosition,
	drag_current: ViewportPosition,
	angle: f64,
	weight: f64,
	path: Option<Vec<LayerId>>,
	snap_manager: SnapManager,
}

impl Fsm for LineToolFsmState {
	type ToolData = LineToolData;
	type ToolOptions = LineOptions;

	fn transition(
		self,
		event: ToolMessage,
		tool_data: &mut Self::ToolData,
		(document, global_tool_data, input, font_cache): ToolActionHandlerData,
		tool_options: &Self::ToolOptions,
		responses: &mut VecDeque<Message>,
	) -> Self {
		use LineToolFsmState::*;
		use LineToolMessage::*;

		if let ToolMessage::Line(event) = event {
			match (self, event) {
				(Ready, DragStart) => {
					tool_data.snap_manager.start_snap(document, document.bounding_boxes(None, None, font_cache), true, true);
					tool_data.snap_manager.add_all_document_handles(document, &[], &[], &[]);
					tool_data.drag_start = tool_data.snap_manager.snap_position(responses, document, input.mouse.position);

					responses.push_back(DocumentMessage::StartTransaction.into());
					tool_data.path = Some(document.get_path_for_new_layer());
					responses.push_back(DocumentMessage::DeselectAllLayers.into());

					tool_data.weight = tool_options.line_weight;

					responses.push_back(
						Operation::AddLine {
							path: tool_data.path.clone().unwrap(),
							insert_index: -1,
							transform: DAffine2::ZERO.to_cols_array(),
							style: style::PathStyle::new(Some(style::Stroke::new(global_tool_data.primary_color, tool_data.weight)), style::Fill::None),
						}
						.into(),
					);

					Drawing
				}
				(Drawing, Redraw { center, snap_angle, lock_angle }) => {
					tool_data.drag_current = tool_data.snap_manager.snap_position(responses, document, input.mouse.position);

					let values: Vec<_> = [lock_angle, snap_angle, center].iter().map(|k| input.keyboard.get(*k as usize)).collect();
					responses.push_back(generate_transform(tool_data, values[0], values[1], values[2]));

					Drawing
				}
				(Drawing, DragStop) => {
					tool_data.drag_current = tool_data.snap_manager.snap_position(responses, document, input.mouse.position);
					tool_data.snap_manager.cleanup(responses);

					match tool_data.drag_start.distance(input.mouse.position) <= DRAG_THRESHOLD {
						true => responses.push_back(DocumentMessage::AbortTransaction.into()),
						false => responses.push_back(DocumentMessage::CommitTransaction.into()),
					}

					tool_data.path = None;

					Ready
				}
				(Drawing, Abort) => {
					tool_data.snap_manager.cleanup(responses);
					responses.push_back(DocumentMessage::AbortTransaction.into());
					tool_data.path = None;
					Ready
				}
				_ => self,
			}
		} else {
			self
		}
	}

	fn update_hints(&self, responses: &mut VecDeque<Message>) {
		let hint_data = match self {
			LineToolFsmState::Ready => HintData(vec![HintGroup(vec![
				HintInfo {
					key_groups: vec![],
					key_groups_mac: None,
					mouse: Some(MouseMotion::LmbDrag),
					label: String::from("Draw Line"),
					plus: false,
				},
				HintInfo {
					key_groups: vec![KeysGroup(vec![Key::Shift])],
					key_groups_mac: None,
					mouse: None,
					label: String::from("Snap 15°"),
					plus: true,
				},
				HintInfo {
					key_groups: vec![KeysGroup(vec![Key::Alt])],
					key_groups_mac: None,
					mouse: None,
					label: String::from("From Center"),
					plus: true,
				},
				HintInfo {
					key_groups: vec![KeysGroup(vec![Key::Control])],
					key_groups_mac: None,
					mouse: None,
					label: String::from("Lock Angle"),
					plus: true,
				},
			])]),
			LineToolFsmState::Drawing => HintData(vec![HintGroup(vec![
				HintInfo {
					key_groups: vec![KeysGroup(vec![Key::Shift])],
					key_groups_mac: None,
					mouse: None,
					label: String::from("Snap 15°"),
					plus: false,
				},
				HintInfo {
					key_groups: vec![KeysGroup(vec![Key::Alt])],
					key_groups_mac: None,
					mouse: None,
					label: String::from("From Center"),
					plus: false,
				},
				HintInfo {
					key_groups: vec![KeysGroup(vec![Key::Control])],
					key_groups_mac: None,
					mouse: None,
					label: String::from("Lock Angle"),
					plus: false,
				},
			])]),
		};

		responses.push_back(FrontendMessage::UpdateInputHints { hint_data }.into());
	}

	fn update_cursor(&self, responses: &mut VecDeque<Message>) {
		responses.push_back(FrontendMessage::UpdateMouseCursor { cursor: MouseCursorIcon::Crosshair }.into());
	}
}

fn generate_transform(tool_data: &mut LineToolData, lock: bool, snap: bool, center: bool) -> Message {
	let mut start = tool_data.drag_start;
	let stop = tool_data.drag_current;

	let dir = stop - start;

	let mut angle = -dir.angle_between(DVec2::X);

	if lock {
		angle = tool_data.angle
	};

	if snap {
		let snap_resolution = LINE_ROTATE_SNAP_ANGLE.to_radians();
		angle = (angle / snap_resolution).round() * snap_resolution;
	}

	tool_data.angle = angle;

	let mut scale = dir.length();

	if lock {
		let angle_vec = DVec2::new(angle.cos(), angle.sin());
		scale = dir.dot(angle_vec);
	}

	if center {
		start -= scale * DVec2::new(angle.cos(), angle.sin());
		scale *= 2.;
	}

	Operation::SetLayerTransformInViewport {
		path: tool_data.path.clone().unwrap(),
		transform: glam::DAffine2::from_scale_angle_translation(DVec2::new(scale, 1.), angle, start).to_cols_array(),
	}
	.into()
}
