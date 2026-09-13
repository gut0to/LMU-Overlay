//! Raw LMU shared-memory layout constants and safe byte views.
//!
//! LMU shared-memory layout adapted from TinyPedal/pyLMUSharedMemory,
//! based on Studio 397's official LMU SharedMemoryInterface.
//! See THIRD_PARTY_LICENSES.md.

pub const MAX_VEHICLES: usize = 104;
pub const LMU_DATA_SIZE: usize = 324_820;
pub const SCORING_DATA_OFFSET: usize = 1_632;
pub const TELEMETRY_DATA_OFFSET: usize = 128_464;
pub const SCORING_INFO_SIZE: usize = 548;
pub const VEHICLE_SCORING_SIZE: usize = 584;
pub const VEHICLE_TELEMETRY_SIZE: usize = 1_888;
pub const SCORING_VEHICLES_OFFSET: usize = SCORING_DATA_OFFSET + 560;
pub const TELEMETRY_VEHICLES_OFFSET: usize = TELEMETRY_DATA_OFFSET + 4;

pub const GENERIC_GAME_VERSION: usize = 64;
pub const SCORING_INFO: usize = SCORING_DATA_OFFSET;
pub const TELEMETRY_ACTIVE_VEHICLES: usize = TELEMETRY_DATA_OFFSET;
pub const TELEMETRY_PLAYER_INDEX: usize = TELEMETRY_DATA_OFFSET + 1;
pub const TELEMETRY_PLAYER_HAS_VEHICLE: usize = TELEMETRY_DATA_OFFSET + 2;

pub mod telemetry {
    pub const ID: usize = 0;
    pub const ELAPSED_TIME: usize = 12;
    pub const LAP_NUMBER: usize = 20;
    pub const LAP_START_ET: usize = 24;
    pub const VEHICLE_NAME: usize = 32;
    pub const TRACK_NAME: usize = 96;
    pub const WORLD_POSITION: usize = 160;
    pub const LOCAL_VELOCITY: usize = 184;
    pub const LOCAL_ACCELERATION: usize = 208;
    pub const GEAR: usize = 352;
    pub const RPM: usize = 356;
    pub const WATER_TEMP: usize = 364;
    pub const OIL_TEMP: usize = 372;
    pub const THROTTLE: usize = 388;
    pub const BRAKE: usize = 396;
    pub const STEERING: usize = 404;
    pub const CLUTCH: usize = 412;
    pub const STEERING_TORQUE: usize = 444;
    pub const FUEL: usize = 524;
    pub const MAX_RPM: usize = 532;
    pub const SCHEDULED_STOPS: usize = 540;
    pub const OVERHEATING: usize = 541;
    pub const DETACHED: usize = 542;
    pub const HEADLIGHTS: usize = 543;
    pub const DENT_SEVERITY: usize = 544;
    pub const LAST_IMPACT_ET: usize = 552;
    pub const LAST_IMPACT_MAGNITUDE: usize = 560;
    pub const LAST_IMPACT_POSITION: usize = 568;
    pub const ENGINE_TORQUE: usize = 592;
    pub const CURRENT_SECTOR: usize = 600;
    pub const SPEED_LIMITER: usize = 604;
    pub const FUEL_CAPACITY: usize = 608;
    pub const REAR_BRAKE_BIAS: usize = 664;
    pub const TURBO_BOOST: usize = 672;
    pub const BATTERY_CHARGE_FRACTION: usize = 704;
    pub const ELECTRIC_MOTOR_TORQUE: usize = 712;
    pub const ELECTRIC_MOTOR_RPM: usize = 720;
    pub const ELECTRIC_MOTOR_TEMP: usize = 728;
    pub const ELECTRIC_WATER_TEMP: usize = 736;
    pub const ELECTRIC_MOTOR_STATE: usize = 744;
    pub const LAP_INVALIDATED: usize = 745;
    pub const ABS_ACTIVE: usize = 746;
    pub const TC_ACTIVE: usize = 747;
    pub const SPEED_LIMITER_ACTIVE: usize = 748;
    pub const WIPER_STATE: usize = 749;
    pub const TC: usize = 750;
    pub const TC_MAX: usize = 751;
    pub const TC_SLIP: usize = 752;
    pub const TC_SLIP_MAX: usize = 753;
    pub const TC_CUT: usize = 754;
    pub const TC_CUT_MAX: usize = 755;
    pub const ABS: usize = 756;
    pub const ABS_MAX: usize = 757;
    pub const MOTOR_MAP: usize = 758;
    pub const MOTOR_MAP_MAX: usize = 759;
    pub const MIGRATION: usize = 760;
    pub const MIGRATION_MAX: usize = 761;
    pub const FRONT_ANTI_SWAY: usize = 762;
    pub const FRONT_ANTI_SWAY_MAX: usize = 763;
    pub const REAR_ANTI_SWAY: usize = 764;
    pub const REAR_ANTI_SWAY_MAX: usize = 765;
    pub const LIFT_AND_COAST: usize = 766;
    pub const TRACK_LIMIT_STEPS: usize = 767;
    pub const REGEN: usize = 768;
    pub const STATE_OF_CHARGE: usize = 772;
    pub const VIRTUAL_ENERGY: usize = 776;
    pub const GAP_CAR_AHEAD: usize = 780;
    pub const GAP_CAR_BEHIND: usize = 784;
    pub const GAP_PLACE_AHEAD: usize = 788;
    pub const GAP_PLACE_BEHIND: usize = 792;
    pub const WHEELS: usize = 848;
}

pub mod wheel {
    pub const SIZE: usize = 260;
    pub const SUSPENSION_DEFLECTION: usize = 0;
    pub const RIDE_HEIGHT: usize = 8;
    pub const SUSPENSION_FORCE: usize = 16;
    pub const PRESSURE: usize = 120;
    pub const TEMPERATURE: usize = 128;
    pub const WEAR: usize = 152;
    pub const BRAKE_TEMP: usize = 24;
    pub const BRAKE_PRESSURE: usize = 32;
    pub const ROTATION: usize = 40;
    pub const CAMBER: usize = 80;
    pub const TYRE_LOAD: usize = 104;
    pub const SURFACE_TYPE: usize = 176;
    pub const FLAT: usize = 177;
    pub const DETACHED: usize = 178;
    pub const VERTICAL_DEFLECTION: usize = 180;
    pub const WHEEL_Y_LOCATION: usize = 188;
    pub const TOE: usize = 196;
    pub const CARCASS_TEMP: usize = 204;
    pub const INNER_TEMP: usize = 212;
    pub const OPTIMAL_TEMP: usize = 236;
    pub const COMPOUND_INDEX: usize = 240;
    pub const COMPOUND_TYPE: usize = 241;
}

pub mod scoring {
    pub const SLOT_ID: usize = 0;
    pub const DRIVER_NAME: usize = 4;
    pub const VEHICLE_NAME: usize = 36;
    pub const TOTAL_LAPS: usize = 100;
    pub const SECTOR: usize = 102;
    pub const LAP_DISTANCE: usize = 104;
    pub const FINISH_STATUS: usize = 103;
    pub const BEST_SECTOR1: usize = 128;
    pub const BEST_SECTOR2: usize = 136;
    pub const BEST_LAP: usize = 144;
    pub const LAST_SECTOR1: usize = 152;
    pub const LAST_SECTOR2: usize = 160;
    pub const LAST_LAP: usize = 168;
    pub const CURRENT_SECTOR1: usize = 176;
    pub const CURRENT_SECTOR2: usize = 184;
    pub const NUM_PITSTOPS: usize = 192;
    pub const NUM_PENALTIES: usize = 194;
    pub const IS_PLAYER: usize = 196;
    pub const CONTROL: usize = 197;
    pub const IN_PITS: usize = 198;
    pub const PLACE: usize = 199;
    pub const VEHICLE_CLASS: usize = 200;
    pub const TIME_BEHIND_NEXT: usize = 232;
    pub const LAPS_BEHIND_NEXT: usize = 240;
    pub const TIME_BEHIND_LEADER: usize = 244;
    pub const LAPS_BEHIND_LEADER: usize = 252;
    pub const WORLD_POSITION: usize = 264;
    pub const PIT_STATE: usize = 488;
    pub const FLAG: usize = 504;
    pub const COUNT_LAP_FLAG: usize = 506;
    pub const IN_GARAGE_STALL: usize = 507;
    pub const DRS_STATE: usize = 579;
}

pub mod session {
    pub const TRACK_NAME: usize = 0;
    pub const SESSION: usize = 64;
    pub const CURRENT_ET: usize = 68;
    pub const END_ET: usize = 76;
    pub const MAX_LAPS: usize = 84;
    pub const TRACK_LENGTH: usize = 88;
    pub const VEHICLE_COUNT: usize = 104;
    pub const GAME_PHASE: usize = 108;
    pub const YELLOW_FLAG: usize = 109;
    pub const SECTOR_FLAGS: usize = 110;
    pub const START_LIGHT: usize = 113;
    pub const AMBIENT_TEMP: usize = 228;
    pub const TRACK_TEMP: usize = 236;
    pub const RAINING: usize = 220;
    pub const MIN_WETNESS: usize = 268;
    pub const MAX_WETNESS: usize = 276;
    pub const SESSION_REMAINING: usize = 340;
    pub const TIME_OF_DAY: usize = 344;
    pub const CLOUD_COVERAGE: usize = 350;
    pub const TRACK_GRIP: usize = 348;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_sizes_match_mit_reference() {
        assert_eq!(VEHICLE_TELEMETRY_SIZE, 1_888);
        assert_eq!(VEHICLE_SCORING_SIZE, 584);
        assert_eq!(SCORING_INFO_SIZE, 548);
        assert_eq!(LMU_DATA_SIZE, 324_820);
    }

    #[test]
    fn critical_offsets_match_mit_reference() {
        assert_eq!(telemetry::RPM, 356);
        assert_eq!(telemetry::FUEL, 524);
        assert_eq!(telemetry::WORLD_POSITION, 160);
        assert_eq!(telemetry::LOCAL_VELOCITY, 184);
        assert_eq!(telemetry::ENGINE_TORQUE, 592);
        assert_eq!(telemetry::FUEL_CAPACITY, 608);
        assert_eq!(telemetry::LAP_INVALIDATED, 745);
        assert_eq!(telemetry::WHEELS, 848);
        assert_eq!(wheel::WEAR, 152);
        assert_eq!(wheel::CARCASS_TEMP, 204);
        assert_eq!(wheel::INNER_TEMP, 212);
        assert_eq!(scoring::DRIVER_NAME, 4);
        assert_eq!(scoring::FINISH_STATUS, 103);
        assert_eq!(scoring::WORLD_POSITION, 264);
        assert_eq!(scoring::BEST_LAP, 144);
        assert_eq!(session::CURRENT_ET, 68);
    }
}
