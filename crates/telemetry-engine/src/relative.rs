use lmu_telemetry::VehicleScoringSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelativeField {
    /// Field indexes ordered from nearest to farthest ahead.
    pub ahead: Vec<usize>,
    pub player: usize,
    /// Field indexes ordered from nearest to farthest behind.
    pub behind: Vec<usize>,
}

/// Classifies valid opponents by physical circular distance from the player.
/// Missing opponent positions are skipped; a missing player position still
/// makes the result unavailable because there is no reliable origin.
pub fn relative_neighbors(
    field: &[VehicleScoringSnapshot],
    player_slot_id: i32,
    same_class_only: bool,
    track_length_m: Option<f64>,
) -> Option<RelativeField> {
    let track_length_m = track_length_m.filter(|value| value.is_finite() && *value > 0.0)?;
    let player = field
        .iter()
        .position(|car| car.is_player || car.slot_id == player_slot_id)?;
    let player_distance = field[player]
        .lap_distance_m
        .filter(|value| value.is_finite())?;
    let player_class = field[player].vehicle_class.as_deref();

    let mut ahead = Vec::new();
    let mut behind = Vec::new();
    for (index, car) in field.iter().enumerate() {
        if index == player || (same_class_only && car.vehicle_class.as_deref() != player_class) {
            continue;
        }
        let Some(distance) = car.lap_distance_m.filter(|value| value.is_finite()) else {
            continue;
        };
        let mut delta = distance - player_distance;
        if delta > track_length_m / 2.0 {
            delta -= track_length_m;
        } else if delta < -track_length_m / 2.0 {
            delta += track_length_m;
        }
        if delta >= 0.0 {
            ahead.push((index, delta));
        } else {
            behind.push((index, delta.abs()));
        }
    }
    ahead.sort_by(|(left_index, left_delta), (right_index, right_delta)| {
        left_delta
            .total_cmp(right_delta)
            .then_with(|| left_index.cmp(right_index))
    });
    behind.sort_by(|(left_index, left_delta), (right_index, right_delta)| {
        left_delta
            .total_cmp(right_delta)
            .then_with(|| left_index.cmp(right_index))
    });

    Some(RelativeField {
        ahead: ahead.into_iter().map(|(index, _)| index).collect(),
        player,
        behind: behind.into_iter().map(|(index, _)| index).collect(),
    })
}

/// Returns field indexes ordered from the nearest car ahead, through the
/// player, to the nearest car behind. Race classification is not used.
pub fn order_by_track_proximity(
    field: &[VehicleScoringSnapshot],
    player_slot_id: i32,
    same_class_only: bool,
    track_length_m: Option<f64>,
) -> Option<Vec<usize>> {
    let neighbors = relative_neighbors(field, player_slot_id, same_class_only, track_length_m)?;
    Some(
        neighbors
            .ahead
            .into_iter()
            .chain(std::iter::once(neighbors.player))
            .chain(neighbors.behind)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn car(
        slot_id: i32,
        lap_number: i32,
        distance: f64,
        class: &str,
        player: bool,
    ) -> VehicleScoringSnapshot {
        VehicleScoringSnapshot {
            slot_id,
            driver_name: None,
            vehicle_name: None,
            vehicle_class: Some(class.to_string()),
            place: None,
            lap_number,
            lap_distance_m: Some(distance),
            current_sector: None,
            last_lap_seconds: None,
            best_lap_seconds: None,
            gap_to_next_seconds: None,
            gap_to_leader_seconds: None,
            laps_behind_next: None,
            laps_behind_leader: None,
            in_pits: false,
            in_garage: false,
            pit_state: None,
            finish_status: None,
            flag: None,
            is_player: player,
            world_position: None,
        }
    }

    #[test]
    fn orders_physical_neighbors_across_start_finish() {
        let field = vec![
            car(1, 5, 4_900.0, "Hypercar", false),
            car(2, 5, 100.0, "Hypercar", true),
            car(3, 5, 2_000.0, "Hypercar", false),
        ];

        assert_eq!(
            order_by_track_proximity(&field, 2, false, Some(5_000.0)),
            Some(vec![2, 1, 0])
        );
    }

    #[test]
    fn keeps_lapped_traffic_near_the_player() {
        let field = vec![
            car(1, 1, 2_000.0, "Hypercar", false),
            car(2, 2, 100.0, "Hypercar", true),
            car(3, 2, 120.0, "Hypercar", false),
        ];

        assert_eq!(
            order_by_track_proximity(&field, 2, false, Some(5_000.0)),
            Some(vec![2, 0, 1])
        );
    }

    #[test]
    fn missing_opponent_position_does_not_hide_valid_neighbors() {
        let mut field = vec![
            car(1, 1, 100.0, "Hypercar", true),
            car(2, 1, 120.0, "Hypercar", false),
        ];
        field[1].lap_distance_m = None;

        assert_eq!(
            order_by_track_proximity(&field, 1, false, Some(5_000.0)),
            Some(vec![0])
        );
    }

    #[test]
    fn missing_player_position_keeps_relative_unavailable() {
        let mut field = vec![
            car(1, 1, 100.0, "Hypercar", true),
            car(2, 1, 120.0, "Hypercar", false),
        ];
        field[0].lap_distance_m = None;

        assert_eq!(relative_neighbors(&field, 1, false, Some(5_000.0)), None);
    }

    #[test]
    fn exposes_nearest_ahead_and_behind_independently() {
        let field = vec![
            car(1, 1, 100.0, "Hypercar", true),
            car(2, 1, 300.0, "Hypercar", false),
            car(3, 1, 4_950.0, "Hypercar", false),
            car(4, 2, 120.0, "Hypercar", false),
        ];

        assert_eq!(
            relative_neighbors(&field, 1, false, Some(5_000.0)),
            Some(RelativeField {
                ahead: vec![3, 1],
                player: 0,
                behind: vec![2],
            })
        );
    }

    #[test]
    fn filters_by_class_without_changing_physical_order() {
        let field = vec![
            car(1, 1, 100.0, "Hypercar", true),
            car(2, 1, 120.0, "LMP2", false),
            car(3, 1, 80.0, "Hypercar", false),
        ];

        assert_eq!(
            order_by_track_proximity(&field, 1, true, Some(5_000.0)),
            Some(vec![0, 2])
        );
    }
}
