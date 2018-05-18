//! Bluetooth events and event deserialization.
//!
//! This module defines all of the HCI events that can be generated by the Bluetooth controller. In
//! addition to all of the event types, the core functionality of the module is [`parse_event`],
//! which converts a byte buffer into an HCI event.
//!
//! # Future possibilities
//!
//! There are some 76 events defined by the Bluetooth specification (Version 5.0), in addition to
//! vendor events (which would be defined by vendor support crates). That many structs may make this
//! module unwieldly, so this may need to be split up into submodules for different events, similar
//! to the way CommandComplete is split into its own submodule.

#![macro_use]

macro_rules! require_len {
    ($left:expr, $right:expr) => {
        if $left.len() != $right {
            return Err(::event::Error::BadLength($left.len(), $right));
        }
    };
}

macro_rules! require_len_at_least {
    ($left:expr, $right:expr) => {
        if $left.len() < $right {
            return Err(::event::Error::BadLength($left.len(), $right));
        }
    };
}

/// Converts a specific generic enum value between specializations.  This is used below to convert
/// from Error<!> to Error<VendorError> in various places where only one error value is possible
/// (such as from try_into).
macro_rules! self_convert {
    ($val:path) => {
        |e| {
            if let $val(value) = e {
                return $val(value);
            }
            unreachable!();
        }
    };
}

pub mod command;

use byteorder::{ByteOrder, LittleEndian};
use core::convert::{TryFrom, TryInto};
use core::marker::Sized;

/// Potential events that can be generated by the controller.
///
/// See the Bluetooth Spec v4.1, Vol 2, Part E, Section 7.7 for a description of each event.  The
/// events are the same for version 4.2 and 5.0 except where noted.
#[derive(Clone, Debug)]
pub enum Event<V> {
    /// Vol 2, Part E, Section 7.7.3
    ConnectionComplete(ConnectionComplete),

    /// Vol 2, Part E, Section 7.7.5
    DisconnectionComplete(DisconnectionComplete),

    /// Vol 2, Part E, Section 7.7.8
    EncryptionChange(EncryptionChange),

    /// Vol 2, Part E, Section 7.7.12
    ReadRemoteVersionInformationComplete(RemoteVersionInformation),

    /// Vol 2, Part E, Section 7.7.14
    CommandComplete(command::CommandComplete),

    /// Vol 2, Part E, Section 7.7.15
    CommandStatus(CommandStatus),

    /// Vendor-specific events (opcode 0xFF)
    Vendor(V),
}

/// Trait for vendor-specific events.
pub trait VendorEvent {
    /// Enumeration of vendor-specific errors that may occur when deserializing events. Generally,
    /// this means some values in the buffer are out of range for the event.
    type Error;

    /// Creates a new vendor-specific event from the contents of buffer. The buffer contains only
    /// the payload of the event, which does not include the BLE event type (which must be 0xFF) or
    /// the parameter length (which is provided by `buffer.len()`).
    ///
    /// # Errors
    ///
    /// - Shall return one of the appropriate error types (potentially including vendor-specific
    ///   errors) if the buffer does not describe a valid event.
    fn new(buffer: &[u8]) -> Result<Self, Error<Self::Error>>
    where
        Self: Sized;
}

/// Errors that may occur when deserializing an event. Must be specialized by the vendor crate to
/// allow for vendor-specific event errors.
#[derive(Copy, Clone, Debug)]
pub enum Error<V> {
    /// The event type byte was unknown. The byte is provided.
    UnknownEvent(u8),

    /// The buffer provided that is supposed to contain an event does not have the correct
    /// length. Field 0 is the provided length, field 1 is the expected length.
    BadLength(usize, usize),

    /// For all events: The status was not recognized (reserved for future use). Includes the
    /// unrecognized byte.
    BadStatus(u8),

    /// For the ConnectionComplete event: the link type was not recognized (reserved for future
    /// use). Includes the unrecognized byte.
    BadLinkType(u8),

    /// For the ConnectionComplete event: the encryption enabled value was not recognized (reserved
    /// for future use). Includes the unrecognized byte.
    BadEncryptionEnabledValue(u8),

    /// For the DisconnectionComplete event: the disconnection reason was not recognized.  Includes
    /// the unrecognized byte.
    BadReason(u8),

    /// For the EncryptionChange event: The encryption type was not recognized.  Includes the
    /// unrecognized byte.
    BadEncryptionType(u8),

    /// For the Command Complete event: The event indicated a command completed whose opcode was not
    /// recognized. Includes the unrecognized opcode.
    UnknownOpCode(::opcode::OpCode),

    /// A vendor-specific error was detected when deserializing a vendor-specific event.
    Vendor(V),
}

fn rewrap_bad_status<VE>(bad_status: ::BadStatusError) -> Error<VE> {
    let ::BadStatusError::BadValue(v) = bad_status;
    Error::BadStatus(v)
}

fn rewrap_bad_reason<VE>(bad_status: ::BadStatusError) -> Error<VE> {
    let ::BadStatusError::BadValue(v) = bad_status;
    Error::BadReason(v)
}

/// Defines a newtype to indicate that the buffer is supposed to contain an HCI event.
pub struct Packet<'a>(pub &'a [u8]);

impl<'a> Packet<'a> {
    fn full_length(&self) -> usize {
        PACKET_HEADER_LENGTH + self.0[PARAM_LEN_BYTE] as usize
    }
}

const PACKET_HEADER_LENGTH: usize = 2;
const EVENT_TYPE_BYTE: usize = 0;
const PARAM_LEN_BYTE: usize = 1;

impl<VEvent, VError> Event<VEvent>
where
    VEvent: VendorEvent<Error = VError>,
{
    /// Deserializes an event from the given packet. The packet should contain all of the data
    /// needed to deserialize the event.
    ///
    /// # Errors
    ///
    /// - Returns an UnknownEvent error if the first byte of the header is not a recognized event
    ///   type. This includes events that may be valid BLE events, but are not yet be implemented by
    ///   this crate.
    ///
    /// - Returns a BadLength error if the length of the packet is not sufficient to either (1)
    ///   contain a packet header, or (2) contain the packet data as defined by the header.
    ///
    /// - Returns other errors if the particular event cannot be correctly deserialized from the
    ///   packet. This includes vendor-specific errors for vendor events.
    pub fn new(packet: Packet) -> Result<Event<VEvent>, Error<VError>> {
        require_len_at_least!(packet.0, PACKET_HEADER_LENGTH);
        require_len!(packet.0, packet.full_length());

        let event_type = packet.0[EVENT_TYPE_BYTE];
        let payload = &packet.0[PACKET_HEADER_LENGTH..packet.full_length()];
        match event_type {
            0x03 => Ok(Event::ConnectionComplete(to_connection_complete(payload)?)),
            0x05 => Ok(Event::DisconnectionComplete(to_disconnection_complete(
                payload,
            )?)),
            0x08 => Ok(Event::EncryptionChange(to_encryption_change(payload)?)),
            0x0C => Ok(Event::ReadRemoteVersionInformationComplete(
                to_remote_version_info(payload)?,
            )),
            0x0E => Ok(Event::CommandComplete(command::CommandComplete::new(
                payload,
            )?)),
            0x0F => Ok(Event::CommandStatus(CommandStatus::new(payload)?)),
            0xFF => Ok(Event::Vendor(VEvent::new(payload)?)),
            _ => Err(Error::UnknownEvent(event_type)),
        }
    }
}

/// The Connection Complete event indicates to both of the Hosts forming the connection that a new
/// connection has been established. This event also indicates to the Host which issued the
/// connection command and then received a Command Status event, if the issued command failed or was
/// successful.
#[derive(Copy, Clone, Debug)]
pub struct ConnectionComplete {
    /// Result of the connection attempt.
    pub status: ::Status,
    /// Connection_Handle to be used to identify a connection between two BR/ EDR Controllers. The
    /// Connection_Handle is used as an identifier for transmitting and receiving voice or data.
    pub conn_handle: ::ConnectionHandle,
    /// BD_ADDR of the other connected Device forming the connection.
    pub bdaddr: ::BdAddr,
    /// Type of connection.
    pub link_type: LinkType,
    /// True if the connection is encrypted.
    pub encryption_enabled: bool,
}

/// Permissible values for [`ConnectionComplete`] `link_type`
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LinkType {
    /// Synchronous, connection-oriented link
    Sco,
    /// Asynchronous, connection-less link
    Acl,
}

impl TryFrom<u8> for LinkType {
    type Error = Error<!>;

    fn try_from(value: u8) -> Result<LinkType, Self::Error> {
        match value {
            0 => Ok(LinkType::Sco),
            1 => Ok(LinkType::Acl),
            _ => Err(Error::BadLinkType(value)),
        }
    }
}

fn to_connection_complete<VE>(payload: &[u8]) -> Result<ConnectionComplete, Error<VE>> {
    require_len!(payload, 11);

    let mut bdaddr = ::BdAddr([0; 6]);
    bdaddr.0.copy_from_slice(&payload[3..9]);
    Ok(ConnectionComplete {
        status: payload[0].try_into().map_err(rewrap_bad_status)?,
        conn_handle: ::ConnectionHandle(LittleEndian::read_u16(&payload[1..])),
        bdaddr: bdaddr,
        link_type: payload[9]
            .try_into()
            .map_err(self_convert!(Error::BadLinkType))?,
        encryption_enabled: try_into_encryption_enabled(payload[10])
            .map_err(self_convert!(Error::BadEncryptionEnabledValue))?,
    })
}

fn try_into_encryption_enabled(value: u8) -> Result<bool, Error<!>> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::BadEncryptionEnabledValue(value)),
    }
}

/// The Disconnection Complete event occurs when a connection is terminated.
///
/// Note: When a physical link fails, one Disconnection Complete event will be returned for each
/// logical channel on the physical link with the corresponding Connection_Handle as a parameter.
///
/// See the Bluetooth v4.1 spec, Vol 2, Part E, Section 7.7.5.
#[derive(Copy, Clone, Debug)]
pub struct DisconnectionComplete {
    /// Indicates if the disconnection was successful or not.
    pub status: ::Status,

    /// Connection_Handle which was disconnected.
    pub conn_handle: ::ConnectionHandle,

    /// Indicates the reason for the disconnection if the disconnection was successful. If the
    /// disconnection was not successful, the value of the reason parameter can be ignored by the
    /// Host. For example, this can be the case if the Host has issued the Disconnect command and
    /// there was a parameter error, or the command was not presently allowed, or a
    /// Connection_Handle that didn’t correspond to a connection was given.
    pub reason: ::Status,
}

fn to_disconnection_complete<VE>(payload: &[u8]) -> Result<DisconnectionComplete, Error<VE>> {
    require_len!(payload, 4);

    Ok(DisconnectionComplete {
        status: payload[0].try_into().map_err(rewrap_bad_status)?,
        conn_handle: ::ConnectionHandle(LittleEndian::read_u16(&payload[1..])),
        reason: payload[3].try_into().map_err(rewrap_bad_reason)?,
    })
}

/// The Encryption Change event is used to indicate that the change of the encryption mode has been
/// completed.
///
/// This event will occur on both devices to notify the Hosts when Encryption has changed for the
/// specified connection handle between two devices. Note: This event shall not be generated if
/// encryption is paused or resumed; during a role switch, for example.
#[derive(Copy, Clone, Debug)]
pub struct EncryptionChange {
    /// Indicates if the encryption change was successful or not.
    pub status: ::Status,

    /// Connection handle for which the link layer encryption has been enabled/disabled for all
    /// connection handles with the same BR/EDR Controller endpoint as the specified
    /// connection handle.
    ///
    /// The connection handle will be a connection handle for an ACL connection.
    pub conn_handle: ::ConnectionHandle,

    /// Specifies the new encryption type parameter for conn_handle.
    pub encryption: Encryption,
}

/// The meaning of the encryption type depends on whether the Host has indicated support for Secure
/// Connections in the secure connections host support parameter. When secure connections host
/// support is 'disabled' or the connection handle refers to an LE link, the Controller shall only
/// use values 0x00 (OFF) and 0x01 (ON). When secure connections host support is 'enabled' and the
/// connection handle refers to a BR/EDR link, the Controller shall set the encryption type to 0x00
/// when encryption is off, to 0x01 when encryption is on and using E0 and to 0x02 when encryption
/// is on and using AES-CCM.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Encryption {
    /// Encryption is disabled.
    Off,
    /// - On a BR/EDR link, encryption is enabled using E0
    /// - On an LE link, encryption is enabled using AES-CCM
    On,
    /// On a BR/EDR link, encryption is enabled using AES-CCM. Unused for LE links.
    OnAesCcmForBrEdr,
}

impl TryFrom<u8> for Encryption {
    type Error = Error<!>;
    fn try_from(value: u8) -> Result<Encryption, Self::Error> {
        match value {
            0x00 => Ok(Encryption::Off),
            0x01 => Ok(Encryption::On),
            0x02 => Ok(Encryption::OnAesCcmForBrEdr),
            _ => Err(Error::BadEncryptionType(value)),
        }
    }
}

fn to_encryption_change<VE>(payload: &[u8]) -> Result<EncryptionChange, Error<VE>> {
    require_len!(payload, 4);
    Ok(EncryptionChange {
        status: payload[0].try_into().map_err(rewrap_bad_status)?,
        conn_handle: ::ConnectionHandle(LittleEndian::read_u16(&payload[1..])),
        encryption: payload[3]
            .try_into()
            .map_err(self_convert!(Error::BadEncryptionType))?,
    })
}

/// The Read Remote Version Information Complete event is used to indicate the completion of the
/// process obtaining the version information of the remote Controller specified by the
/// conn_handle event parameter.
#[derive(Copy, Clone, Debug)]
pub struct RemoteVersionInformation {
    /// Status of the read event.
    pub status: ::Status,

    /// Connection Handle which is used for the Read_Remote_Version_Information command.  The
    /// connection handle shall be for an ACL connection.
    pub conn_handle: ::ConnectionHandle,

    /// Version of the Current LMP in the remote Controller. See [LMP] version and [Link Layer]
    /// version in the Bluetooth Assigned Numbers.
    ///
    /// - When the connection handle is associated with a BR/EDR ACL-U logical link, the Version
    ///   event parameter shall be LMP version parameter
    ///
    /// - When the connection handle is associated with an LE-U logical link, the Version event
    ///   parameter shall be Link Layer version parameter
    ///
    /// [LMP]: https://www.bluetooth.com/specifications/assigned-numbers/link-manager
    /// [Link Layer]: https://www.bluetooth.com/specifications/assigned-numbers/link-layer
    pub version: u8,

    /// Manufacturer Name of the remote Controller. See [CompId] in the Bluetooth Assigned Numbers.
    ///
    /// [CompId]: https://www.bluetooth.com/specifications/assigned-numbers/company-identifiers
    pub mfgr_name: u16,

    /// Subversion of the LMP in the remote Controller. See the Bluetooth Spec, v4.1, Vol 2, Part C,
    /// Table 5.2 and Vol 6, Part B, Section 2.4.2.13(SubVersNr).  The sections are the same in v4.2
    /// and v5.0 of the spec.
    ///
    /// SubVersNr field shall contain a unique value for each implementation or revision of an
    /// implementation of the Bluetooth Controller.
    ///
    /// The meaning of the subversion is implementation-defined.
    pub subversion: u16,
}

fn to_remote_version_info<VE>(payload: &[u8]) -> Result<RemoteVersionInformation, Error<VE>> {
    require_len!(payload, 8);
    Ok(RemoteVersionInformation {
        status: payload[0].try_into().map_err(rewrap_bad_status)?,
        conn_handle: ::ConnectionHandle(LittleEndian::read_u16(&payload[1..])),
        version: payload[3],
        mfgr_name: LittleEndian::read_u16(&payload[4..]),
        subversion: LittleEndian::read_u16(&payload[6..]),
    })
}

/// The Command Status event. This event is generated to indicate that an asynchronous operation has
/// begun (or could not begin).
///
/// Defined in Vol 2, Part E, Section 7.7.15 of the spec.
#[derive(Copy, Clone, Debug)]
pub struct CommandStatus {
    /// Status of the command that has started.
    pub status: ::Status,

    /// Number of HCI Command packets that can be sent to the controller from the host.
    pub num_hci_command_packets: u8,

    /// Opcode of the command that generated this CommandStatus event. The controller can generate a
    /// spontaneous CommandStatus with opcode 0 if the number of allowed HCI commands has changed.
    pub op_code: ::opcode::OpCode,
}

impl CommandStatus {
    const LENGTH: usize = 4;

    fn new<VE>(buffer: &[u8]) -> Result<CommandStatus, Error<VE>> {
        if buffer.len() != Self::LENGTH {
            return Err(Error::BadLength(buffer.len(), Self::LENGTH));
        }

        Ok(CommandStatus {
            status: buffer[0].try_into().map_err(rewrap_bad_status)?,
            num_hci_command_packets: buffer[1],
            op_code: ::opcode::OpCode(LittleEndian::read_u16(&buffer[2..])),
        })
    }
}
