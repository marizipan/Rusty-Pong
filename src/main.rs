use bevy::prelude::*;
use bevy::sprite::*;
use bevy::ui::*;
use bevy::text::*;
use bevy::input::ButtonInput;
use bevy::input::keyboard::Key;



// ...your constants and components...

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;

const PADDLE_HEIGHT: f32 = 100.0;
const PADDLE_WIDTH: f32 = 20.0;
const PADDLE_MARGIN: f32 = 30.0;

const BALL_SIZE: f32 = 46.0;
const BALL_START_SPEED: f32 = 200.0;
const BALL_SPEED_MAX: f32 = 800.0;

const PADDLE_SPEED: f32 = 12.0;

#[derive(States, Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Splash,
    Playing,
}

#[derive(Component)]
struct SplashScreen;

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Velocity(Vec2);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.13, 0.1, 0.2)))
        .add_plugins((
            DefaultPlugins,
            SpritePlugin,
            UiPlugin,
            TextPlugin,
        ))
        .insert_state(GameState::Splash)
        .add_systems(OnEnter(GameState::Splash), setup_splash)
        .add_systems(Update, start_button.run_if(in_state(GameState::Splash)))
        .add_systems(OnEnter(GameState::Playing), setup_game)
        .add_systems(
            Update,
            (
                paddle_movement_system,
                ball_movement,
                ball_collision_system
            ).run_if(in_state(GameState::Playing)),
        )
        .run();
}

fn setup_splash(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(SpriteBundle {
            texture: asset_server.load("splash.jpg"),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(SplashScreen);

    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            size: Size::new(Val::Px(200.0), Val::Px(80.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..Default::default()
                        },
                        background_color: Color::srgb(0.25, 0.25, 0.85).into(),
                        ..Default::default()
                    },
                    StartButton,
                ))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Start",
                        TextStyle {
                            font: asset_server.load("FiraSans-Bold.ttf"),
                            font_size: 40.0,
                            color: Color::WHITE,
                        },
                    ));
                });
        });
}

fn start_button(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<StartButton>),
    >,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    splash_query: Query<Entity, With<SplashScreen>>,
    button_parent_query: Query<Entity, With<NodeBundle>>,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                for entity in &splash_query {
                    commands.entity(entity).despawn();
                }
                for entity in &button_parent_query {
                    commands.entity(entity).despawn();
                }
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                *color = Color::srgb(0.35, 0.35, 0.95).into();
            }
            Interaction::None => {
                *color = Color::srgb(0.25, 0.25, 0.85).into();
            }
        }
    }
}

fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Paddle
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_xyz(
                0.0,
                -WINDOW_HEIGHT / 2.0 + PADDLE_MARGIN + PADDLE_HEIGHT / 2.0,
                0.0,
            ),
            sprite: Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Paddle);

    // Ferris Ball in center
    commands
        .spawn(SpriteBundle {
            texture: asset_server.load("ferris.png"),
            transform: Transform::from_xyz(0.0, 0.0, 1.0)
                .with_scale(Vec3::splat(BALL_SIZE / 400.0)),
            ..Default::default()
        })
        .insert(Ball)
        .insert(Velocity(Vec2::new(BALL_START_SPEED, BALL_START_SPEED)));
}

fn paddle_movement_system(
    input: Res<ButtonInput<Key>>,
    mut query: Query<&mut Transform, With<Paddle>>,
) {
    for mut transform in query.iter_mut() {
        let mut direction = 0.0;
        if input.pressed(Key::Character("a".into())) || input.pressed(Key::ArrowLeft) {
            direction -= 1.0;
        }
        if input.pressed(Key::Character("d".into())) || input.pressed(Key::ArrowRight) {
            direction += 1.0;
        }
        transform.translation.x += direction * PADDLE_SPEED;
        transform.translation.x = transform
            .translation
            .x
            .clamp(
                -WINDOW_WIDTH / 2.0 + PADDLE_WIDTH / 2.0,
                WINDOW_WIDTH / 2.0 - PADDLE_WIDTH / 2.0,
            );
    }
}

fn ball_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity), With<Ball>>,
) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * time.delta().as_secs_f32();
        transform.translation.y += velocity.0.y * time.delta().as_secs_f32();
    }
}

fn ball_collision_system(
    mut ball_query: Query<(&mut Velocity, &mut Transform), With<Ball>>,
    paddle_query: Query<&Transform, With<Paddle>>,
) {
    let (mut velocity, mut transform) = match ball_query.single_mut() {
        Ok(res) => res,
        Err(_) => return,
    };

    if transform.translation.x + BALL_SIZE / 2.0 > WINDOW_WIDTH / 2.0
        || transform.translation.x - BALL_SIZE / 2.0 < -WINDOW_WIDTH / 2.0
    {
        velocity.0.x = -velocity.0.x;
        velocity.0 *= 0.9;
    }

    if transform.translation.y + BALL_SIZE / 2.0 > WINDOW_HEIGHT / 2.0 {
        velocity.0.y = -velocity.0.y;
        velocity.0 *= 0.9;
    }

    // Paddle Collisions
    for paddle_transform in paddle_query.iter() {
        if collide(&transform, paddle_transform) {
            velocity.0.x = -velocity.0.x;
            velocity.0 *= 1.1;
        }

        let paddle_pos = paddle_transform.translation;
        if velocity.0.y < 0.0
            && transform.translation.y - BALL_SIZE / 2.0 <= paddle_pos.y + PADDLE_HEIGHT / 2.0
            && transform.translation.x + BALL_SIZE / 2.0 > paddle_pos.x - PADDLE_WIDTH / 2.0
            && transform.translation.x - BALL_SIZE / 2.0 < paddle_pos.x + PADDLE_WIDTH / 2.0
        {
            velocity.0.y = velocity.0.y.abs();
            velocity.0 *= 1.1;
        }
    }

    let speed = velocity.0.length().clamp(BALL_START_SPEED, BALL_SPEED_MAX);
    velocity.0 = velocity.0.normalize() * speed;
}

fn collide(a: &Transform, b: &Transform) -> bool {
    let a_half = Vec2::splat(BALL_SIZE / 2.0);
    let b_half = Vec2::new(PADDLE_WIDTH / 2.0, PADDLE_HEIGHT / 2.0);

    let a_center = Vec2::new(a.translation.x, a.translation.y);
    let b_center = Vec2::new(b.translation.x, b.translation.y);

    let delta = a_center - b_center;
    let overlap_x = a_half.x + b_half.x - delta.x.abs();
    let overlap_y = a_half.y + b_half.y - delta.y.abs();

    overlap_x > 0.0 && overlap_y > 0.0
}
