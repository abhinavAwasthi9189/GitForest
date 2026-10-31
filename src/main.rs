mod modules;
use bevy::app::AppExit;
use bevy::ecs::message::MessageReader;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::PrimaryWindow;
use modules::Backend::{backend, Info, Repo};
use rand::RngExt;

const TILE_SIZE_L: u32 = 64;
const TREE_FRAMES_L: usize = 4;
const SPF: f32 = 0.25;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let asset_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/assets");
    let asset_path = asset_path.to_string_lossy().to_string();
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)))
        .insert_resource::<InputState>(InputState::default())
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            file_path: asset_path,
            ..default()
        }).set(WindowPlugin {
            primary_window: Some(Window {
                title: "Git Forest".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_input, process_login, render_screen, tree_hover, animate_trees, update_loading_dots, clear_toast, date_click, escape_quit))
        .run();
    Ok(())
}

#[derive(Resource)]
struct InputState {
    username: String,
    confirmed: Option<Vec<Info>>,
    date_index: usize,
    loading: bool,
    pending_username: Option<String>,
    toast: Option<(String, f32)>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            username: String::new(),
            confirmed: None,
            date_index: 0,
            loading: false,
            pending_username: None,
            toast: None,
        }
    }
}

#[derive(Resource)]
struct TreePlacements(Vec<(f32, f32, String)>);

#[derive(Resource)]
struct LoadingDots(Timer, u32);

#[derive(Resource)]
struct TreeAssets {
    layout_l: Handle<TextureAtlasLayout>,
    texture_l: Handle<Image>,
}

#[derive(Resource)]
struct GameFont(Handle<Font>);

#[derive(Resource)]
struct HoveredTree(Option<(String, i32, Entity)>);

#[derive(Resource)]
struct BgAssets {
    bg: Handle<Image>,
    title: Handle<Image>,
}

#[derive(Component)]
struct Ui;

#[derive(Component)]
struct Tree(String, i32);

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

#[derive(Component)]
struct Escape;

#[derive(Component)]
struct LoadingDotsIndicator;

#[derive(Component)]
struct LeftArrow;

#[derive(Component)]
struct RightArrow;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_layout: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2d);
    commands.insert_resource(LoadingDots(Timer::from_seconds(1.5, TimerMode::Repeating), 0));
    commands.insert_resource(TreePlacements(Vec::new()));
    commands.insert_resource(HoveredTree(None));

    let texture_l = asset_server.load("TreeL.png");
    let layout_l = atlas_layout.add(TextureAtlasLayout::from_grid(
        UVec2::splat(TILE_SIZE_L), TREE_FRAMES_L as u32, 1, None, None,
    ));

    let font: Handle<Font> = asset_server.load("PixelifySans-Regular.ttf");
    let bg = asset_server.load("BG.png");
    let title = asset_server.load("Title.png");

    commands.insert_resource(TreeAssets { layout_l, texture_l });
    commands.insert_resource(GameFont(font));
    commands.insert_resource(BgAssets { bg, title });
}

fn readit(usr: &str) -> Result<Vec<Info>, Box<dyn std::error::Error>> {
    let mut user = String::from(usr);
    user.push_str(".json");
    let data = std::fs::read_to_string(user)?;
    let json: Vec<Info> = serde_json::from_str(&data)?;
    Ok(json)
}

fn handle_input(
    mut input: ResMut<InputState>,
    mut reader: MessageReader<KeyboardInput>,
) {
    for ev in reader.read() {
        if ev.state == ButtonState::Pressed {
            match &ev.logical_key {
                Key::Character(c) if input.confirmed.is_none() && !input.loading => {
                    input.username.push_str(c.as_str());
                }
                Key::Backspace if input.confirmed.is_none() && !input.loading => {
                    input.username.pop();
                }
                Key::Enter if input.confirmed.is_none() && !input.loading => {
                    let name = input.username.trim().to_string();
                    if !name.is_empty() {
                        input.pending_username = Some(name);
                        input.loading = true;
                    }
                }
                Key::ArrowLeft | Key::ArrowRight if input.confirmed.is_some() => {
                    let dir = if matches!(ev.logical_key, Key::ArrowLeft) { -1isize } else { 1 };
                    if let Some(data) = &input.confirmed {
                        let new = input.date_index as isize + dir;
                        if new >= 0 && (new as usize) < data.len() {
                            input.date_index = new as usize;
                        }
                    }
                }
                Key::Escape => {}
                _ => {}
            }
        }
    }
}

fn process_login(
    mut input: ResMut<InputState>,
    time: Res<Time>,
) {
    if let Some(name) = input.pending_username.take() {
        let mut n = name.clone();
        match backend(&mut n) {
            Ok(()) => println!("Data fetched successfully"),
            Err(e) => println!("Error: {e}\nwill try offline copy"),
        }
        match readit(&name) {
            Ok(data) => {
                input.date_index = if data.is_empty() { 0 } else { data.len() - 1 };
                input.confirmed = Some(data);
                input.loading = false;
            }
            Err(e) => {
                input.loading = false;
                input.toast = Some((format!("{}", e), time.elapsed_secs()));
            }
        }
    }
}

fn render_screen(
    input: Res<InputState>,
    tree_assets: Res<TreeAssets>,
    font: Res<GameFont>,
    bg_assets: Res<BgAssets>,
    mut placements: ResMut<TreePlacements>,
    mut commands: Commands,
    existing: Query<Entity, With<Ui>>,
    trees: Query<Entity, With<Tree>>,
    window: Query<&Window, With<PrimaryWindow>>,
    time: Res<Time>,
) {
    if !input.is_changed() && !existing.is_empty() {
        return;
    }
    for e in &existing { commands.entity(e).despawn(); }
    for e in &trees { commands.entity(e).despawn(); }

    let (ww, wh) = window.single().map_or((1280.0, 720.0), |w| (w.width(), w.height()));

    if let Some(data) = &input.confirmed {
        draw_forest(&mut commands, &tree_assets, &font, &bg_assets, &mut placements, data, input.date_index, ww, wh);
    } else {
        draw_title(&mut commands, &font, &bg_assets, &input.username, input.loading, ww, wh);
    }

    if let Some((msg, start)) = &input.toast {
        if time.elapsed_secs() - start < 3.0 {
            commands.spawn((
                Text2d::new(msg.clone()),
                TextFont { font: font.0.clone(), font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.9, 0.3, 0.3)),
                Transform::from_xyz(-(ww / 2.0) + 10.0, -(wh / 2.0) + 10.0, 2.0),
                Anchor::BOTTOM_LEFT,
                Ui,
            ));
        }
    }
}

fn draw_title(commands: &mut Commands, font: &GameFont, bg: &BgAssets, username: &str, loading: bool, ww: f32, _wh: f32) {
    commands.spawn((
        Sprite {
            image: bg.bg.clone(),
            custom_size: Some(Vec2::new(ww, _wh)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -10.0),
        Ui,
    ));
    commands.spawn((
        Sprite {
            image: bg.title.clone(),
            ..default()
        },
        Transform::from_xyz(0.0, 80.0, 0.0),
        Ui,
    ));
    commands.spawn((
        Sprite { color: Color::srgb(0.15, 0.15, 0.2), custom_size: Some(Vec2::new(400.0, 50.0)), ..default() },
        Transform::from_xyz(0.0, 10.0, -1.0), Ui,
    ));
    commands.spawn((
        Sprite { color: Color::srgb(0.25, 0.25, 0.3), custom_size: Some(Vec2::new(400.0, 50.0)), ..default() },
        Transform::from_xyz(0.0, 10.0, 0.0), Ui,
    ));

    let display = if username.is_empty() { "enter username..." } else { username };
    commands.spawn((
        Text2d::new(display.to_string()),
        TextFont { font: font.0.clone(), font_size: 24.0, ..default() },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Transform::from_xyz(0.0, 10.0, 1.0), Anchor::CENTER, Ui,
    ));

    if loading {
        commands.spawn((
            Text2d::new("..loading..".to_string()),
            TextFont { font: font.0.clone(), font_size: 14.0, ..default() },
            TextColor(Color::srgb(0.6, 0.6, 0.6)),
            Transform::from_xyz(0.0, -30.0, 1.0), Anchor::CENTER, Ui, LoadingDotsIndicator,
        ));
    } else {
        commands.spawn((
            Text2d::new("Press Enter to continue"),
            TextFont { font: font.0.clone(), font_size: 14.0, ..default() },
            TextColor(Color::srgb(0.4, 0.4, 0.4)),
            Transform::from_xyz(0.0, -30.0, 0.0), Anchor::CENTER, Ui,
        ));
    }

    commands.spawn((
        Text2d::new("Hold ESC to quit"),
        TextFont { font: font.0.clone(), font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.3, 0.3, 0.3)),
        Transform::from_xyz(ww / 2.0 - 80.0, -(_wh / 2.0) + 30.0, 0.0), Anchor::CENTER, Ui, Escape,
        AnimationTimer(Timer::from_seconds(1.5, TimerMode::Repeating)),
    ));
}

fn tree_scale(commit: i32) -> f32 {
    0.5 + (commit as f32 + 1.0).ln() / 1001.0_f32.ln() * 2.5
}

fn draw_forest(
    commands: &mut Commands,
    assets: &TreeAssets,
    font: &GameFont,
    bg: &BgAssets,
    placements: &mut TreePlacements,
    data: &[Info],
    date_index: usize,
    ww: f32,
    wh: f32,
) {
    let month_data = &data[date_index];
    let month = &month_data.date;

    let nav_y = wh / 2.0 - 30.0;

    commands.spawn((
        Sprite {
            image: bg.bg.clone(),
            custom_size: Some(Vec2::new(ww, wh)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -10.0),
        Ui,
    ));

    // clickable <
    commands.spawn((
        Sprite { color: Color::srgba(0.0, 0.0, 0.0, 0.0), custom_size: Some(Vec2::new(40.0, 40.0)), ..default() },
        Transform::from_xyz(-80.0, nav_y, 1.0), Ui, LeftArrow,
    ));
    commands.spawn((
        Text2d::new("<"),
        TextFont { font: font.0.clone(), font_size: 36.0, ..default() },
        TextColor(Color::srgb(0.3, 0.3, 0.3)),
        Transform::from_xyz(-80.0, nav_y, 2.0), Anchor::CENTER, Ui,
    ));

    // date
    commands.spawn((
        Text2d::new(month.clone()),
        TextFont { font: font.0.clone(), font_size: 36.0, ..default() },
        TextColor(Color::srgb(0.3, 0.3, 0.3)),
        Transform::from_xyz(0.0, nav_y, 1.0), Anchor::CENTER, Ui,
    ));

    // clickable >
    commands.spawn((
        Sprite { color: Color::srgba(0.0, 0.0, 0.0, 0.0), custom_size: Some(Vec2::new(40.0, 40.0)), ..default() },
        Transform::from_xyz(80.0, nav_y, 1.0), Ui, RightArrow,
    ));
    commands.spawn((
        Text2d::new(">"),
        TextFont { font: font.0.clone(), font_size: 36.0, ..default() },
        TextColor(Color::srgb(0.3, 0.3, 0.3)),
        Transform::from_xyz(80.0, nav_y, 2.0), Anchor::CENTER, Ui,
    ));

    // sort repos by commit descending
    let mut sorted: Vec<&Repo> = month_data.repos.iter().filter(|r| r.commit > 0).collect();
    sorted.sort_by(|a, b| b.commit.cmp(&a.commit));

    let margin = 100.0;
    let min_x = -(ww / 2.0) + margin;
    let max_x = (ww / 2.0) - margin;
    let min_y = -(wh / 2.0) + margin;
    let max_y = (wh / 2.0) - margin - 60.0;

    // one-time placement
    let base_half = TILE_SIZE_L as f32 / 2.0;
    if placements.0.is_empty() {
        let mut placed: Vec<(f32, f32, f32, f32)> = Vec::new();
        for repo in &sorted {
            let scale = tree_scale(repo.commit);
            let half = base_half * scale;
            let mut rng = rand::rng();
            let mut pos = None;
            for _ in 0..20 {
                let x = rng.random_range(min_x + half..max_x - half);
                let y = rng.random_range(min_y + half..max_y - half);
                let overlapping = placed.iter().any(|(px, py, pw, _ph)| {
                    (x - px).abs() < (half + pw) && (y - py).abs() < (half + half)
                });
                if !overlapping {
                    pos = Some((x, y));
                    break;
                }
            }
            let (x, y) = pos.unwrap_or((0.0, 0.0));
            placed.push((x, y, half, half));
            placements.0.push((x, y, repo.name.clone()));
        }
    }

    // spawn trees with current date's commit counts
    let commit_map: std::collections::HashMap<&str, i32> = month_data.repos.iter().map(|r| (r.name.as_str(), r.commit)).collect();

    for (x, y, name) in &placements.0 {
        let commit = commit_map.get(name.as_str()).copied().unwrap_or(0);
        if commit == 0 {
            continue;
        }
        let scale = tree_scale(commit);
        commands.spawn((
            Sprite::from_atlas_image(
                assets.texture_l.clone(),
                TextureAtlas { layout: assets.layout_l.clone(), index: 0 },
            ),
            Transform {
                translation: Vec3::new(*x, *y, 0.0),
                scale: Vec3::splat(scale),
                ..default()
            },
            Tree(name.clone(), commit),
            AnimationTimer(Timer::from_seconds(SPF, TimerMode::Repeating)),
        ));
    }

    // ESC quit
    commands.spawn((
        Text2d::new("Hold ESC to quit"),
        TextFont { font: font.0.clone(), font_size: 12.0, ..default() },
        TextColor(Color::srgb(0.3, 0.3, 0.3)),
        Transform::from_xyz(ww / 2.0 - 80.0, -(wh / 2.0) + 30.0, 0.0), Anchor::CENTER, Ui, Escape,
        AnimationTimer(Timer::from_seconds(1.5, TimerMode::Repeating)),
    ));
}

fn date_click(
    buttons: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut input: ResMut<InputState>,
    left_q: Query<&Transform, With<LeftArrow>>,
    right_q: Query<&Transform, With<RightArrow>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let win = match window.single() {
        Ok(w) => w,
        Err(_) => return,
    };
    let cursor = match win.cursor_position() {
        Some(p) => p,
        None => return,
    };
    let ww = win.width();
    let wh = win.height();
    let cx = cursor.x - ww / 2.0;
    let cy = -(cursor.y - wh / 2.0);

    if let Ok(t) = left_q.single() {
        let dx = (cx - t.translation.x).abs();
        let dy = (cy - t.translation.y).abs();
        if dx < 20.0 && dy < 20.0 {
            if let Some(_data) = &input.confirmed {
                if input.date_index > 0 {
                    input.date_index -= 1;
                }
            }
        }
    }
    if let Ok(t) = right_q.single() {
        let dx = (cx - t.translation.x).abs();
        let dy = (cy - t.translation.y).abs();
        if dx < 20.0 && dy < 20.0 {
            if let Some(_data) = &input.confirmed {
                if input.date_index < _data.len().saturating_sub(1) {
                    input.date_index += 1;
                }
            }
        }
    }
}

#[derive(Component)]
struct Tooltip;

fn tree_hover(
    window: Query<&Window, With<PrimaryWindow>>,
    trees: Query<(&Transform, &Tree), With<Sprite>>,
    font: Res<GameFont>,
    mut hover: ResMut<HoveredTree>,
    mut commands: Commands,
    tooltip: Query<Entity, With<Tooltip>>,
) {
    let win = match window.single() {
        Ok(w) => w,
        Err(_) => return,
    };
    let cursor = match win.cursor_position() {
        Some(p) => p,
        None => {
            if hover.0.is_some() {
                for e in &tooltip { commands.entity(e).despawn(); }
                hover.0 = None;
            }
            return;
        }
    };
    let ww = win.width();
    let wh = win.height();
    let cx = cursor.x - ww / 2.0;
    let cy = -(cursor.y - wh / 2.0);

    let base_size = TILE_SIZE_L as f32 / 2.0;
    let mut found: Option<(String, i32, Vec3)> = None;

    for (transform, tree) in &trees {
        let scale = transform.scale.x;
        let hit_half = base_size * scale;
        let dx = (cx - transform.translation.x).abs();
        let dy = (cy - transform.translation.y).abs();
        if dx < hit_half && dy < hit_half {
            found = Some((tree.0.clone(), tree.1, transform.translation));
            break;
        }
    }

    let same = match (&found, &hover.0) {
        (Some((n1, c1, _)), Some((n2, c2, _))) => n1 == n2 && c1 == c2,
        (None, None) => true,
        _ => false,
    };

    if !same {
        for e in &tooltip { commands.entity(e).despawn(); }

        if let Some((name, commit, tree_pos)) = &found {
            let tooltip_text = format!("{} ({})", name, commit);
            let text_len = tooltip_text.len() as f32 * 7.0;
            let box_w = text_len + 12.0;
            let box_h = 22.0;

            let mut tx = tree_pos.x + 10.0;
            let mut ty = tree_pos.y + 10.0;

            if tx + box_w > ww / 2.0 {
                tx = tree_pos.x - box_w - 10.0;
            }
            if ty + box_h / 2.0 > wh / 2.0 {
                ty = tree_pos.y - box_h / 2.0 - 10.0;
            }

            commands.spawn((
                Sprite {
                    color: Color::srgba(0.0, 0.0, 0.0, 0.8),
                    custom_size: Some(Vec2::new(box_w, box_h)),
                    ..default()
                },
                Transform::from_xyz(tx + box_w / 2.0, ty + box_h / 2.0, 4.0),
                Tooltip,
            ));

            commands.spawn((
                Text2d::new(tooltip_text),
                TextFont { font: font.0.clone(), font_size: 13.0, ..default() },
                TextColor(Color::srgb(0.9, 0.9, 0.8)),
                Transform::from_xyz(tx + 6.0, ty + box_h / 2.0, 5.0),
                Anchor::CENTER_LEFT,
                Tooltip,
            ));
        }

        hover.0 = found.map(|(n, c, _)| (n, c, Entity::PLACEHOLDER));
    }
}

fn animate_trees(
    time: Res<Time>,
    mut query: Query<(&mut AnimationTimer, &mut Sprite), With<Tree>>,
) {
    let mut rng = rand::rng();
    for (mut timer, mut sprite) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            if rng.random_bool(0.8) {
                if let Some(atlas) = &mut sprite.texture_atlas {
                    if atlas.index == 3 { atlas.index = 0; } else { atlas.index += 1; }
                }
            }
        }
    }
}

fn update_loading_dots(
    time: Res<Time>,
    input: Res<InputState>,
    mut dots: ResMut<LoadingDots>,
    mut query: Query<&mut Text2d, With<LoadingDotsIndicator>>,
) {
    if !input.loading { return; }
    dots.0.tick(time.delta());
    if dots.0.just_finished() { dots.1 = (dots.1 + 1) % 5; }
    let label = match dots.1 {
        0 => "..loading..", 1 => "..loading. .", 2 => "..loading. ..",
        3 => "..loading. ...", _ => "..loading....",
    };
    for mut text in query.iter_mut() { text.0 = label.to_string(); }
}

fn clear_toast(time: Res<Time>, mut input: ResMut<InputState>) {
    if let Some((_, start)) = &input.toast {
        if time.elapsed_secs() - start >= 3.0 { input.toast = None; }
    }
}

fn escape_quit(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut exit: MessageWriter<AppExit>,
    mut timer: Query<&mut AnimationTimer, With<Escape>>,
) {
    if let Ok(mut gt) = timer.single_mut() {
        if keys.pressed(KeyCode::Escape) {
            gt.tick(time.delta());
            if gt.just_finished() { exit.write(AppExit::Success); }
        } else {
            let gtnewval = (gt.elapsed_secs() - time.delta_secs()).max(0.0);
            gt.set_elapsed(std::time::Duration::from_secs_f32(gtnewval));
        }
    }
}
