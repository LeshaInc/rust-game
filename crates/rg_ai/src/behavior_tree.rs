use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};

#[derive(Default)]
pub struct BehaviorTreePlugin;

impl Plugin for BehaviorTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<BehaviorTree>()
            .configure_sets(
                FixedUpdate,
                (
                    BehaviorTreeSystem::Process,
                    BehaviorTreeSystem::Transition,
                    BehaviorTreeSystem::TransitionFlush,
                    BehaviorTreeSystem::PreInstantiate,
                    BehaviorTreeSystem::Instantiate,
                )
                    .chain(),
            )
            .add_systems(
                FixedUpdate,
                (
                    initialize_agents.in_set(BehaviorTreeSystem::Transition),
                    apply_deferred.in_set(BehaviorTreeSystem::TransitionFlush),
                    reset_instantiated_flag.in_set(BehaviorTreeSystem::PreInstantiate),
                ),
            );
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, SystemSet)]
pub enum BehaviorTreeSystem {
    /// Process behavior logic
    Process,
    /// Transition to child or parent nodes
    Transition,
    /// apply_deferred is called here
    TransitionFlush,
    /// Runs before `Instantiate`
    PreInstantiate,
    /// Create concrete Behavior<A> instances
    Instantiate,
}

#[derive(Debug, Default, Clone, TypeUuid, TypePath)]
#[uuid = "8c479413-a75b-47a1-93ee-91af54fc5e79"]
pub struct BehaviorTree {
    nodes: Vec<Node>,
}

impl BehaviorTree {
    pub fn new() -> BehaviorTree {
        default()
    }

    pub fn add_node(&mut self, node: impl Into<Node>) -> NodeId {
        let id = NodeId(self.nodes.len());
        self.nodes.push(node.into());
        id
    }

    pub fn add_child(&mut self, parent_id: NodeId, child_id: NodeId) {
        assert!(self.nodes[child_id.0 as usize].parent.is_none());
        self.nodes[parent_id.0 as usize].children.push(child_id);
        self.nodes[child_id.0 as usize].parent = Some(parent_id);
    }

    pub fn get_node(&self, node_id: NodeId) -> &Node {
        &self.nodes[node_id.0]
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct NodeId(pub usize);

impl NodeId {
    pub const ROOT: NodeId = NodeId(0);
}

#[derive(Debug)]
pub struct Node {
    action: Box<dyn Reflect>,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
}

impl Node {
    pub fn get_action(&self) -> Box<dyn Reflect> {
        self.action.clone_value()
    }

    pub fn num_children(&self) -> usize {
        self.children.len()
    }

    pub fn parent_id(&self) -> Option<NodeId> {
        self.parent
    }

    pub fn child_id(&self, index: usize) -> NodeId {
        self.children[index]
    }
}

impl<A: Action> From<A> for Node {
    fn from(action: A) -> Self {
        Self {
            action: Box::new(action).into_reflect(),
            parent: None,
            children: Vec::new(),
        }
    }
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Node {
            action: self.action.clone_value(),
            parent: self.parent,
            children: self.children.clone(),
        }
    }
}

pub trait Action: Reflect + FromReflect {
    fn register(app: &mut App);
}

pub trait AddAction {
    fn add_action<A: Action>(&mut self) -> &mut App;
}

impl AddAction for App {
    fn add_action<A: Action>(&mut self) -> &mut App {
        A::register(self);

        self.add_systems(
            FixedUpdate,
            (
                remove_stale_agents::<A>.in_set(BehaviorTreeSystem::Transition),
                transition_behaviors::<A>.in_set(BehaviorTreeSystem::Transition),
                instantiate_behaviors::<A>.in_set(BehaviorTreeSystem::Instantiate),
            ),
        );

        self
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum BehaviorResult {
    Success,
    Failure,
}

#[derive(Debug, Copy, Clone)]
pub enum BehaviorCommand {
    Continue,
    Exit { result: BehaviorResult },
    RunChild { index: usize },
}

#[derive(Debug, Component)]
#[component(storage = "SparseSet")]
pub struct Behavior<A> {
    pub action: A,
    command: BehaviorCommand,
    node_id: NodeId,
    num_children: usize,
    child_result: Option<BehaviorResult>,
}

impl<A> Behavior<A> {
    pub fn num_children(&self) -> usize {
        self.num_children
    }

    pub fn has_returned_from_child(&self) -> bool {
        self.child_result.is_some()
    }

    pub fn child_failed(&self) -> bool {
        self.child_result == Some(BehaviorResult::Failure)
    }

    pub fn child_succeeded(&self) -> bool {
        self.child_result == Some(BehaviorResult::Failure)
    }

    pub fn run_child(&mut self, index: usize) {
        self.command = BehaviorCommand::RunChild { index };
    }

    pub fn success(&mut self) {
        self.command = BehaviorCommand::Exit {
            result: BehaviorResult::Success,
        };
    }

    pub fn failure(&mut self) {
        self.command = BehaviorCommand::Exit {
            result: BehaviorResult::Failure,
        };
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct BehaviorStack {
    stack: Vec<Box<dyn Reflect>>,
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PassBehavior {
    node_id: NodeId,
    num_children: usize,
    child_result: Option<BehaviorResult>,
}

pub fn transition_behaviors<A: Action>(
    mut q_agents: Query<(
        Entity,
        &Behavior<A>,
        &mut BehaviorStack,
        &Handle<BehaviorTree>,
    )>,
    mut commands: Commands,
    trees: Res<Assets<BehaviorTree>>,
) {
    for (entity, behavior, mut behavior_stack, tree_handle) in &mut q_agents {
        if let BehaviorCommand::Continue = behavior.command {
            continue;
        }

        let Some(tree) = trees.get(tree_handle) else {
            continue;
        };

        let node = tree.get_node(behavior.node_id);

        let (node_id, num_children, child_result) = match behavior.command {
            BehaviorCommand::Exit { result } => {
                if behavior_stack.stack.is_empty() {
                    // start over, resetting to initial state
                    let action = tree.get_node(behavior.node_id).get_action();
                    behavior_stack.stack.push(action);
                    (behavior.node_id, behavior.num_children, None)
                } else {
                    // return to parent
                    let parent_id = node.parent_id().unwrap();
                    let parent_node = tree.get_node(parent_id);
                    (parent_id, parent_node.num_children(), Some(result))
                }
            }

            BehaviorCommand::RunChild { index } => {
                // save state
                let this_action = behavior.action.as_reflect().clone_value();
                behavior_stack.stack.push(this_action);

                let child_id = node.child_id(index);
                let child_node = tree.get_node(child_id);
                let child_action = child_node.get_action();
                behavior_stack.stack.push(child_action);

                (child_id, child_node.num_children(), None)
            }

            _ => unreachable!(),
        };

        commands
            .entity(entity)
            .remove::<Behavior<A>>()
            .insert(PassBehavior {
                node_id,
                num_children,
                child_result,
            });
    }
}

pub fn initialize_agents(
    mut q_agents: Query<(Entity, &Handle<BehaviorTree>), Without<BehaviorStack>>,
    mut commands: Commands,
    trees: Res<Assets<BehaviorTree>>,
) {
    for (entity, tree_handle) in &mut q_agents {
        let Some(tree) = trees.get(tree_handle) else {
            continue;
        };

        let node = tree.get_node(NodeId::ROOT);
        let stack = BehaviorStack {
            stack: vec![node.get_action()],
        };

        commands.entity(entity).insert((
            stack,
            InstantiatedFlag(false),
            PassBehavior {
                node_id: NodeId::ROOT,
                num_children: node.num_children(),
                child_result: None,
            },
        ));
    }
}

#[derive(Copy, Clone, Component)]
pub struct InstantiatedFlag(bool);

pub fn reset_instantiated_flag(
    mut q_agents: Query<(Entity, &mut InstantiatedFlag), (With<PassBehavior>, With<BehaviorStack>)>,
) {
    for (_entity, mut instantiated) in &mut q_agents {
        instantiated.0 = false;
    }
}

pub fn instantiate_behaviors<A: Action>(
    mut q_agents: Query<(
        Entity,
        &PassBehavior,
        &mut BehaviorStack,
        &mut InstantiatedFlag,
    )>,
    mut commands: Commands,
) {
    for (entity, pass_behavior, mut behavior_stack, mut instantiated) in &mut q_agents {
        if instantiated.0 {
            continue;
        }

        let Some(last) = behavior_stack.stack.last() else {
            continue;
        };

        if !last.represents::<A>() {
            continue;
        }

        instantiated.0 = true;
        let action = A::take_from_reflect(behavior_stack.stack.pop().unwrap()).unwrap();

        commands
            .entity(entity)
            .remove::<PassBehavior>()
            .insert(Behavior::<A> {
                action,
                command: BehaviorCommand::Continue,
                node_id: pass_behavior.node_id,
                num_children: pass_behavior.num_children,
                child_result: pass_behavior.child_result,
            });
    }
}

pub fn remove_stale_agents<A: Action>(
    mut q_agents: Query<Entity, (With<Behavior<A>>, Without<Handle<BehaviorTree>>)>,
    mut commands: Commands,
) {
    for entity in &mut q_agents {
        commands
            .entity(entity)
            .remove::<Behavior<A>>()
            .remove::<BehaviorStack>()
            .remove::<InstantiatedFlag>();
    }
}
