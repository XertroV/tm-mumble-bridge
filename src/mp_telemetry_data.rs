/*
C Headers file:

enum {
    ECurVersion = 3,
};

typedef unsigned int Nat32;
typedef unsigned int Bool;

struct Vec3 {
    float x,y,z;
};
struct Quat {
    float w,x,y,z;
};

struct STelemetry {
    struct SHeader {
        char        Magic[32];              //  "ManiaPlanet_Telemetry"
        Nat32       Version;
        Nat32       Size;                   // == sizeof(STelemetry)
    };
    enum EGameState {
        EState_Starting = 0,
        EState_Menus,
        EState_Running,
        EState_Paused,
    };
    enum ERaceState {
        ERaceState_BeforeState = 0,
        ERaceState_Running,
        ERaceState_Finished,
    };
    struct SGameState {
        EGameState  State;
        char        GameplayVariant[64];    // player model 'StadiumCar', 'CanyonCar', ....
        char        MapId[64];
        char        MapName[256];
        char        __future__[128];
    };
    struct SRaceState {
        ERaceState  State;
        Nat32       Time;
        Nat32       NbRespawns;
        Nat32       NbCheckpoints;
        Nat32       CheckpointTimes[125];
        Nat32       NbCheckpointsPerLap;
        Nat32       NbLapsPerRace;
        Nat32       Timestamp;
        Nat32       StartTimestamp;         // timestamp when the State will change to 'Running', or has changed when after the racestart.
        char        __future__[16];
    };
    struct SObjectState {
        Nat32       Timestamp;
        Nat32       DiscontinuityCount;     // the number changes everytime the object is moved not continuously (== teleported).
        Quat        Rotation;
        Vec3        Translation;            // +x is "left", +y is "up", +z is "front"
        Vec3        Velocity;               // (world velocity)
        Nat32       LatestStableGroundContactTime;
        char        __future__[32];
    };
    struct SVehicleState {
        Nat32       Timestamp;

        float       InputSteer;
        float       InputGasPedal;
        Bool        InputIsBraking;
        Bool        InputIsHorn;

        float       EngineRpm;              // 1500 -> 10000
        int         EngineCurGear;
        float       EngineTurboRatio;       // 1 turbo starting/full .... 0 -> finished
        Bool        EngineFreeWheeling;

        Bool        WheelsIsGroundContact[4];
        Bool        WheelsIsSliping[4];
        float       WheelsDamperLen[4];
        float       WheelsDamperRangeMin;
        float       WheelsDamperRangeMax;

        float       RumbleIntensity;

        Nat32       SpeedMeter;             // unsigned km/h
        Bool        IsInWater;
        Bool        IsSparkling;
        Bool        IsLightTrails;
        Bool        IsLightsOn;
        Bool        IsFlying;               // long time since touching ground.
        Bool        IsOnIce;

        Nat32       Handicap;               // bit mask: [reserved..] [NoGrip] [NoSteering] [NoBrakes] [EngineForcedOn] [EngineForcedOff]
        float       BoostRatio;             // 1 thrusters starting/full .... 0 -> finished

        char        __future__[20];
    };
    struct SDeviceState {   // VrChair state.
        Vec3        Euler;                  // yaw, pitch, roll  (order: pitch, roll, yaw)
        float       CenteredYaw;            // yaw accumulated + recentered to apply onto the device
        float       CenteredAltitude;       // Altitude accumulated + recentered

        char        __future__[32];
    };

    struct SPlayerState {
        Bool        IsLocalPlayer;          // Is the locally controlled player, or else it is a remote player we're spectating, or a replay.
        char        Trigram[4];             // 'TMN'
        char        DossardNumber[4];       // '01'
        float       Hue;
        char        UserName[256];
        char        __future__[28];
    };

    SHeader         Header;

    Nat32           UpdateNumber;
    SGameState      Game;
    SRaceState      Race;
    SObjectState    Object;
    SVehicleState   Vehicle;
    SDeviceState    Device;
    SPlayerState    Player;
};


*/

use std::fmt::{Debug, Formatter, self};

use cgmath::Vector3;


#[allow(unused)]
const MP_T_VERSION: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Debug for Vec3 {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<{:.3}, {:.3}, {:.3}>", self.x, self.y, self.z)
    }
}

impl From<Vec3> for [f32; 3] {
    fn from(v: Vec3) -> Self {
        [v.x, v.y, v.z]
    }
}

impl From<Vec3> for Vector3<f32> {
    fn from(v: Vec3) -> Self {
        Vector3::new(v.x, v.y, v.z)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Quat {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Debug for Quat {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<{:.3}, {:.3}, {:.3}, {:.3}>", self.w, self.x, self.y, self.z)
    }
}

impl From<Quat> for cgmath::Quaternion<f32> {
    fn from(q: Quat) -> Self {
        cgmath::Quaternion::new(q.w, q.x, q.y, q.z)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SHeader {
    pub magic: [u8; 32],
    pub version: u32,
    pub size: u32,
}

impl Debug for SHeader {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "SHeader {{ magic: {:?},
        version: {},
        size: {} }}",
               String::from_utf8_lossy(&self.magic.as_cstr_vec()), self.version, self.size)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SGameState {
    pub state: u32,
    pub gameplay_variant: [u8; 64],
    pub map_id: [u8; 64],
    pub map_name: [u8; 256],
    pub future: [u8; 128],
}

impl Debug for SGameState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "SGameState {{ state: {},
        gameplay_variant: {:?},
        map_id: {:?},
        map_name: {:?} }}",
               self.state, String::from_utf8_lossy(&self.gameplay_variant.as_cstr_vec()), String::from_utf8_lossy(&self.map_id.as_cstr_vec()), String::from_utf8_lossy(&self.map_name.as_cstr_vec()))
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SRaceState {
    pub state: u32,
    pub time: u32,
    pub nb_respawns: u32,
    pub nb_checkpoints: u32,
    pub checkpoint_times: [u32; 125],
    pub nb_checkpoints_per_lap: u32,
    pub nb_laps_per_race: u32,
    pub timestamp: u32,
    pub start_timestamp: u32,
    pub future: [u8; 16],
}


impl Debug for SRaceState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "SRaceState {{ state: {},
        time: {},
        nb_respawns: {},
        nb_checkpoints: {},
        checkpoint_times: {:?},
        nb_checkpoints_per_lap: {},
        nb_laps_per_race: {},
        timestamp: {},
        start_timestamp: {} }}",
               self.state, self.time, self.nb_respawns, self.nb_checkpoints, &self.checkpoint_times[..self.nb_checkpoints as usize], self.nb_checkpoints_per_lap, self.nb_laps_per_race, self.timestamp, self.start_timestamp)
    }
}


#[repr(C)]
#[derive(Clone, Copy)]
pub struct SObjectState {
    pub timestamp: u32,
    pub discontinuity_count: u32,
    pub rotation: Quat,
    pub translation: Vec3,
    pub velocity: Vec3,
    pub latest_stable_ground_contact_time: u32,
    pub future: [u8; 32],
}

impl Debug for SObjectState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "SObjectState {{ timestamp: {},
        discontinuity_count: {},
        rotation: {:?},
        translation: {:?},
        velocity: {:?},
        latest_stable_ground_contact_time: {} }}",
               self.timestamp, self.discontinuity_count, self.rotation, self.translation, self.velocity, self.latest_stable_ground_contact_time)
    }
}



#[repr(C)]
#[derive(Clone, Copy)]
pub struct SVehicleState {
    pub timestamp: u32,
    pub input_steer: f32,
    pub input_gas_pedal: f32,
    pub input_is_braking: u32,
    pub input_is_horn: u32,
    pub engine_rpm: f32,
    pub engine_cur_gear: i32,
    pub engine_turbo_ratio: f32,
    pub engine_freewheeling: u32,
    pub wheels_is_ground_contact: [u32; 4],
    pub wheels_is_slipping: [u32; 4],
    pub wheels_damper_len: [f32; 4],
    pub wheels_damper_range_min: f32,
    pub wheels_damper_range_max: f32,
    pub rumble_intensity: f32,
    pub speed_meter: u32,
    pub is_in_water: u32,
    pub is_sparkling: u32,
    pub is_light_trails: u32,
    pub is_lights_on: u32,
    pub is_flying: u32,
    pub is_on_ice: u32,
    pub handicap: u32,
    pub boost_ratio: f32,
    pub future: [u8; 20],
}

impl Debug for SVehicleState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "SVehicleState {{ timestamp: {},
        input_steer: {},
        input_gas_pedal: {},
        input_is_braking: {},
        input_is_horn: {},
        engine_rpm: {},
        engine_cur_gear: {},
        engine_turbo_ratio: {},
        engine_freewheeling: {},
        wheels_is_ground_contact: {:?},
        wheels_is_slipping: {:?},
        wheels_damper_len: [{:.3}, {:.3}, {:.3}, {:.3}],
        wheels_damper_range_min: {},
        wheels_damper_range_max: {},
        rumble_intensity: {},
        speed_meter: {},
        is_in_water: {},
        is_sparkling: {},
        is_light_trails: {},
        is_lights_on: {},
        is_flying: {},
        is_on_ice: {},
        handicap: {},
        boost_ratio: {} }}",
               self.timestamp, self.input_steer, self.input_gas_pedal, self.input_is_braking, self.input_is_horn, self.engine_rpm, self.engine_cur_gear, self.engine_turbo_ratio, self.engine_freewheeling, &self.wheels_is_ground_contact[..], &self.wheels_is_slipping[..],
               self.wheels_damper_len[0], self.wheels_damper_len[1], self.wheels_damper_len[2], self.wheels_damper_len[3], self.wheels_damper_range_min, self.wheels_damper_range_max, self.rumble_intensity, self.speed_meter, self.is_in_water, self.is_sparkling, self.is_light_trails, self.is_lights_on, self.is_flying, self.is_on_ice, self.handicap, self.boost_ratio)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SDeviceState {
    pub euler: Vec3,
    pub centered_yaw: f32,
    pub centered_altitude: f32,
    pub future: [u8; 32],
}

impl Debug for SDeviceState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "SDeviceState {{ euler: {:?},
        centered_yaw: {:.3},
        centered_altitude: {:.3} }}",
               self.euler, self.centered_yaw, self.centered_altitude)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SPlayerState {
    pub is_local_player: u32,
    pub trigram: [u8; 4],
    pub dossard_number: [u8; 4],
    pub hue: f32,
    pub user_name: [u8; 256],
    pub future: [u8; 28],
}

impl Debug for SPlayerState {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "SPlayerState {{ is_local_player: {},
        trigram: {:?},
        dossard_number: {:?},
        hue: {},
        user_name: {:?} }}",
               self.is_local_player, String::from_utf8_lossy(&self.trigram.as_cstr_vec()), String::from_utf8_lossy(&self.dossard_number.as_cstr_vec()), self.hue, String::from_utf8_lossy(&self.user_name.as_cstr_vec()))
    }
}



#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct STelemetry {
    pub header: SHeader,
    pub update_number: u32,
    pub game: SGameState,
    pub race: SRaceState,
    pub object: SObjectState,
    pub vehicle: SVehicleState,
    pub device: SDeviceState,
    pub player: SPlayerState,
}



pub trait AsCStrSlice {
    fn as_cstr_vec(&self) -> Vec<u8>;
}

impl AsCStrSlice for [u8; 4] {
    fn as_cstr_vec(&self) -> Vec<u8> {
        self.iter().take_while(|c| *c != &0).cloned().collect::<Vec<_>>()
    }
}

impl AsCStrSlice for [u8; 256] {
    fn as_cstr_vec(&self) -> Vec<u8> {
        self.iter().take_while(|c| *c != &0).cloned().collect::<Vec<_>>()
    }
}

impl AsCStrSlice for [u8; 64] {
    fn as_cstr_vec(&self) -> Vec<u8> {
        self.iter().take_while(|c| *c != &0).cloned().collect::<Vec<_>>()
    }
}

impl AsCStrSlice for [u8; 32] {
    fn as_cstr_vec(&self) -> Vec<u8> {
        self.iter().take_while(|c| *c != &0).cloned().collect::<Vec<_>>()
    }
}

impl AsCStrSlice for [u8; 16] {
    fn as_cstr_vec(&self) -> Vec<u8> {
        self.iter().take_while(|c| *c != &0).cloned().collect::<Vec<_>>()
    }
}

impl AsCStrSlice for [u8; 128] {
    fn as_cstr_vec(&self) -> Vec<u8> {
        self.iter().take_while(|c| *c != &0).cloned().collect::<Vec<_>>()
    }
}

impl AsCStrSlice for [u8; 20] {
    fn as_cstr_vec(&self) -> Vec<u8> {
        self.iter().take_while(|c| *c != &0).cloned().collect::<Vec<_>>()
    }
}
