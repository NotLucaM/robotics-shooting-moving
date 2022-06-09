extern crate nalgebra as na;

use bevy::math::vec3;
use bevy::prelude::*;
use bevy_prototype_debug_lines::*;
use na::{vector, Vector3};
use std::f64::consts::PI;

const G: f64 = -9.8;
const UP: f64 = 10.0;
const ROBOT_COLOR: Color = Color::rgb(1.0, 0.2, 0.2);
const TARGET_COLOR: Color = Color::rgb(0.0, 1.0, 0.2);
const BALL_COLOR: Color = Color::rgb(0.0, 0.2, 1.0);
const ARENA_WIDTH: f32 = 40.0;
const ARENA_HEIGHT: f32 = 20.0;
const AIR_RES: f64 = 0.5 * 0.47 * 1.28;
const ROBOT_VEL: f64 = 2.0;

fn main() {
    App::new()
        .insert_resource(Target {
            target: vector![ARENA_WIDTH as f64 / 2.0, ARENA_HEIGHT as f64 / 2.0, 0.0],
        })
        .add_startup_system(setup_camera)
        .add_startup_system(setup_stage)
        .add_system(keyboard_input)
        .add_system(move_objects)
        .add_plugins(DefaultPlugins)
        .add_plugin(DebugLinesPlugin::default())
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation)
                .with_system(size_scaling),
        )
        .run();
}

fn distance(p1: &Vector3<f64>, p2: &Vector3<f64>) -> f64 {
    ((p1[0] - p2[0]).powi(2) + (p1[1] - p2[1]).powi(2)).sqrt()
}

fn law_cosines(d1: f64, d2: f64, a: f64) -> f64 {
    (d1.powi(2) + d2.powi(2) - 2.0 * d1 * d2 * a.cos()).sqrt()
}

fn law_sines(d1: f64, a1: f64, d2: f64) -> f64 {
    (d2 * a1.sin() / d1).asin()
}

fn translate(vec: &Vector3<f64>, window: &Windows) -> Vec3 {
    translate_vec(vec3(vec.x as f32, vec.y as f32, vec.z as f32), window)
}

fn translate_vec(vec: Vec3, window: &Windows) -> Vec3 {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = window.get_primary().unwrap();
    Vec3::new(
        convert(vec.x, window.width() as f32, ARENA_WIDTH),
        convert(vec.y, window.height() as f32, ARENA_HEIGHT),
        vec.z,
    )
}

fn setup_camera(mut commands: Commands) {
    let mut camera = OrthographicCameraBundle::new_2d();
    camera.transform = Transform::from_translation(vec3(0.0, 0.0, 5.0));
    commands.spawn_bundle(camera);
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    target: Res<Target>,
    window: Res<Windows>,
    mut commands: Commands,
    mut player: Query<(&mut Robot, &mut Body)>,
    mut lines: ResMut<DebugLines>,
) {
    for (mut p, mut t) in player.iter_mut() {
        t.vel = vector![0.0, 0.0, 0.0];
        if keys.pressed(KeyCode::A) {
            p.bearing += 0.1;
        } else if keys.pressed(KeyCode::D) {
            p.bearing -= 0.1;
        }
        if keys.pressed(KeyCode::S) {
            p.vel = -ROBOT_VEL;
        } else if keys.pressed(KeyCode::W) {
            p.vel = ROBOT_VEL;
        } else {
            p.vel = 0.0;
        }

        t.vel.x = p.vel * p.bearing.cos();
        t.vel.y = p.vel * p.bearing.sin();

        p.turret_angle =
            ((target.target.y - t.pos.y) / (target.target.x - t.pos.x)).atan() - p.bearing;

        lines.line_colored(
            translate_vec(vec3(t.pos.x as f32, t.pos.y as f32, 1.0), &window),
            translate_vec(
                vec3(
                    (t.pos.x + 2.0 * (p.turret_angle + p.bearing).cos()) as f32,
                    (t.pos.y + 2.0 * (p.turret_angle + p.bearing).sin()) as f32,
                    1.0,
                ),
                &window,
            ),
            0.0,
            Color::GREEN,
        );

        if target.target.x - t.pos.x < 0.0 {
            p.turret_angle += PI;
        }

        let ball_time = (- UP - (UP * UP + 4.0 * 0.5 * 9.8).sqrt()) / (- 9.8);

        // shooting while moving
        let future_distance = law_cosines(
            p.vel * ball_time,
            distance(&target.target, &t.pos),
            p.turret_angle,
        );

        let future_pos = vec3(
            (t.pos.x + t.vel.x * ball_time) as f32,
            (t.pos.y + t.vel.y * ball_time) as f32,
            0.0,
        );

        lines.line_colored(
            translate(&target.target, &window),
            translate(&t.pos, &window),
            0.0,
            Color::PINK,
        );
        lines.line_colored(
            translate(&t.pos, &window),
            translate_vec(future_pos, &window),
            0.0,
            Color::BLUE,
        );
        lines.line_colored(
            translate(&target.target, &window),
            translate_vec(future_pos, &window),
            0.0,
            Color::YELLOW,
        );
        lines.line_colored(
            translate(&target.target, &window),
            translate(&target.target, &window) + translate_vec(future_pos, &window)
                - translate(&t.pos, &window),
            0.0,
            Color::MIDNIGHT_BLUE,
        );
        lines.line_colored(
            translate_vec(future_pos, &window),
            translate(&target.target, &window) + translate_vec(future_pos, &window)
                - translate(&t.pos, &window),
            0.0,
            Color::TURQUOISE,
        );

        let vel = p.get_ball_shot(&t, &target.target);

        p.turret_angle += law_sines(future_distance, p.turret_angle, p.vel * ball_time);

        let ball_x = (p.bearing + p.turret_angle).cos() * vel.0;
        let ball_y = (p.bearing + p.turret_angle).sin() * vel.0;

        let shot = t.vel + vector![ball_x, ball_y, vel.1];

        lines.line_colored(
            translate_vec(vec3(t.pos.x as f32, t.pos.y as f32, 1.0), &window),
            translate_vec(
                vec3(
                    (t.pos.x + 2.0 * (p.turret_angle + p.bearing).cos()) as f32,
                    (t.pos.y + 2.0 * (p.turret_angle + p.bearing).sin()) as f32,
                    1.0,
                ),
                &window,
            ),
            0.0,
            Color::BLACK,
        );

        if keys.just_pressed(KeyCode::Space) {
            commands
                .spawn_bundle(SpriteBundle {
                    sprite: Sprite {
                        color: BALL_COLOR,
                        ..default()
                    },
                    ..default()
                })
                .insert(Ball {})
                .insert(Body {
                    size: 0.1,
                    pos: vector![t.pos.x, t.pos.y, 0.0],
                    vel: shot,
                    acc: vector![0.0, 0.0, -9.81],
                })
                .insert(Size::square(1.0));
        }
    }
}

fn setup_stage(mut commands: Commands, target: Res<Target>) {
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: ROBOT_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Robot {
            bearing: 0.0,
            turret_angle: 0.0,
            vel: 0.0,
        })
        .insert(Size::square(2.0))
        .insert(Body {
            size: 0.0,
            pos: vector![0.0, 0.0, 0.0],
            vel: vector![0.0, 0.0, 0.0],
            acc: vector![0.0, 0.0, 0.0],
        });
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: TARGET_COLOR,
                ..default()
            },
            ..default()
        })
        .insert(Body {
            size: 0.0,
            pos: target.target,
            vel: Default::default(),
            acc: Default::default(),
        })
        .insert(Size::square(2.0));
}

fn position_translation(windows: Res<Windows>, mut q: Query<(&Body, &mut Transform)>) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = windows.get_primary().unwrap();
    for (pos, mut transform) in q.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(
                pos.pos.y as f32,
                window.height() as f32,
                ARENA_HEIGHT as f32,
            ),
            0.0,
        );
    }
}

fn size_scaling(windows: Res<Windows>, mut q: Query<(&Size, &mut Transform)>) {
    let window = windows.get_primary().unwrap();
    for (sprite_size, mut transform) in q.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width() as f32,
            sprite_size.height / ARENA_HEIGHT as f32 * window.height() as f32,
            1.0,
        );
    }
}

fn move_objects(mut commands: Commands, mut query: Query<(Entity, &mut Body)>, time: Res<Time>) {
    for (ent, mut transform) in query.iter_mut() {
        let acc = transform.acc * time.delta_seconds_f64();
        transform.vel += acc;
        let speed = (transform.vel[0].powi(2) + transform.vel[1].powi(2) + transform.vel[2].powi(2)).sqrt() * time.delta_seconds_f64();
        if transform.vel[0] != 0.0 {
            let angle = (transform.vel[1] / transform.vel[0]).atan();
            transform.vel[0] -= AIR_RES * transform.size * speed * angle.cos();
        } else {
            transform.vel[1] -= AIR_RES * transform.size * speed;
        }
        transform.vel[2] -= AIR_RES * transform.size * speed;
        let vel = transform.vel * time.delta_seconds_f64();
        transform.pos += vel;
        if transform.pos.z < 0.0 {
            commands.entity(ent).despawn();
        }
    }
}

#[derive(Component)]
struct Target {
    target: Vector3<f64>,
}

#[derive(Component)]
struct Robot {
    bearing: f64,
    turret_angle: f64,
    vel: f64,
}

impl Robot {
    fn tree_map(dist: f64) -> (f64, f64) {
        (-G * dist / (2.0 * UP), UP)
    }

    fn get_ball_shot(&self, loc: &Body, target: &Vector3<f64>) -> (f64, f64) {
        let dist = distance(&loc.pos, target);

        let ball_time = (- UP - (UP * UP + 4.0 * 0.5 * 9.8).sqrt()) / (- 9.8);

        let dist = law_cosines(self.vel * ball_time, dist, self.turret_angle);

        Robot::tree_map(dist)
    }
}

#[derive(Component)]
struct Ball {}

#[derive(Component)]
struct Body {
    size: f64, // Surface area / mass
    pos: Vector3<f64>,
    vel: Vector3<f64>,
    acc: Vector3<f64>,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}
