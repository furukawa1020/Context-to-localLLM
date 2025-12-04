#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    render! {
        div {
            "Hello, Dioxus!"
        }
    }
}
