extern crate bluetooth_hci as hci;
extern crate nb;

use hci::host::*;
use std::time::Duration;

struct RecordingSink {
    written_data: Vec<u8>,
}

#[derive(Debug, PartialEq)]
struct RecordingSinkError;

impl hci::Controller for RecordingSink {
    type Error = RecordingSinkError;

    fn write(&mut self, header: &[u8], payload: &[u8]) -> nb::Result<(), Self::Error> {
        self.written_data.resize(header.len() + payload.len(), 0);
        {
            let (h, p) = self.written_data.split_at_mut(header.len());
            h.copy_from_slice(header);
            p.copy_from_slice(payload);
        }
        Ok(())
    }

    fn read_into(&mut self, _buffer: &mut [u8]) -> nb::Result<(), Self::Error> {
        Err(nb::Error::Other(RecordingSinkError {}))
    }

    fn peek(&mut self, _n: usize) -> nb::Result<u8, Self::Error> {
        Err(nb::Error::Other(RecordingSinkError {}))
    }
}

impl RecordingSink {
    fn new() -> RecordingSink {
        RecordingSink {
            written_data: Vec::new(),
        }
    }

    fn as_controller(&mut self) -> &mut Hci<RecordingSinkError, uart::CommandHeader> {
        self as &mut Hci<RecordingSinkError, uart::CommandHeader>
    }
}

#[test]
fn disconnect() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .disconnect(hci::ConnectionHandle(0x0201), hci::Status::AuthFailure)
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x06, 0x04, 3, 0x01, 0x02, 0x05]);
}

#[test]
fn disconnect_bad_reason() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .disconnect(hci::ConnectionHandle(0x0201), hci::Status::UnknownCommand)
        .err()
        .unwrap();
    assert_eq!(
        err,
        nb::Error::Other(Error::BadDisconnectionReason(hci::Status::UnknownCommand))
    );
    assert_eq!(sink.written_data, []);
}

#[test]
fn read_remote_version_information() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .read_remote_version_information(hci::ConnectionHandle(0x0201))
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x1D, 0x04, 2, 0x01, 0x02]);
}

#[test]
fn set_event_mask() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .set_event_mask(EventFlags::INQUIRY_COMPLETE | EventFlags::AUTHENTICATION_COMPLETE)
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x01, 0x0C, 8, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn reset() {
    let mut sink = RecordingSink::new();
    sink.as_controller().reset().unwrap();
    assert_eq!(sink.written_data, [1, 0x03, 0x0C, 0]);
}

#[test]
fn read_tx_power_level() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .read_tx_power_level(hci::ConnectionHandle(0x0201), TxPowerLevel::Current)
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x2D, 0x0C, 3, 0x01, 0x02, 0x00])
}

#[test]
fn read_local_version_information() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .read_local_version_information()
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x01, 0x10, 0])
}

#[test]
fn read_local_supported_commands() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .read_local_supported_commands()
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x02, 0x10, 0]);
}

#[test]
fn read_local_supported_features() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .read_local_supported_features()
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x03, 0x10, 0]);
}

#[test]
fn read_bd_addr() {
    let mut sink = RecordingSink::new();
    sink.as_controller().read_bd_addr().unwrap();
    assert_eq!(sink.written_data, [1, 0x09, 0x10, 0]);
}

#[test]
fn read_rssi() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .read_rssi(hci::ConnectionHandle(0x0201))
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x05, 0x14, 2, 0x01, 0x02]);
}

#[test]
fn le_set_event_mask() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_event_mask(
            LeEventFlags::CONNECTION_COMPLETE | LeEventFlags::REMOTE_CONNECTION_PARAMETER_REQUEST,
        )
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x01, 0x20, 8, 0x21, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn le_read_buffer_size() {
    let mut sink = RecordingSink::new();
    sink.as_controller().le_read_buffer_size().unwrap();
    assert_eq!(sink.written_data, [1, 0x02, 0x20, 0]);
}

#[test]
fn le_read_local_supported_features() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_read_local_supported_features()
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x03, 0x20, 0]);
}

#[test]
fn le_set_random_address() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_random_address(hci::BdAddr([0x01, 0x02, 0x04, 0x08, 0x10, 0x20]))
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x05, 0x20, 6, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20]
    );
}

#[test]
fn le_set_random_address_invalid_addr_type() {
    let mut sink = RecordingSink::new();
    for bd_addr in [
        // The most significant bits of the BD ADDR must be either 11 (static address) or 00
        // (non-resolvable private address), or 10 (resolvable private address).  An MSB of 01 is
        // not valid.
        hci::BdAddr([0x01, 0x02, 0x04, 0x08, 0x10, 0b01000000]),
        // The random part of a static address must contain at least one 0.
        hci::BdAddr([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
        // The random part of a static address must contain at least one 1.
        hci::BdAddr([0x00, 0x00, 0x00, 0x00, 0x00, 0b11000000]),
        // The random part of a non-resolvable private address must contain at least one 0.
        hci::BdAddr([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0b00111111]),
        // The random part of a non-resolvable private address must contain at least one 1.
        hci::BdAddr([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        // The random part of a resolvable private address must contain at least one 0.  The first 3
        // bytes are a hash, which can have any value.
        hci::BdAddr([0x01, 0x02, 0x04, 0xFF, 0xFF, 0b10111111]),
        // The random part of a resolvable private address must contain at least one 1.  The first 3
        // bytes are a hash, which can have any value.
        hci::BdAddr([0x01, 0x02, 0x04, 0x00, 0x00, 0b10000000]),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_set_random_address(*bd_addr)
            .err()
            .unwrap();
        assert_eq!(err, nb::Error::Other(Error::BadRandomAddress(*bd_addr)));
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_set_advertising_parameters() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_advertising_parameters(&AdvertisingParameters {
            advertising_interval: (Duration::from_millis(21), Duration::from_millis(1000)),
            advertising_type: AdvertisingType::ConnectableUndirected,
            own_address_type: OwnAddressType::Public,
            peer_address: hci::BdAddrType::Random(hci::BdAddr([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            ])),
            advertising_channel_map: Channels::CH_37 | Channels::CH_39,
            advertising_filter_policy: AdvertisingFilterPolicy::AllowConnectionAndScan,
        })
        .unwrap();
    assert_eq!(
        sink.written_data,
        [
            1,
            0x06,
            0x20,
            15,
            0x21, // 0x21, 0x00 = 0x0021 = 33 ~= 21 ms / 0.625 ms
            0x00,
            0x40, // 0x40, 0x06 = 0x0640 = 1600 = 1000 ms / 0.625 ms
            0x06,
            0x00,
            0x00,
            0x01,
            0x01,
            0x02,
            0x03,
            0x04,
            0x05,
            0x06,
            0b0000_0101,
            0x00
        ]
    );
}

#[test]
fn le_set_advertising_parameters_bad_range() {
    let mut sink = RecordingSink::new();
    for (min, max) in [
        (Duration::from_millis(19), Duration::from_millis(1000)),
        (Duration::from_millis(100), Duration::from_millis(10241)),
        (Duration::from_millis(500), Duration::from_millis(499)),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_set_advertising_parameters(&AdvertisingParameters {
                advertising_interval: (*min, *max),
                advertising_type: AdvertisingType::ConnectableUndirected,
                own_address_type: OwnAddressType::Random,
                peer_address: hci::BdAddrType::Random(hci::BdAddr([
                    0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                ])),
                advertising_channel_map: Channels::CH_37 | Channels::CH_39,
                advertising_filter_policy: AdvertisingFilterPolicy::AllowConnectionAndScan,
            })
            .err()
            .unwrap();
        assert_eq!(
            err,
            nb::Error::Other(Error::BadAdvertisingInterval(*min, *max))
        );
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_set_advertising_parameters_bad_channel_map() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .le_set_advertising_parameters(&AdvertisingParameters {
            advertising_interval: (Duration::from_millis(20), Duration::from_millis(1000)),
            advertising_type: AdvertisingType::ConnectableUndirected,
            own_address_type: OwnAddressType::Public,
            peer_address: hci::BdAddrType::Random(hci::BdAddr([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            ])),
            advertising_channel_map: Channels::empty(),
            advertising_filter_policy: AdvertisingFilterPolicy::AllowConnectionAndScan,
        })
        .err()
        .unwrap();
    assert_eq!(
        err,
        nb::Error::Other(Error::BadChannelMap(Channels::empty()))
    );
    assert_eq!(sink.written_data, []);
}

#[cfg(not(feature = "version-5-0"))]
#[test]
fn le_set_advertising_parameters_bad_higher_min() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .le_set_advertising_parameters(&AdvertisingParameters {
            advertising_interval: (Duration::from_millis(99), Duration::from_millis(1000)),
            advertising_type: AdvertisingType::ScannableUndirected,
            own_address_type: OwnAddressType::Random,
            peer_address: hci::BdAddrType::Random(hci::BdAddr([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            ])),
            advertising_channel_map: Channels::all(),
            advertising_filter_policy: AdvertisingFilterPolicy::AllowConnectionAndScan,
        })
        .err()
        .unwrap();
    assert_eq!(
        err,
        nb::Error::Other(Error::BadAdvertisingIntervalMin(
            Duration::from_millis(99),
            AdvertisingType::ScannableUndirected
        ))
    );
    assert_eq!(sink.written_data, []);
}

#[cfg(feature = "version-5-0")]
#[test]
fn le_set_advertising_parameters_ok_no_higher_min() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_advertising_parameters(&AdvertisingParameters {
            advertising_interval: (Duration::from_millis(99), Duration::from_millis(1000)),
            advertising_type: AdvertisingType::ScannableUndirected,
            own_address_type: OwnAddressType::PrivateFallbackPublic,
            peer_address: hci::BdAddrType::Random(hci::BdAddr([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            ])),
            advertising_channel_map: Channels::default(),
            advertising_filter_policy: AdvertisingFilterPolicy::AllowConnectionAndScan,
        })
        .unwrap();
    assert_eq!(
        sink.written_data,
        [
            1,
            0x06,
            0x20,
            15,
            0x9E,
            0x00,
            0x40,
            0x06,
            0x02,
            0x02,
            0x01,
            0x01,
            0x02,
            0x03,
            0x04,
            0x05,
            0x06,
            0b0000_0111,
            0x00
        ]
    );
}

#[test]
fn le_set_advertising_parameters_ignore_interval_for_high_duty_cycle() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_advertising_parameters(&AdvertisingParameters {
            // Bad interval in every way, but it is ignored for this advertising type
            advertising_interval: (Duration::from_millis(20000), Duration::from_millis(2)),
            advertising_type: AdvertisingType::ConnectableDirectedHighDutyCycle,
            own_address_type: OwnAddressType::Random,
            peer_address: hci::BdAddrType::Random(hci::BdAddr([
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
            ])),
            advertising_channel_map: Channels::CH_37 | Channels::CH_39,
            advertising_filter_policy: AdvertisingFilterPolicy::AllowConnectionAndScan,
        })
        .unwrap();
    assert_eq!(
        sink.written_data,
        [
            1,
            0x06,
            0x20,
            15,
            0x00, // advertising_interval is not used for ConnectableDirectedHighDutyCycle
            0x00,
            0x00,
            0x00,
            0x01, // advertising type
            0x01,
            0x01,
            0x01,
            0x02,
            0x03,
            0x04,
            0x05,
            0x06,
            0b0000_0101,
            0x00
        ]
    );
}

#[test]
fn le_read_advertising_channel_tx_power() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_read_advertising_channel_tx_power()
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x07, 0x20, 0]);
}

#[test]
fn le_set_advertising_data_empty() {
    let mut sink = RecordingSink::new();
    sink.as_controller().le_set_advertising_data(&[]).unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x08, 0x20, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn le_set_advertising_data_partial() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_advertising_data(&[1, 2, 3, 4, 5, 6, 7, 8])
        .unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x08, 0x20, 32, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn le_set_advertising_data_full() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_advertising_data(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31,
        ])
        .unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x08, 0x20, 32, 31, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
            19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ]
    );
}

#[test]
fn le_set_advertising_data_too_long() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .le_set_advertising_data(&[0; 32])
        .err()
        .unwrap();
    assert_eq!(err, nb::Error::Other(Error::AdvertisingDataTooLong(32)));
}

#[test]
fn le_set_scan_response_data_empty() {
    let mut sink = RecordingSink::new();
    sink.as_controller().le_set_scan_response_data(&[]).unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x09, 0x20, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn le_set_scan_response_data_partial() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_scan_response_data(&[1, 2, 3, 4, 5, 6, 7, 8])
        .unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x09, 0x20, 32, 8, 1, 2, 3, 4, 5, 6, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn le_set_scan_response_data_full() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_scan_response_data(&[
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31,
        ])
        .unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x09, 0x20, 32, 31, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
            19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        ]
    );
}

#[test]
fn le_set_scan_response_data_too_long() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .le_set_scan_response_data(&[0; 32])
        .err()
        .unwrap();
    assert_eq!(err, nb::Error::Other(Error::AdvertisingDataTooLong(32)));
}

#[cfg(not(feature = "version-5-0"))]
#[test]
fn le_set_advertise_enable() {
    let mut sink = RecordingSink::new();
    sink.as_controller().le_set_advertise_enable(true).unwrap();
    assert_eq!(sink.written_data, [1, 0x0A, 0x20, 1, 1]);
}

#[cfg(feature = "version-5-0")]
#[test]
fn le_set_advertising_enable() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_advertising_enable(true)
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x0A, 0x20, 1, 1]);
}

#[test]
fn le_set_scan_parameters() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_scan_parameters(&ScanParameters {
            scan_type: ScanType::Passive,
            scan_interval: Duration::from_millis(21),
            scan_window: Duration::from_millis(10),
            own_address_type: OwnAddressType::Public,
            filter_policy: ScanFilterPolicy::AcceptAll,
        })
        .unwrap();

    // bytes 5-6: 0x21, 0x00 = 0x0021 = 33 ~= 21 ms / 0.625 ms
    // bytes 7-8: 0x10, 0x00 = 0x0010 = 16 = 10 ms / 0.625 ms
    assert_eq!(
        sink.written_data,
        [1, 0x0B, 0x20, 7, 0x00, 0x21, 0x00, 0x10, 0x00, 0x00, 0x00]
    );
}

#[test]
fn le_set_scan_parameters_bad_window() {
    let mut sink = RecordingSink::new();
    for (interval, window) in [
        (Duration::from_millis(19), Duration::from_millis(20)),
        (Duration::from_millis(2), Duration::from_millis(1)),
        (Duration::from_millis(12), Duration::from_millis(2)),
        (Duration::from_millis(10241), Duration::from_millis(100)),
        (Duration::from_millis(102), Duration::from_millis(10241)),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_set_scan_parameters(&ScanParameters {
                scan_type: ScanType::Passive,
                scan_interval: *interval,
                scan_window: *window,
                own_address_type: OwnAddressType::Public,
                filter_policy: ScanFilterPolicy::AcceptAll,
            })
            .err()
            .unwrap();

        assert_eq!(
            err,
            nb::Error::Other(Error::BadScanInterval(*interval, *window))
        );
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_set_scan_enable() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_scan_enable(true, false)
        .unwrap();
    assert_eq!(sink.written_data, [1, 0x0C, 0x20, 2, 1, 0]);
}

#[test]
fn le_create_connection_no_whitelist() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_create_connection(&ConnectionParameters {
            scan_interval: Duration::from_millis(50),
            scan_window: Duration::from_millis(25),
            initiator_filter_policy: ConnectionFilterPolicy::UseAddress,
            peer_address: PeerAddrType::PublicDeviceAddress(hci::BdAddr([1, 2, 3, 4, 5, 6])),
            own_address_type: OwnAddressType::Public,
            conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
            conn_latency: 10,
            supervision_timeout: Duration::from_secs(15),
            expected_connection_length_range: (
                Duration::from_millis(200),
                Duration::from_millis(500),
            ),
        })
        .unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x0D, 0x20, 25, 0x50, 0x00, 0x28, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x06, 0x00, 0x28, 0x00, 0x90, 0x01, 0x0A, 0x00, 0xDC, 0x05, 0x40, 0x01, 0x20, 0x03,
        ]
    );
}

#[test]
fn le_create_connection_use_whitelist() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_create_connection(&ConnectionParameters {
            scan_interval: Duration::from_millis(50),
            scan_window: Duration::from_millis(25),
            initiator_filter_policy: ConnectionFilterPolicy::WhiteList,
            peer_address: PeerAddrType::PublicDeviceAddress(hci::BdAddr([1, 2, 3, 4, 5, 6])),
            own_address_type: OwnAddressType::Public,
            conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
            conn_latency: 10,
            supervision_timeout: Duration::from_secs(15),
            expected_connection_length_range: (
                Duration::from_millis(200),
                Duration::from_millis(500),
            ),
        })
        .unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x0D, 0x20, 25, 0x50, 0x00, 0x28, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x28, 0x00, 0x90, 0x01, 0x0A, 0x00, 0xDC, 0x05, 0x40, 0x01, 0x20, 0x03,
        ]
    );
}

#[test]
fn le_create_connection_bad_window() {
    let mut sink = RecordingSink::new();
    for (interval, window) in [
        (Duration::from_millis(19), Duration::from_millis(20)),
        (Duration::from_millis(2), Duration::from_millis(1)),
        (Duration::from_millis(12), Duration::from_millis(2)),
        (Duration::from_millis(10241), Duration::from_millis(100)),
        (Duration::from_millis(102), Duration::from_millis(10241)),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_create_connection(&ConnectionParameters {
                scan_interval: *interval,
                scan_window: *window,
                initiator_filter_policy: ConnectionFilterPolicy::WhiteList,
                peer_address: PeerAddrType::PublicDeviceAddress(hci::BdAddr([1, 2, 3, 4, 5, 6])),
                own_address_type: OwnAddressType::Public,
                conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
                conn_latency: 10,
                supervision_timeout: Duration::from_millis(50),
                expected_connection_length_range: (
                    Duration::from_millis(200),
                    Duration::from_millis(500),
                ),
            })
            .err()
            .unwrap();

        assert_eq!(
            err,
            nb::Error::Other(Error::BadScanInterval(*interval, *window))
        );
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_create_connection_bad_connection_interval() {
    let mut sink = RecordingSink::new();
    for (min, max) in [
        (Duration::from_millis(4), Duration::from_millis(1000)),
        (Duration::from_millis(100), Duration::from_millis(4001)),
        (Duration::from_millis(500), Duration::from_millis(499)),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_create_connection(&ConnectionParameters {
                scan_interval: Duration::from_millis(100),
                scan_window: Duration::from_millis(50),
                initiator_filter_policy: ConnectionFilterPolicy::WhiteList,
                peer_address: PeerAddrType::PublicDeviceAddress(hci::BdAddr([1, 2, 3, 4, 5, 6])),
                own_address_type: OwnAddressType::Public,
                conn_interval: (*min, *max),
                conn_latency: 10,
                supervision_timeout: Duration::from_millis(50),
                expected_connection_length_range: (
                    Duration::from_millis(9),
                    Duration::from_millis(8),
                ),
            })
            .err()
            .unwrap();

        assert_eq!(
            err,
            nb::Error::Other(Error::BadConnectionInterval(*min, *max))
        );
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_create_connection_bad_connection_latency() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .le_create_connection(&ConnectionParameters {
            scan_interval: Duration::from_millis(100),
            scan_window: Duration::from_millis(50),
            initiator_filter_policy: ConnectionFilterPolicy::WhiteList,
            peer_address: PeerAddrType::PublicDeviceAddress(hci::BdAddr([1, 2, 3, 4, 5, 6])),
            own_address_type: OwnAddressType::Public,
            conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
            conn_latency: 515,
            supervision_timeout: Duration::from_secs(12),
            expected_connection_length_range: (
                Duration::from_millis(200),
                Duration::from_millis(500),
            ),
        })
        .err()
        .unwrap();

    assert_eq!(err, nb::Error::Other(Error::BadConnectionLatency(515)));
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_create_connection_bad_supervision_timeout() {
    let mut sink = RecordingSink::new();
    for timeout in [
        Duration::from_millis(9),
        Duration::from_millis(10999),
        Duration::from_millis(32001),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_create_connection(&ConnectionParameters {
                scan_interval: Duration::from_millis(100),
                scan_window: Duration::from_millis(50),
                initiator_filter_policy: ConnectionFilterPolicy::WhiteList,
                peer_address: PeerAddrType::PublicDeviceAddress(hci::BdAddr([1, 2, 3, 4, 5, 6])),
                own_address_type: OwnAddressType::Public,
                conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
                conn_latency: 10,
                supervision_timeout: *timeout,
                expected_connection_length_range: (
                    Duration::from_millis(200),
                    Duration::from_millis(500),
                ),
            })
            .err()
            .unwrap();

        assert_eq!(
            err,
            nb::Error::Other(Error::BadSupervisionTimeout(
                *timeout,
                Duration::from_secs(11)
            ))
        );
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_create_connection_cancel() {
    let mut sink = RecordingSink::new();
    sink.as_controller().le_create_connection_cancel().unwrap();
    assert_eq!(sink.written_data, [1, 0x0E, 0x20, 0]);
}

#[test]
fn le_read_white_list_size() {
    let mut sink = RecordingSink::new();
    sink.as_controller().le_read_white_list_size().unwrap();
    assert_eq!(sink.written_data, [1, 0x0F, 0x20, 0]);
}

#[test]
fn le_clear_white_list() {
    let mut sink = RecordingSink::new();
    sink.as_controller().le_clear_white_list().unwrap();
    assert_eq!(sink.written_data, [1, 0x10, 0x20, 0]);
}

#[test]
fn le_add_device_to_white_list() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_add_device_to_white_list(hci::BdAddrType::Public(hci::BdAddr([1, 2, 3, 4, 5, 6])))
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x11, 0x20, 7, 0x00, 1, 2, 3, 4, 5, 6]
    );
}

#[cfg(feature = "version-5-0")]
#[test]
fn le_add_anon_advertising_devices_to_white_list() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_add_anon_advertising_devices_to_white_list()
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x11, 0x20, 7, 0xFF, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn le_remove_device_from_white_list() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_remove_device_from_white_list(hci::BdAddrType::Public(hci::BdAddr([1, 2, 3, 4, 5, 6])))
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x12, 0x20, 7, 0x00, 1, 2, 3, 4, 5, 6]
    );
}

#[cfg(feature = "version-5-0")]
#[test]
fn le_remove_anon_advertising_devices_from_white_list() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_remove_anon_advertising_devices_from_white_list()
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x12, 0x20, 7, 0xFF, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn le_connection_update() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_connection_update(&ConnectionUpdateParameters {
            conn_handle: hci::ConnectionHandle(0x0201),
            conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
            conn_latency: 10,
            supervision_timeout: Duration::from_secs(15),
            expected_connection_length_range: (
                Duration::from_millis(200),
                Duration::from_millis(500),
            ),
        })
        .unwrap();
    assert_eq!(
        sink.written_data,
        vec![
            1, 0x13, 0x20, 14, 0x01, 0x02, 0x28, 0x00, 0x90, 0x01, 0x0A, 0x00, 0xDC, 0x05, 0x40,
            0x01, 0x20, 0x03,
        ]
    );
}

#[test]
fn le_connection_update_bad_connection_interval() {
    let mut sink = RecordingSink::new();
    for (min, max) in [
        (Duration::from_millis(4), Duration::from_millis(1000)),
        (Duration::from_millis(100), Duration::from_millis(4001)),
        (Duration::from_millis(500), Duration::from_millis(499)),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_connection_update(&ConnectionUpdateParameters {
                conn_handle: hci::ConnectionHandle(0x0201),
                conn_interval: (*min, *max),
                conn_latency: 10,
                supervision_timeout: Duration::from_millis(50),
                expected_connection_length_range: (
                    Duration::from_millis(9),
                    Duration::from_millis(8),
                ),
            })
            .err()
            .unwrap();

        assert_eq!(
            err,
            nb::Error::Other(Error::BadConnectionInterval(*min, *max))
        );
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_connection_update_bad_connection_latency() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .le_connection_update(&ConnectionUpdateParameters {
            conn_handle: hci::ConnectionHandle(0x0201),
            conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
            conn_latency: 515,
            supervision_timeout: Duration::from_secs(12),
            expected_connection_length_range: (
                Duration::from_millis(200),
                Duration::from_millis(500),
            ),
        })
        .err()
        .unwrap();

    assert_eq!(err, nb::Error::Other(Error::BadConnectionLatency(515)));
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_connection_update_bad_supervision_timeout() {
    let mut sink = RecordingSink::new();
    for timeout in [
        Duration::from_millis(9),
        Duration::from_millis(10999),
        Duration::from_millis(32001),
    ].iter()
    {
        let err = sink
            .as_controller()
            .le_connection_update(&ConnectionUpdateParameters {
                conn_handle: hci::ConnectionHandle(0x0201),
                conn_interval: (Duration::from_millis(50), Duration::from_millis(500)),
                conn_latency: 10,
                supervision_timeout: *timeout,
                expected_connection_length_range: (
                    Duration::from_millis(200),
                    Duration::from_millis(500),
                ),
            })
            .err()
            .unwrap();

        assert_eq!(
            err,
            nb::Error::Other(Error::BadSupervisionTimeout(
                *timeout,
                Duration::from_secs(11)
            ))
        );
    }
    assert_eq!(sink.written_data, []);
}

#[test]
fn le_set_host_channel_classification() {
    let mut sink = RecordingSink::new();
    sink.as_controller()
        .le_set_host_channel_classification(
            ChannelClassification::CH_0
                | ChannelClassification::CH_4
                | ChannelClassification::CH_8
                | ChannelClassification::CH_12
                | ChannelClassification::CH_16
                | ChannelClassification::CH_20
                | ChannelClassification::CH_24
                | ChannelClassification::CH_28
                | ChannelClassification::CH_32
                | ChannelClassification::CH_36,
        )
        .unwrap();
    assert_eq!(
        sink.written_data,
        [1, 0x14, 0x20, 5, 0x11, 0x11, 0x11, 0x11, 0x11]
    );
}

#[test]
fn le_set_host_channel_classification_failed_empty() {
    let mut sink = RecordingSink::new();
    let err = sink
        .as_controller()
        .le_set_host_channel_classification(ChannelClassification::empty())
        .err()
        .unwrap();
    assert_eq!(err, nb::Error::Other(Error::NoValidChannel));
    assert_eq!(sink.written_data, []);
}
