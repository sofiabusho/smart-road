//! Simulation statistics collection (C05+).

use std::collections::{HashMap, HashSet};

use crate::vehicle::VehicleId;

/// Session metrics displayed on exit (C05/C06).
#[derive(Debug, Clone, PartialEq)]
pub struct Stats {
    pub vehicles_passed: u32,
    /// Cumulative vehicles that completed a crossing (equals `vehicles_passed` until concurrent-peak tracking is added in C06).
    pub max_vehicles_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_crossing_time: f32,
    pub min_crossing_time: f32,
    pub close_calls: u32,
    /// C08 bonus (REQ-B2): total session wall time in seconds.
    pub session_duration_secs: f32,
    /// C08 bonus: mean crossing time across vehicles that completed.
    pub avg_crossing_time_secs: f32,
    /// C08 bonus: peak simultaneous vehicles in the managed/exiting zone.
    pub peak_concurrent_in_zone: u32,
    /// C08 bonus: distinct vehicles that entered the smart intersection zone.
    pub vehicles_entered_zone: u32,
    sum_crossing_time_secs: f32,
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            vehicles_passed: 0,
            max_vehicles_passed: 0,
            max_velocity: 0.0,
            min_velocity: f32::MAX,
            max_crossing_time: 0.0,
            min_crossing_time: f32::MAX,
            close_calls: 0,
            session_duration_secs: 0.0,
            avg_crossing_time_secs: 0.0,
            peak_concurrent_in_zone: 0,
            vehicles_entered_zone: 0,
            sum_crossing_time_secs: 0.0,
        }
    }
}

impl Stats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Finalize derived bonus metrics before display (C08).
    pub fn finalize_session(&mut self, session_duration_secs: f32) {
        self.session_duration_secs = session_duration_secs;
        if self.vehicles_passed > 0 {
            self.avg_crossing_time_secs = self.sum_crossing_time_secs / self.vehicles_passed as f32;
        }
    }
}

/// Events fed into the stats collector from the simulation loop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatsEvent {
    VehicleManaged {
        id: VehicleId,
        t: f32,
    },
    VehicleExited {
        id: VehicleId,
        crossing_time: f32,
        peak_velocity: f32,
    },
    CloseCall {
        ids: (VehicleId, VehicleId),
    },
    VelocitySample {
        id: VehicleId,
        v: f32,
    },
}

/// Apply a single stats event (SDS §13.4).
pub fn apply_event(stats: &mut Stats, event: StatsEvent) {
    match event {
        // Session timing metadata for C06; dedup handled in StatsSession::observe_vehicles.
        StatsEvent::VehicleManaged { .. } => {
            stats.vehicles_entered_zone += 1;
        }
        StatsEvent::VehicleExited {
            crossing_time,
            peak_velocity,
            ..
        } => {
            stats.vehicles_passed += 1;
            stats.max_vehicles_passed = stats.max_vehicles_passed.max(stats.vehicles_passed);
            stats.sum_crossing_time_secs += crossing_time;
            update_velocity_bounds(stats, peak_velocity);
            update_crossing_time_bounds(stats, crossing_time);
        }
        StatsEvent::CloseCall { .. } => {
            stats.close_calls += 1;
        }
        StatsEvent::VelocitySample { v, .. } => {
            update_velocity_bounds(stats, v);
        }
    }
}

fn update_velocity_bounds(stats: &mut Stats, velocity: f32) {
    if velocity < 0.0 {
        return;
    }
    stats.max_velocity = stats.max_velocity.max(velocity);
    stats.min_velocity = stats.min_velocity.min(velocity);
}

fn update_crossing_time_bounds(stats: &mut Stats, crossing_time: f32) {
    if crossing_time <= 0.0 {
        return;
    }
    stats.max_crossing_time = stats.max_crossing_time.max(crossing_time);
    stats.min_crossing_time = stats.min_crossing_time.min(crossing_time);
}

/// Tracks per-vehicle peaks and emits stats events from the game loop.
#[derive(Debug, Default)]
pub struct StatsSession {
    pub stats: Stats,
    peak_velocity: HashMap<VehicleId, f32>,
    managed_ids: HashSet<VehicleId>,
    close_call_pairs: HashSet<(u64, u64)>,
}

impl StatsSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sample active vehicles each frame for velocity and managed detection.
    pub fn observe_vehicles(&mut self, vehicles: &[crate::vehicle::Vehicle], session_time: f32) {
        use crate::vehicle::VehicleState;

        let concurrent_in_zone = vehicles
            .iter()
            .filter(|vehicle| {
                matches!(vehicle.state, VehicleState::Managed | VehicleState::Exiting)
            })
            .count() as u32;
        self.stats.peak_concurrent_in_zone =
            self.stats.peak_concurrent_in_zone.max(concurrent_in_zone);

        for vehicle in vehicles {
            if vehicle.state == VehicleState::Done {
                continue;
            }

            apply_event(
                &mut self.stats,
                StatsEvent::VelocitySample {
                    id: vehicle.id,
                    v: vehicle.velocity,
                },
            );

            let peak = self.peak_velocity.entry(vehicle.id).or_insert(0.0);
            if vehicle.velocity > *peak {
                *peak = vehicle.velocity;
            }

            if vehicle.state == VehicleState::Managed && self.managed_ids.insert(vehicle.id) {
                apply_event(
                    &mut self.stats,
                    StatsEvent::VehicleManaged {
                        id: vehicle.id,
                        t: session_time,
                    },
                );
            }
        }
    }

    /// Record a vehicle that left the canvas after entering the managed zone.
    pub fn record_exit(&mut self, id: VehicleId, crossing_time: f32) {
        let peak_velocity = self.peak_velocity.remove(&id).unwrap_or(0.0);
        apply_event(
            &mut self.stats,
            StatsEvent::VehicleExited {
                id,
                crossing_time,
                peak_velocity,
            },
        );
        self.managed_ids.remove(&id);
    }

    /// Record a close call (REQ-26); deduplicated per vehicle pair per session.
    pub fn record_close_call(&mut self, a: VehicleId, b: VehicleId) {
        let pair = (a.0.min(b.0), a.0.max(b.0));
        if !self.close_call_pairs.insert(pair) {
            return;
        }
        apply_event(&mut self.stats, StatsEvent::CloseCall { ids: (a, b) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_exit_updates_passed_count_and_crossing_bounds() {
        let mut stats = Stats::new();
        apply_event(
            &mut stats,
            StatsEvent::VehicleExited {
                id: VehicleId(1),
                crossing_time: 2.5,
                peak_velocity: 100.0,
            },
        );
        apply_event(
            &mut stats,
            StatsEvent::VehicleExited {
                id: VehicleId(2),
                crossing_time: 1.0,
                peak_velocity: 80.0,
            },
        );

        assert_eq!(stats.vehicles_passed, 2);
        assert_eq!(stats.max_vehicles_passed, 2);
        assert_eq!(stats.max_crossing_time, 2.5);
        assert_eq!(stats.min_crossing_time, 1.0);
        assert_eq!(stats.max_velocity, 100.0);
        assert_eq!(stats.min_velocity, 80.0);
    }

    #[test]
    fn velocity_samples_track_min_and_max() {
        let mut stats = Stats::new();
        for v in [120.0, 60.0, 90.0] {
            apply_event(
                &mut stats,
                StatsEvent::VelocitySample {
                    id: VehicleId(1),
                    v,
                },
            );
        }

        assert_eq!(stats.max_velocity, 120.0);
        assert_eq!(stats.min_velocity, 60.0);
    }

    #[test]
    fn close_call_increments_counter() {
        let mut stats = Stats::new();
        apply_event(
            &mut stats,
            StatsEvent::CloseCall {
                ids: (VehicleId(1), VehicleId(2)),
            },
        );
        assert_eq!(stats.close_calls, 1);
    }

    #[test]
    fn close_call_deduplicates_vehicle_pairs() {
        let mut session = StatsSession::new();
        session.record_close_call(VehicleId(1), VehicleId(2));
        session.record_close_call(VehicleId(2), VehicleId(1));
        assert_eq!(session.stats.close_calls, 1);
    }

    #[test]
    fn stats_session_records_managed_once_and_exit() {
        use crate::intersection::{Cardinal, Route};
        use crate::vehicle::{Vehicle, VehicleState};

        let mut session = StatsSession::new();
        let vehicle = Vehicle {
            id: VehicleId(7),
            lane_id: crate::intersection::LaneId(0),
            route: Route::Straight,
            approach: Cardinal::South,
            position: crate::intersection::Vec2::new(0.0, 0.0),
            heading_rad: 0.0,
            velocity: 100.0,
            commanded_velocity: 100.0,
            nominal_velocity: 100.0,
            state: VehicleState::Managed,
            path_index: 0,
            distance_in_crossing: 0.0,
            time_in_crossing: 1.5,
            reservation_granted: true,
            scheduler_yield: false,
            reservation_hold: false,
        };

        session.observe_vehicles(std::slice::from_ref(&vehicle), 0.0);
        session.observe_vehicles(std::slice::from_ref(&vehicle), 0.1);
        session.record_exit(vehicle.id, vehicle.time_in_crossing);

        assert_eq!(session.stats.vehicles_passed, 1);
        assert_eq!(session.stats.max_velocity, 100.0);
    }

    #[test]
    fn bonus_stats_track_zone_entries_and_finalize_average() {
        let mut stats = Stats::new();
        apply_event(
            &mut stats,
            StatsEvent::VehicleManaged {
                id: VehicleId(1),
                t: 0.0,
            },
        );
        apply_event(
            &mut stats,
            StatsEvent::VehicleExited {
                id: VehicleId(1),
                crossing_time: 2.0,
                peak_velocity: 90.0,
            },
        );
        apply_event(
            &mut stats,
            StatsEvent::VehicleExited {
                id: VehicleId(2),
                crossing_time: 4.0,
                peak_velocity: 100.0,
            },
        );

        assert_eq!(stats.vehicles_entered_zone, 1);
        stats.finalize_session(10.0);
        assert_eq!(stats.session_duration_secs, 10.0);
        assert!((stats.avg_crossing_time_secs - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn peak_concurrent_in_zone_tracks_managed_and_exiting() {
        use crate::intersection::{Cardinal, Route};
        use crate::vehicle::{Vehicle, VehicleState};

        let mut session = StatsSession::new();
        let vehicles = [
            Vehicle {
                id: VehicleId(1),
                lane_id: crate::intersection::LaneId(0),
                route: Route::Straight,
                approach: Cardinal::South,
                position: crate::intersection::Vec2::new(0.0, 0.0),
                heading_rad: 0.0,
                velocity: 100.0,
                commanded_velocity: 100.0,
                nominal_velocity: 100.0,
                state: VehicleState::Managed,
                path_index: 0,
                distance_in_crossing: 0.0,
                time_in_crossing: 0.0,
                reservation_granted: true,
                scheduler_yield: false,
                reservation_hold: false,
            },
            Vehicle {
                id: VehicleId(2),
                lane_id: crate::intersection::LaneId(1),
                route: Route::Straight,
                approach: Cardinal::West,
                position: crate::intersection::Vec2::new(0.0, 0.0),
                heading_rad: 0.0,
                velocity: 100.0,
                commanded_velocity: 100.0,
                nominal_velocity: 100.0,
                state: VehicleState::Exiting,
                path_index: 0,
                distance_in_crossing: 0.0,
                time_in_crossing: 0.0,
                reservation_granted: true,
                scheduler_yield: false,
                reservation_hold: false,
            },
        ];

        session.observe_vehicles(&vehicles, 0.0);
        assert_eq!(session.stats.peak_concurrent_in_zone, 2);
    }
}
