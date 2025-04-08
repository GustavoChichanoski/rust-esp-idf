use embassy_futures::join::join;
use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use embedded_io_async::{Read, Write};
use esp_wifi::wifi::{
    ClientConfiguration, Configuration, WifiController, WifiDevice, WifiStaDevice, WifiState,
};
use reqwless::{
    client::{HttpClient, HttpResource},
    headers::ContentType,
    request::RequestBuilder,
    response::Response,
};

use crate::devices::display::DISPLAY_SIGNAL;

#[derive(PartialEq)]
enum WifiStatus {
    Disconnected,
    Connected,
}

static WIFI_SIGNAL_CONNECT: Signal<CriticalSectionRawMutex, WifiStatus> = Signal::new();

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const URL: &str = env!("URL");
const BUFFER_SIZE: usize = 1024;

async fn send_post<'a, C: Read + Write>(
    rx_buffer: &'a mut [u8; BUFFER_SIZE],
    resource: &'a mut HttpResource<'a, C>,
) -> Result<Response<'a, 'a, impl Read + 'a>, reqwless::Error> {
    let body_requests = b"{code: 1; quantity: 400}".as_slice();
    let response = resource
        .post("/api/v1/caixas")
        .body(body_requests)
        .content_type(ContentType::ApplicationJson)
        .send(rx_buffer)
        .await?;

    esp_println::println!("[WIFI] Request completed");
    Ok(response)
}

#[embassy_executor::task]
pub async fn request_http(stack: embassy_net::Stack<'static>) {
    esp_println::println!("[WIFI] Starting http task");
    let client_state = TcpClientState::<1, BUFFER_SIZE, BUFFER_SIZE>::new();
    let dns = DnsSocket::new(stack);
    let client = TcpClient::new(stack, &client_state);
    let mut http_client = HttpClient::new(&client, &dns); // Types implementing embedded-nal-async
    let mut rx_buf: [u8; BUFFER_SIZE] = [0u8; BUFFER_SIZE];
    let mut tx_buf: [u8; BUFFER_SIZE] = [0u8; BUFFER_SIZE];

    loop {
        let val = WIFI_SIGNAL_CONNECT.wait().await;
        if val == WifiStatus::Disconnected {
            continue;
        }

        let mut resource;

        match http_client.resource(URL).await {
            Ok(http_resource) => {
                resource = http_resource;
                esp_println::println!("[WIFI] 🌍 Connected");
            }
            Err(err) => {
                esp_println::println!("[WIFI] 💥 Failed to connect to wifi: {:?}", err);
                continue;
            }
        }

        match send_post(&mut rx_buf, &mut resource).await {
            Ok(response) => {
                let len = response
                    .body()
                    .reader()
                    .read_to_end(&mut tx_buf)
                    .await
                    .unwrap();
                esp_println::println!("[WIFI] response: {:?}", len);
            }
            Err(err) => {
                esp_println::println!("[WIFI] Failed to connect to wifi: {:?}", err);
                continue;
            }
        }
        esp_println::println!("[WIFI] Request");
    }
}

#[embassy_executor::task]
pub async fn network(
    stack: embassy_net::Stack<'static>,
    mut runner: embassy_net::Runner<'static, WifiDevice<'static, WifiStaDevice>>,
    mut controller: WifiController<'static>,
) {
    esp_println::println!("[WIFI] Start connection task");
    let wifi = async {
        loop {
            if esp_wifi::wifi::sta_state() == WifiState::StaConnected {
                controller
                    .wait_for_event(esp_wifi::wifi::WifiEvent::StaDisconnected)
                    .await;
                continue;
            }

            if !matches!(controller.is_started(), Ok(true)) {
                let client_config = Configuration::Client(ClientConfiguration {
                    ssid: SSID.try_into().expect("[WIFI] ssid larger than 32 bytes"),
                    password: PASSWORD
                        .try_into()
                        .expect("[WIFI] password larger than 64 bytes"),
                    ..Default::default()
                });

                match controller.set_configuration(&client_config) {
                    Ok(_) => {
                        esp_println::println!("[WIFI] Started wifi");
                    }
                    Err(err) => {
                        esp_println::println!("[WIFI] Failed to start wifi: {:#?}", err);
                        continue;
                    }
                };
                esp_println::println!("[WIFI] Starting wifi");
                match controller.start_async().await {
                    Ok(_) => {}
                    Err(err) => {
                        esp_println::println!("[WIFI] Failed to start wifi: {:#?}", err);
                        continue;
                    }
                };
                esp_println::println!("[WIFI] Wifi started!");
            }
            esp_println::println!("[WIFI] About to connect ...");

            match controller.connect_async().await {
                Ok(_) => {
                    esp_println::println!("[WIFI] Wifi connected, Waiting to get IP address...");
                    stack.wait_config_up().await;
                    WIFI_SIGNAL_CONNECT.signal(WifiStatus::Connected);

                    if let Some(config) = stack.config_v4() {
                        esp_println::println!("[WIFI] IP: {}", config.address);
                        esp_println::println!("[WIFI] Wifi connected signal");
                        let mut msg = heapless::String::<64>::new();
                        match core::fmt::write(
                            &mut msg,
                            core::format_args!("IP: {}", config.address),
                        ) {
                            Ok(_) => {}
                            Err(e) => esp_println::println!("[WIFI] Format error: {:?}", e),
                        };
                        DISPLAY_SIGNAL.signal(msg);
                    }
                }
                Err(_) => {
                    esp_println::println!("[WIFI] Failed to connect to wifi");
                    Timer::after(Duration::from_millis(1000)).await;
                    continue;
                }
            }

            loop {
                let connected = match controller.is_connected() {
                    Ok(connected) => connected,
                    Err(err) => {
                        esp_println::println!("[WIFI] Failed to check connection: {:#?}", err);
                        Timer::after(Duration::from_millis(1000)).await;
                        break;
                    }
                };

                if !connected {
                    break;
                }
                
                Timer::after(Duration::from_millis(500)).await;
            }

            WIFI_SIGNAL_CONNECT.signal(WifiStatus::Disconnected);
            esp_println::println!("[WIFI] Send disconnected signal");
        }
    };

    join(runner.run(), wifi).await;
}
