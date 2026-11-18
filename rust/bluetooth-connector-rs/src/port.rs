use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use exceptions_rs::port::HeadphonesError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    pub name: String,
    pub mac: String,
}

pub trait IBluetoothConnector: Send {
    fn send(&mut self, buf: &[u8]) -> Result<usize, HeadphonesError>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, HeadphonesError>;
    fn connect(&mut self, addr: &str) -> Result<(), HeadphonesError>;
    fn disconnect(&mut self);
    fn is_connected(&self) -> bool;
    fn get_connected_devices(&self) -> Result<Vec<BluetoothDevice>, HeadphonesError>;
}

#[derive(Debug, Clone, Default)]
pub struct MockBluetoothConnector {
    connected: Arc<AtomicBool>,
    pub devices: Vec<BluetoothDevice>,
    pub sent_frames: Arc<Mutex<Vec<Vec<u8>>>>,
    pub recv_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
}

impl MockBluetoothConnector {
    pub fn new(devices: Vec<BluetoothDevice>) -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            devices,
            sent_frames: Arc::new(Mutex::new(Vec::new())),
            recv_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn push_incoming(&self, frame: Vec<u8>) {
        if let Ok(mut q) = self.recv_queue.lock() {
            q.push_back(frame);
        }
    }
}

impl IBluetoothConnector for MockBluetoothConnector {
    fn send(&mut self, buf: &[u8]) -> Result<usize, HeadphonesError> {
        if !self.is_connected() {
            return Err(HeadphonesError::recoverable("Not connected", true));
        }
        if let Ok(mut sent) = self.sent_frames.lock() {
            sent.push(buf.to_vec());
        }
        Ok(buf.len())
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, HeadphonesError> {
        let mut q = self
            .recv_queue
            .lock()
            .map_err(|_| HeadphonesError::runtime("Mock recv lock poisoned"))?;
        let Some(front) = q.front_mut() else {
            return Err(HeadphonesError::recoverable(
                "No mock data available",
                false,
            ));
        };
        let n = front.len().min(buf.len());
        buf[..n].copy_from_slice(&front[..n]);
        front.drain(..n);
        if front.is_empty() {
            q.pop_front();
        }
        Ok(n)
    }

    fn connect(&mut self, _addr: &str) -> Result<(), HeadphonesError> {
        self.connected.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn disconnect(&mut self) {
        self.connected.store(false, Ordering::SeqCst);
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    fn get_connected_devices(&self) -> Result<Vec<BluetoothDevice>, HeadphonesError> {
        Ok(self.devices.clone())
    }
}

#[cfg(target_os = "windows")]
pub mod win {
    use std::mem::size_of;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicBool, Ordering};

    use byte_magic_rs::port::{mac_bytes_to_string, mac_string_to_long};
    use constants_rs::port::{SERVICE_UUID, SERVICE_UUID_IN_BYTES};
    use exceptions_rs::port::HeadphonesError;
    use windows_sys::Win32::Devices::Bluetooth::{
        BLUETOOTH_DEVICE_INFO, BLUETOOTH_DEVICE_SEARCH_PARAMS, BLUETOOTH_FIND_RADIO_PARAMS,
        BTHPROTO_RFCOMM, BluetoothFindDeviceClose, BluetoothFindFirstDevice,
        BluetoothFindFirstRadio, BluetoothFindNextDevice, BluetoothFindNextRadio,
        BluetoothFindRadioClose, SO_BTH_AUTHENTICATE, SO_BTH_ENCRYPT, SOCKADDR_BTH, SOL_RFCOMM,
    };
    use windows_sys::Win32::Foundation::{ERROR_NO_MORE_ITEMS, GetLastError, HANDLE};
    use windows_sys::Win32::Networking::WinSock::{
        INVALID_SOCKET, SD_BOTH, SOCK_STREAM, SOCKET, SOCKET_ERROR, WSADATA, WSAGetLastError,
        WSAStartup, closesocket, connect as wsa_connect, recv as wsa_recv, send as wsa_send,
        setsockopt, shutdown, socket as wsa_socket,
    };
    use windows_sys::core::GUID;

    use super::{BluetoothDevice, IBluetoothConnector};

    const AF_BTH: u16 = 32;
    const NO_BT_ERR: &str = "No Bluetooth radios were found - is your adapter connected?";

    fn ensure_wsa_startup() -> Result<(), HeadphonesError> {
        static INIT: OnceLock<Result<(), String>> = OnceLock::new();
        let res = INIT.get_or_init(|| {
            let mut data = WSADATA::default();
            // SAFETY: Valid mutable pointer to WSADATA for WinSock 2.2 startup.
            let rc = unsafe { WSAStartup(0x0202, &mut data) };
            if rc != 0 {
                Err(format!("WSAStartup failed: {rc}"))
            } else {
                Ok(())
            }
        });
        res.clone().map_err(HeadphonesError::runtime)
    }

    fn sony_service_guid() -> GUID {
        let _ = SERVICE_UUID;
        GUID::from_u128(u128::from_be_bytes(SERVICE_UUID_IN_BYTES))
    }

    pub struct WindowsBluetoothConnector {
        socket: SOCKET,
        connected: AtomicBool,
    }

    impl WindowsBluetoothConnector {
        pub fn new() -> Result<Self, HeadphonesError> {
            ensure_wsa_startup()?;
            Ok(Self {
                socket: INVALID_SOCKET,
                connected: AtomicBool::new(false),
            })
        }

        fn init_socket(&mut self) -> Result<(), HeadphonesError> {
            let sock = {
                // SAFETY: Standard Winsock AF_BTH RFCOMM socket creation.
                unsafe { wsa_socket(i32::from(AF_BTH), SOCK_STREAM, BTHPROTO_RFCOMM as i32) }
            };
            if sock == INVALID_SOCKET {
                // SAFETY: Calling WSAGetLastError immediately after failed socket call.
                let err = unsafe { WSAGetLastError() };
                return Err(HeadphonesError::runtime(format!(
                    "Couldn't create socket: {err}"
                )));
            }
            let enable: u32 = 1;
            let opt_ptr = (&enable as *const u32).cast::<u8>();
            for (name, opt) in [
                ("SO_BTH_AUTHENTICATE", SO_BTH_AUTHENTICATE),
                ("SO_BTH_ENCRYPT", SO_BTH_ENCRYPT),
            ] {
                // SAFETY: Valid socket handle and 4-byte u32 option buffer.
                let rc = unsafe { setsockopt(sock, SOL_RFCOMM as i32, opt as i32, opt_ptr, 4) };
                if rc != 0 {
                    // SAFETY: Retrieving error and closing socket on setsockopt failure.
                    let err = unsafe {
                        let e = WSAGetLastError();
                        closesocket(sock);
                        e
                    };
                    return Err(HeadphonesError::runtime(format!(
                        "Couldn't set {name}: {err}"
                    )));
                }
            }
            self.socket = sock;
            Ok(())
        }

        fn find_devices_in_radio(
            search: &BLUETOOTH_DEVICE_SEARCH_PARAMS,
        ) -> Result<Vec<BluetoothDevice>, HeadphonesError> {
            let mut res = Vec::new();
            let mut info = BLUETOOTH_DEVICE_INFO {
                dwSize: size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
                ..Default::default()
            };
            // SAFETY: Valid pointers to initialized search params and device info struct.
            let handle = unsafe { BluetoothFindFirstDevice(search, &mut info) };
            if handle.is_null() {
                // SAFETY: Reading Win32 last error after null handle return.
                let err = unsafe { GetLastError() };
                if err == ERROR_NO_MORE_ITEMS {
                    return Ok(res);
                }
                return Err(HeadphonesError::runtime(format!(
                    "BluetoothFindFirstDevice() failed with error code: {err}"
                )));
            }
            loop {
                let nul = info
                    .szName
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(info.szName.len());
                let name = String::from_utf16_lossy(&info.szName[..nul]);
                // SAFETY: Address.Anonymous.rgBytes is a valid 6-byte MAC field inside the union.
                let mac = mac_bytes_to_string(unsafe { &info.Address.Anonymous.rgBytes });
                res.push(BluetoothDevice { name, mac });
                // SAFETY: Iterating valid Bluetooth device find handle.
                if unsafe { BluetoothFindNextDevice(handle, &mut info) } == 0 {
                    break;
                }
            }
            // SAFETY: Closing valid device enumeration handle.
            if unsafe { BluetoothFindDeviceClose(handle) } == 0 {
                // SAFETY: Reading Win32 last error on close failure.
                let err = unsafe { GetLastError() };
                return Err(HeadphonesError::runtime(format!(
                    "BluetoothFindDeviceClose() failed with error code: {err}"
                )));
            }
            Ok(res)
        }
    }

    impl Drop for WindowsBluetoothConnector {
        fn drop(&mut self) {
            if self.socket != INVALID_SOCKET {
                // SAFETY: Closing owned socket on drop.
                unsafe { closesocket(self.socket) };
                self.socket = INVALID_SOCKET;
            }
        }
    }

    impl IBluetoothConnector for WindowsBluetoothConnector {
        fn connect(&mut self, addr: &str) -> Result<(), HeadphonesError> {
            if self.socket == INVALID_SOCKET {
                self.init_socket()?;
            }
            let sab = SOCKADDR_BTH {
                addressFamily: AF_BTH,
                btAddr: mac_string_to_long(addr)?,
                serviceClassId: sony_service_guid(),
                port: 0,
            };
            let ptr = (&sab as *const SOCKADDR_BTH).cast();
            // SAFETY: Valid initialized RFCOMM socket and SOCKADDR_BTH length.
            if unsafe { wsa_connect(self.socket, ptr, size_of::<SOCKADDR_BTH>() as i32) } != 0 {
                // SAFETY: Reading Winsock error after connect failure.
                let err = unsafe { WSAGetLastError() };
                return Err(HeadphonesError::recoverable(
                    format!("Couldn't connect: {err}"),
                    true,
                ));
            }
            self.connected.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn send(&mut self, buf: &[u8]) -> Result<usize, HeadphonesError> {
            // SAFETY: Valid connected socket and byte slice pointer/length.
            let sent = unsafe { wsa_send(self.socket, buf.as_ptr(), buf.len() as i32, 0) };
            if sent == SOCKET_ERROR {
                // SAFETY: Reading Winsock error after send failure.
                let err = unsafe { WSAGetLastError() };
                return Err(HeadphonesError::recoverable(
                    format!("Couldn't send ({err})"),
                    true,
                ));
            }
            Ok(sent as usize)
        }

        fn recv(&mut self, buf: &mut [u8]) -> Result<usize, HeadphonesError> {
            // SAFETY: Valid connected socket and mutable byte slice pointer/length.
            let recvd = unsafe { wsa_recv(self.socket, buf.as_mut_ptr(), buf.len() as i32, 0) };
            if recvd == SOCKET_ERROR {
                // SAFETY: Reading Winsock error after recv failure.
                let err = unsafe { WSAGetLastError() };
                return Err(HeadphonesError::recoverable(
                    format!("Couldn't recv ({err})"),
                    true,
                ));
            }
            Ok(recvd as usize)
        }

        fn disconnect(&mut self) {
            if self.socket != INVALID_SOCKET {
                self.connected.store(false, Ordering::SeqCst);
                // SAFETY: Shutting down and closing owned Winsock socket.
                unsafe {
                    shutdown(self.socket, SD_BOTH);
                    closesocket(self.socket);
                }
                self.socket = INVALID_SOCKET;
            }
        }

        fn is_connected(&self) -> bool {
            self.connected.load(Ordering::SeqCst)
        }

        fn get_connected_devices(&self) -> Result<Vec<BluetoothDevice>, HeadphonesError> {
            let mut res = Vec::new();
            let mut radio: HANDLE = std::ptr::null_mut();
            let params = BLUETOOTH_FIND_RADIO_PARAMS {
                dwSize: size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
            };
            // SAFETY: Valid pointers for BluetoothFindFirstRadio enumeration.
            let find = unsafe { BluetoothFindFirstRadio(&params, &mut radio) };
            if find.is_null() {
                // SAFETY: Reading Win32 error after null handle return.
                let err = unsafe { GetLastError() };
                if err == ERROR_NO_MORE_ITEMS {
                    return Err(HeadphonesError::recoverable(NO_BT_ERR, false));
                }
                return Err(HeadphonesError::runtime(format!(
                    "BluetoothFindFirstRadio() failed with error code {err}"
                )));
            }
            let mut search = BLUETOOTH_DEVICE_SEARCH_PARAMS {
                dwSize: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
                fReturnConnected: 1,
                cTimeoutMultiplier: 15,
                ..Default::default()
            };
            loop {
                search.hRadio = radio;
                res.extend(Self::find_devices_in_radio(&search)?);
                // SAFETY: Advancing valid radio enumeration handle.
                if unsafe { BluetoothFindNextRadio(find, &mut radio) } == 0 {
                    break;
                }
            }
            // SAFETY: Closing valid radio enumeration handle.
            if unsafe { BluetoothFindRadioClose(find) } == 0 {
                // SAFETY: Reading Win32 error on close failure.
                let err = unsafe { GetLastError() };
                return Err(HeadphonesError::runtime(format!(
                    "BluetoothFindRadioClose() failed with error code {err}"
                )));
            }
            Ok(res)
        }
    }
}
