use crate::prelude::*;

pub fn install(session: &mut CoreSession) {
    session
        .stages
        .add_system_to_stage(CoreStage::PreUpdate, hydrate)
        .add_system_to_stage(CoreStage::PostUpdate, update_lit_kick_bombs)
        .add_system_to_stage(CoreStage::PostUpdate, update_idle_kick_bombs);
}

#[derive(Clone, TypeUlid, Debug, Copy)]
#[ulid = "01GQ0ZWBNA8HZRXYKZXCT05CXT"]
pub struct IdleKickBomb;

#[derive(Clone, TypeUlid, Debug)]
#[ulid = "01GQ0ZWFYZSHJPESPY9QPSTARR"]
pub struct LitKickBomb {
    arm_delay: Timer,
    fuse_time: Timer,
}

#[derive(Clone, TypeUlid, Debug)]
#[ulid = "01GQ0ZWFYZSHJPESPY9QPSTARR"]
pub struct KickCount {
    count: u32,
}

fn hydrate(
    game_meta: Res<CoreMetaArc>,
    mut items: CompMut<Item>,
    mut item_throws: CompMut<ItemThrow>,
    mut item_grabs: CompMut<ItemGrab>,
    mut entities: ResMut<Entities>,
    mut bodies: CompMut<KinematicBody>,
    mut transforms: CompMut<Transform>,
    mut idle_bombs: CompMut<IdleKickBomb>,
    mut atlas_sprites: CompMut<AtlasSprite>,
    element_assets: BevyAssets<ElementMeta>,
    mut hydrated: CompMut<MapElementHydrated>,
    mut element_handles: CompMut<ElementHandle>,
    mut animated_sprites: CompMut<AnimatedSprite>,
    mut respawn_points: CompMut<DehydrateOutOfBounds>,
    mut spawner_manager: SpawnerManager,
) {
    let mut not_hydrated_bitset = hydrated.bitset().clone();
    not_hydrated_bitset.bit_not();
    not_hydrated_bitset.bit_and(element_handles.bitset());

    let spawner_entities = entities
        .iter_with_bitset(&not_hydrated_bitset)
        .collect::<Vec<_>>();

    for spawner_ent in spawner_entities {
        let transform = *transforms.get(spawner_ent).unwrap();
        let element_handle = element_handles.get(spawner_ent).unwrap();
        let Some(element_meta) = element_assets.get(&element_handle.get_bevy_handle()) else {
            continue;
        };

        if let BuiltinElementKind::KickBomb {
            atlas,
            fin_anim,
            grab_offset,
            body_diameter,
            can_rotate,
            bounciness,
            throw_velocity,
            angular_velocity,
            ..
        } = &element_meta.builtin
        {
            hydrated.insert(spawner_ent, MapElementHydrated);

            let entity = entities.create();
            items.insert(entity, Item);
            item_throws.insert(
                entity,
                ItemThrow::strength(*throw_velocity).with_spin(*angular_velocity),
            );
            item_grabs.insert(
                entity,
                ItemGrab {
                    fin_anim: *fin_anim,
                    sync_animation: false,
                    grab_offset: *grab_offset,
                },
            );
            idle_bombs.insert(entity, IdleKickBomb);
            atlas_sprites.insert(entity, AtlasSprite::new(atlas.clone()));
            respawn_points.insert(entity, DehydrateOutOfBounds(spawner_ent));
            transforms.insert(entity, transform);
            element_handles.insert(entity, element_handle.clone());
            hydrated.insert(entity, MapElementHydrated);
            animated_sprites.insert(entity, default());
            bodies.insert(
                entity,
                KinematicBody {
                    shape: ColliderShape::Circle {
                        diameter: *body_diameter,
                    },
                    gravity: game_meta.physics.gravity,
                    has_mass: true,
                    has_friction: true,
                    can_rotate: *can_rotate,
                    bounciness: *bounciness,
                    ..default()
                },
            );
            spawner_manager.create_spawner(spawner_ent, vec![entity])
        }
    }
}

fn update_idle_kick_bombs(
    entities: Res<Entities>,
    mut commands: Commands,
    mut items_used: CompMut<ItemUsed>,
    mut audio_events: ResMut<AudioEvents>,
    element_handles: Comp<ElementHandle>,
    mut idle_bombs: CompMut<IdleKickBomb>,
    element_assets: BevyAssets<ElementMeta>,
    mut animated_sprites: CompMut<AnimatedSprite>,
) {
    for (entity, (_kick_bomb, element_handle)) in
        entities.iter_with((&mut idle_bombs, &element_handles))
    {
        let Some(element_meta) = element_assets.get(&element_handle.get_bevy_handle()) else {
            continue;
        };

        let BuiltinElementKind::KickBomb {
            fuse_sound,
            fuse_sound_volume,
            arm_delay,
            fuse_time,
            ..
        } = &element_meta.builtin else {
            unreachable!();
        };

        let arm_delay = *arm_delay;
        let fuse_time = *fuse_time;

        if items_used.get(entity).is_some() {
            audio_events.play(fuse_sound.clone(), *fuse_sound_volume);
            items_used.remove(entity);
            let animated_sprite = animated_sprites.get_mut(entity).unwrap();
            animated_sprite.frames = Arc::from([3, 4, 5]);
            animated_sprite.repeat = true;
            animated_sprite.fps = 8.0;
            commands.add(
                move |mut idle: CompMut<IdleKickBomb>, mut lit: CompMut<LitKickBomb>| {
                    idle.remove(entity);
                    lit.insert(
                        entity,
                        LitKickBomb {
                            arm_delay: Timer::new(arm_delay, TimerMode::Once),
                            fuse_time: Timer::new(fuse_time, TimerMode::Once),
                        },
                    );
                },
            );
        }
    }
}

fn update_lit_kick_bombs(
    entities: Res<Entities>,
    element_handles: Comp<ElementHandle>,
    element_assets: BevyAssets<ElementMeta>,

    collision_world: CollisionWorld,
    player_indexes: Comp<PlayerIdx>,
    mut audio_events: ResMut<AudioEvents>,
    mut trauma_events: ResMut<CameraTraumaEvents>,
    mut lit_grenades: CompMut<LitKickBomb>,
    mut sprites: CompMut<AtlasSprite>,
    mut bodies: CompMut<KinematicBody>,
    mut hydrated: CompMut<MapElementHydrated>,
    mut attachments: CompMut<PlayerBodyAttachment>,
    mut player_layers: CompMut<PlayerLayers>,
    player_inventories: PlayerInventories,
    mut transforms: CompMut<Transform>,
    mut commands: Commands,
    time: Res<Time>,
    spawners: Comp<DehydrateOutOfBounds>,
    invincibles: CompMut<Invincibility>,
    mut kicked_counts: CompMut<KickCount>, // Component to store kick counts 
) {
    for (entity, (kick_bomb, element_handle, spawner)) in
        entities.iter_with((&mut lit_grenades, &element_handles, &spawners))
    {
        let Some(element_meta) = element_assets.get(&element_handle.get_bevy_handle()) else {
            continue;
        };

        let BuiltinElementKind::KickBomb {
            grab_offset,
            explosion_sound,
            explosion_volume,
            kick_velocity,
            damage_region_lifetime,
            damage_region_size,
            explosion_lifetime,
            explosion_atlas,
            explosion_fps,
            explosion_frames,
            fin_anim,
            ..
        } = &element_meta.builtin else {
            unreachable!();
        };

        kick_bomb.fuse_time.tick(time.delta());
        kick_bomb.arm_delay.tick(time.delta());

        let mut should_explode = false;
        let mut kicked_count = 0; // Initialize the kicked count variable

        // If the item is being held
        if let Some(inventory) = player_inventories
            .iter()
            .find_map(|x| x.filter(|x| x.inventory == entity))
        {
            // Increment the kicked count
            kicked_count += 1;

            // Check if kicked_count reaches 3 
            if kicked_count == 3 {
                should_explode = true;
            } 
            
            let player = inventory.player;
            let body = bodies.get_mut(entity).unwrap();
            player_layers.get_mut(player).unwrap().fin_anim = *fin_anim;

            // Deactivate held items
            body.is_deactivated = true;

            // Attach to the player
            attachments.insert(
                entity,
                PlayerBodyAttachment {
                    player,
                    sync_color: false,
                    sync_animation: false,
                    head: false,
                    offset: grab_offset.extend(1.0),
                },
            );
        }
        // The item is on the ground
        else if let Some(player_entity) = collision_world
            .actor_collisions_filtered(entity, |e| invincibles.get(e).is_none())
            .into_iter()
            .find(|&x| player_indexes.contains(x))
        {
            let body = bodies.get_mut(entity).unwrap();
            let translation = transforms.get_mut(entity).unwrap().translation;

            let player_sprite = sprites.get_mut(player_entity).unwrap();
            let player_translation = transforms.get(player_entity).unwrap().translation;

            let player_standing_left = player_translation.x <= translation.x;

            if body.velocity.x == 0.0 {
                body.velocity = *kick_velocity;
                if player_sprite.flip_x {
                    body.velocity.x *= -1.0;
                }
            } else if player_standing_left && !player_sprite.flip_x {
                body.velocity.x = kick_velocity.x;
                body.velocity.y = kick_velocity.y;
            } else if !player_standing_left && player_sprite.flip_x {
                body.velocity.x = -kick_velocity.x;
                body.velocity.y = kick_velocity.y;
            } else if kick_bomb.arm_delay.finished() {
                should_explode = true;
            }
        }

        kicked_counts.insert( entity, KickCount { count: kicked_count }); 
        
        // If it's time to explode
        if kick_bomb.fuse_time.finished() || should_explode {
            audio_events.play(explosion_sound.clone(), *explosion_volume);

            trauma_events.send(7.5);

            // Cause the item to respawn by un-hydrating it's spawner.
            hydrated.remove(**spawner);
            let mut explosion_transform = *transforms.get(entity).unwrap();
            explosion_transform.translation.z = -10.0; // On top of almost everything
            explosion_transform.rotation = Quat::IDENTITY;

            // Clone types for move into closure
            let damage_region_size = *damage_region_size;
            let damage_region_lifetime = *damage_region_lifetime;
            let explosion_lifetime = *explosion_lifetime;
            let explosion_atlas = explosion_atlas.clone();
            let explosion_fps = *explosion_fps;
            let explosion_frames = *explosion_frames;
            commands.add(
                move |mut entities: ResMut<Entities>,
                      mut transforms: CompMut<Transform>,
                      mut damage_regions: CompMut<DamageRegion>,
                      mut lifetimes: CompMut<Lifetime>,
                      mut sprites: CompMut<AtlasSprite>,
                      mut animated_sprites: CompMut<AnimatedSprite>| {
                    // Despawn the kick bomb
                    entities.kill(entity);

                    // Spawn the damage region
                    let ent = entities.create();
                    transforms.insert(ent, explosion_transform);
                    damage_regions.insert(
                        ent,
                        DamageRegion {
                            size: damage_region_size,
                        },
                    );
                    lifetimes.insert(ent, Lifetime::new(damage_region_lifetime));

                    // Spawn the explosion animation
                    let ent = entities.create();
                    transforms.insert(ent, explosion_transform);
                    sprites.insert(
                        ent,
                        AtlasSprite {
                            atlas: explosion_atlas.clone(),
                            ..default()
                        },
                    );
                    animated_sprites.insert(
                        ent,
                        AnimatedSprite {
                            frames: (0..explosion_frames).collect(),
                            fps: explosion_fps,
                            repeat: false,
                            ..default()
                        },
                    );
                    lifetimes.insert(ent, Lifetime::new(explosion_lifetime));
                },
            );
        }
    }
}
