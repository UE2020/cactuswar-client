#![allow(dead_code)]
#![allow(non_upper_case_globals)]
use crate::binary;
use crate::do_error_log;
use crate::do_info_log;
use crate::engine;
use crate::util;
use crate::wrapper;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::collections::HashMap;

/// This trait allows objects that implement it to be sent (and recieved!) over websockets.
pub trait Protocol {
    /// Pack the struct into a `binary::StreamPeerBuffer`.
    ///
    /// `encode` will consume its host object.
    fn encode(&self) -> binary::StreamPeerBuffer;

    /// An associated function that takes a `binary::StreamPeerBuffer` and returns an instance
    /// of `Self`
    fn decode(buf: binary::StreamPeerBuffer) -> Self;

    /// The id of the packet
    ///
    /// See also: `protocol::Packet`
    const id: u8;
}

/// A type of packet.
/// Examples include:
/// * Census
/// * Init
/// * Input
/// * Upgrade
#[derive(FromPrimitive)]
pub enum Packet {
    Init = 0,
    Input = 1,
    Census = 2,
    Handshake = 3,
    Message = 4,
    Respawn = 6,
    Death = 5,
    Leaderboard = 7,
}

/// Packet that registers the player with the server.
pub struct InitPacket {
    pub name: String,
}

impl Protocol for InitPacket {
    fn encode(&self) -> binary::StreamPeerBuffer {
        let mut buf = binary::StreamPeerBuffer::new();
        buf.put_u8(Self::id);
        buf.put_utf8(self.name.as_str());
        buf
    }

    const id: u8 = Packet::Init as u8;

    fn decode(_: binary::StreamPeerBuffer) -> Self {
        unimplemented!()
    }
}

/// Packet that sends chat messages
pub struct MessagePacket {
    pub message: String,
}

impl Protocol for MessagePacket {
    fn encode(&self) -> binary::StreamPeerBuffer {
        let mut buf = binary::StreamPeerBuffer::new();
        buf.put_u8(Self::id);
        buf.put_utf8(self.message.as_str());
        buf
    }

    const id: u8 = Packet::Message as u8;

    fn decode(_: binary::StreamPeerBuffer) -> Self {
        unimplemented!()
    }
}

/// Packet that informs the server about which keys are down.
/// Bitflags are used to pack every key into a u8.
///
/// See also: https://diep.io
#[allow(non_snake_case)]
pub struct InputPacket {
    pub W: bool,
    pub A: bool,
    pub S: bool,
    pub D: bool,
    pub mouse_down: bool,
    pub mouse_position: util::Vector2<i16>,
}

impl Protocol for InputPacket {
    fn encode(&self) -> binary::StreamPeerBuffer {
        let mut buf = binary::StreamPeerBuffer::new();
        buf.put_u8(Self::id);

        // Send the directional keys using bitflags.
        let w = if self.W { 0b10000 } else { 0b00000 };
        let a = if self.A { 0b01000 } else { 0b00000 };
        let s = if self.S { 0b00100 } else { 0b00000 };
        let d = if self.D { 0b00010 } else { 0b00000 };
        let mouse_down = if self.mouse_down { 0b00001 } else { 0b00000 };

        buf.put_u8(w | a | s | d | mouse_down);

        buf.put_16(self.mouse_position.x);
        buf.put_16(self.mouse_position.y);

        buf
    }

    const id: u8 = Packet::Input as u8;

    fn decode(_: binary::StreamPeerBuffer) -> Self {
        unimplemented!()
    }
}

impl InputPacket {
    /// Create an InputPacket using an instance of `engine::Input`.
    pub fn from_input(input: engine::Input) -> Self {
        Self {
            W: input.W,
            A: input.A,
            S: input.S,
            D: input.D,
            mouse_down: input.mouse_down,
            mouse_position: input.mouse_position,
        }
    }
}

/// ## Base
/// A census parser. Optimizations could be made by fixing the spb reader, but that's not a high prioirity.
///
/// * Id (u8)
/// * Entities (u16)
///
/// ## Shape
///
/// * Id (u8)
/// * Game Id (u32) (1)
/// * Position (i16, i16)
/// * Health (float)
///
/// ## Tank
///
/// * Id (u8)
/// * Game Id (u32) (0)
/// * Position (i16, i16)
/// * Rotation (f32)
/// * Velocity (i16, i16)
/// * Mockup (u8)
///
/// ## Bullet
///
/// * Id (u8)
/// * Game Id (u32) (1)
/// * Position (i16, i16)
/// * Radius (u16)
/// * Velocity (i16, i16)
#[derive(Debug)]
pub struct Census {
    pub entity_count: u16,
    pub arena_size: u16,

    // player data
    pub level: f32,

    pub entities: HashMap<u32, Entity>, // A protocol entity, not an engine entity.
}

/// Represents the structure of a `Tank` when packed into a `Census`.
#[derive(Debug)]
pub struct TankPacket {
    pub id: u32,
    pub position: util::Vector2<i16>,
    pub rotation: f32,
    pub velocity: util::Vector2<i16>,
    pub mockup: u8,
    pub health: f32,
    pub radius: u16,
    pub name: String,
    pub message: String,
}

/// Represents the structure of a `Shape` when packed into a `Census`.
#[derive(Debug)]
pub struct ShapePacket {
    pub id: u32,
    pub position: util::Vector2<i16>,
    pub health: f32,
    pub radius: u16,
}

/// Represents the structure of a `Bullet` when packed into a `Census`.
#[derive(Debug)]
pub struct BulletPacket {
    pub id: u32,
    pub position: util::Vector2<i16>,
    pub radius: u16,
    pub velocity: util::Vector2<i16>,
    pub owner: u32,
}

/// Represents an entity id packed into a `Census`.
#[derive(FromPrimitive)]
pub enum EntityType {
    Tank = 0,
    Shape = 1,
    Bullet = 2,
}

#[derive(Debug)]
pub enum Entity {
    Tank(TankPacket),
    Shape(ShapePacket),
    Bullet(BulletPacket),
}

impl Protocol for Census {
    fn encode(&self) -> binary::StreamPeerBuffer {
        unimplemented!()
    }

    const id: u8 = Packet::Census as u8;

    fn decode(mut buf: binary::StreamPeerBuffer) -> Self {
        let entity_count = buf.get_u16();
        let arena_size = buf.get_u16();
        let level = buf.get_float();
        let mut entities = HashMap::new();
        for _ in 0..entity_count {
            match FromPrimitive::from_u8(buf.get_u8()) {
                Some(EntityType::Tank) => {
                    let game_id = buf.get_u32();
                    entities.insert(
                        game_id,
                        Entity::Tank(TankPacket {
                            id: game_id,
                            position: util::Vector2 {
                                x: buf.get_16(),
                                y: buf.get_16(),
                            },
                            rotation: buf.get_float(),
                            velocity: util::Vector2 {
                                x: buf.get_16(),
                                y: buf.get_16(),
                            },
                            mockup: buf.get_u8(),
                            health: {
                                let health = buf.get_float();
                                if health < 0. {
                                    0.
                                } else { health } 
                            },
                            radius: buf.get_u16(),
                            name: buf.get_utf8(),
                            message: buf.get_utf8()
                        }),
                    );
                }

                Some(EntityType::Shape) => {
                    let game_id = buf.get_u32();
                    entities.insert(
                        game_id,
                        Entity::Shape(ShapePacket {
                            id: game_id,
                            position: util::Vector2 {
                                x: buf.get_16(),
                                y: buf.get_16(),
                            },
                            health: buf.get_float(),
                            radius: buf.get_u16(),
                        }),
                    );
                }

                Some(EntityType::Bullet) => {
                    let game_id = buf.get_u32();
                    entities.insert(
                        game_id,
                        Entity::Bullet(BulletPacket {
                            id: game_id,
                            position: util::Vector2 {
                                x: buf.get_16(),
                                y: buf.get_16(),
                            },
                            radius: buf.get_u16(),
                            velocity: util::Vector2 {
                                x: buf.get_16(),
                                y: buf.get_16(),
                            },
                            owner: buf.get_u32(),
                        }),
                    );
                }

                None => do_error_log!("Fatal Error: Unknown entity with id {}", buf.get_u32()),
            }
            //entities.insert(buf.get_u32(), );
        }
        Self {
            entity_count,
            level,
            arena_size,
            entities,
        }
    }
}

/// Packet that acknowledges `InitPacket`.
#[derive(Debug)]
pub struct HandshakePacket {
    pub id: u32,
    pub mockups: Vec<TankMockup>,
}

/// Represents Barrel as packed into HandshakePacket
#[derive(Debug)]
pub struct BarrelMockup {
    pub width: f32,
    pub length: f32,
    pub angle: f32,
}

#[derive(Debug)]
pub struct TankMockup {
    pub name: String,
    pub fov: u8,
    pub barrels: Vec<BarrelMockup>,
}

impl Protocol for HandshakePacket {
    fn encode(&self) -> binary::StreamPeerBuffer {
        unimplemented!();
    }

    const id: u8 = Packet::Handshake as u8;

    fn decode(mut buf: binary::StreamPeerBuffer) -> Self {
        let my_id = buf.get_u32();
        let mockup_count = buf.get_u8();
        let mut mockups = vec![];

        for _ in 0..mockup_count {
            let name = buf.get_utf8();
            let fov = buf.get_u8();
            let barrel_count = buf.get_u8();
            let mut barrels = vec![];
            for _ in 0..barrel_count {
                barrels.push(BarrelMockup {
                    width: buf.get_float(),
                    length: buf.get_float(),
                    angle: buf.get_float(),
                });
            }
            mockups.push(TankMockup { name, fov, barrels });
        }

        Self { id: my_id, mockups }
    }
}

#[derive(Debug)]
pub struct RespawnPacket;

impl Protocol for RespawnPacket {
    fn encode(&self) -> binary::StreamPeerBuffer {
        let mut buf = binary::StreamPeerBuffer::new();
        buf.put_u8(Self::id as u8);
        buf
    }

    const id: u8 = Packet::Respawn as u8;

    fn decode(_buf: binary::StreamPeerBuffer) -> Self {
        unimplemented!()
    }
}


#[derive(Debug)]
pub struct DeathPacket {
    pub time_alive: f64
}

impl Protocol for DeathPacket {
    fn encode(&self) -> binary::StreamPeerBuffer {
        unimplemented!()
    }

    const id: u8 = Packet::Death as u8;

    fn decode(mut buf: binary::StreamPeerBuffer) -> Self {
        Self {
            time_alive: buf.get_double(),
        }
    }
}

#[derive(Debug)]
pub struct LeaderboardPacket {
    pub entries: Vec<LeaderboardEntry>
}

#[derive(Debug)]
pub struct LeaderboardEntry {
    pub name: String,
    pub level: f32,
    pub mockup: u8,
}

impl Protocol for LeaderboardPacket {
    fn encode(&self) -> binary::StreamPeerBuffer {
        unimplemented!()
    }

    const id: u8 = Packet::Leaderboard as u8;

    fn decode(mut buf: binary::StreamPeerBuffer) -> Self {
        let count = buf.get_u8();
        let mut leaderboard = vec![];
        for _ in 0..count {
            leaderboard.push(LeaderboardEntry {
                name: buf.get_utf8(),
                level: buf.get_float(),
                mockup: buf.get_u8(),
            });
        }
        Self {
            entries: leaderboard,
        }
    }
}