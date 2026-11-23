use cleanserve_proxy::{HmrEvent, HmrState, ProxyServer};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::connect_async;

fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

#[tokio::test]
async fn hmr_websocket_upgrades_on_main_port_and_delivers_events() {
    let port = free_port();
    let state = Arc::new(RwLock::new(HmrState::new()));
    let proxy = ProxyServer::new_with_hmr(port, ".".to_string(), state.clone());
    tokio::spawn(async move { let _ = proxy.start().await; });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    let (mut ws, resp) = connect_async(format!("ws://127.0.0.1:{port}/__cleanserve_hmr"))
        .await
        .expect("websocket upgrade on the main port");
    assert_eq!(resp.status().as_u16(), 101);

    let hello = ws.next().await.unwrap().unwrap();
    assert!(hello.to_text().unwrap().contains("connected"));

    state.read().await.emit(HmrEvent::StyleReload("public/a b\\c.css".into()));
    let msg = ws.next().await.unwrap().unwrap();
    let v: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(v["type"], "style");
    assert_eq!(v["path"], "public/a b\\c.css"); // valid JSON even with odd characters

    state.read().await.emit(HmrEvent::PhpReload);
    let msg = ws.next().await.unwrap().unwrap();
    assert!(msg.to_text().unwrap().contains("reload"));
}
