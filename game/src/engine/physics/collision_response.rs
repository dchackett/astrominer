use bevy::prelude::*;
use super::components::*;
use super::collision_detection::Collisions;
use super::toroidal::WorldBounds;
use crate::engine::asteroids::components::{Asteroid, AsteroidTier};
use crate::engine::asteroids::fracture::FractureQueue;
use crate::engine::rendering::dust::spawn_dust_burst;
use crate::engine::units::components::{Tug, Station, Rocket, TeamResources, Team};
use crate::config::GameConfig;

#[derive(Component)]
pub struct Bullet {
    pub lifetime: f32,
    pub team: Team,
}

/// Elastic collision response between two asteroids.
pub fn resolve_asteroid_asteroid(
    collisions: Res<Collisions>,
    mut query: Query<(&mut Transform, &mut Velocity, &Mass), With<Asteroid>>,
) {
    for event in &collisions.0 {
        let Ok([mut a, mut b]) = query.get_many_mut([event.entity_a, event.entity_b]) else {
            continue;
        };

        let total_mass = a.2 .0 + b.2 .0;
        let sep_a = event.overlap * (b.2 .0 / total_mass);
        let sep_b = event.overlap * (a.2 .0 / total_mass);
        a.0.translation -= (event.normal * sep_a).extend(0.0);
        b.0.translation += (event.normal * sep_b).extend(0.0);

        let v_a = a.1 .0.dot(event.normal);
        let v_b = b.1 .0.dot(event.normal);
        let m_a = a.2 .0;
        let m_b = b.2 .0;

        let new_v_a = ((m_a - m_b) * v_a + 2.0 * m_b * v_b) / total_mass;
        let new_v_b = ((m_b - m_a) * v_b + 2.0 * m_a * v_a) / total_mass;

        a.1 .0 += (new_v_a - v_a) * event.normal;
        b.1 .0 += (new_v_b - v_b) * event.normal;
    }
}

/// Bounce rocket off asteroids and apply damage.
pub fn resolve_rocket_asteroid(
    collisions: Res<Collisions>,
    mut rockets: Query<(&mut Transform, &mut Velocity, &Mass, &mut Health), With<Rocket>>,
    mut asteroids: Query<
        (&mut Transform, &mut Velocity, &Mass),
        (With<Asteroid>, Without<Rocket>),
    >,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (rocket_entity, asteroid_entity, normal) =
            if rockets.contains(event.entity_a) && asteroids.contains(event.entity_b) {
                (event.entity_a, event.entity_b, event.normal)
            } else if rockets.contains(event.entity_b) && asteroids.contains(event.entity_a) {
                (event.entity_b, event.entity_a, -event.normal)
            } else {
                continue;
            };

        let Ok(mut rocket) = rockets.get_mut(rocket_entity) else {
            continue;
        };
        let Ok(mut asteroid) = asteroids.get_mut(asteroid_entity) else {
            continue;
        };

        let total_mass = rocket.2 .0 + asteroid.2 .0;
        rocket.0.translation -=
            (normal * event.overlap * (asteroid.2 .0 / total_mass)).extend(0.0);
        asteroid.0.translation +=
            (normal * event.overlap * (rocket.2 .0 / total_mass)).extend(0.0);

        let v_r = rocket.1 .0.dot(normal);
        let v_a = asteroid.1 .0.dot(normal);
        let m_r = rocket.2 .0;
        let m_a = asteroid.2 .0;

        let new_v_r = ((m_r - m_a) * v_r + 2.0 * m_a * v_a) / total_mass;
        let new_v_a = ((m_a - m_r) * v_a + 2.0 * m_r * v_r) / total_mass;

        rocket.1 .0 += (new_v_r - v_r) * normal;
        asteroid.1 .0 += (new_v_a - v_a) * normal;

        let impact_speed = (v_r - v_a).abs();
        rocket.3.current -= impact_speed * config.physics.collision_damage_factor;
    }
}

/// Bullet hits asteroid: destroy bullet, damage asteroid, spawn dust, potentially fracture.
pub fn resolve_bullet_asteroid(
    mut commands: Commands,
    collisions: Res<Collisions>,
    bullets: Query<(&Transform, &Velocity), With<Bullet>>,
    mut asteroids: Query<(Entity, &mut Health, &AsteroidTier), With<Asteroid>>,
    mut fracture_queue: ResMut<FractureQueue>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (bullet_entity, asteroid_entity, _normal) =
            if bullets.contains(event.entity_a) && asteroids.contains(event.entity_b) {
                (event.entity_a, event.entity_b, event.normal)
            } else if bullets.contains(event.entity_b) && asteroids.contains(event.entity_a) {
                (event.entity_b, event.entity_a, -event.normal)
            } else {
                continue;
            };

        let Ok((bullet_tf, bullet_vel)) = bullets.get(bullet_entity) else { continue };
        let impact_pos = bullet_tf.translation.truncate();
        let impact_vel = bullet_vel.0 * 0.3;

        spawn_dust_burst(&mut commands, impact_pos, 5, 40.0, impact_vel);
        commands.entity(bullet_entity).despawn();

        let Ok((entity, mut health, tier)) = asteroids.get_mut(asteroid_entity) else {
            continue;
        };
        health.current -= config.bullet.damage;

        if health.current <= 0.0 {
            fracture_queue.0.push((entity, tier.0));
        }
    }
}

/// Elastic collision between tugs and asteroids with damage.
pub fn resolve_tug_asteroid(
    collisions: Res<Collisions>,
    mut tugs: Query<(&mut Transform, &mut Velocity, &Mass, &mut Health), (With<Tug>, Without<Asteroid>)>,
    mut asteroids: Query<(&mut Transform, &mut Velocity, &Mass), (With<Asteroid>, Without<Tug>)>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (tug_entity, asteroid_entity, normal) =
            if tugs.contains(event.entity_a) && asteroids.contains(event.entity_b) {
                (event.entity_a, event.entity_b, event.normal)
            } else if tugs.contains(event.entity_b) && asteroids.contains(event.entity_a) {
                (event.entity_b, event.entity_a, -event.normal)
            } else {
                continue;
            };

        let Ok(mut tug) = tugs.get_mut(tug_entity) else { continue };
        let Ok(mut asteroid) = asteroids.get_mut(asteroid_entity) else { continue };

        let total_mass = tug.2 .0 + asteroid.2 .0;
        tug.0.translation -= (normal * event.overlap * (asteroid.2 .0 / total_mass)).extend(0.0);
        asteroid.0.translation += (normal * event.overlap * (tug.2 .0 / total_mass)).extend(0.0);

        let v_t = tug.1 .0.dot(normal);
        let v_a = asteroid.1 .0.dot(normal);
        let m_t = tug.2 .0;
        let m_a = asteroid.2 .0;

        let new_v_t = ((m_t - m_a) * v_t + 2.0 * m_a * v_a) / total_mass;
        let new_v_a = ((m_a - m_t) * v_a + 2.0 * m_t * v_t) / total_mass;

        tug.1 .0 += (new_v_t - v_t) * normal;
        asteroid.1 .0 += (new_v_a - v_a) * normal;

        let impact_speed = (v_t - v_a).abs();
        tug.3.current -= impact_speed * config.physics.collision_damage_factor;
    }
}

/// Station-asteroid collision: gather slow small rocks, bounce everything else.
pub fn resolve_station_asteroid(
    mut commands: Commands,
    collisions: Res<Collisions>,
    stations: Query<(&Transform, &Mass, &Team), (With<Station>, Without<Asteroid>)>,
    mut asteroids: Query<(Entity, &mut Transform, &mut Velocity, &Mass, &AsteroidTier), (With<Asteroid>, Without<Station>)>,
    mut resources: ResMut<TeamResources>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (station_entity, asteroid_entity, normal) =
            if stations.contains(event.entity_a) && asteroids.contains(event.entity_b) {
                (event.entity_a, event.entity_b, event.normal)
            } else if stations.contains(event.entity_b) && asteroids.contains(event.entity_a) {
                (event.entity_b, event.entity_a, -event.normal)
            } else {
                continue;
            };

        let Ok((_station_tf, _station_mass, team)) = stations.get(station_entity) else { continue };
        let Ok((entity, mut tf, mut vel, _mass, tier)) = asteroids.get_mut(asteroid_entity) else { continue };

        let speed = vel.0.length();

        // Small rocks arriving gently get collected
        if tier.0 <= config.asteroids.max_gatherable_tier && speed < config.station.gather_speed_threshold {
            let mineral_value = config.economy.mineral_value(tier.0);
            resources.add(*team, mineral_value);
            commands.entity(entity).despawn();
            continue;
        }

        // Otherwise bounce off (station is immovable)
        tf.translation += (normal * event.overlap).extend(0.0);
        let v_along = vel.0.dot(normal);
        if v_along < 0.0 {
            vel.0 -= 2.0 * v_along * normal;
            vel.0 *= config.physics.station_bounce_energy;
        }
    }
}

/// Elastic collision between tugs and station (station is immovable).
pub fn resolve_tug_station(
    collisions: Res<Collisions>,
    stations: Query<Entity, (With<Station>, Without<Tug>)>,
    mut tugs: Query<(&mut Transform, &mut Velocity, &Mass), (With<Tug>, Without<Station>)>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (tug_entity, normal) =
            if tugs.contains(event.entity_a) && stations.contains(event.entity_b) {
                (event.entity_a, event.normal)
            } else if tugs.contains(event.entity_b) && stations.contains(event.entity_a) {
                (event.entity_b, -event.normal)
            } else {
                continue;
            };

        let Ok(mut tug) = tugs.get_mut(tug_entity) else { continue };

        tug.0.translation -= (normal * event.overlap).extend(0.0);
        let v_along = tug.1 .0.dot(normal);
        if v_along > 0.0 {
            tug.1 .0 -= 2.0 * v_along * normal;
            tug.1 .0 *= config.physics.station_bounce_energy;
        }
    }
}

/// Bounce rockets off the station (station is immovable).
pub fn resolve_rocket_station(
    collisions: Res<Collisions>,
    stations: Query<Entity, (With<Station>, Without<Rocket>)>,
    mut rockets: Query<(&mut Transform, &mut Velocity, &Mass), (With<Rocket>, Without<Station>)>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (rocket_entity, normal) =
            if rockets.contains(event.entity_a) && stations.contains(event.entity_b) {
                (event.entity_a, event.normal)
            } else if rockets.contains(event.entity_b) && stations.contains(event.entity_a) {
                (event.entity_b, -event.normal)
            } else {
                continue;
            };

        let Ok(mut rocket) = rockets.get_mut(rocket_entity) else { continue };

        rocket.0.translation -= (normal * event.overlap).extend(0.0);
        let v_along = rocket.1 .0.dot(normal);
        if v_along > 0.0 {
            rocket.1 .0 -= 2.0 * v_along * normal;
            rocket.1 .0 *= config.physics.station_bounce_energy;
        }
    }
}

/// Bullet hits rocket: despawn bullet, damage rocket, spawn dust.
pub fn resolve_bullet_rocket(
    mut commands: Commands,
    collisions: Res<Collisions>,
    bullets: Query<(&Transform, &Velocity, &Bullet)>,
    mut rockets: Query<(Entity, &mut Health, &Team), With<Rocket>>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (bullet_entity, rocket_entity) =
            if bullets.contains(event.entity_a) && rockets.contains(event.entity_b) {
                (event.entity_a, event.entity_b)
            } else if bullets.contains(event.entity_b) && rockets.contains(event.entity_a) {
                (event.entity_b, event.entity_a)
            } else {
                continue;
            };

        let Ok((bullet_tf, bullet_vel, bullet)) = bullets.get(bullet_entity) else { continue };
        let Ok((_entity, mut health, team)) = rockets.get_mut(rocket_entity) else { continue };

        if !config.bullet.friendly_fire && bullet.team == *team { continue; }

        let impact_pos = bullet_tf.translation.truncate();
        let impact_vel = bullet_vel.0 * 0.3;
        spawn_dust_burst(&mut commands, impact_pos, 5, 40.0, impact_vel);
        commands.entity(bullet_entity).despawn();
        health.current -= config.bullet.damage;
    }
}

/// Bullet hits station: despawn bullet, damage station, spawn dust.
pub fn resolve_bullet_station(
    mut commands: Commands,
    collisions: Res<Collisions>,
    bullets: Query<(&Transform, &Velocity, &Bullet)>,
    mut stations: Query<(Entity, &mut Health, &Team), With<Station>>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (bullet_entity, station_entity) =
            if bullets.contains(event.entity_a) && stations.contains(event.entity_b) {
                (event.entity_a, event.entity_b)
            } else if bullets.contains(event.entity_b) && stations.contains(event.entity_a) {
                (event.entity_b, event.entity_a)
            } else {
                continue;
            };

        let Ok((bullet_tf, bullet_vel, bullet)) = bullets.get(bullet_entity) else { continue };
        let Ok((_entity, mut health, team)) = stations.get_mut(station_entity) else { continue };

        if !config.bullet.friendly_fire && bullet.team == *team { continue; }

        let impact_pos = bullet_tf.translation.truncate();
        let impact_vel = bullet_vel.0 * 0.3;
        spawn_dust_burst(&mut commands, impact_pos, 8, 50.0, impact_vel);
        commands.entity(bullet_entity).despawn();
        health.current -= config.bullet.damage;
    }
}

/// Bullet hits tug: despawn bullet, damage tug, spawn dust.
pub fn resolve_bullet_tug(
    mut commands: Commands,
    collisions: Res<Collisions>,
    bullets: Query<(&Transform, &Velocity, &Bullet)>,
    mut tugs: Query<(Entity, &mut Health, &Team), With<Tug>>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (bullet_entity, tug_entity) =
            if bullets.contains(event.entity_a) && tugs.contains(event.entity_b) {
                (event.entity_a, event.entity_b)
            } else if bullets.contains(event.entity_b) && tugs.contains(event.entity_a) {
                (event.entity_b, event.entity_a)
            } else {
                continue;
            };

        let Ok((bullet_tf, bullet_vel, bullet)) = bullets.get(bullet_entity) else { continue };
        let Ok((_entity, mut health, team)) = tugs.get_mut(tug_entity) else { continue };

        if !config.bullet.friendly_fire && bullet.team == *team { continue; }

        let impact_pos = bullet_tf.translation.truncate();
        let impact_vel = bullet_vel.0 * 0.3;
        spawn_dust_burst(&mut commands, impact_pos, 5, 40.0, impact_vel);
        commands.entity(bullet_entity).despawn();
        health.current -= config.bullet.damage;
    }
}

/// Elastic collision between two rockets with damage.
pub fn resolve_rocket_rocket(
    collisions: Res<Collisions>,
    mut rockets: Query<(&mut Transform, &mut Velocity, &Mass, &mut Health), With<Rocket>>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let Ok([mut a, mut b]) = rockets.get_many_mut([event.entity_a, event.entity_b]) else {
            continue;
        };

        let total_mass = a.2 .0 + b.2 .0;
        let sep_a = event.overlap * (b.2 .0 / total_mass);
        let sep_b = event.overlap * (a.2 .0 / total_mass);
        a.0.translation -= (event.normal * sep_a).extend(0.0);
        b.0.translation += (event.normal * sep_b).extend(0.0);

        let v_a = a.1 .0.dot(event.normal);
        let v_b = b.1 .0.dot(event.normal);
        let m_a = a.2 .0;
        let m_b = b.2 .0;

        let new_v_a = ((m_a - m_b) * v_a + 2.0 * m_b * v_b) / total_mass;
        let new_v_b = ((m_b - m_a) * v_b + 2.0 * m_a * v_a) / total_mass;

        a.1 .0 += (new_v_a - v_a) * event.normal;
        b.1 .0 += (new_v_b - v_b) * event.normal;

        let impact_speed = (v_a - v_b).abs();
        let damage = impact_speed * config.physics.collision_damage_factor;
        a.3.current -= damage;
        b.3.current -= damage;
    }
}

/// Elastic collision between rocket and tug with damage to both.
pub fn resolve_rocket_tug(
    collisions: Res<Collisions>,
    mut rockets: Query<(&mut Transform, &mut Velocity, &Mass, &mut Health), (With<Rocket>, Without<Tug>)>,
    mut tugs: Query<(&mut Transform, &mut Velocity, &Mass, &mut Health), (With<Tug>, Without<Rocket>)>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let (rocket_entity, tug_entity, normal) =
            if rockets.contains(event.entity_a) && tugs.contains(event.entity_b) {
                (event.entity_a, event.entity_b, event.normal)
            } else if rockets.contains(event.entity_b) && tugs.contains(event.entity_a) {
                (event.entity_b, event.entity_a, -event.normal)
            } else {
                continue;
            };

        let Ok(mut rocket) = rockets.get_mut(rocket_entity) else { continue };
        let Ok(mut tug) = tugs.get_mut(tug_entity) else { continue };

        let total_mass = rocket.2 .0 + tug.2 .0;
        rocket.0.translation -= (normal * event.overlap * (tug.2 .0 / total_mass)).extend(0.0);
        tug.0.translation += (normal * event.overlap * (rocket.2 .0 / total_mass)).extend(0.0);

        let v_r = rocket.1 .0.dot(normal);
        let v_t = tug.1 .0.dot(normal);
        let m_r = rocket.2 .0;
        let m_t = tug.2 .0;

        let new_v_r = ((m_r - m_t) * v_r + 2.0 * m_t * v_t) / total_mass;
        let new_v_t = ((m_t - m_r) * v_t + 2.0 * m_r * v_r) / total_mass;

        rocket.1 .0 += (new_v_r - v_r) * normal;
        tug.1 .0 += (new_v_t - v_t) * normal;

        let impact_speed = (v_r - v_t).abs();
        let damage = impact_speed * config.physics.collision_damage_factor;
        rocket.3.current -= damage;
        tug.3.current -= damage;
    }
}

/// Elastic collision between two tugs with damage.
pub fn resolve_tug_tug(
    collisions: Res<Collisions>,
    mut tugs: Query<(&mut Transform, &mut Velocity, &Mass, &mut Health), With<Tug>>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let Ok([mut a, mut b]) = tugs.get_many_mut([event.entity_a, event.entity_b]) else {
            continue;
        };

        let total_mass = a.2 .0 + b.2 .0;
        let sep_a = event.overlap * (b.2 .0 / total_mass);
        let sep_b = event.overlap * (a.2 .0 / total_mass);
        a.0.translation -= (event.normal * sep_a).extend(0.0);
        b.0.translation += (event.normal * sep_b).extend(0.0);

        let v_a = a.1 .0.dot(event.normal);
        let v_b = b.1 .0.dot(event.normal);
        let m_a = a.2 .0;
        let m_b = b.2 .0;

        let new_v_a = ((m_a - m_b) * v_a + 2.0 * m_b * v_b) / total_mass;
        let new_v_b = ((m_b - m_a) * v_b + 2.0 * m_a * v_a) / total_mass;

        a.1 .0 += (new_v_a - v_a) * event.normal;
        b.1 .0 += (new_v_b - v_b) * event.normal;

        let impact_speed = (v_a - v_b).abs();
        let damage = impact_speed * config.physics.collision_damage_factor;
        a.3.current -= damage;
        b.3.current -= damage;
    }
}

/// Two bullets collide: despawn both, spawn dust.
pub fn resolve_bullet_bullet(
    mut commands: Commands,
    collisions: Res<Collisions>,
    bullets: Query<(&Transform, &Velocity), With<Bullet>>,
) {
    for event in &collisions.0 {
        let Ok((ta, va)) = bullets.get(event.entity_a) else { continue };
        let Ok((tb, vb)) = bullets.get(event.entity_b) else { continue };

        let mid = (ta.translation.truncate() + tb.translation.truncate()) * 0.5;
        let avg_vel = (va.0 + vb.0) * 0.5;
        spawn_dust_burst(&mut commands, mid, 4, 30.0, avg_vel);
        commands.entity(event.entity_a).despawn();
        commands.entity(event.entity_b).despawn();
    }
}

/// Spawn dust at collision points between asteroids.
pub fn spawn_collision_dust(
    mut commands: Commands,
    collisions: Res<Collisions>,
    asteroids: Query<(&Transform, &Velocity), With<Asteroid>>,
    bounds: Res<WorldBounds>,
    config: Res<GameConfig>,
) {
    for event in &collisions.0 {
        let Ok((ta, va)) = asteroids.get(event.entity_a) else { continue };
        let Ok((tb, vb)) = asteroids.get(event.entity_b) else { continue };

        let pos_a = ta.translation.truncate();
        let delta = bounds.shortest_delta(pos_a, tb.translation.truncate());
        let contact = pos_a + delta * 0.5;

        let rel_speed = (va.0 - vb.0).length();
        if rel_speed < config.physics.dust_speed_threshold {
            continue;
        }

        let avg_vel = (va.0 + vb.0) * 0.5;
        let count = (rel_speed * 0.1).min(8.0) as usize;
        spawn_dust_burst(&mut commands, contact, count.max(3), rel_speed * 0.3, avg_vel);
    }
}
