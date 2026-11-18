#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use bluetooth_connector_rs::port::{BluetoothDevice, IBluetoothConnector};
use command_serializer_rs::port::{BtMessage, package_data_for_bt, unpack_bt_message};
use constants_rs::port::{DataType, END_MARKER, MAX_BLUETOOTH_MESSAGE_SIZE, START_MARKER};
use exceptions_rs::port::HeadphonesError;

struct WrapperState {
    connector: Box<dyn IBluetoothConnector>,
    seq_number: u8,
    rx_buf: Vec<u8>,
    pending_messages: VecDeque<BtMessage>,
}

pub struct BluetoothWrapper {
    state: Mutex<WrapperState>,
    connected: AtomicBool,
}

impl BluetoothWrapper {
    pub fn new(connector: Box<dyn IBluetoothConnector>) -> Self {
        let is_conn = connector.is_connected();
        Self {
            state: Mutex::new(WrapperState {
                connector,
                seq_number: 0,
                rx_buf: Vec::new(),
                pending_messages: VecDeque::new(),
            }),
            connected: AtomicBool::new(is_conn),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn seq_number(&self) -> Result<u8, HeadphonesError> {
        let guard = self
            .state
            .lock()
            .map_err(|_| HeadphonesError::runtime("BluetoothWrapper mutex poisoned"))?;
        Ok(guard.seq_number)
    }

    pub fn connect(&self, addr: &str) -> Result<(), HeadphonesError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| HeadphonesError::runtime("BluetoothWrapper mutex poisoned"))?;
        guard.seq_number = 0;
        guard.rx_buf.clear();
        guard.pending_messages.clear();
        guard.connector.connect(addr)?;
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.state.lock() {
            guard.seq_number = 0;
            guard.rx_buf.clear();
            guard.pending_messages.clear();
            guard.connector.disconnect();
        }
    }

    pub fn get_connected_devices(&self) -> Result<Vec<BluetoothDevice>, HeadphonesError> {
        let guard = self
            .state
            .lock()
            .map_err(|_| HeadphonesError::runtime("BluetoothWrapper mutex poisoned"))?;
        guard.connector.get_connected_devices()
    }

    pub fn send_command(&self, bytes: &[u8]) -> Result<usize, HeadphonesError> {
        self.send_command_with_type(bytes, DataType::DataMdr)
    }

    pub fn send_command_with_type(
        &self,
        bytes: &[u8],
        dtype: DataType,
    ) -> Result<usize, HeadphonesError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| HeadphonesError::runtime("BluetoothWrapper mutex poisoned"))?;
        let seq = guard.seq_number;
        let data = package_data_for_bt(bytes, dtype, seq)?;
        let sent = guard.connector.send(&data)?;
        if dtype != DataType::Ack {
            Self::wait_for_ack_locked(&mut guard)?;
        }
        Ok(sent)
    }

    pub fn send_ack(&self, seq_number: u8) -> Result<usize, HeadphonesError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| HeadphonesError::runtime("BluetoothWrapper mutex poisoned"))?;
        Self::send_ack_locked(&mut guard, seq_number)
    }

    pub fn read_message(&self) -> Result<BtMessage, HeadphonesError> {
        let mut guard = self
            .state
            .lock()
            .map_err(|_| HeadphonesError::runtime("BluetoothWrapper mutex poisoned"))?;
        Self::read_message_locked(&mut guard)
    }

    pub fn drain_pending(&self) -> Vec<BtMessage> {
        self.state
            .lock()
            .map(|mut g| g.pending_messages.drain(..).collect())
            .unwrap_or_default()
    }

    fn send_ack_locked(guard: &mut WrapperState, seq_number: u8) -> Result<usize, HeadphonesError> {
        let data = package_data_for_bt(&[], DataType::Ack, seq_number ^ 0x01)?;
        guard.connector.send(&data)
    }

    fn wait_for_ack_locked(guard: &mut WrapperState) -> Result<(), HeadphonesError> {
        loop {
            let msg = Self::read_message_locked(guard)?;
            match msg.data_type {
                DataType::Ack => {
                    guard.seq_number = msg.seq_number;
                    return Ok(());
                }
                DataType::DataMdr | DataType::DataMdrNo2 => {
                    Self::send_ack_locked(guard, msg.seq_number)?;
                    guard.pending_messages.push_back(msg);
                }
                _ => {
                    guard.pending_messages.push_back(msg);
                }
            }
        }
    }

    fn read_message_locked(guard: &mut WrapperState) -> Result<BtMessage, HeadphonesError> {
        let mut chunk = [0u8; MAX_BLUETOOTH_MESSAGE_SIZE];
        loop {
            if let Some(frame) = Self::extract_next_frame(&mut guard.rx_buf)? {
                return unpack_bt_message(&frame);
            }
            let n = guard.connector.recv(&mut chunk)?;
            if n == 0 {
                return Err(HeadphonesError::recoverable(
                    "Bluetooth connection closed",
                    true,
                ));
            }
            guard.rx_buf.extend_from_slice(&chunk[..n]);
        }
    }

    fn extract_next_frame(buf: &mut Vec<u8>) -> Result<Option<Vec<u8>>, HeadphonesError> {
        let Some(start) = buf.iter().position(|&b| b == START_MARKER) else {
            buf.clear();
            return Ok(None);
        };
        for i in (start + 1)..buf.len() {
            if buf[i] == START_MARKER {
                buf.drain(..=i);
                return Err(HeadphonesError::recoverable(
                    "Invalid: Multiple start markers without an end marker",
                    true,
                ));
            }
            if buf[i] == END_MARKER {
                let frame = buf[(start + 1)..i].to_vec();
                buf.drain(..=i);
                return Ok(Some(frame));
            }
        }
        if start > 0 {
            buf.drain(..start);
        }
        Ok(None)
    }
}
