use bevy::asset::HandleId;
use bevy::core_pipeline::core_3d::Opaque3d;
use bevy::ecs::query::ROQueryItem;
use bevy::ecs::system::lifetimeless::{Read, SRes};
use bevy::ecs::system::SystemParamItem;
use bevy::pbr::{MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup};
use bevy::prelude::*;
use bevy::reflect::TypeUuid;
use bevy::render::extract_component::{ComponentUniforms, DynamicUniformIndex};
use bevy::render::mesh::MeshVertexBufferLayout;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
    DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
    TrackedRenderPass,
};
use bevy::render::render_resource::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, BufferBindingType, IndexFormat, PipelineCache,
    PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, ShaderStages, ShaderType,
    SpecializedMeshPipeline, SpecializedMeshPipelineError, SpecializedMeshPipelines,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::view::ExtractedView;

use crate::instance::{BillboardVertex, MultiBillboardHandle, MultiBillboardUniform};
use crate::{BillboardInstance, MultiBillboard};

#[derive(Resource)]
pub struct MultiBillboardBindGroup(BindGroup);

pub fn queue_multi_billboard_bind_group(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<MultiBillboardPipeline>,
    uniform: Res<ComponentUniforms<MultiBillboardUniform>>,
) {
    if let Some(binding) = uniform.uniforms().binding() {
        commands.insert_resource(MultiBillboardBindGroup(render_device.create_bind_group(
            &BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }],
                label: Some("multi billboard bind group"),
                layout: &pipeline.uniform_layout,
            },
        )));
    }
}

pub fn queue_multi_billboards(
    mut q_views: Query<(&ExtractedView, &mut RenderPhase<Opaque3d>)>,
    q_multi_billboards: Query<(Entity, &MultiBillboardUniform)>,
    draw_functions: Res<DrawFunctions<Opaque3d>>,
    meshes: Res<RenderAssets<Mesh>>,
    pipeline: Res<MultiBillboardPipeline>,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedMeshPipelines<MultiBillboardPipeline>>,
) {
    let draw_function = draw_functions
        .read()
        .get_id::<DrawMultiBillboard>()
        .unwrap();

    let dummy_mesh_handle = Handle::weak(DUMMY_MESH_HANDLE_ID);
    let Some(dummy_mesh) = meshes.get(&dummy_mesh_handle) else {
        return;
    };

    for (view, mut opaque_phase) in &mut q_views {
        let key = MeshPipelineKey::from_hdr(view.hdr);

        let pipeline = pipelines
            .specialize(&pipeline_cache, &pipeline, key, &dummy_mesh.layout)
            .unwrap();

        let rangefinder = view.rangefinder3d();

        for (entity, uniform) in &q_multi_billboards {
            let distance = rangefinder.distance(&uniform.transform);
            opaque_phase.add(Opaque3d {
                distance,
                pipeline,
                entity,
                draw_function,
            });
        }
    }
}

#[derive(Resource)]
pub struct DummyMesh(pub Handle<Mesh>);

pub const DUMMY_MESH_HANDLE_ID: HandleId = HandleId::new(Mesh::TYPE_UUID, 2923664944183213475);

impl FromWorld for DummyMesh {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let handle = meshes.set(
            DUMMY_MESH_HANDLE_ID,
            Mesh::new(PrimitiveTopology::TriangleList),
        );
        DummyMesh(handle)
    }
}

#[derive(Resource)]
pub struct MultiBillboardPipeline {
    mesh_pipeline: MeshPipeline,
    uniform_layout: BindGroupLayout,
    shader: Handle<Shader>,
}

type Type = bevy::prelude::World;

impl FromWorld for MultiBillboardPipeline {
    fn from_world(world: &mut Type) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();
        let mesh_pipeline = world.resource::<MeshPipeline>();

        let uniform_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("multi billboard uniform layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(MultiBillboardUniform::min_size()),
                },
                count: None,
            }],
        });

        let shader = asset_server.load("shaders/billboard.wgsl");

        Self {
            mesh_pipeline: mesh_pipeline.clone(),
            uniform_layout,
            shader,
        }
    }
}

impl SpecializedMeshPipeline for MultiBillboardPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers = vec![
            BillboardVertex::vertex_buffer_layout(),
            BillboardInstance::vertex_buffer_layout(),
        ];
        descriptor.layout[1] = self.uniform_layout.clone();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.primitive = PrimitiveState::default();

        Ok(descriptor)
    }
}

pub type DrawMultiBillboard = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMultiBillboardBindGroup<1>,
    DrawMultiBillboardCall,
);

pub struct SetMultiBillboardBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetMultiBillboardBindGroup<I> {
    type Param = SRes<MultiBillboardBindGroup>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<MultiBillboardUniform>>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        uniform_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.into_inner().0, &[uniform_index.index()]);
        RenderCommandResult::Success
    }
}

pub struct DrawMultiBillboardCall;

impl<P: PhaseItem> RenderCommand<P> for DrawMultiBillboardCall {
    type Param = SRes<RenderAssets<MultiBillboard>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<MultiBillboardHandle>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        handle: ROQueryItem<'w, Self::ItemWorldQuery>,
        multi_billboards: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(multi_billboard) = multi_billboards.into_inner().get(&handle.0) {
            pass.set_vertex_buffer(0, multi_billboard.vertex_buffer.slice(..));
            pass.set_vertex_buffer(1, multi_billboard.instance_buffer.slice(..));
            pass.set_index_buffer(
                multi_billboard.index_buffer.slice(..),
                0,
                IndexFormat::Uint32,
            );

            pass.draw_indexed(0..6, 0, 0..multi_billboard.num_instances);

            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}
