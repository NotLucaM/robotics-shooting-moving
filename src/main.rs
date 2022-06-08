extern crate nalgebra as na;

use std::f64::consts::PI;
use bevy::prelude::*;
use na::{vector, Vector3};

const G: f64 = -9.8;
const UP: f64 = 15.0;
const ROBOT_COLOR: Color = Color::rgb(1.0, 0.2, 0.2);
const TARGET_COLOR: Color = Color::rgb(0.0, 1.0, 0.2);
const BALL_COLOR: Color = Color::rgb(0.0, 0.2, 1.0);
const ARENA_WIDTH: f32 = 40.0;
const ARENA_HEIGHT: f32 = 20.0;

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
        .add_system_set_to_stage(
            CoreStage::PostUpdate,
            SystemSet::new()
                .with_system(position_translation)
                .with_system(size_scaling),
        )
        .run();
}

fn distance(p1: &Vector3<f64>, p2: &Vector3<f64>) -> f64 {
    ((p1[0] - p2[0]).powi(2)
        + (p1[1] - p2[1]).powi(2)).sqrt()
}

fn law_cosines(d1: f64, d2: f64, a: f64) -> f64 {
    (d1.powi(2) + d2.powi(2) - 2.0 * d1 * d2 * a.cos()).sqrt()
}

fn law_sines(d1: f64, a1: f64, d2: f64) -> f64 {
    (d2 * a1.sin() / d1).asin()
}

fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

fn keyboard_input(
    keys: Res<Input<KeyCode>>,
    target: Res<Target>,
    mut commands: Commands,
    mut player: Query<(&mut Robot, &mut Body)>,
) {
    for (mut p, mut t) in player.iter_mut() {
        if keys.just_pressed(KeyCode::Space) {
            let shot = p.get_ball_shot(&t, &target.target);
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
                    pos: vector![t.pos.x, t.pos.y, 0.0],
                    vel: shot,
                    acc: vector![0.0, 0.0, -9.81],
                })
                .insert(Size::square(1.0));
        }

        t.vel = vector![0.0, 0.0, 0.0];
        if keys.pressed(KeyCode::A) {
            t.vel.x = -5.0;
            p.bearing = PI;
        } else if keys.pressed(KeyCode::D) {
            t.vel.x = 5.0;
            p.bearing = 0.0;
        }
        if keys.pressed(KeyCode::S) {
            t.vel.y = -5.0;
            if keys.pressed(KeyCode::A) {
                p.bearing = 5.0 * PI / 4.0;
            } else if keys.pressed(KeyCode::D) {
                p.bearing = 7.0 * PI / 4.0;
            } else {
                p.bearing = 3.0 * PI / 2.0;
            }
        } else if keys.pressed(KeyCode::W) {
            t.vel.y = 5.0;
            if keys.pressed(KeyCode::A) {
                p.bearing = 3.0 * PI / 4.0;
            } else if keys.pressed(KeyCode::D) {
                p.bearing = PI / 4.0;
            } else {
                p.bearing = PI / 2.0;
            }
        }

        // traditional way of getting turret angle
        p.turret_angle = ((target.target.y - t.pos.y) / (target.target.x - t.pos.x)).atan() - p.bearing;
        if target.target.x - t.pos.x < 0.0 {
            p.turret_angle += PI;
        }

        // shooting while moving
        let velocity = (t.vel.x.powi(2) + t.vel.y.powi(2)).sqrt();
        let future_distance = law_cosines(velocity * 3.06, distance(&target.target, &t.pos), p.turret_angle);
        p.turret_angle += law_sines(future_distance, p.turret_angle, velocity * 3.06)
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
        })
        .insert(Size::square(2.0))
        .insert(Body {
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
            pos.pos.z as f32,
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
}

impl Robot {
    fn tree_map(dist: f64) -> (f64, f64) {
        (-G * dist / (2.0 * UP), UP)
    }

    fn get_ball_shot(&self, loc: &Body, target: &Vector3<f64>) -> Vector3<f64> {
        let dist = distance(&loc.pos, target);

        let vel = (loc.vel.x.powi(2) + loc.vel.y.powi(2)).sqrt();
        let dist = law_cosines(vel * 3.06, dist, self.turret_angle);

        let tree_map = Robot::tree_map(dist);

        let ball_x = (self.bearing + self.turret_angle).cos() * tree_map.0;
        let ball_y = (self.bearing + self.turret_angle).sin() * tree_map.0;

        loc.vel + vector![ball_x, ball_y, tree_map.1]
    }
}

#[derive(Component)]
struct Ball {}

#[derive(Component)]
struct Body {
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
