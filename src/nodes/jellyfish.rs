use macroquad::{
    color,
    prelude::{
        animation::{AnimatedSprite, Animation},
        collections::storage,
        coroutines::{start_coroutine, Coroutine},
        draw_texture_ex,
        scene::{self, Handle, HandleUntyped, RefMut},
        vec2, DrawTextureParams, Rect, Vec2,
    },
};

use crate::Resources;

use super::{
    player::{capabilities, PhysicsBody, Weapon, PLAYER_HITBOX_HEIGHT, PLAYER_HITBOX_WIDTH},
    FlappyJellyfish, Player,
};

const JELLYFISH_WIDTH: f32 = 32.;
const JELLYFISH_HEIGHT: f32 = 29.;
const JELLYFISH_ANIMATION_BASE: &'static str = "base";

/// Statuses, in order
#[derive(Copy, Clone, Debug)]
pub enum MountStatus {
    // This is the normal sequence of statuses. Death will reset the state to Dropped
    Dropped,
    Mounted,
    Driving,
    Dismounted,
}

pub struct Jellyfish {
    jellyfish_sprite: AnimatedSprite,

    pub mount_status: MountStatus,

    pub body: PhysicsBody,

    origin_pos: Vec2,
    deadly_dangerous: bool,
}

impl Jellyfish {
    pub fn new(facing: bool, pos: Vec2) -> Self {
        let jellyfish_sprite = AnimatedSprite::new(
            JELLYFISH_WIDTH as u32,
            JELLYFISH_HEIGHT as u32,
            &[Animation {
                name: JELLYFISH_ANIMATION_BASE.to_string(),
                row: 0,
                frames: 1,
                fps: 1,
            }],
            false,
        );

        Self {
            jellyfish_sprite,
            body: PhysicsBody {
                pos,
                facing,
                angle: 0.0,
                speed: vec2(0., 0.),
                collider: None,
                on_ground: false,
                last_frame_on_ground: false,
                have_gravity: true,
                bouncyness: 0.0,
            },
            mount_status: MountStatus::Mounted,
            origin_pos: pos,
            deadly_dangerous: false,
        }
    }

    pub fn throw(&mut self, force: bool) {
        self.mount_status = MountStatus::Dropped;

        if force {
            self.body.speed = if self.body.facing {
                vec2(600., -200.)
            } else {
                vec2(-600., -200.)
            };
        } else {
            self.body.angle = 3.5;
        }

        let mut resources = storage::get_mut::<Resources>();

        let jellyfish_mount_pos = if self.body.facing {
            vec2(30., 10.)
        } else {
            vec2(-50., 10.)
        };

        if self.body.collider.is_none() {
            self.body.collider = Some(resources.collision_world.add_actor(
                self.body.pos + jellyfish_mount_pos,
                40,
                30,
            ));
        } else {
            resources.collision_world.set_actor_position(
                self.body.collider.unwrap(),
                self.body.pos + jellyfish_mount_pos,
            );
        }
        self.origin_pos = self.body.pos + jellyfish_mount_pos / 2.;
    }

    pub fn shoot(node_h: Handle<Jellyfish>, player: Handle<Player>) -> Coroutine {
        let coroutine = async move {
            {
                let mut node = scene::get_node(node_h);
                let player = &mut *scene::get_node(player);

                match node.mount_status {
                    MountStatus::Mounted => {
                        let was_spawned = FlappyJellyfish::spawn(&mut *node, player);

                        if !was_spawned {
                            player.state_machine.set_state(Player::ST_NORMAL);
                        }
                    }
                    MountStatus::Dismounted => {
                        Jellyfish::throw(&mut *node, true);
                        player.weapon = None;
                        player.state_machine.set_state(Player::ST_NORMAL);
                    }

                    _ => panic!("Unexpected jellyfish mount status: {:?}", node.mount_status),
                }

                player.floating = false;
            }
        };

        start_coroutine(coroutine)
    }

    pub fn gun_capabilities() -> capabilities::Gun {
        fn throw(node: HandleUntyped, force: bool) {
            let mut node = scene::get_untyped_node(node)
                .unwrap()
                .to_typed::<Jellyfish>();

            Jellyfish::throw(&mut *node, force);
        }

        fn shoot(node: HandleUntyped, player: Handle<Player>) -> Coroutine {
            let node = scene::get_untyped_node(node)
                .unwrap()
                .to_typed::<Jellyfish>()
                .handle();

            Jellyfish::shoot(node, player)
        }

        fn is_thrown(node: HandleUntyped) -> bool {
            let node = scene::get_untyped_node(node)
                .unwrap()
                .to_typed::<Jellyfish>();

            matches!(node.mount_status, MountStatus::Dropped)
        }

        fn pick_up(node: HandleUntyped) {
            let mut node = scene::get_untyped_node(node)
                .unwrap()
                .to_typed::<Jellyfish>();

            node.body.angle = 0.;
            node.mount_status = MountStatus::Mounted;
        }

        capabilities::Gun {
            throw,
            shoot,
            is_thrown,
            pick_up,
        }
    }
}

impl scene::Node for Jellyfish {
    fn ready(mut node: RefMut<Self>) {
        node.provides::<Weapon>((
            node.handle().untyped(),
            node.handle().lens(|node| &mut node.body),
            Self::gun_capabilities(),
        ));
    }

    fn fixed_update(mut node: RefMut<Self>) {
        node.jellyfish_sprite.update();

        if matches!(node.mount_status, MountStatus::Dropped) {
            node.body.update();
            node.body.update_throw();

            if (node.origin_pos - node.body.pos).length() > 70. {
                node.deadly_dangerous = true;
            }
            if node.body.speed.length() <= 200.0 {
                node.deadly_dangerous = false;
            }
            if node.body.on_ground {
                node.deadly_dangerous = false;
            }

            if node.deadly_dangerous {
                let others = scene::find_nodes_by_type::<crate::nodes::Player>();
                let jellyfish_hitbox = Rect::new(
                    node.body.pos.x,
                    node.body.pos.y,
                    JELLYFISH_WIDTH,
                    JELLYFISH_HEIGHT,
                );

                for mut player in others {
                    let player_hitbox = Rect::new(
                        player.body.pos.x,
                        player.body.pos.y,
                        PLAYER_HITBOX_WIDTH,
                        PLAYER_HITBOX_HEIGHT,
                    );
                    if player_hitbox.overlaps(&jellyfish_hitbox) {
                        player.kill(!node.body.facing);
                    }
                }
            }
        }
    }

    fn draw(node: RefMut<Self>) {
        let resources = storage::get_mut::<Resources>();

        let jellyfish_mount_pos = match node.mount_status {
            MountStatus::Dropped => {
                if node.body.facing {
                    vec2(-25., 0.)
                } else {
                    vec2(5., 0.)
                }
            }
            _ => {
                if node.body.facing {
                    vec2(-8., -19.)
                } else {
                    vec2(4., -19.)
                }
            }
        };

        draw_texture_ex(
            resources.jellyfish,
            node.body.pos.x + jellyfish_mount_pos.x,
            node.body.pos.y + jellyfish_mount_pos.y,
            color::WHITE,
            DrawTextureParams {
                source: Some(node.jellyfish_sprite.frame().source_rect),
                dest_size: Some(node.jellyfish_sprite.frame().dest_size),
                flip_x: !node.body.facing,
                rotation: node.body.angle,
                ..Default::default()
            },
        );
    }
}