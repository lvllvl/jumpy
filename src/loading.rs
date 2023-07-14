//! Initial game loading implementation.

use bevy::ecs::system::SystemParam;
use bevy_egui::{egui, EguiContexts};
use bevy_fluent::Locale;
use leafwing_input_manager::{
    axislike::{AxisType, SingleAxis},
    prelude::InputMap,
    InputManagerBundle,
};

use crate::{
    editor::{MapTilesetEguiTextureinfo, MapTilesetEguiTextures},
    prelude::*,
};

/// Loading plugin.
pub struct JumpyLoadingPlugin;

impl Plugin for JumpyLoadingPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup).add_system(
            load_game
                .run_if(in_state(EngineState::LoadingGameData))
                .run_if(core_assets_loaded),
        );

        // Configure hot reload
        if ENGINE_CONFIG.hot_reload {
            app.add_system(
                hot_reload_game
                    .in_base_set(CoreSet::Last)
                    .run_if(in_state(EngineState::InGame)),
            );
        }
    }
}

/// Run criteria that waits until the necessary core assets have loaded.
///
/// Not all the assets need to be loaded, just the ones we need immediately for the menu load, the
/// rest will be loading while the menu is running, in the background.
fn core_assets_loaded(
    game_handle: Res<GameMetaHandle>,
    game_assets: Res<Assets<GameMeta>>,
    core_assets: Res<Assets<CoreMeta>>,
    player_assets: Res<Assets<PlayerMeta>>,
    atlas_assets: Res<Assets<TextureAtlas>>,
    hat_assets: Res<Assets<HatMeta>>,
) -> bool {
    // The game asset
    let Some(game) = game_assets.get(&game_handle) else {
        return false;
    };
    // The core asset
    let Some(core) = core_assets.get(&game.core.inner) else {
        return false;
    };

    // Egui assets
    //
    // The assets below must be loaded because they are used during the game load process for
    // getting egui textures for images that need to be displayed in egui. If we add new egui
    // textures to the load process we need to make sure they are loaded here.

    // The player assets
    for player in &core.players {
        let Some(player) = player_assets.get(&player.get_bevy_handle()) else {
            return false;
        };

        // The player atlases ( needed for the player selection screen )
        if atlas_assets
            .get(&player.layers.body.atlas.get_bevy_handle_untyped().typed())
            .is_none()
        {
            return false;
        }
        if atlas_assets
            .get(&player.layers.fin.atlas.get_bevy_handle_untyped().typed())
            .is_none()
        {
            return false;
        }
        if atlas_assets
            .get(&player.layers.face.atlas.get_bevy_handle_untyped().typed())
            .is_none()
        {
            return false;
        }
    }
    // The map tilesets
    for tileset_handle in &core.map_tilesets {
        if atlas_assets
            .get(&tileset_handle.get_bevy_handle_untyped().typed())
            .is_none()
        {
            return false;
        }
    }

    // Hats
    for hats_handle in &core.player_hats {
        if hat_assets
            .get(&hats_handle.get_bevy_handle_untyped().typed())
            .is_none()
        {
            return false;
        }
    }

    true
}

/// Component added to the entities used to collect player input.
///
/// The inner [`usize`] is the player index.
///
/// The `leafwing-input-manager` tracks input for specific entities, instead of globally collecting
/// input. This usually makes things easier, but for our usje-case, we need to be able to collect
/// user input even for players that haven't, spawned yet.
///
/// To facilitate this, for every user we spawn [`jumpy_core::MAX_PLAYERS`] entities with a
/// [`PlayerInputCollector`] and the `InputManagerBundle` that is needed for
/// `leafwing-input-manager`.
#[derive(Component)]
pub struct PlayerInputCollector(pub usize);

/// Systemrunonce to spawn the menu input collector.
fn setup(mut commands: Commands) {
    commands.spawn((
        Name::new("Menu Input Collector"),
        InputManagerBundle {
            input_map: menu_input_map(),
            ..default()
        },
    ));
}

/// Resource containing mappings of asset paths to their egui textures.
///
/// This is populated during the game load.
#[derive(Resource)]
pub struct AtlasEguiTextures(pub HashMap<bones::AssetPath, egui::TextureId>);

/// System param used to load and hot reload the game.
#[derive(SystemParam)]
pub struct GameLoader<'w, 's> {
    skip_next_asset_update_event: Local<'s, bool>,
    commands: Commands<'w, 's>,
    game_handle: Res<'w, GameMetaHandle>,
    game_assets: ResMut<'w, Assets<GameMeta>>,
    core_assets: ResMut<'w, Assets<CoreMeta>>,
    events: EventReader<'w, 's, AssetEvent<GameMeta>>,
    // active_scripts: ResMut<'w, ActiveScripts>,
    storage: ResMut<'w, Storage>,
    player_assets: ResMut<'w, Assets<PlayerMeta>>,
    hat_assets: ResMut<'w, Assets<HatMeta>>,
    texture_atlas_assets: Res<'w, Assets<TextureAtlas>>,
    egui_ctx: EguiContexts<'w, 's>,
}

impl<'w, 's> GameLoader<'w, 's> {
    /// This function is called once when the game starts up and, when hot reload is enabled, on
    /// update, to check for asset changed events and to update the [`GameMeta`] resource.
    ///
    /// The `is_hot_reload` argument is used to indicate whether the function should check for asset
    /// updates and reload, or whether it should run the one-time initialization of the game.
    fn load(mut self, is_hot_reload: bool) {
        // Check to make sure we shouldn't skip this execution
        // ( i.e. if this is a hot reload run without any changed assets )
        if self.should_skip_run(is_hot_reload) {
            return;
        }

        let Self {
            mut skip_next_asset_update_event,
            mut commands,
            game_handle,
            mut game_assets,
            mut core_assets,
            mut egui_ctx,
            mut storage,
            ..
        } = self;

        let game = game_assets.get_mut(&game_handle).unwrap();
        let core = core_assets.get_mut(&game.core).unwrap();

        // Hot reload preparation
        if is_hot_reload {
            // Since we are modifying the game asset, which will trigger another asset changed
            // event, we need to skip the next update event.
            *skip_next_asset_update_event = true;

            // // Clear the active scripts
            // active_scripts.clear();

            // One-time initialization
        } else {
            spawn_menu_camera(&mut commands, core);

            // Initialize empty fonts for all game fonts.
            //
            // This makes sure Egui will not panic if we try to use a font that is still loading.
            let mut egui_fonts = egui::FontDefinitions::default();
            for font_name in game.ui_theme.font_families.keys() {
                let font_family = egui::FontFamily::Name(font_name.clone().into());
                egui_fonts.families.insert(font_family, vec![]);
            }
            egui_ctx.ctx_mut().set_fonts(egui_fonts.clone());
            commands.insert_resource(EguiFontDefinitions(egui_fonts));

            // Spawn player input collectors.
            let settings = storage.get(Settings::STORAGE_KEY);
            let settings = settings.as_ref().unwrap_or(&game.default_settings);
            for player in 0..MAX_PLAYERS {
                commands.spawn((
                    Name::new(format!("Player Input Collector {player}")),
                    PlayerInputCollector(player),
                    InputManagerBundle {
                        input_map: settings.player_controls.get_input_map(player),
                        ..default()
                    },
                ));
            }

            // Transition to the main menu when we are done
            commands.insert_resource(NextState(Some(EngineState::MainMenu)));
        }

        // Set the locale resource
        let translations = &game.translations;
        commands.insert_resource(
            Locale::new(translations.detected_locale.clone())
                .with_default(translations.default_locale.clone()),
        );

        let mut visuals = egui::Visuals::dark();
        visuals.widgets = game.ui_theme.widgets.get_egui_widget_style();
        visuals.window_fill = game.ui_theme.debug_window_fill.into_egui();
        visuals.panel_fill = visuals.window_fill;
        let [red, green, blue, alpha] = visuals.window_fill.to_srgba_unmultiplied();
        let [red, green, blue, alpha] = [
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            alpha as f32 / 255.0,
        ];
        commands.insert_resource(ClearColor(Color::Rgba {
            red,
            green,
            blue,
            alpha,
        }));
        egui_ctx.ctx_mut().set_visuals(visuals);

        // Helper to load border images
        let mut load_border_image = |border: &mut BorderImageMeta| {
            border.egui_texture = egui_ctx.add_image(border.image.inner.clone_weak());
        };

        // Add Border images to egui context
        load_border_image(&mut game.ui_theme.hud.portrait_frame);
        load_border_image(&mut game.ui_theme.panel.border);
        load_border_image(&mut game.ui_theme.hud.lifebar.background_image);
        load_border_image(&mut game.ui_theme.hud.lifebar.progress_image);
        for button in game.ui_theme.button_styles.as_list() {
            load_border_image(&mut button.borders.default);
            if let Some(border) = &mut button.borders.clicked {
                load_border_image(border);
            }
            if let Some(border) = &mut button.borders.focused {
                load_border_image(border);
            }
        }

        // Add editor icons to egui context
        for icon in game.ui_theme.editor.icons.as_mut_list() {
            icon.egui_texture_id = egui_ctx.add_image(icon.image.inner.clone_weak());
        }

        // Insert the game resource
        commands.insert_resource(game.clone());
        commands.insert_resource(CoreMetaArc(Arc::new(core.clone())));

        // Load player atlas egui handles
        let mut player_atlas_egui_textures = HashMap::default();
        for player_handle in &core.players {
            let player_meta = self
                .player_assets
                .get(&player_handle.get_bevy_handle())
                .unwrap();

            for (path, handle) in [
                (
                    player_meta.layers.body.atlas.path.clone(),
                    player_meta.layers.body.atlas.get_bevy_handle_untyped(),
                ),
                (
                    player_meta.layers.fin.atlas.path.clone(),
                    player_meta.layers.fin.atlas.get_bevy_handle_untyped(),
                ),
                (
                    player_meta.layers.face.atlas.path.clone(),
                    player_meta.layers.face.atlas.get_bevy_handle_untyped(),
                ),
            ] {
                let texture_atlas = self.texture_atlas_assets.get(&handle.typed()).unwrap();

                let egui_texture = egui_ctx.add_image(texture_atlas.texture.clone_weak());
                player_atlas_egui_textures.insert(path, egui_texture);
            }
        }
        // Load player hat atlase egui handles
        for hat_handle in &core.player_hats {
            let hat_meta = self.hat_assets.get(&hat_handle.get_bevy_handle()).unwrap();
            let path = hat_meta.atlas.path.clone();
            let texture_atlas = self
                .texture_atlas_assets
                .get(&hat_meta.atlas.get_bevy_handle_untyped().typed())
                .unwrap();
            let egui_texture = egui_ctx.add_image(texture_atlas.texture.clone_weak());

            player_atlas_egui_textures.insert(path, egui_texture);
        }
        commands.insert_resource(AtlasEguiTextures(player_atlas_egui_textures));

        // load map tileset egui handles
        let mut map_tileset_egui_textures = HashMap::default();
        for tileset_handle in &core.map_tilesets {
            let tileset_meta = self
                .texture_atlas_assets
                .get(&tileset_handle.get_bevy_handle_untyped().typed())
                .unwrap();
            let size = tileset_meta.size;
            let tile_size = tileset_meta.textures[0].size(); // All tiles have to be the same size
            let texture = egui_ctx.add_image(tileset_meta.texture.clone_weak());

            map_tileset_egui_textures.insert(
                tileset_handle.path.clone(),
                MapTilesetEguiTextureinfo {
                    texture,
                    size,
                    tile_size,
                },
            );
        }
        commands.insert_resource(MapTilesetEguiTextures(map_tileset_egui_textures));

        // NOTE: If you add more egui texture loading to this phase you need to make sure they are
        // loaded in the `core_assets_loaded` function.
    }

    // Run checks to see if we should skip running the system
    fn should_skip_run(&mut self, is_hot_reload: bool) -> bool {
        // If this is a hot reload run, check for modified asset events
        if is_hot_reload {
            let mut has_update = false;
            for (event, event_id) in self.events.iter_with_id() {
                if let AssetEvent::Modified { .. } = event {
                    // We may need to skip an asset update event
                    if *self.skip_next_asset_update_event {
                        *self.skip_next_asset_update_event = false;
                    } else {
                        debug!(%event_id, "Game updated");
                        has_update = true;
                    }
                }
            }

            // If there was no update, skip execution
            if !has_update {
                return true;
            }
        }

        false
    }
}

/// Get the input map for the menu controls.
fn menu_input_map() -> InputMap<MenuAction> {
    InputMap::default()
        .set_gamepad(Gamepad::new(0))
        // Up
        .insert(KeyCode::Up, MenuAction::Up)
        .insert(GamepadButtonType::DPadUp, MenuAction::Up)
        .insert(
            SingleAxis {
                axis_type: AxisType::Gamepad(GamepadAxisType::LeftStickY),
                positive_low: 0.5,
                negative_low: -1.0,
                value: None,
            },
            MenuAction::Up,
        )
        // Left
        .insert(KeyCode::Left, MenuAction::Left)
        .insert(GamepadButtonType::DPadLeft, MenuAction::Left)
        .insert(
            SingleAxis {
                axis_type: AxisType::Gamepad(GamepadAxisType::LeftStickX),
                positive_low: 1.0,
                negative_low: -0.5,
                value: None,
            },
            MenuAction::Left,
        )
        // Down
        .insert(KeyCode::Down, MenuAction::Down)
        .insert(GamepadButtonType::DPadDown, MenuAction::Down)
        .insert(
            SingleAxis {
                axis_type: AxisType::Gamepad(GamepadAxisType::LeftStickY),
                positive_low: 1.0,
                negative_low: -0.5,
                value: None,
            },
            MenuAction::Down,
        )
        // Right
        .insert(KeyCode::Right, MenuAction::Right)
        .insert(GamepadButtonType::DPadRight, MenuAction::Right)
        .insert(
            SingleAxis {
                axis_type: AxisType::Gamepad(GamepadAxisType::LeftStickX),
                positive_low: 0.5,
                negative_low: -1.0,
                value: None,
            },
            MenuAction::Right,
        )
        // Start
        .insert(GamepadButtonType::Start, MenuAction::Start)
        // Confirm
        .insert(KeyCode::Return, MenuAction::Confirm)
        .insert(GamepadButtonType::South, MenuAction::Confirm)
        .insert(GamepadButtonType::Start, MenuAction::Confirm)
        // Back
        .insert(KeyCode::Escape, MenuAction::Back)
        .insert(GamepadButtonType::East, MenuAction::Back)
        // Toggle Fullscreen
        .insert(KeyCode::F11, MenuAction::ToggleFullscreen)
        .insert(GamepadButtonType::Mode, MenuAction::ToggleFullscreen)
        // Pause
        .insert(KeyCode::Escape, MenuAction::Pause)
        .insert(GamepadButtonType::Start, MenuAction::Pause)
        .build()
}

/// System to run the initial game load.
fn load_game(loader: GameLoader) {
    loader.load(false);
}

/// System to check for asset changes and hot reload the game.
fn hot_reload_game(loader: GameLoader) {
    loader.load(true);
}
