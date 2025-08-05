use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::input::keyboard::Key;
use bevy::ecs::query::Or;

const WINDOW_WIDTH: f32 = 1280.0;
const WINDOW_HEIGHT: f32 = 720.0;

const PADDLE_HEIGHT: f32 = 20.0;
const PADDLE_WIDTH: f32 = 100.0;
const PADDLE_MARGIN: f32 = 30.0;

const BALL_SIZE: f32 = 46.0;
const BALL_COLLISION_MARGIN: f32 = 10.0; // Invisible area around ball for better hit detection
const BALL_START_SPEED: f32 = 200.0;
const BALL_SPEED_MAX: f32 = 1000.0;

const PADDLE_SPEED: f32 = 12.0;

#[derive(States, Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
enum GameState {
    #[default]
    Splash,
    Playing,
    GameWon,
}

#[derive(Component)]
struct SplashScreen;

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct WinScreen;

#[derive(Component)]
struct RestartButton;

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

#[derive(Component)]
struct PaddleBounce {
    original_y: f32,
    bounce_timer: f32,
    is_bouncing: bool,
}

#[derive(Resource)]
struct GameScore(u32);

// Component to track ball's last block destruction time
#[derive(Component)]
struct BallBlockCooldown(f32);

fn main() {
    // Configure logging to suppress calloop warnings
    std::env::set_var("RUST_LOG", "error");
    
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
                ball_collision_system,
                check_win_condition,
                ball_bump_system,
                ball_bounds_check,
            ).run_if(in_state(GameState::Playing)),
        )
        .add_systems(OnEnter(GameState::GameWon), (clear_game_camera, setup_win_screen))
        .add_systems(Update, restart_button.run_if(in_state(GameState::GameWon)))
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
            PaddleBounce {
                original_y: -WINDOW_HEIGHT / 2.0 + PADDLE_MARGIN + PADDLE_HEIGHT / 2.0 + 100.0,
                bounce_timer: 0.0,
                is_bouncing: false,
            },
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
            BallBlockCooldown(0.0),
        ));

    // Create 4 layers of blocks at the top
    let block_width = 80.0;
    let block_height = 20.0;
    let blocks_per_row = (WINDOW_WIDTH / block_width) as i32;
    let start_x = -(blocks_per_row as f32 * block_width) / 2.0 + block_width / 2.0;
    
    for layer in 0..4 {
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
    mut ball_query: Query<(&mut Velocity, &mut Transform, &mut BallBlockCooldown), With<Ball>>,
    paddle_query: Query<&Transform, (With<Paddle>, Without<Ball>)>,
    block_query: Query<(Entity, &Transform), (With<Block>, Without<Ball>)>,
    mut commands: Commands,
    mut score: ResMut<GameScore>,
    mut score_text: Query<&mut Text2d, With<Score>>,
    time: Res<Time>,
) {
    let (mut velocity, mut transform, mut cooldown) = match ball_query.single_mut() {
        Ok(res) => res,
        Err(_) => return,
    };

    // Wall bounce (left/right = no speed change)
    let effective_ball_size = BALL_SIZE + BALL_COLLISION_MARGIN * 2.0;
    
    if transform.translation.x + effective_ball_size / 2.0 > WINDOW_WIDTH / 2.0 {
        velocity.0.x = -velocity.0.x.abs();
        // Clamp ball position to prevent it from going through the wall
        transform.translation.x = WINDOW_WIDTH / 2.0 - effective_ball_size / 2.0;
    } else if transform.translation.x - effective_ball_size / 2.0 < -WINDOW_WIDTH / 2.0 {
        velocity.0.x = velocity.0.x.abs();
        // Clamp ball position to prevent it from going through the wall
        transform.translation.x = -WINDOW_WIDTH / 2.0 + effective_ball_size / 2.0;
    }

    // Bottom wall bounce (far, slow down)
    if transform.translation.y - effective_ball_size / 2.0 < -WINDOW_HEIGHT / 2.0 {
        velocity.0.y = velocity.0.y.abs();
        velocity.0 *= 0.9;
    }

    // Top wall bounce (far, slow down)
    if transform.translation.y + effective_ball_size / 2.0 > WINDOW_HEIGHT / 2.0 {
        velocity.0.y = -velocity.0.y.abs();
        velocity.0 *= 0.9;
    }

    // Paddle Collisions with improved hit detection and directional reflection
    for paddle_transform in paddle_query.iter() {
        let paddle_pos = paddle_transform.translation;
        let effective_ball_size = BALL_SIZE + BALL_COLLISION_MARGIN * 2.0;
        
        // Check if ball is hitting the paddle from above (moving down)
        if velocity.0.y < 0.0
            && transform.translation.y - effective_ball_size / 2.0 <= paddle_pos.y + PADDLE_HEIGHT / 2.0
            && transform.translation.y - effective_ball_size / 2.0 >= paddle_pos.y - PADDLE_HEIGHT / 2.0
            && transform.translation.x + effective_ball_size / 2.0 > paddle_pos.x - PADDLE_WIDTH / 2.0
            && transform.translation.x - effective_ball_size / 2.0 < paddle_pos.x + PADDLE_WIDTH / 2.0
        {
            velocity.0.y = velocity.0.y.abs();
            
            // Calculate where on the paddle the ball hit (for directional reflection)
            let ball_relative_x = transform.translation.x - paddle_pos.x;
            let paddle_half_width = PADDLE_WIDTH / 2.0;
            
            // Determine horizontal direction based on where ball hit paddle
            if ball_relative_x > paddle_half_width * 0.1 {
                // Right side of paddle - send ball right
                velocity.0.x = BALL_START_SPEED * 0.8; // Give it positive horizontal velocity
            } else if ball_relative_x < -paddle_half_width * 0.1 {
                // Left side of paddle - send ball left
                velocity.0.x = -BALL_START_SPEED * 0.8; // Give it negative horizontal velocity
            } else {
                // Center area - send ball directly upwards (no horizontal movement)
                velocity.0.x = 0.0;
            }
            
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
            && transform.translation.y + effective_ball_size / 2.0 >= paddle_pos.y - PADDLE_HEIGHT / 2.0
            && transform.translation.y + effective_ball_size / 2.0 <= paddle_pos.y + PADDLE_HEIGHT / 2.0
            && transform.translation.x + effective_ball_size / 2.0 > paddle_pos.x - PADDLE_WIDTH / 2.0
            && transform.translation.x - effective_ball_size / 2.0 < paddle_pos.x + PADDLE_WIDTH / 2.0
        {
            velocity.0.y = -velocity.0.y.abs();
            
            // Calculate where on the paddle the ball hit (for directional reflection)
            let ball_relative_x = transform.translation.x - paddle_pos.x;
            let paddle_half_width = PADDLE_WIDTH / 2.0;
            
            // Determine horizontal direction based on where ball hit paddle
            if ball_relative_x > paddle_half_width * 0.1 {
                // Right side of paddle - send ball right
                velocity.0.x = BALL_START_SPEED * 0.8; // Give it positive horizontal velocity
            } else if ball_relative_x < -paddle_half_width * 0.1 {
                // Left side of paddle - send ball left
                velocity.0.x = -BALL_START_SPEED * 0.8; // Give it negative horizontal velocity
            } else {
                // Center area - send ball directly upwards (no horizontal movement)
                velocity.0.x = 0.0;
            }
            
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
        
        // Check for side collisions with the paddle (ball hitting left/right edges)
        let ball_left = transform.translation.x - effective_ball_size / 2.0;
        let ball_right = transform.translation.x + effective_ball_size / 2.0;
        let paddle_left = paddle_pos.x - PADDLE_WIDTH / 2.0;
        let paddle_right = paddle_pos.x + PADDLE_WIDTH / 2.0;
        let paddle_top = paddle_pos.y + PADDLE_HEIGHT / 2.0;
        let paddle_bottom = paddle_pos.y - PADDLE_HEIGHT / 2.0;
        
        // Check if ball is hitting the left side of the paddle
        if ball_right >= paddle_left && ball_left <= paddle_left
            && transform.translation.y + effective_ball_size / 2.0 > paddle_bottom
            && transform.translation.y - effective_ball_size / 2.0 < paddle_top
        {
            velocity.0.x = -velocity.0.x.abs();
        }
        
        // Check if ball is hitting the right side of the paddle
        if ball_left <= paddle_right && ball_right >= paddle_right
            && transform.translation.y + effective_ball_size / 2.0 > paddle_bottom
            && transform.translation.y - effective_ball_size / 2.0 < paddle_top
        {
            velocity.0.x = velocity.0.x.abs();
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
            // Check cooldown to prevent multiple block destructions
            if cooldown.0 <= 0.0 {
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
                
                // Set cooldown to prevent multiple destructions
                cooldown.0 = 0.1; // 0.1 second cooldown
            }
        }
    }
    
    // Update cooldown timer
    cooldown.0 -= time.delta_secs();
    if cooldown.0 < 0.0 {
        cooldown.0 = 0.0;
    }

    // Clamp speed
    let speed = velocity.0.length().clamp(BALL_START_SPEED, BALL_SPEED_MAX);
    velocity.0 = velocity.0.normalize() * speed;
}

fn check_win_condition(
    block_query: Query<&Block>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // If there are no blocks left, the player has won
    if block_query.is_empty() {
        next_state.set(GameState::GameWon);
    }
}

fn clear_game_camera(mut commands: Commands, camera_query: Query<Entity, With<Camera>>) {
    // Clear any existing cameras to prevent ambiguity
    for camera_entity in camera_query.iter() {
        commands.entity(camera_entity).despawn();
    }
}

fn ball_bump_system(
    input: Res<ButtonInput<Key>>,
    mut paddle_query: Query<(&mut Transform, &mut PaddleBounce), With<Paddle>>,
    mut ball_query: Query<(&mut Velocity, &Transform), (With<Ball>, Without<Paddle>)>,
    time: Res<Time>,
) {
    // Check if spacebar is pressed
    if input.just_pressed(Key::Space) {
        if let Ok((mut paddle_transform, mut paddle_bounce)) = paddle_query.single_mut() {
            if let Ok((mut ball_velocity, ball_transform)) = ball_query.single_mut() {
                // Check if ball is touching the paddle
                let paddle_pos = paddle_transform.translation;
                let ball_pos = ball_transform.translation;
                
                // Check collision between ball and paddle with improved hit detection
                let effective_ball_size = BALL_SIZE + BALL_COLLISION_MARGIN * 2.0;
                let collision = ball_pos.x + effective_ball_size / 2.0 > paddle_pos.x - PADDLE_WIDTH / 2.0
                    && ball_pos.x - effective_ball_size / 2.0 < paddle_pos.x + PADDLE_WIDTH / 2.0
                    && ball_pos.y + effective_ball_size / 2.0 > paddle_pos.y - PADDLE_HEIGHT / 2.0
                    && ball_pos.y - effective_ball_size / 2.0 < paddle_pos.y + PADDLE_HEIGHT / 2.0;
                
                // Always bounce the paddle when spacebar is pressed
                if !paddle_bounce.is_bouncing {
                    paddle_bounce.original_y = paddle_transform.translation.y;
                    paddle_bounce.is_bouncing = true;
                    paddle_bounce.bounce_timer = 0.2; // Bounce for 0.2 seconds
                    paddle_transform.translation.y += 15.0; // Move paddle up by 15 pixels (reduced to prevent missing the ball)
                }
                
                // If ball is touching paddle, affect ball speed
                if collision {
                    // Increase ball speed significantly
                    ball_velocity.0 *= 1.5;
                    
                    // Clamp the speed to prevent it from going too fast
                    let speed = ball_velocity.0.length().clamp(BALL_START_SPEED, BALL_SPEED_MAX);
                    ball_velocity.0 = ball_velocity.0.normalize() * speed;
                }
            }
        }
    }
    
    // Update paddle bounce timer and return to original position
    for (mut paddle_transform, mut paddle_bounce) in paddle_query.iter_mut() {
        if paddle_bounce.is_bouncing {
            paddle_bounce.bounce_timer -= time.delta_secs();
            
            if paddle_bounce.bounce_timer <= 0.0 {
                // Return to original position
                paddle_transform.translation.y = paddle_bounce.original_y;
                paddle_bounce.is_bouncing = false;
            }
        }
    }
}

fn ball_bounds_check(
    mut ball_query: Query<(&mut Transform, &mut Velocity), With<Ball>>,
) {
    if let Ok((mut transform, mut velocity)) = ball_query.single_mut() {
        // Check if ball is way out of bounds and reset it
        let max_allowed_distance = WINDOW_WIDTH / 2.0 + 100.0; // Screen half-width + buffer
        
        if transform.translation.x.abs() > max_allowed_distance 
            || transform.translation.y.abs() > max_allowed_distance {
            transform.translation = Vec3::new(0.0, 0.0, 1.0);
            velocity.0 = Vec2::new(BALL_START_SPEED, BALL_START_SPEED);
        }
        
        // Also check if ball velocity is too low (stuck)
        if velocity.0.length() < BALL_START_SPEED * 0.5 {
            velocity.0 = velocity.0.normalize() * BALL_START_SPEED;
        }
    }
}

fn setup_win_screen(mut commands: Commands, _asset_server: Res<AssetServer>) {
    // UI Camera for win screen
    commands.spawn((Camera2d, IsDefaultUiCamera));

    // Win screen background (dark overlay)
    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 0.0, 0.8),
            custom_size: Some(Vec2::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        WinScreen,
    ));

    // "You won!" text
    commands.spawn((
        Text2d("You won!".to_string()),
        Transform::from_xyz(0.0, 50.0, 2.0),
        WinScreen,
    ));

    // Restart button - a large, visible colored rectangle
    commands.spawn((
        Sprite {
            color: Color::srgb(0.25, 0.25, 0.85),
            custom_size: Some(Vec2::new(300.0, 100.0)),
            ..default()
        },
        Transform::from_xyz(0.0, -100.0, 1.0),
        Button,
        RestartButton,
    ));

    // Restart button text
    commands.spawn((
        Text2d("Press Spacebar to Restart".to_string()),
        Transform::from_xyz(0.0, -100.0, 2.0),
        RestartButton,
    ));
}

fn restart_button(
    mut interaction_query: Query<
        (&Interaction, &mut Sprite),
        (Changed<Interaction>, With<RestartButton>),
    >,
    input: Res<ButtonInput<Key>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
    win_screen_query: Query<Entity, With<WinScreen>>,
    button_query: Query<Entity, With<RestartButton>>,
    game_entities: Query<Entity, (Or<(With<Paddle>, With<Ball>, With<Block>, With<Score>)>, Without<WinScreen>, Without<RestartButton>)>,
    mut score: ResMut<GameScore>,
) {
    // Check for mouse interaction
    for (interaction, mut sprite) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // Clear win screen entities
                for entity in &win_screen_query {
                    commands.entity(entity).despawn();
                }
                for entity in &button_query {
                    commands.entity(entity).despawn();
                }
                // Clear all game entities
                for entity in &game_entities {
                    commands.entity(entity).despawn();
                }
                // Reset score
                score.0 = 0;
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
        // Clear win screen entities
        for entity in &win_screen_query {
            commands.entity(entity).despawn();
        }
        for entity in &button_query {
            commands.entity(entity).despawn();
        }
        // Clear all game entities
        for entity in &game_entities {
            commands.entity(entity).despawn();
        }
        // Reset score
        score.0 = 0;
        next_state.set(GameState::Playing);
    }
}
