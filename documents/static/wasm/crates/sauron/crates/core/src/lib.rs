#![deny(
    warnings,
    missing_docs,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces
)]
#![deny(unsafe_code)]
#![deny(clippy::all)]

//! The core components of sauron
#[macro_use]
extern crate doc_comment;

/// prelude
pub mod prelude {
    pub use crate::html;
    pub use crate::html::{
        attributes::commons::*,
        attributes::key,
        attributes::{
            attr, checked, class, classes, classes_flag, disabled, empty_attr, r#type, styles_flag,
        },
        br, comment,
        commons::*,
        hr, img, input, lazy_view_if, text,
        units::{ch, cm, deg, ex, grad, mm, ms, percent, pt, px, rad, rgb, rgba, s, turn, vh, vw},
        view_if,
    };

    pub use crate::svg;
    pub use crate::svg::attributes::commons::*;
    pub use crate::svg::attributes::special::*;
    pub use crate::svg::commons::*;
    pub use crate::svg::special::*;
    pub use crate::vdom::{
        diff, Attribute, AttributeValue, Element, EventCallback, Node, Patch, TreePath, Value,
    };

    use cfg_if::cfg_if;
    cfg_if! {if #[cfg(feature = "with-dom")] {
        pub use web_sys;
        pub use wasm_bindgen_futures;
        pub use js_sys;
        pub use wasm_bindgen;
        #[doc(hidden)]
        pub use wasm_bindgen::prelude::*;
        pub use serde_wasm_bindgen;
        pub use crate::html::events::*;
        pub use crate::dom::{Application, SkipDiff, skip_if, events, Program, document, Document, now, window, Window, Dispatch,
            AnimationFrameHandle, Component, StatefulComponent, Effects, Measurements, MountAction,
            MountTarget, Cmd, TimeoutCallbackHandle, DomAttrValue,
            stateful_component, Time,
        };
    }}
}

#[macro_use]
pub mod html;
#[macro_use]
pub mod svg;
pub mod dom;
pub mod vdom;
