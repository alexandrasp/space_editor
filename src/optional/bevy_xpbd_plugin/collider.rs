
use bevy::prelude::*;
use bevy_xpbd_3d::math::*;
use bevy_xpbd_3d::prelude::*;
pub use bevy_inspector_egui::prelude::*;

use crate::prefab::component::MeshPrimitivePrefab;

use super::RigidBodyPrefab;



#[derive(Reflect, Debug, Clone, PartialEq, Component, InspectorOptions)]
#[reflect(Component, Default)]
pub enum ColliderPrefab {
    FromMesh,
    FromPrefabMesh,
    Primitive{pos : Vector, rot : Vector, primitive : ColliderPrimitive},
    Compound(ColliderPrefabCompound)
}

#[derive(Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(Default)]
pub struct ColliderPrefabCompound {
    pub parts : Vec<ColliderPart>
}

impl Default for ColliderPrefab {
    fn default() -> Self {
        ColliderPrefab::Primitive { pos: Vector::default(), rot: Vector::default(), primitive: ColliderPrimitive::Cuboid(Vector::ONE) }
    }
}

#[derive(Reflect, Debug, Clone, PartialEq, Default)]
#[reflect(Default)]
pub struct ColliderPart {
    pub pos : Vector,
    pub rot : Vector,
    pub primitive : ColliderPrimitive
}

#[derive(Debug, Reflect, Clone, PartialEq)]
#[reflect(Default)]
pub enum ColliderPrimitive {
    Cuboid(Vector),
    Capsule{height : Scalar, radius : Scalar},
    CapsuleEndpoints{a : Vector, b : Vector, radius : Scalar},
    Cone{height : Scalar, radius : Scalar},
    Cylinder{height : Scalar, radius : Scalar},
    Halfspace{outward_normal : Vector},
    Triangle{a : Vector, b : Vector, c : Vector},
    Ball(Scalar),
    Segment{a : Vector, b : Vector},
}


impl Default for ColliderPrimitive {
    fn default() -> Self {
        ColliderPrimitive::Cuboid(Vector::new(1.0, 1.0, 1.0))
    }
}

impl ColliderPrimitive {
    pub fn to_collider(&self) -> Collider {
        match self {
            ColliderPrimitive::Cuboid(bbox) => {
                Collider::cuboid(bbox.x, bbox.y, bbox.z)
            },
            ColliderPrimitive::Capsule { height, radius } => Collider::capsule(*height, *radius),
            ColliderPrimitive::CapsuleEndpoints { a, b, radius } => Collider::capsule_endpoints(*a, *b, *radius),
            ColliderPrimitive::Cone { height, radius } => Collider::cone(*height, *radius),
            ColliderPrimitive::Cylinder { height, radius } => Collider::cylinder(*height, *radius),
            ColliderPrimitive::Halfspace { outward_normal } => Collider::halfspace(*outward_normal),
            ColliderPrimitive::Triangle { a, b, c } => Collider::triangle(*a, *b, *c),
            ColliderPrimitive::Ball(radius) => Collider::ball(*radius),
            ColliderPrimitive::Segment { a, b } => Collider::segment(*a, *b),
        }
    }
}

pub fn update_collider(
    mut commands : Commands,
    query : Query<(Entity, &ColliderPrefab, Option<&RigidBodyPrefab>, Option<&Transform>, Option<&Handle<Mesh>>, Option<&MeshPrimitivePrefab>), Changed<ColliderPrefab>>,
    updated_meshes : Query<(Entity, &ColliderPrefab, &Handle<Mesh>), Changed<Handle<Mesh>>>,
    updated_prefab_meshes : Query<(Entity, &ColliderPrefab, &MeshPrimitivePrefab), Changed<MeshPrimitivePrefab>>,
    meshes : Res<Assets<Mesh>>
) {
    for (e, collider, rigidbody, transform, mesh, prefab_mesh) in query.iter() {
        commands.entity(e).remove::<Collider>();
        let col = get_collider(collider, mesh, &meshes, prefab_mesh);
        commands.entity(e).insert(col);

        if rigidbody.is_none() {
            commands.entity(e).insert(RigidBodyPrefab::Static);
        }

        if transform.is_none() {
            commands.entity(e).insert(TransformBundle::default());
        }
    }

    for (e, collider, mesh) in updated_meshes.iter() {
        if *collider == ColliderPrefab::FromMesh {
            if let Some(mesh) = meshes.get(mesh) {
                if let Some(col) = Collider::convex_decomposition_from_bevy_mesh(mesh) {
                    commands.entity(e).insert(col);
                } else {
                    commands.entity(e).insert(Collider::trimesh_from_bevy_mesh(mesh).unwrap_or_default());
                }
            } else {
                commands.entity(e).insert(Collider::default());
            }
        }
    }

    for (e, collider, mesh) in updated_prefab_meshes.iter() {
        if *collider == ColliderPrefab::FromPrefabMesh {
            commands.entity(e).remove::<Collider>();
            commands.entity(e).insert(get_prefab_mesh_collider(mesh));
        }
    }
}

fn get_collider(collider: &ColliderPrefab, mesh: Option<&Handle<Mesh>>, meshes: &Assets<Mesh>, prefab_mesh: Option<&MeshPrimitivePrefab>) -> Collider {
    match collider {
        ColliderPrefab::FromMesh => {
            if let Some(mesh) = mesh {
                if let Some(mesh) = meshes.get(mesh) {
                    Collider::trimesh_from_bevy_mesh(mesh).unwrap_or_default()
                } else {
                    Collider::default()
                } 
            } else {
                Collider::default()
            }
        },
        ColliderPrefab::FromPrefabMesh => {
            if let Some(mesh) = prefab_mesh {
                
                get_prefab_mesh_collider(mesh)
            } else {
                Collider::default()
            }
        },
        ColliderPrefab::Primitive { pos, rot, primitive } => {
            Collider::compound(vec![(*pos, Quaternion::from_euler(EulerRot::XYZ, rot.x, rot.y, rot.z), primitive.to_collider())])
        },
        ColliderPrefab::Compound(com) => {
            if !com.parts.is_empty() {
                return Collider::compound(
                    com.parts.iter().map(|p| (p.pos,  Quaternion::from_euler(EulerRot::XYZ, p.rot.x, p.rot.y, p.rot.z), p.primitive.to_collider())).collect()
                );
            } else {
                Collider::default()
            }
        }
    }
}

fn get_prefab_mesh_collider(mesh: &MeshPrimitivePrefab) -> Collider {
    const EPS : f32 = 0.00001;
    
    match mesh {
        MeshPrimitivePrefab::Cube(val) => Collider::cuboid(*val as Scalar, *val as Scalar, *val as Scalar),
        MeshPrimitivePrefab::Box(val) => Collider::cuboid(val.w as Scalar, val.h as Scalar, val.d as Scalar),
        MeshPrimitivePrefab::Sphere(val) => Collider::ball(val.r as Scalar),
        MeshPrimitivePrefab::Quad(val) => Collider::cuboid(val.size.x as Scalar, val.size.y as Scalar, EPS as Scalar),
        MeshPrimitivePrefab::Capsule(val) => Collider::capsule(1.0, val.r as Scalar), 
        MeshPrimitivePrefab::Circle(val) =>Collider::trimesh_from_bevy_mesh(&val.to_mesh()).unwrap_or_default(),
        MeshPrimitivePrefab::Cylinder(val) => Collider::cylinder(1.0, val.r as Scalar),
        MeshPrimitivePrefab::Icosphere(val) => Collider::trimesh_from_bevy_mesh(&val.to_mesh()).unwrap_or_default(),
        MeshPrimitivePrefab::Plane(val) => Collider::cuboid(val.size as Scalar, EPS as Scalar, val.size as Scalar),
        MeshPrimitivePrefab::RegularPolygon(val) => Collider::trimesh_from_bevy_mesh(&val.to_mesh()).unwrap_or_default(),
        MeshPrimitivePrefab::Torus(val) => Collider::trimesh_from_bevy_mesh(&val.to_mesh()).unwrap_or_default(),
    }
}

// pub fn debug_draw_collider(
//     mut gizmo : Gizmos,
//     query : Query<(Entity, &Collider), Changed<ColliderPrefab>>
// ) {

// }