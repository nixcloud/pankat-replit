#![deny(warnings)]
use sauron::svg::attributes::*;
use sauron::svg::*;
use sauron::*;

#[test]
fn children() {
    let lines: Vec<Node<()>> = (0..5)
        .map(|_| line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]))
        .collect();
    let html = svg(vec![], vec![circle(vec![], vec![])]).with_children(lines);
    let expect = svg(
        vec![],
        vec![
            circle(vec![], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
        ],
    );
    assert_eq!(html, expect, "Should be the same");
}

#[test]
fn children_using_macro_mix() {
    let lines: Vec<Node<()>> = (0..5)
        .map(|_| line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]))
        .collect();
    let html = svg(vec![], vec![circle(vec![], vec![])]).with_children(lines);
    let expect = svg(
        vec![],
        vec![
            circle(vec![], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
            line(vec![x1(100), x2(100), y1(100), y2(200)], vec![]),
        ],
    );
    assert_eq!(html, expect, "Should be the same");
}
