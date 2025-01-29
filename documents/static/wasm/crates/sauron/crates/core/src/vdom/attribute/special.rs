//! special attributes which is treated differently
//!
//!
use super::{attr, Attribute, Value};
use crate::vdom::AttributeName;

/// Special Node attributes that are treated differently
/// such as key and skip which both greatly affects the diffing algorithm

/// NOTE: this is specific to sauron framework
/// The key attribute
pub static KEY: &AttributeName = &"key";

/// NOTE: this is specific to sauron framework
/// The replace attribute
pub static REPLACE: &AttributeName = &"replace";

/// NOTE: this is specific to sauron framework
/// The skip attribute
pub static SKIP: &AttributeName = &"skip";

/// NOTE: this is specific to sauron framework
/// The skip criteria attribute
pub static SKIP_CRITERIA: &AttributeName = &"skip_criteria";

///
/// NOTE: The following attributes have special behaviour in the dom, the framework
/// need to call the specific methods to reflect the state of this attribute to the element
///
/// the value attribute
#[cfg(feature = "ensure-attr-set")]
pub static VALUE: &AttributeName = &"value";
/// the open attribute
#[cfg(feature = "ensure-attr-set")]
pub static OPEN: &AttributeName = &"open";
/// the checked attribute
#[cfg(feature = "ensure-attr-set")]
pub static CHECKED: &AttributeName = &"checked";
/// the disabled attribute
#[cfg(feature = "ensure-attr-set")]
pub static DISABLED: &AttributeName = &"disabled";

/// creates a key attribute using a formatter
/// # Examples
/// ```rust
/// use sauron::{*, html::{*, attributes::*}};
///
/// let number = 42;
/// let button:Node<()> = button([key!("content-{}", 42)], [text("Click")]);
///
/// assert_eq!(node!{<button key=format!("content-42")>"Click"</button>}, button);
/// ```
#[macro_export]
macro_rules! key {
    ( $($arg: tt)* ) => {
        $crate::html::attributes::key(format!($($arg)*))
    };
}

/// key attributes is used to match
/// old element and new element when diffing
pub fn key<V, MSG>(v: V) -> Attribute<MSG>
where
    V: Into<Value>,
{
    attr(KEY, v)
}

/// if the value is true, then the diffing of this element
/// and its descendants are skip entirely
pub fn skip<MSG>(v: bool) -> Attribute<MSG> {
    attr(SKIP, v)
}

/// if the value of this attribute of the old element and the new element is the same
/// the diffing of this element and its descentdants are skip entirely
pub fn skip_criteria<V, MSG>(v: V) -> Attribute<MSG>
where
    V: Into<Value>,
{
    attr(SKIP_CRITERIA, v.into())
}

/// if the value is true, then this node is made to replace the old
/// node it matches
pub fn replace<MSG>(v: bool) -> Attribute<MSG> {
    attr(REPLACE, v)
}
