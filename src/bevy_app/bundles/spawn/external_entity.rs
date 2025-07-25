use crate::bevy_app::components::*;
use crate::bevy_app::constants::{
    BUTTON_WIDTH_HALF, EXTERNAL_ENTITY_LINE_WIDTH, EXTERNAL_ENTITY_SELECTED_LINE_WIDTH,
    EXTERNAL_ENTITY_WIDTH_HALF, EXTERNAL_ENTITY_Z, LABEL_Z,
};
use crate::bevy_app::events::ExternalEntityDrag;
use crate::bevy_app::plugins::label::{add_name_label, Alignment, CopyPositionArgs};
use crate::bevy_app::plugins::lyon_selection::HighlightBundles;
use crate::bevy_app::plugins::mouse_interaction::DragPosition;
use crate::bevy_app::plugins::mouse_interaction::PickSelection;
use crate::bevy_app::resources::{
    FixedSystemElementGeometriesByNestingLevel, IsSameAsIdCounter, StrokeTessellator,
};
use crate::bevy_app::utils::ui_transform_from_button;
use crate::data_model::WorldModel;
use crate::plugins::label::{
    add_marker_with_text, AutoContrastTextColor, CopyPositions, HorizontalAttachmentAnchor,
    MarkerLabel, VerticalAttachmentAnchor,
};
use bevy::math::{vec2, vec3};
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy_prototype_lyon::prelude::*;

pub fn spawn_external_entity(
    commands: &mut Commands,
    subsystem_query: &Query<&Subsystem>,
    nesting_level_query: &Query<&NestingLevel>,
    focused_system: Entity,
    interface_type: InterfaceType,
    substance_type: SubstanceType,
    flow_entity: Entity,
    transform: &Transform,
    fixed_system_element_geometries: &mut ResMut<FixedSystemElementGeometriesByNestingLevel>,
    zoom: f32,
    is_selected: bool,
    meshes: &mut ResMut<Assets<Mesh>>,
    tess: &mut ResMut<StrokeTessellator>,
    name: &str,
    description: &str,
    consider_button: bool,
) -> Entity {
    let (transform, initial_position) = ui_transform_from_button(
        transform,
        EXTERNAL_ENTITY_Z,
        if consider_button {
            EXTERNAL_ENTITY_WIDTH_HALF - BUTTON_WIDTH_HALF
        } else {
            EXTERNAL_ENTITY_WIDTH_HALF / zoom
        },
        zoom,
    );

    let nesting_level = NestingLevel::current(focused_system, nesting_level_query);

    let external_entity = spawn_external_entity_only(
        commands,
        substance_type,
        is_selected,
        name,
        description,
        transform,
        initial_position,
        nesting_level,
        zoom,
        fixed_system_element_geometries,
        meshes,
        tess,
    );

    if let Ok(subsystem) = subsystem_query.get(focused_system) {
        commands
            .entity(subsystem.parent_system)
            .add_child(external_entity);
    }

    let mut entity_commands = commands.entity(flow_entity);

    match interface_type {
        InterfaceType::Import => {
            entity_commands.insert(FlowStartConnection {
                target: external_entity,
                target_type: StartTargetType::Source,
            });
        }
        InterfaceType::Export => {
            entity_commands.insert(FlowEndConnection {
                target: external_entity,
                target_type: EndTargetType::Sink,
            });
        }
    }

    external_entity
}

pub fn spawn_external_entity_only(
    commands: &mut Commands,
    substance_type: SubstanceType,
    is_selected: bool,
    name: &str,
    description: &str,
    transform: Transform,
    initial_position: InitialPosition,
    nesting_level: u16,
    zoom: f32,
    fixed_system_element_geometries: &mut ResMut<FixedSystemElementGeometriesByNestingLevel>,
    meshes: &mut ResMut<Assets<Mesh>>,
    tess: &mut ResMut<StrokeTessellator>,
) -> Entity {
    let color = substance_type.flow_color_default();

    let scale = NestingLevel::compute_scale(nesting_level, zoom);

    commands
        .spawn((
            ExternalEntity {
                equivalence: "".to_string(),
                model: "".to_string(),
            },
            transform,
            HighlightBundles {
                idle: Stroke::new(color, EXTERNAL_ENTITY_LINE_WIDTH * scale),
                selected: Stroke {
                    color,
                    options: StrokeOptions::default()
                        .with_line_width(EXTERNAL_ENTITY_SELECTED_LINE_WIDTH)
                        .with_line_cap(LineCap::Round),
                },
            },
            PickingBehavior::default(),
            RayCastPickable::default(),
            PickSelection { is_selected },
            SystemElement::ExternalEntity,
            Name::new(name.to_string()),
            ElementDescription::new(description),
            initial_position,
            fixed_system_element_geometries
                .get_or_create(nesting_level, zoom, meshes, tess)
                .external_entity,
            NestingLevel::new(nesting_level),
        ))
        .observe(
            |trigger: Trigger<DragPosition>, mut writer: EventWriter<ExternalEntityDrag>| {
                writer.send(trigger.into());
            },
        )
        .id()
}

// TODO : do the entire external entity like this autospawn?
pub fn auto_spawn_external_entity_label(
    mut commands: Commands,
    external_entity_query: Query<(Entity, &NestingLevel), Added<ExternalEntity>>,
    name_query: Query<&Name>,
    asset_server: Res<AssetServer>,
) {
    for (external_entity, nesting_level) in external_entity_query.iter() {
        add_name_label(
            &mut commands,
            external_entity,
            vec2(70.0, 100.0),
            None,
            Some(CopyPositionArgs {
                offset: vec3(1.0, 0.0, 0.0),
                horizontal_alignment: Alignment::Auto,
                vertical_alignment: Alignment::Center,
                horizontal_anchor: HorizontalAttachmentAnchor::default(),
                vertical_anchor: VerticalAttachmentAnchor::default(),
            }),
            true,
            &name_query,
            &asset_server,
            None,
            *nesting_level,
        );
    }
}

pub fn auto_spawn_source_sink_equivalence(
    mut commands: Commands,
    mut is_same_as_query: Query<
        (
            Entity,
            &mut CopyPositions,
            &Aabb,
            &IsSameAsId,
            &NestingLevel,
        ),
        Added<IsSameAsId>,
    >,
    asset_server: Res<AssetServer>,
) {
    let button_width = BUTTON_WIDTH_HALF * 2.0;

    for (entity, ref mut copy_positions, aabb, is_same_as_id, nesting_level) in
        is_same_as_query.iter_mut()
    {
        add_marker_with_text(
            &mut commands,
            entity,
            copy_positions,
            aabb,
            vec2(button_width, button_width),
            Some(CopyPositionArgs {
                offset: vec3(10.0, 10.0, LABEL_Z),
                horizontal_alignment: Alignment::Center,
                vertical_alignment: Alignment::Center,
                horizontal_anchor: HorizontalAttachmentAnchor::WestLocal,
                vertical_anchor: VerticalAttachmentAnchor::NorthWorld,
            }),
            &(**is_same_as_id).to_string(),
            "label-icons/equivalence_annotation.png",
            &asset_server,
            Some(AutoContrastTextColor::default()),
            *nesting_level,
        );
    }
}

pub fn update_is_same_as_id_label(
    is_same_as_id_query: Query<(&IsSameAsId, &MarkerLabel), Changed<IsSameAsId>>,
    mut target_query: Query<&mut Text2d>,
) {
    for (is_same_as_id, marker_label) in is_same_as_id_query.iter() {
        if let Ok(mut text) = target_query.get_mut(marker_label.label) {
            text.0 = (**is_same_as_id).to_string();
        }
    }
}

pub fn spawn_is_same_as_id_counter(
    world_model: &WorldModel,
    is_same_as_id_counter: &mut ResMut<IsSameAsIdCounter>,
) {
    let mut highest_id = 0;
    for system in world_model.systems.iter() {
        for ext in system.sources.iter().chain(system.sinks.iter()) {
            if let Some(ext) = ext.is_same_as_id {
                if ext > highest_id {
                    highest_id = ext;
                }
            }
        }
    }

    for ext in world_model
        .environment
        .sources
        .iter()
        .chain(world_model.environment.sinks.iter())
    {
        if let Some(ext) = ext.is_same_as_id {
            if ext > highest_id {
                highest_id = ext;
            }
        }
    }

    ***(is_same_as_id_counter) = highest_id;
}
