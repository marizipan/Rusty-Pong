use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::input::keyboard::Key;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;

const PADDLE_HEIGHT: f32 = 20.0;
const PADDLE_WIDTH: f32 = 100.0;
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

#[derive(Component)]
struct Block;

#[derive(Component)]
struct Score;

#[derive(Resource)]
struct GameScore(u32);

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.13, 0.1, 0.2)))
        .insert_resource(GameScore(0))
        .add_plugins(DefaultPlugins)
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
    // UI Camera for splash screen
    commands.spawn((Camera2d, IsDefaultUiCamera));

    // Splash image as background
    commands.spawn((
        Sprite {
            image: asset_server.load("splash.png"),
            custom_size: Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        SplashScreen,
    ));

    // Start button - a large, visible colored rectangle
    commands.spawn((
        Sprite {
            color: Color::srgb(0.25, 0.25, 0.85),
            custom_size: Some(Vec2::new(300.0, 100.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -100.0, 1.0),
        Button,
        StartButton,
    ));

    // Start button text
    commands.spawn((
        Text2d("Press Spacebar to Start".to_string()), // Changed text
        Transform::from_xyz(0.0, -100.0, 2.0),
        StartButton,
    ));
}

fn start_button(
    mut interaction_query: Query<
        (&Interaction, &mut Sprite),
        (Changed<Interaction>, With<StartButton>),
    >,
    input: Res<ButtonInput<Key>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    splash_query: Query<Entity, With<SplashScreen>>,
    button_query: Query<Entity, With<StartButton>>,
) {
    // Check for mouse interaction
    for (interaction, mut sprite) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                println!("Button pressed! Starting game...");
                for entity in &splash_query {
                    commands.entity(entity).despawn();
                }
                for entity in &button_query {
                    commands.entity(entity).despawn();
                }
                next_state.set(GameState::Playing);
            }
            Interaction::Hovered => {
                sprite.color = Color::srgb(0.35, 0.35, 0.95);
            }
            Interaction::None => {
                sprite.color = Color::srgb(0.25, 0.25, 0.85);
            }
        }
    }

    // Check for keyboard input (spacebar)
    if input.just_pressed(Key::Space) {
        println!("Spacebar pressed! Starting game...");
        for entity in &splash_query {
            commands.entity(entity).despawn();
        }
        for entity in &button_query {
            commands.entity(entity).despawn();
        }
        next_state.set(GameState::Playing);
    }
}

fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Game Camera - removed to fix camera ambiguity

    // Paddle
    commands
        .spawn((
            Sprite {
                color: Color::WHITE,
                custom_size: Some(Vec2::new(PADDLE_WIDTH, PADDLE_HEIGHT)),
                ..Default::default()
            },
            Transform::from_xyz(
                0.0,
                -WINDOW_HEIGHT / 2.0 + PADDLE_MARGIN + PADDLE_HEIGHT / 2.0 + 100.0, // Moved up by 100 pixels
                0.0,
            ),
            Paddle,
        ));

    // Ferris Ball in center
    commands
        .spawn((
            Sprite {
                image: asset_server.load("ferris.png"),
                custom_size: Some(Vec2::splat(BALL_SIZE)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 1.0),
            Ball,
            Velocity(Vec2::new(BALL_START_SPEED, BALL_START_SPEED)),
        ));

    // Create 3 layers of blocks at the top
    let block_width = 80.0;
    let block_height = 20.0;
    let blocks_per_row = (WINDOW_WIDTH / block_width) as i32;
    let start_x = -(blocks_per_row as f32 * block_width) / 2.0 + block_width / 2.0;
    
    for layer in 0..3 {
        let y_pos = WINDOW_HEIGHT / 2.0 - 50.0 - (layer as f32 * (block_height + 10.0));
        for i in 0..blocks_per_row {
            let x_pos = start_x + (i as f32 * block_width);
            commands.spawn((
                Sprite {
                    color: Color::srgb(0.8, 0.2, 0.2),
                    custom_size: Some(Vec2::new(block_width - 5.0, block_height)),
                    ..default()
                },
                Transform::from_xyz(x_pos, y_pos, 0.0),
                Block,
            ));
        }
    }

    // Score display
    commands.spawn((
        Text2d("Score: 0".to_string()),
        Transform::from_xyz(-WINDOW_WIDTH / 2.0 + 100.0, WINDOW_HEIGHT / 2.0 - 50.0, 2.0),
        Score,
    ));

    // Bottom wall to keep ball in play
    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(WINDOW_WIDTH, 20.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -WINDOW_HEIGHT / 2.0 + 10.0, 0.0),
    ));

    // Top wall to keep ball in play
    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::new(WINDOW_WIDTH, 20.0)),
            ..default()
        },
        Transform::from_xyz(0.0, WINDOW_HEIGHT / 2.0 - 10.0, 0.0),
    ));
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
    paddle_query: Query<&Transform, (With<Paddle>, Without<Ball>)>,
    block_query: Query<(Entity, &Transform), (With<Block>, Without<Ball>)>,
    mut commands: Commands,
    mut score: ResMut<GameScore>,
    mut score_text: Query<&mut Text2d, With<Score>>,
) {
    let (mut velocity, transform) = match ball_query.single_mut() {
        Ok(res) => res,
        Err(_) => return,
    };

    // Wall bounce (left/right = no speed change)
    if transform.translation.x + BALL_SIZE / 2.0 > WINDOW_WIDTH / 2.0
        || transform.translation.x - BALL_SIZE / 2.0 < -WINDOW_WIDTH / 2.0
    {
        velocity.0.x = -velocity.0.x;
        // No speed change for side walls
    }

    // Bottom wall bounce (far, slow down)
    if transform.translation.y - BALL_SIZE / 2.0 < -WINDOW_HEIGHT / 2.0 {
        velocity.0.y = velocity.0.y.abs();
        velocity.0 *= 0.9;
    }

    // Top wall bounce (far, slow down)
    if transform.translation.y + BALL_SIZE / 2.0 > WINDOW_HEIGHT / 2.0 {
        velocity.0.y = -velocity.0.y.abs();
        velocity.0 *= 0.9;
    }

    // Paddle Collisions (close, speed up)
    for paddle_transform in paddle_query.iter() {
        let paddle_pos = paddle_transform.translation;
        
        // Check if ball is hitting the paddle from above (moving down)
        if velocity.0.y < 0.0
            && transform.translation.y - BALL_SIZE / 2.0 <= paddle_pos.y + PADDLE_HEIGHT / 2.0
            && transform.translation.y - BALL_SIZE / 2.0 >= paddle_pos.y - PADDLE_HEIGHT / 2.0
            && transform.translation.x + BALL_SIZE / 2.0 > paddle_pos.x - PADDLE_WIDTH / 2.0
            && transform.translation.x - BALL_SIZE / 2.0 < paddle_pos.x + PADDLE_WIDTH / 2.0
        {
            velocity.0.y = velocity.0.y.abs();
            
            // Check if ball hit the bottom half of the paddle (closer to player)
            let ball_relative_y = transform.translation.y - paddle_pos.y;
            if ball_relative_y < 0.0 {
                // Bottom half of paddle - speed up twice as fast
                velocity.0 *= 1.3;
            } else {
                // Top half of paddle - normal speed up
                velocity.0 *= 1.15;
            }
        }
        
        // Check if ball is hitting the paddle from below (moving up)
        if velocity.0.y > 0.0
            && transform.translation.y + BALL_SIZE / 2.0 >= paddle_pos.y - PADDLE_HEIGHT / 2.0
            && transform.translation.y + BALL_SIZE / 2.0 <= paddle_pos.y + PADDLE_HEIGHT / 2.0
            && transform.translation.x + BALL_SIZE / 2.0 > paddle_pos.x - PADDLE_WIDTH / 2.0
            && transform.translation.x - BALL_SIZE / 2.0 < paddle_pos.x + PADDLE_WIDTH / 2.0
        {
            velocity.0.y = -velocity.0.y.abs();
            
            // Check if ball hit the bottom half of the paddle (closer to player)
            let ball_relative_y = transform.translation.y - paddle_pos.y;
            if ball_relative_y < 0.0 {
                // Bottom half of paddle - speed up twice as fast
                velocity.0 *= 1.3;
            } else {
                // Top half of paddle - normal speed up
                velocity.0 *= 1.15;
            }
        }
    }

    // Block Collisions
    for (block_entity, block_transform) in block_query.iter() {
        let block_pos = block_transform.translation;
        let block_width = 75.0; // block_width - 5.0
        let block_height = 20.0;
        
        if transform.translation.x + BALL_SIZE / 2.0 > block_pos.x - block_width / 2.0
            && transform.translation.x - BALL_SIZE / 2.0 < block_pos.x + block_width / 2.0
            && transform.translation.y + BALL_SIZE / 2.0 > block_pos.y - block_height / 2.0
            && transform.translation.y - BALL_SIZE / 2.0 < block_pos.y + block_height / 2.0
        {
            // Remove the block
            commands.entity(block_entity).despawn();
            
            // Update score
            score.0 += 1;
            
            // Update score display
            for mut text in score_text.iter_mut() {
                *text = Text2d(format!("Score: {}", score.0));
            }
            
            // Bounce the ball (reverse Y velocity)
            velocity.0.y = -velocity.0.y;
            velocity.0 *= 1.1; // Speed up when hitting blocks (more aggressive)
        }
    }

    // Clamp speed
    let speed = velocity.0.length().clamp(BALL_START_SPEED, BALL_SPEED_MAX);
    velocity.0 = velocity.0.normalize() * speed;
}
