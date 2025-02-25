use futures::StreamExt;
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_utils::window;
use sauron_core::{
    dom::{self, DomNode},
    prelude::Node,
    vdom,
    vdom::diff,
};
use sauron_html_parser::{parse_html, raw_html};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Element, HtmlElement};

#[derive(Clone)]
struct DomUpdater {
    id: String,
    current_vdom: Node<()>,
    root_node: Rc<RefCell<Option<DomNode>>>,
    mount_node: Rc<RefCell<Option<DomNode>>>,
}

impl DomUpdater {
    fn new(id: String) -> Self {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");

        let div: web_sys::Element = document
            .get_element_by_id(id.as_str())
            .expect("Element with specified id not found");

        let web_sys_node: web_sys::Node = web_sys::Node::from(div);
        let div_node = DomNode::from(web_sys_node);

        let current_vdom: Node<()> = parse_html::<()>("").unwrap().unwrap();
        let ev_callback = |_| {};
        let root: DomNode = dom::create_dom_node(&current_vdom, ev_callback);

        DomUpdater {
            id,
            current_vdom,
            root_node: Rc::new(RefCell::new(Some(root))),
            mount_node: Rc::new(RefCell::new(Some(div_node))),
        }
    }
    fn update(&mut self, next_html: String) {
        let new_node: Node<()> = parse_html::<()>(next_html.as_str()).unwrap().unwrap();

        let old_vdom = self.current_vdom.clone();

        //log::debug!("-------------------------------------------------");
        //log::debug!("old_node: {}", old_vdom.render_to_string());
        //log::debug!("inner_html: {}", self.inner_html());
        // fn same(a: String, b: String) -> String {
        //     if a == b {
        //         "same".to_string()
        //     } else {
        //         "different".to_string()
        //     }
        // }
        // log::debug!(
        //     "   => {}",
        //     same(old_vdom.render_to_string(), self.inner_html())
        // );
        //log::debug!("new_node: {}", new_node.render_to_string());
        // log::debug!("new_node: {:#?}", new_node);

        let vdom_patches = vdom::diff(&old_vdom, &new_node).unwrap();

        //log::debug!("Created {} VDOM patch(es)", vdom_patches.len());
        //log::debug!("Created {:#?}", vdom_patches);
        let dom_patches = dom::convert_patches(
            self.root_node.borrow().as_ref().unwrap(),
            &vdom_patches,
            |_| {},
        )
        .unwrap();
        //log::debug!("Converted {} DOM patch(es)", dom_patches.len());
        //log::debug!("Converted {:#?}", dom_patches);
        //log::debug!("-------------------------------------------------");
        dom::apply_dom_patches(
            Rc::clone(&self.root_node),
            Rc::clone(&self.mount_node),
            dom_patches,
        )
        .unwrap();
        self.current_vdom = new_node.clone();

        //assert_eq!(next_html, self.inner_html());
    }
    fn inner_html(&self) -> String {
        let window = web_sys::window().expect("no global `window` exists");
        let document = window.document().expect("should have a document on window");
        let target: Element = document.get_element_by_id(self.id.as_str()).unwrap();
        target.inner_html()
    }
}

fn ws_close() {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    if let Some(status_element) = document.get_element_by_id("websocketStatus") {
        if let Some(status_element) = status_element.dyn_ref::<HtmlElement>() {
            status_element
                .class_list()
                .add_1("glyphicon-remove")
                .expect("Failed to add class");
            status_element
                .class_list()
                .remove_1("glyphicon-ok")
                .expect("Failed to remove class");
        }
    }
}

fn ws_open() {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    if let Some(ws_element) = document.get_element_by_id("websocket") {
        if let Some(ws_element) = ws_element.dyn_ref::<HtmlElement>() {
            ws_element
                .style()
                .set_property("display", "block")
                .expect("Failed to set display property");
        }
    }

    if let Some(status_element) = document.get_element_by_id("websocketStatus") {
        if let Some(status_element) = status_element.dyn_ref::<HtmlElement>() {
            status_element
                .class_list()
                .remove_1("glyphicon-remove")
                .expect("Failed to remove class");
            status_element
                .class_list()
                .add_1("glyphicon-ok")
                .expect("Failed to add class");
        }
    }
}

#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_log::init_with_level(log::Level::Info).expect("error initializing log");
    log::info!("Now executing WASM code from lib.rs in pankat_wasm");

    spawn_local({
        async move {
            let id: String = "NavAndContent".to_string();
            let mut dom_updater: DomUpdater = DomUpdater::new(id.clone());

            loop {
                let location = window().location();
                let protocol = if location.protocol().unwrap() == "https:" {
                    "wss"
                } else {
                    "ws"
                };

                let host = location.host().unwrap();
                let websocket_address = format!("{protocol}://{host}/api/ws");

                match WebSocket::open(&websocket_address) {
                    Ok(ws) => {
                        ws_open(); // Move ws_open here since we have established a WebSocket connection
                        let (mut write, mut read) = ws.split();
                        use futures::SinkExt;
                        // spawn_local(async move {
                        //     loop {
                        //         gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
                        //         if write.send(Message::Text("ping".to_string())).await.is_err() {
                        //             log::warn!("Failed to send ping");
                        //             return;
                        //         }
                        //     }
                        // });

                        while let Some(result) = read.next().await {
                            match result {
                                Ok(msg) => match msg {
                                    Message::Text(message) => {
                                        match message.as_str() {
                                            "pong" => {
                                                log::info!("WS: received a pong to our ping, connection is working!");
                                            }
                                            _ => {}
                                        }
                                        log::info!("Received WS message");
                                        dom_updater.update(format!(
                                            r#"<div class=\"article\">{}</div>"#,
                                            message
                                        ));
                                    }
                                    Message::Bytes(_) => {
                                        log::warn!("Binary messages are not supported yet");
                                    }
                                },
                                Err(e) => {
                                    log::warn!("Err0r {e}");
                                    return;
                                }
                            }
                        }
                        log::info!(
                            "WebSocket disconnected, attempting to reconnect in 1 second..."
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to connect: {}", e);
                    }
                }
                ws_close();
                gloo_timers::future::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });

    Ok(())
}
