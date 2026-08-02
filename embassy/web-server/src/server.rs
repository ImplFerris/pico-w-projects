use embassy_executor::Spawner;
use picoserve::extract::State;
use picoserve::response::File as PicoFile;
use picoserve::{
    AppBuilder, AppRouter, make_static,
    response::DebugValue,
    routing::{PathRouter, get, get_service, parse_path_segment},
};

// defmt Logging
use defmt::{info, unwrap};

use cyw43::Control;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

type ControlMutex = Mutex<CriticalSectionRawMutex, Control<'static>>;

#[derive(Clone, Copy)]
struct SharedControl(&'static ControlMutex);

#[derive(Clone, Copy)]

struct AppState {
    shared_control: SharedControl,
}

// impl picoserve::extract::FromRef<AppState> for SharedControl {
//     fn from_ref(state: &AppState) -> Self {
//         state.shared_control
//     }
// }

struct AppProps {
    state: AppState,
}

impl AppBuilder for AppProps {
    type PathRouter = impl PathRouter;

    fn build_app(self) -> picoserve::Router<Self::PathRouter> {
        let Self { state } = self;

        picoserve::Router::new()
            .route("/", get_service(PicoFile::html(include_str!("index.html"))))
            .route(
                "/index.css",
                get_service(PicoFile::css(include_str!("index.css"))),
            )
            .route(
                "/index.js",
                get_service(PicoFile::javascript(include_str!("index.js"))),
            )
            .route(
                ("/set_led", parse_path_segment()),
                get(
                    // |led_is_on, State(SharedControl(control)): State<SharedControl>| async move {
                    |led_is_on: bool, State(state): State<AppState>| async move {
                        let SharedControl(control) = state.shared_control;

                        info!("Setting led to {}", if led_is_on { "ON" } else { "OFF" });
                        control.lock().await.gpio_set(0, led_is_on).await;
                        DebugValue(led_is_on)
                    },
                ),
            )
            .with_state(state)
    }
}

static CONFIG: picoserve::Config = picoserve::Config::const_default().keep_connection_alive();

const WEB_TASK_POOL_SIZE: usize = 4;

#[embassy_executor::task(pool_size = WEB_TASK_POOL_SIZE)]
async fn web_task(
    task_id: usize,
    stack: embassy_net::Stack<'static>,
    app: &'static AppRouter<AppProps>,
) -> ! {
    let port = 80;
    let mut tcp_rx_buffer = [0; 1024];
    let mut tcp_tx_buffer = [0; 1024];
    let mut http_buffer = [0; 2048];

    picoserve::Server::new(app, &CONFIG, &mut http_buffer)
        .listen_and_serve(task_id, stack, port, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
        .into_never()
}

pub fn start(spawner: Spawner, stack: embassy_net::Stack<'static>, control: Control<'static>) {
    let shared_control = SharedControl(make_static!(ControlMutex, Mutex::new(control)));

    let app = make_static!(
        AppRouter<AppProps>,
        AppProps {
            state: AppState { shared_control }
        }
        .build_app()
    );

    info!("Running the server...");

    for task_id in 0..WEB_TASK_POOL_SIZE {
        spawner.spawn(unwrap!(web_task(task_id, stack, app)));
    }
}
