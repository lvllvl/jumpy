// #[cfg(not(target_arch = "wasm32"))]
// use crate::networking::{NetworkMatchSocket, SocketTarget};

use super::*;

// const GAMEPAD_ACTION_IDX: usize = 0;
// const KEYPAD_ACTION_IDX: usize = 1;

#[derive(Default, Clone, Debug)]
pub struct PlayerSelectState {
    pub slots: [PlayerSlot; MAX_PLAYERS],
}

#[derive(Default, Clone, Copy, Debug)]
pub struct PlayerSlot {
    pub active: bool,
    pub confirmed: bool,
    pub selected_player: Handle<PlayerMeta>,
    pub selected_hat: Option<Handle<HatMeta>>,
    pub control_source: Option<ControlSource>,
}

impl PlayerSlot {
    pub fn is_ai(&self) -> bool {
        self.control_source.is_none()
    }
}

// /// Network message that may be sent during player selection.
// #[derive(Serialize, Deserialize)]
// pub enum PlayerSelectMessage {
//     SelectPlayer(Handle<PlayerMeta>),
//     SelectHat(Option<Handle<HatMeta>>),
//     ConfirmSelection(bool),
// }

pub fn widget(
    mut ui: In<&mut egui::Ui>,
    meta: Root<GameMeta>,
    localization: Localization<GameMeta>,
    controls: Res<GlobalPlayerControls>,
    world: &World,
) {
    let is_online = false;
    let mut state = ui.ctx().get_state::<PlayerSelectState>();
    ui.ctx().set_state(EguiInputSettings {
        disable_keyboard_input: true,
        disable_gamepad_input: true,
    });

    // #[cfg(not(target_arch = "wasm32"))]
    // handle_match_setup_messages(&mut params);

    // Whether or not the continue button should be enabled
    let mut ready_players = 0;
    let mut unconfirmed_players = 0;

    for slot in &state.slots {
        if slot.confirmed {
            ready_players += 1;
        } else if slot.active {
            unconfirmed_players += 1;
        }
    }
    let may_continue = ready_players >= 1 && unconfirmed_players == 0;

    // #[cfg(not(target_arch = "wasm32"))]
    // if let Some(socket) = &params.network_socket {
    //     if may_continue {
    //         // The first player picks the map
    //         let is_waiting = socket.player_idx() != 0;

    //         *params.menu_page = MenuPage::MapSelect { is_waiting };
    //     }
    // }

    let bigger_text_style = &meta
        .theme
        .font_styles
        .bigger
        .with_color(meta.theme.panel.font_color);
    let heading_text_style = &meta
        .theme
        .font_styles
        .heading
        .with_color(meta.theme.panel.font_color);
    let normal_button_style = &meta.theme.buttons.normal;

    ui.vertical_centered(|ui| {
        ui.add_space(heading_text_style.size / 4.0);

        // Title
        if is_online {
            ui.label(heading_text_style.rich(localization.get("online-game")));
        } else {
            ui.label(heading_text_style.rich(localization.get("local-game")));
        }

        ui.label(bigger_text_style.rich(localization.get("player-select-title")));
        ui.add_space(normal_button_style.font.size);

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.add_space(normal_button_style.font.size * 2.0);
            ui.horizontal(|ui| {
                // Calculate button size and spacing
                let width = ui.available_width();
                let button_width = width / 3.0;
                let button_min_size = vec2(button_width, 0.0);
                let button_spacing = (width - 2.0 * button_width) / 3.0;

                ui.add_space(button_spacing);

                // Back button
                let back_button =
                    BorderedButton::themed(normal_button_style, localization.get("back"))
                        .min_size(button_min_size)
                        .show(ui)
                        .focus_by_default(ui);

                if back_button.clicked()
                    || (ready_players == 0
                        && unconfirmed_players == 0
                        && controls.values().any(|x| x.menu_back_just_pressed))
                {
                    ui.ctx().set_state(MenuPage::Home);
                    ui.ctx().set_state(EguiInputSettings::default());
                    ui.ctx().set_state(PlayerSelectState::default());

                    // #[cfg(not(target_arch = "wasm32"))]
                    // if let Some(socket) = params.network_socket {
                    //     socket.close();
                    // }
                }

                ui.add_space(button_spacing);

                // Continue button
                let continue_button = ui
                    .scope(|ui| {
                        ui.set_enabled(may_continue);

                        BorderedButton::themed(normal_button_style, localization.get("continue"))
                            .min_size(button_min_size)
                            .show(ui)
                    })
                    .inner;

                if !controls.values().any(|x| x.menu_back_just_pressed)
                    && (continue_button.clicked()
                        || (controls.values().any(|x| x.menu_start_just_pressed) && may_continue))
                {
                    ui.ctx()
                        .set_state(MenuPage::MapSelect { is_waiting: false });
                    ui.ctx().set_state(EguiInputSettings::default());
                    ui.ctx().set_state(PlayerSelectState::default());
                }
            });

            ui.add_space(normal_button_style.font.size);

            ui.vertical_centered(|ui| {
                ui.set_width(ui.available_width() - normal_button_style.font.size * 2.0);

                ui.columns(MAX_PLAYERS, |columns| {
                    for (i, ui) in columns.iter_mut().enumerate() {
                        world.run_initialized_system(player_select_panel, (ui, i, &mut state))
                    }
                });
            });
        });
    });

    ui.ctx().set_state(state);
}

// #[cfg(not(target_arch = "wasm32"))]
// fn handle_match_setup_messages(params: &mut PlayerSelectMenu) {
//     if let Some(socket) = &params.network_socket {
//         let datas: Vec<(usize, Vec<u8>)> = socket.recv_reliable();

//         for (player, data) in datas {
//             match postcard::from_bytes::<PlayerSelectMessage>(&data) {
//                 Ok(message) => match message {
//                     PlayerSelectMessage::SelectPlayer(player_handle) => {
//                         params.player_select_state.slots[player].selected_player = player_handle;
//                     }
//                     PlayerSelectMessage::ConfirmSelection(confirmed) => {
//                         params.player_select_state.slots[player].confirmed = confirmed;
//                     }
//                     PlayerSelectMessage::SelectHat(hat) => {
//                         params.player_select_state.slots[player].selected_hat = hat;
//                     }
//                 },
//                 Err(e) => warn!("Ignoring network message that was not understood: {e}"),
//             }
//         }
//     }
// }

fn player_select_panel(
    mut params: In<(&mut egui::Ui, usize, &mut PlayerSelectState)>,
    meta: Root<GameMeta>,
    controls: Res<GlobalPlayerControls>,
    asset_server: Res<AssetServer>,
    localization: Localization<GameMeta>,
    mapping: Res<PlayerControlMapping>,
    world: &World,
) {
    let (ui, slot_id, state) = &mut *params;

    let is_network = false;
    // #[cfg(target_arch = "wasm32")]
    // let is_network = false;
    // #[cfg(not(target_arch = "wasm32"))]
    // let is_network = params.network_socket.is_some();

    // let player_map = params
    //     .players
    //     .iter()
    //     .find(|(player_idx, _, _)| player_idx.0 == player_id)
    //     .unwrap()
    //     .2;

    // #[cfg(not(target_arch = "wasm32"))]
    // let dummy_actions = default();
    // let (player_actions, player_action_map) = if is_network {
    //     // #[cfg(not(target_arch = "wasm32"))]
    //     // if let Some(socket) = &params.network_socket {
    //     //     let actions = if player_id == socket.player_idx() {
    //     //         params
    //     //             .players
    //     //             .iter()
    //     //             .find(|(player_idx, _, _)| player_idx.0 == 0)
    //     //             .unwrap()
    //     //             .1
    //     //     } else {
    //     //         &dummy_actions
    //     //     };
    //     //     let map = None;
    //     //     (actions, map)
    //     // } else {
    //     //     unreachable!();
    //     // }

    //     // #[cfg(target_arch = "wasm32")]
    //     // unreachable!()
    // } else {
    //     let actions = players
    //         .iter()
    //         .find(|(player_idx, _, _)| player_idx.0 == player_id)
    //         .unwrap()
    //         .1;
    //     let map = Some(get_player_actions(player_id, player_map));
    //     (actions, map)
    // };

    // #[cfg(not(target_arch = "wasm32"))]
    // if let Some(socket) = &params.network_socket {
    //     // Don't show panels for non-connected players.
    //     if player_id + 1 > socket.player_count() {
    //         return;
    //     } else {
    //         slot.active = true;
    //     }
    // }

    // Get the ID of the first un-occupied slot
    let next_open_slot = state
        .slots
        .iter()
        .enumerate()
        .find_map(|(i, slot)| (!slot.active).then_some(i));

    // Check if a new player is trying to join
    let new_player_join = controls.iter().find_map(|(source, control)| {
        (
            // If this control input is pressing the join button
            control.menu_confirm_just_pressed &&
            // And this is the next open slot
            next_open_slot == Some(*slot_id) &&
            // And this control source is not bound to a player slot already
            !state
            .slots
            .iter()
            .any(|s| s.control_source == Some(*source))
        )
        // Return this source
        .then_some(*source)
    });

    // Input sources that may be used to join a new player
    let available_input_sources = {
        let mut sources = SmallVec::<[_; 3]>::from_slice(&[
            ControlSource::Keyboard1,
            ControlSource::Keyboard2,
            ControlSource::Gamepad(0),
        ]);

        for slot in &state.slots {
            if matches!(
                slot.control_source,
                Some(ControlSource::Keyboard1 | ControlSource::Keyboard2)
            ) {
                sources.retain(|&mut x| x != slot.control_source.unwrap());
            }
        }
        sources
    };

    let slot = &mut state.slots[*slot_id];
    let player_handle = &mut slot.selected_player;

    // If the handle is empty
    if *player_handle == default() {
        // Select the first player
        *player_handle = meta.core.players[0];
    }

    // Handle player joining
    if let Some(control_source) = new_player_join {
        slot.active = true;
        slot.confirmed = false;
        slot.control_source = Some(control_source);
    }

    let player_control = slot
        .control_source
        .as_ref()
        .map(|s| *controls.get(s).unwrap())
        .unwrap_or_default();

    if player_control.menu_confirm_just_pressed && new_player_join.is_none() {
        slot.confirmed = true;

        // #[cfg(not(target_arch = "wasm32"))]
        // if let Some(socket) = &params.network_socket {
        //     socket.send_reliable(
        //         SocketTarget::All,
        //         &postcard::to_allocvec(&PlayerSelectMessage::ConfirmSelection(slot.confirmed))
        //             .unwrap(),
        //     );
        // }
    } else if player_control.menu_back_just_pressed {
        if !is_network {
            if slot.confirmed {
                slot.confirmed = false;
            } else if slot.active {
                slot.active = false;
                slot.control_source = None;
            }
        } else {
            slot.confirmed = false;
        }

        // #[cfg(not(target_arch = "wasm32"))]
        // if let Some(socket) = &params.network_socket {
        //     socket.send_reliable(
        //         SocketTarget::All,
        //         &postcard::to_allocvec(&PlayerSelectMessage::ConfirmSelection(slot.confirmed))
        //             .unwrap(),
        //     );
        // }
    } else if player_control.just_moved {
        let direction = player_control.move_direction;

        // Select a hat if the player has been confirmed
        if slot.confirmed {
            let current_hat_handle_idx = slot.selected_hat.as_ref().map(|player_hat| {
                meta.core
                    .player_hats
                    .iter()
                    .enumerate()
                    .find(|(_, handle)| *handle == player_hat)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });

            let next_idx = if direction.x > 0.0 {
                current_hat_handle_idx
                    .map(|x| {
                        if x < meta.core.player_hats.len() - 1 {
                            Some(x + 1)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(Some(0))
            } else {
                current_hat_handle_idx
                    .map(|x| if x == 0 { None } else { Some(x - 1) })
                    .unwrap_or(Some(meta.core.player_hats.len() - 1))
            };
            slot.selected_hat = next_idx.map(|idx| *meta.core.player_hats.get(idx).unwrap());

            // #[cfg(not(target_arch = "wasm32"))]
            // if let Some(socket) = &params.network_socket {
            //     socket.send_reliable(
            //         SocketTarget::All,
            //         &postcard::to_allocvec(&PlayerSelectMessage::SelectHat(player_hat.clone()))
            //             .unwrap(),
            //     );
            // }

            // Select player skin if the player has not be confirmed
        } else {
            let current_player_handle_idx = meta
                .core
                .players
                .iter()
                .enumerate()
                .find(|(_, handle)| *handle == player_handle)
                .map(|(i, _)| i)
                .unwrap_or(0);

            if direction.x > 0.0 {
                *player_handle = meta
                    .core
                    .players
                    .get(current_player_handle_idx + 1)
                    .cloned()
                    .unwrap_or_else(|| meta.core.players[0]);
            } else if direction.x <= 0.0 {
                if current_player_handle_idx > 0 {
                    *player_handle = meta
                        .core
                        .players
                        .get(current_player_handle_idx - 1)
                        .cloned()
                        .unwrap();
                } else {
                    *player_handle = *meta.core.players.iter().last().unwrap();
                }
            }

            // #[cfg(not(target_arch = "wasm32"))]
            // if let Some(socket) = &params.network_socket {
            //     socket.send_reliable(
            //         SocketTarget::All,
            //         &postcard::to_allocvec(&PlayerSelectMessage::SelectPlayer(
            //             player_handle.clone(),
            //         ))
            //         .unwrap(),
            //     );
            // }
        }
    }

    let panel = &meta.theme.panel;
    BorderedFrame::new(&panel.border)
        .padding(panel.padding)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());

            let normal_font = &meta.theme.font_styles.normal.with_color(panel.font_color);
            let smaller_font = &meta.theme.font_styles.smaller.with_color(panel.font_color);
            let heading_font = &meta.theme.font_styles.heading.with_color(panel.font_color);

            // Marker for current player in online matches
            // #[cfg(not(target_arch = "wasm32"))]
            // if let Some(socket) = &params.network_socket {
            //     if socket.player_idx() == player_id {
            //         ui.vertical_centered(|ui| {
            //             ui.themed_label(normal_font, &params.localization.get("you-marker"));
            //         });
            //     } else {
            //         ui.add_space(normal_font.size);
            //     }
            // } else {
            //     ui.add_space(normal_font.size);
            // }

            ui.add_space(normal_font.size);

            if slot.active {
                let confirm_binding = slot.control_source.map(|s| {
                    match s {
                        ControlSource::Keyboard1 => &mapping.keyboard1.menu_confirm,
                        ControlSource::Keyboard2 => &mapping.keyboard2.menu_confirm,
                        ControlSource::Gamepad(_) => &mapping.gamepad.menu_confirm,
                    }
                    .to_string()
                });
                let back_binding = slot.control_source.map(|s| {
                    match s {
                        ControlSource::Keyboard1 => &mapping.keyboard1.menu_back,
                        ControlSource::Keyboard2 => &mapping.keyboard2.menu_back,
                        ControlSource::Gamepad(_) => &mapping.gamepad.menu_back,
                    }
                    .to_string()
                });
                ui.vertical_centered(|ui| {
                    let player_meta = asset_server.get(slot.selected_player);
                    let hat_meta = slot
                        .selected_hat
                        .as_ref()
                        .map(|handle| asset_server.get(*handle));

                    ui.label(normal_font.rich(localization.get("pick-a-fish")));

                    if !slot.confirmed {
                        ui.label(normal_font.rich(localization.get_with(
                            "press-button-to-lock-in",
                            &fluent_args! {
                                "button" => confirm_binding.as_ref().unwrap().as_str()
                            },
                        )));

                        ui.label(normal_font.rich(localization.get_with(
                            "press-button-to-remove",
                            &fluent_args! {
                                "button" => back_binding.as_ref().unwrap().as_str()
                            },
                        )));
                    } else {
                        ui.label(normal_font.rich(localization.get("waiting")));
                    }

                    ui.vertical_centered(|ui| {
                        ui.set_height(heading_font.size * 1.5);

                        if slot.confirmed && !slot.is_ai() {
                            ui.label(
                                heading_font
                                    .with_color(meta.theme.colors.positive)
                                    .rich(localization.get("player-select-ready")),
                            );
                            ui.add_space(normal_font.size / 2.0);
                            ui.label(normal_font.rich(localization.get_with(
                                "player-select-unready",
                                &fluent_args! {
                                    "button" => back_binding.as_ref().unwrap().as_str()
                                },
                            )));
                        }
                        if slot.is_ai() {
                            ui.label(
                                heading_font
                                    .with_color(meta.theme.colors.positive)
                                    .rich(localization.get("ai-player")),
                            );
                            ui.add_space(normal_font.size / 2.0);
                            if BorderedButton::themed(
                                &meta.theme.buttons.normal,
                                localization.get("remove-ai-player"),
                            )
                            .show(ui)
                            .clicked()
                            {
                                slot.confirmed = false;
                                slot.active = false;
                                slot.control_source = None;
                            }
                        }
                    });

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        let name_with_arrows = format!("<  {}  >", player_meta.name);
                        ui.label(normal_font.rich(if slot.confirmed {
                            player_meta.name.to_string()
                        } else {
                            name_with_arrows
                        }));
                        let hat_label = if let Some(hat_meta) = &hat_meta {
                            format!("< {} >", hat_meta.name)
                        } else {
                            format!("< {} >", localization.get("no-hat"))
                        };
                        ui.label(smaller_font.rich(if slot.confirmed { &hat_label } else { "" }));

                        world.run_initialized_system(
                            player_image,
                            (ui, &player_meta, hat_meta.as_deref()),
                        );
                    });
                });

            // If this slot is empty
            } else {
                let bindings = available_input_sources
                    .into_iter()
                    .map(|x| match x {
                        ControlSource::Keyboard1 => mapping.keyboard1.menu_confirm.to_string(),
                        ControlSource::Keyboard2 => mapping.keyboard2.menu_confirm.to_string(),
                        ControlSource::Gamepad(_) => mapping.gamepad.menu_confirm.to_string(),
                    })
                    .collect::<SmallVec<[_; 3]>>();

                ui.vertical_centered(|ui| {
                    ui.label(normal_font.rich(localization.get_with(
                        "press-button-to-join",
                        &fluent_args! {
                            "button" => bindings.join(" / ")
                        },
                    )));

                    if !is_network {
                        ui.add_space(meta.theme.font_styles.bigger.size);
                        if BorderedButton::themed(
                            &meta.theme.buttons.normal,
                            localization.get("add-ai-player"),
                        )
                        .show(ui)
                        .clicked()
                        {
                            slot.confirmed = true;
                            slot.active = true;
                            let rand_idx =
                                THREAD_RNG.with(|rng| rng.usize(0..meta.core.players.len()));
                            slot.selected_player = meta.core.players[rand_idx];
                        }
                    }
                });
            }
        });
}

fn player_image(
    mut params: In<(&mut egui::Ui, &PlayerMeta, Option<&HatMeta>)>,
    egui_textures: Res<EguiTextures>,
    asset_server: Res<AssetServer>,
) {
    let (ui, player_meta, hat_meta) = &mut *params;
    let time = ui.ctx().input(|i| i.time as f32);
    let width = ui.available_width();
    let available_height = ui.available_width();

    let body_rect;
    let body_scale;
    let body_offset;
    let y_offset;
    // Render the body sprite
    {
        let atlas_handle = &player_meta.layers.body.atlas;
        let atlas = asset_server.get(*atlas_handle);
        let anim_clip = player_meta
            .layers
            .body
            .animations
            .frames
            .get(&ustr("idle"))
            .unwrap();
        let fps = anim_clip.fps;
        let frame_in_time_idx = (time * fps).round() as usize;
        let frame_in_clip_idx = frame_in_time_idx % anim_clip.frames.len();
        let frame_in_sheet_idx = anim_clip.frames[frame_in_clip_idx];
        let sprite_pos = atlas.tile_pos(frame_in_sheet_idx);
        body_offset =
            player_meta.layers.body.animations.offsets[&ustr("idle")][frame_in_clip_idx].body;

        let sprite_aspect = atlas.tile_size.y / atlas.tile_size.x;
        let height = sprite_aspect * width;
        y_offset = -(available_height - height) / 2.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

        let uv_min = sprite_pos / atlas.size();
        let uv_max = (sprite_pos + atlas.tile_size) / atlas.size();
        let uv = egui::Rect {
            min: egui::pos2(uv_min.x, uv_min.y),
            max: egui::pos2(uv_max.x, uv_max.y),
        };

        let mut mesh = egui::Mesh {
            texture_id: *egui_textures.0.get(&atlas.image).unwrap(),
            ..default()
        };

        mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
        mesh.translate(egui::vec2(0.0, y_offset));
        ui.painter().add(mesh);

        body_rect = rect;
        body_scale = width / atlas.tile_size.x;
    }

    // Render the fin & face animation
    for layer in [&player_meta.layers.fin, &player_meta.layers.face] {
        let atlas_handle = &layer.atlas;
        let atlas = asset_server.get(*atlas_handle);
        let anim_clip = layer.animations.get(&ustr("idle")).unwrap();
        let fps = anim_clip.fps;
        let frame_in_time_idx = (time * fps).round() as usize;
        let frame_in_clip_idx = frame_in_time_idx % anim_clip.frames.len();
        let frame_in_sheet_idx = anim_clip.frames[frame_in_clip_idx];
        let sprite_pos = atlas.tile_pos(frame_in_sheet_idx);

        let uv_min = sprite_pos / atlas.size();
        let uv_max = (sprite_pos + atlas.tile_size) / atlas.size();
        let uv = egui::Rect {
            min: egui::pos2(uv_min.x, uv_min.y),
            max: egui::pos2(uv_max.x, uv_max.y),
        };

        let mut mesh = egui::Mesh {
            texture_id: *egui_textures.0.get(&atlas.image).unwrap(),
            ..default()
        };

        let sprite_size = atlas.tile_size * body_scale;
        let offset = (layer.offset + body_offset) * body_scale;
        let rect = egui::Rect::from_center_size(
            body_rect.center() + egui::vec2(offset.x, -offset.y + y_offset),
            egui::vec2(sprite_size.x, sprite_size.y),
        );

        mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
        ui.painter().add(mesh);
    }

    // Render the player hat
    if let Some(hat_meta) = hat_meta {
        let atlas_handle = &hat_meta.atlas;
        let atlas = asset_server.get(*atlas_handle);
        let sprite_pos = Vec2::ZERO;

        let uv_min = sprite_pos / atlas.size();
        let uv_max = (sprite_pos + atlas.tile_size) / atlas.size();
        let uv = egui::Rect {
            min: egui::pos2(uv_min.x, uv_min.y),
            max: egui::pos2(uv_max.x, uv_max.y),
        };

        let mut mesh = egui::Mesh {
            texture_id: *egui_textures.0.get(&atlas.image).unwrap(),
            ..default()
        };

        let sprite_size = atlas.tile_size * body_scale;
        let offset = (hat_meta.offset + body_offset) * body_scale;
        let rect = egui::Rect::from_center_size(
            body_rect.center() + egui::vec2(offset.x, -offset.y + y_offset),
            egui::vec2(sprite_size.x, sprite_size.y),
        );

        mesh.add_rect_with_uv(rect, uv, egui::Color32::WHITE);
        ui.painter().add(mesh);
    }
}
