use crate::dom::DomAttr;
use sauron::dom::Component;
use sauron::dom::DomNode;
use sauron::dom::StatefulComponent;
use sauron::prelude::*;

#[derive(Default)]
pub enum Msg {
    Click,
    ExternContMounted(DomNode),
    #[default]
    NoOp,
}

#[derive(Default)]
pub struct Button {
    /// holds the children while the external children node hasn't been mounted
    children: Vec<DomNode>,
    external_children_node: Option<DomNode>,
    cnt: i32,
}

impl Component for Button {
    type MSG = Msg;
    type XMSG = ();

    fn update(&mut self, msg: Msg) -> Effects<Msg, ()> {
        match msg {
            Msg::Click => self.cnt += 1,
            Msg::ExternContMounted(target_node) => {
                log::info!("Button: extenal container mounted...");
                target_node.append_children(self.children.drain(..).collect());
                self.external_children_node = Some(target_node);
            }
            Msg::NoOp => (),
        }
        Effects::none()
    }

    view! {
        <button on_click=|_|Msg::Click >
            <span>Hello!{text!("I'm just a button, clicked {} time(s)", self.cnt)}</span>
            <div class="external_children" on_mount=|me|Msg::ExternContMounted(me.target_node)></div>
        </button>
    }

    fn stylesheet() -> Vec<String> {
        vec![jss! {
            "button": {
              background: "#EE88E5",
              color: "light blue",
              padding: "10px 10px",
              margin: "10px 10px",
              border: 0,
              font_size: "1.5rem",
              border_radius: "5px",
            }
        }]
    }
}

impl StatefulComponent for Button {
    fn attribute_changed(&mut self, attr: DomAttr) {
        log::info!("attribute changed: {attr:?}");
    }

    /// append a child into this component
    fn append_children(&mut self, children: Vec<DomNode>) {
        if let Some(external_children_node) = self.external_children_node.as_ref() {
            log::info!("Btn ok appending..");
            external_children_node.append_children(children);
        } else {
            log::debug!(
                "Button: Just pushing to children since the external holder is not yet mounted"
            );
            self.children.extend(children);
        }
    }
    fn child_container(&self) -> Option<DomNode> {
        self.external_children_node.clone()
    }
}
