use lmu_telemetry::VehicleScoringSnapshot;

/// Returns field indexes ordered from the nearest car ahead, through the
/// player, to the nearest car behind. Race classification is not used.
pub fn order_by_track_proximity(
    field: &[VehicleScoringSnapshot],
    player_slot_id: i32,
    same_class_only: bool,
    track_length_m: Option<f64>,
) -> Option<Vec<usize>> {
    let track_length_m = track_length_m.filter(|value| value.is_finite() && *value > 0.0)?;
    let player_index = field
        .iter()
        .position(|car| car.is_player || car.slot_id == player_slot_id)?;
    let player = &field[player_index];
    let player_distance = player.lap_distance_m.filter(|value| value.is_finite())?;
    let player_class = player.vehicle_class.as_deref();

    let mut ranked = field
        .iter()
        .enumerate()
        .filter(|(_, car)| !same_class_only || car.vehicle_class.as_deref() == player_class)
        .map(|(index, car)| {
            let distance = car.lap_distance_m.filter(|value| value.is_finite())?;
            let mut delta = f64::from(car.lap_number - player.lap_number) * track_length_m
                + distance
                - player_distance;
            if car.lap_number == player.lap_number {
                if delta > track_length_m / 2.0 {
                    delta -= track_length_m;
                } else if delta < -track_length_m / 2.0 {
                    delta += track_length_m;
                }
            }
            Some((index, delta))
        })
        .collect::<Option<Vec<_>>>()?;

    ranked.sort_by(|(left_index, left_delta), (right_index, right_delta)| {
        let left_group = if *left_delta > 0.0 {
            0_u8
        } else if *left_delta < 0.0 {
            2
        } else {
            1
        };
        let right_group = if *right_delta > 0.0 {
            0_u8
        } else if *right_delta < 0.0 {
            2
        } else {
            1
        };
        left_group
            .cmp(&right_group)
            .then_with(|| left_delta.abs().total_cmp(&right_delta.abs()))
            .then_with(|| left_index.cmp(right_index))
    });

    Some(ranked.into_iter().map(|(index, _)| index).collect())
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
            Some(vec![2, 1, 0])
        );
    }

    #[test]
    fn missing_track_position_is_not_replaced_with_fake_data() {
        let mut field = vec![
            car(1, 1, 100.0, "Hypercar", true),
            car(2, 1, 120.0, "Hypercar", false),
        ];
        field[1].lap_distance_m = None;

        assert_eq!(
            order_by_track_proximity(&field, 1, false, Some(5_000.0)),
            None
        );
    }
}
