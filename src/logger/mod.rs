use crate::event::*;

pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

pub struct Logger {
    pub level: Level,
    pub rx: tokio::sync::broadcast::Receiver<Event>,
}

impl Logger {
    async fn run(&mut self) {
        while let Ok(event) = self.rx.recv().await {
            match event {
                Event::Listener(listener_event) => {
                    match listener_event {
                        ListenerEvent::Ready => {
                            println!("[INFO] Watchlist loaded, listener is ready.");
                        }
                        ListenerEvent::ChainUp { chain } => {
                            println!("[INFO] Chain started listening: {}", chain);
                        }
                        ListenerEvent::ChainDown { chain, err } => {
                            println!("[ERROR] Chain failed: {} - {}", chain, err);
                        }
                        ListenerEvent::ListeningLaunches { chain } => {
                            println!("[INFO] Listening to launches on chain: {}", chain);
                        }
                        ListenerEvent::ListeningGraduations { chain } => {
                            println!("[INFO] Listening to graduations on chain: {}", chain);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub async fn start(level: Level, rx: tokio::sync::broadcast::Receiver<Event>) {
        let mut logger = Logger { level, rx };
        logger.run().await;
    }

}