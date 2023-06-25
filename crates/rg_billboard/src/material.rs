use std::hash::Hash;
use std::marker::PhantomData;

use bevy::asset::{AssetPath, HandleId};
use bevy::core_pipeline::core_3d::AlphaMask3d;
use bevy::ecs::query::ROQueryItem;
use bevy::ecs::system::lifetimeless::{Read, SRes};
use bevy::ecs::system::SystemParamItem;
use bevy::pbr::{MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup};
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::extract_component::{
    ComponentUniforms, DynamicUniformIndex, ExtractComponentPlugin,
};
use bevy::render::mesh::MeshVertexBufferLayout;
use bevy::render::render_asset::{PrepareAssetSet, RenderAssets};
use bevy::render::render_phase::{
    AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase,
    SetItemPipeline, TrackedRenderPass,
};
use bevy::render::render_resource::{
    AsBindGroup, AsBindGroupError, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, IndexFormat,
    OwnedBindingResource, PipelineCache, PrimitiveState, PrimitiveTopology,
    RenderPipelineDescriptor, ShaderStages, ShaderType, SpecializedMeshPipeline,
    SpecializedMeshPipelineError, SpecializedMeshPipelines,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::FallbackImage;
use bevy::render::view::{ExtractedView, VisibleEntities};
use bevy::render::{Extract, Render, RenderApp, RenderSet};
use bevy::utils::{HashMap, HashSet};

use crate::instance::{BillboardVertex, MultiBillboardUniform};
use crate::{BillboardInstance, MultiBillboard};

pub trait BillboardMaterial:
    AsBindGroup + Send + Sync + Clone + TypeUuid + TypePath + Sized
{
    fn vertex_shader() -> AssetPath<'static>;

    fn fragment_shader() -> AssetPath<'static>;

    fn specialize(
        pipeline: BillboardMaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
    ) {
        let _ = (pipeline, descriptor);
    }
}

pub struct BillboardMaterialPlugin<M: BillboardMaterial> {
    marker: PhantomData<M>,
}

impl<M: BillboardMaterial> Default for BillboardMaterialPlugin<M> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<M> Plugin for BillboardMaterialPlugin<M>
where
    M: BillboardMaterial,
    M::Data: Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.add_asset::<M>()
            .add_plugins(ExtractComponentPlugin::<Handle<M>>::extract_visible());

        app.sub_app_mut(RenderApp)
            .init_resource::<SpecializedMeshPipelines<BillboardMaterialPipeline<M>>>()
            .init_resource::<ExtractedBillboardMaterials<M>>()
            .init_resource::<PreparedBillboardMaterials<M>>()
            .add_render_command::<AlphaMask3d, DrawMultiBillboard<M>>()
            .add_systems(ExtractSchedule, extract_materials::<M>)
            .add_systems(
                Render,
                (
                    prepare_materials::<M>
                        .in_set(RenderSet::Prepare)
                        .after(PrepareAssetSet::PreAssetPrepare),
                    queue_billboard_uniform_bind_groups::<M>.in_set(RenderSet::Queue),
                    queue_billboard_batches::<M>.in_set(RenderSet::Queue),
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        app.sub_app_mut(RenderApp)
            .init_resource::<BillboardMaterialPipeline<M>>();
    }
}

#[derive(Resource)]
pub struct BillboardUniformBindGroup(BindGroup);

pub fn queue_billboard_uniform_bind_groups<M: BillboardMaterial>(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline: Res<BillboardMaterialPipeline<M>>,
    uniform: Res<ComponentUniforms<MultiBillboardUniform>>,
) {
    if let Some(binding) = uniform.uniforms().binding() {
        commands.insert_resource(BillboardUniformBindGroup(render_device.create_bind_group(
            &BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }],
                label: Some("billboard uniform"),
                layout: &pipeline.uniform_layout,
            },
        )));
    }
}

pub fn queue_billboard_batches<M>(
    mut q_views: Query<(
        &ExtractedView,
        &VisibleEntities,
        &mut RenderPhase<AlphaMask3d>,
    )>,
    q_multi_billboards: Query<(Entity, &MultiBillboardUniform, &Handle<M>)>,
    draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    meshes: Res<RenderAssets<Mesh>>,
    materials: Res<PreparedBillboardMaterials<M>>,
    pipeline: Res<BillboardMaterialPipeline<M>>,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedMeshPipelines<BillboardMaterialPipeline<M>>>,
) where
    M: BillboardMaterial,
    M::Data: Eq + Hash + Clone,
{
    let draw_function = draw_functions
        .read()
        .get_id::<DrawMultiBillboard<M>>()
        .unwrap();

    let dummy_mesh_handle = Handle::weak(DUMMY_MESH_HANDLE_ID);
    let Some(dummy_mesh) = meshes.get(&dummy_mesh_handle) else {
        return;
    };

    for (view, visible_entities, mut opaque_phase) in &mut q_views {
        let mesh_key = MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();

        for (entity, uniform, material) in q_multi_billboards.iter_many(&visible_entities.entities)
        {
            let Some(material) = materials.map.get(material) else {
                continue;
            };

            let key = BillboardMaterialKey {
                mesh_key,
                bind_group_data: material.key.clone(),
            };

            let pipeline = pipelines
                .specialize(&pipeline_cache, &pipeline, key, &dummy_mesh.layout)
                .unwrap();

            let distance = rangefinder.distance(&uniform.transform);
            opaque_phase.add(AlphaMask3d {
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

#[derive(Clone)]
pub struct BillboardMaterialKey<M: BillboardMaterial> {
    pub mesh_key: MeshPipelineKey,
    pub bind_group_data: M::Data,
}

impl<M> Eq for BillboardMaterialKey<M>
where
    M: BillboardMaterial,
    M::Data: Eq,
{
}

impl<M: BillboardMaterial> PartialEq for BillboardMaterialKey<M>
where
    M: BillboardMaterial,
    M::Data: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        return self.mesh_key == other.mesh_key && self.bind_group_data == other.bind_group_data;
    }
}

impl<M: BillboardMaterial> Hash for BillboardMaterialKey<M>
where
    M: BillboardMaterial,
    M::Data: Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.mesh_key.hash(state);
        self.bind_group_data.hash(state);
    }
}

#[derive(Resource)]
pub struct BillboardMaterialPipeline<M: BillboardMaterial> {
    pub mesh_pipeline: MeshPipeline,
    pub uniform_layout: BindGroupLayout,
    pub material_layout: BindGroupLayout,
    pub vertex_shader: Handle<Shader>,
    pub fragment_shader: Handle<Shader>,
    marker: PhantomData<M>,
}

impl<M: BillboardMaterial> FromWorld for BillboardMaterialPipeline<M> {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();
        let mesh_pipeline = world.resource::<MeshPipeline>().clone();

        let uniform_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("billboard uniform"),
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

        let material_layout = M::bind_group_layout(render_device);
        let vertex_shader = asset_server.load(M::vertex_shader());
        let fragment_shader = asset_server.load(M::fragment_shader());

        Self {
            mesh_pipeline,
            uniform_layout,
            material_layout,
            vertex_shader,
            fragment_shader,
            marker: PhantomData,
        }
    }
}

impl<M> SpecializedMeshPipeline for BillboardMaterialPipeline<M>
where
    M: BillboardMaterial,
    M::Data: Eq + Hash + Clone,
{
    type Key = BillboardMaterialKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.mesh_key, layout)?;

        descriptor.vertex.shader = self.vertex_shader.clone();
        descriptor.vertex.buffers = vec![
            BillboardVertex::vertex_buffer_layout(),
            BillboardInstance::vertex_buffer_layout(),
        ];
        descriptor.layout.drain(1..);
        descriptor.layout.push(self.material_layout.clone());
        descriptor.layout.push(self.uniform_layout.clone());
        descriptor.fragment.as_mut().unwrap().shader = self.fragment_shader.clone();
        descriptor.primitive = PrimitiveState::default();

        Ok(descriptor)
    }
}

pub type DrawMultiBillboard<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetBillboardMaterialBindGroup<1, M>,
    SetBillboardUniformBindGroup<2>,
    DrawBillboardBatch,
);

pub struct SetBillboardUniformBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetBillboardUniformBindGroup<I> {
    type Param = SRes<BillboardUniformBindGroup>;
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

pub struct SetBillboardMaterialBindGroup<const I: usize, M: BillboardMaterial>(PhantomData<M>);

impl<const I: usize, M: BillboardMaterial, P: PhaseItem> RenderCommand<P>
    for SetBillboardMaterialBindGroup<I, M>
{
    type Param = SRes<PreparedBillboardMaterials<M>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<M>>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        handle: ROQueryItem<'w, Self::ItemWorldQuery>,
        materials: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let material = materials.into_inner().map.get(handle).unwrap();
        pass.set_bind_group(I, &material.bind_group, &[]);
        RenderCommandResult::Success
    }
}

pub struct DrawBillboardBatch;

impl<P: PhaseItem> RenderCommand<P> for DrawBillboardBatch {
    type Param = SRes<RenderAssets<MultiBillboard>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<MultiBillboard>>;

    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        handle: ROQueryItem<'w, Self::ItemWorldQuery>,
        multi_billboards: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Some(multi_billboard) = multi_billboards.into_inner().get(handle) {
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

#[derive(Resource)]
pub struct ExtractedBillboardMaterials<M: BillboardMaterial> {
    extracted: Vec<(Handle<M>, M)>,
    removed: Vec<Handle<M>>,
}

impl<M: BillboardMaterial> Default for ExtractedBillboardMaterials<M> {
    fn default() -> Self {
        Self {
            extracted: Vec::new(),
            removed: Vec::new(),
        }
    }
}

pub fn extract_materials<M: BillboardMaterial>(
    mut commands: Commands,
    mut events: Extract<EventReader<AssetEvent<M>>>,
    assets: Extract<Res<Assets<M>>>,
) {
    let mut changed_assets = HashSet::default();
    let mut removed = Vec::new();
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                changed_assets.insert(handle.clone_weak());
            }
            AssetEvent::Removed { handle } => {
                changed_assets.remove(handle);
                removed.push(handle.clone_weak());
            }
        }
    }

    let mut extracted_assets = Vec::new();
    for handle in changed_assets.drain() {
        if let Some(asset) = assets.get(&handle) {
            extracted_assets.push((handle, asset.clone()));
        }
    }

    commands.insert_resource(ExtractedBillboardMaterials {
        extracted: extracted_assets,
        removed,
    });
}

#[derive(Resource)]
pub struct PreparedBillboardMaterials<M: BillboardMaterial> {
    map: HashMap<Handle<M>, PreparedBillboardMaterial<M>>,
}

pub struct PreparedBillboardMaterial<M: BillboardMaterial> {
    pub bindings: Vec<OwnedBindingResource>,
    pub bind_group: BindGroup,
    pub key: M::Data,
}

impl<M: BillboardMaterial> Default for PreparedBillboardMaterials<M> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
        }
    }
}

pub struct PrepareNextFrameMaterials<M: BillboardMaterial> {
    assets: Vec<(Handle<M>, M)>,
}

impl<M: BillboardMaterial> Default for PrepareNextFrameMaterials<M> {
    fn default() -> Self {
        Self { assets: Vec::new() }
    }
}

pub fn prepare_materials<M: BillboardMaterial>(
    mut prepare_next_frame: Local<PrepareNextFrameMaterials<M>>,
    mut extracted_materials: ResMut<ExtractedBillboardMaterials<M>>,
    mut prepared_materials: ResMut<PreparedBillboardMaterials<M>>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    fallback_image: Res<FallbackImage>,
    pipeline: Res<BillboardMaterialPipeline<M>>,
) {
    let queued_assets = std::mem::take(&mut prepare_next_frame.assets);
    for (handle, material) in queued_assets.into_iter() {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                prepared_materials.map.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }

    for removed in std::mem::take(&mut extracted_materials.removed) {
        prepared_materials.map.remove(&removed);
    }

    for (handle, material) in std::mem::take(&mut extracted_materials.extracted) {
        match prepare_material(
            &material,
            &render_device,
            &images,
            &fallback_image,
            &pipeline,
        ) {
            Ok(prepared_asset) => {
                prepared_materials.map.insert(handle, prepared_asset);
            }
            Err(AsBindGroupError::RetryNextUpdate) => {
                prepare_next_frame.assets.push((handle, material));
            }
        }
    }
}

fn prepare_material<M: BillboardMaterial>(
    material: &M,
    render_device: &RenderDevice,
    images: &RenderAssets<Image>,
    fallback_image: &FallbackImage,
    pipeline: &BillboardMaterialPipeline<M>,
) -> Result<PreparedBillboardMaterial<M>, AsBindGroupError> {
    let prepared = material.as_bind_group(
        &pipeline.material_layout,
        render_device,
        images,
        fallback_image,
    )?;

    Ok(PreparedBillboardMaterial {
        bindings: prepared.bindings,
        bind_group: prepared.bind_group,
        key: prepared.data,
    })
}
